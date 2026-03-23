use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::markdown::{get_memory_files, read_markdown};
use crate::sessions::pipeline_session_uri;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use super::executor;
use super::session_io::{
    frontmatter_value, persist_pipeline_state_with_status, update_workflow_node_frontmatter,
};
use super::{NodeState, NodeStatus};

static SUPERVISOR_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build pipeline supervisor runtime")
});
static RUNNING_PIPELINES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static TRACKED_PIPELINES: Lazy<Mutex<HashMap<String, TrackedPipeline>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

struct TrackedPipeline {
    started_at: Instant,
    cancel: CancellationToken,
    paused: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

pub(super) fn supervisor_run(root: &Path, session_id: &str, prompt: &str) -> Result<()> {
    let _ = supervisor_detect_orphans(root);

    {
        let guard = TRACKED_PIPELINES.lock();
        if guard.contains_key(session_id) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} is already running"
            )));
        }
    }

    let cancel = CancellationToken::new();
    let paused = Arc::new(AtomicBool::new(false));
    let started_at = Instant::now();
    let task_context = supervisor_context(root);
    let root = root.to_path_buf();
    let session_id_owned = session_id.to_string();
    let prompt_owned = prompt.to_string();
    let cancel_for_task = cancel.clone();
    let paused_for_task = paused.clone();
    let (start_tx, start_rx) = oneshot::channel();

    let handle = SUPERVISOR_RUNTIME.spawn(async move {
        if start_rx.await.is_err() {
            finish_pipeline(&session_id_owned);
            return;
        }

        let result = executor::execute_pipeline(
            &task_context,
            &session_id_owned,
            &prompt_owned,
            cancel_for_task,
        )
        .await;

        if let Err(error) = result {
            eprintln!(
                "[pipeline-supervisor] pipeline {} failed: {}",
                session_id_owned, error
            );
            if pipeline_has_non_terminal_nodes(root.as_path(), &session_id_owned) {
                mark_pipeline_stuck(root.as_path(), &session_id_owned);
            }
        }

        drop(paused_for_task);
        finish_pipeline(&session_id_owned);
    });

    TRACKED_PIPELINES.lock().insert(
        session_id.to_string(),
        TrackedPipeline {
            started_at,
            cancel,
            paused,
            handle,
        },
    );
    RUNNING_PIPELINES.lock().insert(session_id.to_string());
    let _ = start_tx.send(());

    Ok(())
}

pub(super) fn supervisor_detect_orphans(root: &Path) -> usize {
    let context = supervisor_context(root);
    let Ok(session_uris) = get_memory_files(&context.config, Some("memory://pipelines")) else {
        eprintln!("[pipeline-supervisor] failed to list pipeline sessions for orphan detection");
        return 0;
    };

    let running = RUNNING_PIPELINES.lock().clone();
    let orphan_session_ids = session_uris
        .into_iter()
        .filter_map(|uri| {
            let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
                eprintln!(
                    "[pipeline-supervisor] failed to read pipeline session {} during orphan detection",
                    uri
                );
                return None;
            };
            let Some(frontmatter) = frontmatter else {
                eprintln!(
                    "[pipeline-supervisor] pipeline session {} has invalid frontmatter during orphan detection",
                    uri
                );
                return None;
            };
            let Ok(status) = frontmatter_value::<String>(&frontmatter, "status") else {
                eprintln!(
                    "[pipeline-supervisor] pipeline session {} is missing status during orphan detection",
                    uri
                );
                return None;
            };
            if !matches!(status.as_str(), "active" | "paused") {
                return None;
            }

            let Some(session_id) = uri.split('/').next_back() else {
                return None;
            };
            if running.contains(session_id) {
                return None;
            }

            Some(session_id.to_string())
        })
        .collect::<Vec<_>>();

    for session_id in &orphan_session_ids {
        mark_pipeline_stuck(root, session_id);
    }

    orphan_session_ids.len()
}

pub(super) fn supervisor_cancel(session_id: &str) -> Result<()> {
    let tracked = TRACKED_PIPELINES.lock().remove(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;

    tracked.paused.store(false, Ordering::SeqCst);
    tracked.cancel.cancel();
    let join_result = SUPERVISOR_RUNTIME.block_on(async move { tracked.handle.await });
    RUNNING_PIPELINES.lock().remove(session_id);
    join_result.map_err(|error| {
        MaenifoldError::Message(format!(
            "ERROR: Pipeline {session_id} task join failed: {error}"
        ))
    })?;
    Ok(())
}

pub(super) fn supervisor_pause(session_id: &str) -> Result<()> {
    let guard = TRACKED_PIPELINES.lock();
    let tracked = guard.get(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;
    tracked.paused.store(true, Ordering::SeqCst);
    Ok(())
}

pub(super) fn supervisor_resume(session_id: &str) -> Result<()> {
    let guard = TRACKED_PIPELINES.lock();
    let tracked = guard.get(session_id).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Pipeline {session_id} isn't running"))
    })?;
    tracked.paused.store(false, Ordering::SeqCst);
    Ok(())
}

pub(super) fn supervisor_status(session_id: &str) -> Option<Instant> {
    let guard = TRACKED_PIPELINES.lock();
    guard.get(session_id).map(|tracked| tracked.started_at)
}

pub(super) fn supervisor_is_running(session_id: &str) -> bool {
    RUNNING_PIPELINES.lock().contains(session_id)
}

pub(super) async fn supervisor_wait_if_paused(session_id: &str, cancel: &CancellationToken) {
    loop {
        if cancel.is_cancelled() {
            return;
        }

        let paused = TRACKED_PIPELINES
            .lock()
            .get(session_id)
            .map(|tracked| tracked.paused.load(Ordering::SeqCst))
            .unwrap_or(false);
        if !paused {
            return;
        }

        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = sleep(Duration::from_millis(25)) => {}
        }
    }
}

fn finish_pipeline(session_id: &str) {
    TRACKED_PIPELINES.lock().remove(session_id);
    RUNNING_PIPELINES.lock().remove(session_id);
}

#[cfg(test)]
pub(super) fn supervisor_clear_all() {
    let tracked = {
        let mut guard = TRACKED_PIPELINES.lock();
        std::mem::take(&mut *guard)
    };
    RUNNING_PIPELINES.lock().clear();

    for (_, tracked) in tracked {
        tracked.paused.store(false, Ordering::SeqCst);
        tracked.cancel.cancel();
        let _ = SUPERVISOR_RUNTIME.block_on(async move { tracked.handle.await });
    }
}

#[cfg(test)]
pub(super) fn supervisor_is_paused(session_id: &str) -> Option<bool> {
    let guard = TRACKED_PIPELINES.lock();
    guard
        .get(session_id)
        .map(|tracked| tracked.paused.load(Ordering::SeqCst))
}

#[cfg(test)]
fn supervisor_spawn_test_task(session_id: &str) -> Result<()> {
    {
        let guard = TRACKED_PIPELINES.lock();
        if guard.contains_key(session_id) {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Pipeline {session_id} is already running"
            )));
        }
    }

    let cancel = CancellationToken::new();
    let paused = Arc::new(AtomicBool::new(false));
    let started_at = Instant::now();
    let session_id_owned = session_id.to_string();
    let cancel_for_task = cancel.clone();
    let handle = SUPERVISOR_RUNTIME.spawn(async move {
        cancel_for_task.cancelled().await;
        finish_pipeline(&session_id_owned);
    });

    TRACKED_PIPELINES.lock().insert(
        session_id.to_string(),
        TrackedPipeline {
            started_at,
            cancel,
            paused,
            handle,
        },
    );
    RUNNING_PIPELINES.lock().insert(session_id.to_string());
    Ok(())
}

fn pipeline_has_non_terminal_nodes(root: &Path, session_id: &str) -> bool {
    let context = supervisor_context(root);
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, _, _)) = read_markdown(&uri, &context.config) else {
        return false;
    };
    let Some(frontmatter) = frontmatter else {
        return false;
    };
    let Ok(node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        return false;
    };

    node_states.values().any(|state| {
        matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
        )
    })
}

fn mark_pipeline_stuck(root: &Path, session_id: &str) {
    let context = supervisor_context(root);
    let uri = pipeline_session_uri(session_id);
    let Ok((frontmatter, pipeline_content, _)) = read_markdown(&uri, &context.config) else {
        eprintln!(
            "[pipeline-supervisor] failed to read pipeline session {} while marking stuck",
            session_id
        );
        return;
    };
    let Some(mut frontmatter) = frontmatter else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} has invalid frontmatter while marking stuck",
            session_id
        );
        return;
    };
    let Ok(mut node_states) =
        frontmatter_value::<HashMap<String, NodeState>>(&frontmatter, "nodeStates")
    else {
        eprintln!(
            "[pipeline-supervisor] pipeline session {} is missing nodeStates while marking stuck",
            session_id
        );
        return;
    };

    for state in node_states.values_mut() {
        if matches!(
            state.status,
            NodeStatus::Pending | NodeStatus::Ready | NodeStatus::Active
        ) {
            state.status = NodeStatus::Stuck;
            if let Err(err) =
                update_workflow_node_frontmatter(&context, &state.session_id, "stuck", None)
            {
                eprintln!(
                    "[pipeline-supervisor] failed to update workflow {} for stuck pipeline {}: {}",
                    state.session_id, session_id, err
                );
            }
        }
    }

    if let Err(err) = persist_pipeline_state_with_status(
        &mut frontmatter,
        &node_states,
        Some("stuck"),
        &uri,
        &pipeline_content,
        &context,
    ) {
        eprintln!(
            "[pipeline-supervisor] failed to persist stuck state for {}: {}",
            session_id, err
        );
    }
}

fn supervisor_context(root: &Path) -> ToolContext {
    let memory_path = root.join("memory");
    let thinking_path = memory_path.join("thinking");
    ToolContext::new(
        Config {
            maenifold_root: root.to_path_buf(),
            memory_path: memory_path.clone(),
            assets_path: root.join("assets"),
            thinking_path: thinking_path.clone(),
            sequential_path: thinking_path.join("sequential"),
            database_path: root.join("memory.db"),
        },
        OutputFormat::Json,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::read_markdown;
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::test_support::{
        install_acpx_stub, pipeline_test_context, seed_pipeline_session_with_workflows,
        write_acpx_stub_config,
    };
    use crate::workflow::pipeline::{PipelineDefinition, PipelineNode};
    use serde_json::json;
    use serde_yaml::Value as YamlValue;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::thread;

    struct SupervisorReset;

    impl Drop for SupervisorReset {
        fn drop(&mut self) {
            supervisor_clear_all();
        }
    }

    fn reset_supervisor() -> SupervisorReset {
        supervisor_clear_all();
        SupervisorReset
    }

    fn wait_for(condition: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("condition was not met before timeout");
    }

    fn pending_node_state(id: &str, session_id: &str) -> (String, NodeState) {
        (
            id.to_string(),
            NodeState {
                session_id: session_id.to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )
    }

    #[test]
    #[serial(config)]
    fn supervisor_run_rejects_already_running_pipeline() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-supervisor-duplicate";

        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                "workflow-supervisor-duplicate-root": { "delay": 30.0, "text": "still running" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([pending_node_state(
                "root",
                "workflow-supervisor-duplicate-root",
            )]),
            1,
        );

        supervisor_run(
            context.config.maenifold_root.as_path(),
            pipeline_id,
            "ship it",
        )
        .expect("start pipeline");
        wait_for(|| supervisor_is_running(pipeline_id));

        let error = supervisor_run(
            context.config.maenifold_root.as_path(),
            pipeline_id,
            "again",
        )
        .expect_err("second run should fail");
        assert!(error.to_string().contains("already running"));

        supervisor_cancel(pipeline_id).expect("cleanup running pipeline");
    }

    #[test]
    #[serial(config)]
    fn supervisor_cancel_triggers_clean_shutdown() {
        let (_tmp, _context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let pipeline_id = "pipeline-supervisor-cancel";
        supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");
        supervisor_cancel(pipeline_id).expect("cancel pipeline");

        assert!(!supervisor_is_running(pipeline_id));
        assert!(supervisor_status(pipeline_id).is_none());
    }

    #[test]
    #[serial(config)]
    fn supervisor_is_running_reflects_actual_state() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-supervisor-running-state";

        write_acpx_stub_config(
            &context.config.maenifold_root,
            json!({
                "workflow-supervisor-running-state-root": { "delay": 0.5, "text": "done" }
            }),
        );
        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                }],
                edges: vec![],
            },
            HashMap::from([pending_node_state(
                "root",
                "workflow-supervisor-running-state-root",
            )]),
            1,
        );

        supervisor_run(context.config.maenifold_root.as_path(), pipeline_id, "run")
            .expect("start pipeline");
        wait_for(|| supervisor_is_running(pipeline_id));
        assert!(supervisor_status(pipeline_id).is_some());

        wait_for(|| !supervisor_is_running(pipeline_id));
        assert!(supervisor_status(pipeline_id).is_none());
    }

    #[test]
    #[serial(config)]
    fn supervisor_pause_and_resume_toggle_flag_and_gate_dispatch() {
        let (_tmp, _context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let pipeline_id = "pipeline-supervisor-pause";
        supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");

        supervisor_pause(pipeline_id).expect("pause pipeline");
        wait_for(|| supervisor_is_paused(pipeline_id) == Some(true));

        supervisor_resume(pipeline_id).expect("resume pipeline");
        wait_for(|| supervisor_is_paused(pipeline_id) == Some(false));
        assert!(supervisor_is_running(pipeline_id));
        supervisor_cancel(pipeline_id).expect("cleanup running pipeline");
    }

    #[test]
    #[serial(config)]
    fn supervisor_marks_pipeline_stuck_when_retries_exhausted() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let pipeline_id = "pipeline-1735689600999";
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600999-root".to_string(),
                status: NodeStatus::Active,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 1);

        mark_pipeline_stuck(context.config.maenifold_root.as_path(), pipeline_id);

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("stuck")
        );

        let (workflow_frontmatter, _, _) = read_markdown(
            &workflow_session_uri("workflow-1735689600999-root"),
            &context.config,
        )
        .expect("read workflow");
        let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("stuck")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_detect_orphans_marks_stuck() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let pipeline_id = "pipeline-1735689601000";
        let workflow_id = "workflow-1735689601000-root";
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: workflow_id.to_string(),
                status: NodeStatus::Active,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 1);

        let orphan_count = supervisor_detect_orphans(context.config.maenifold_root.as_path());

        assert_eq!(orphan_count, 1);

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("stuck")
        );

        let (workflow_frontmatter, _, _) =
            read_markdown(&workflow_session_uri(workflow_id), &context.config)
                .expect("read workflow");
        let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("stuck")
        );
    }

    #[test]
    #[serial(config)]
    fn pipeline_detect_orphans_skips_running() {
        let (_tmp, context, _config_reset) = pipeline_test_context(OutputFormat::Json);
        let _reset = reset_supervisor();
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-1735689601001";
        let workflow_id = "workflow-1735689601001-root";
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: workflow_id.to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                output: None,
            },
        )]);
        seed_pipeline_session_with_workflows(&context, pipeline_id, definition, node_states, 1);
        supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");
        wait_for(|| supervisor_is_running(pipeline_id));

        let orphan_count = supervisor_detect_orphans(context.config.maenifold_root.as_path());

        assert_eq!(orphan_count, 0);

        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        assert_eq!(
            frontmatter.get("status").and_then(YamlValue::as_str),
            Some("active")
        );

        let (workflow_frontmatter, _, _) =
            read_markdown(&workflow_session_uri(workflow_id), &context.config)
                .expect("read workflow");
        let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("pending")
        );

        supervisor_cancel(pipeline_id).expect("cleanup");
    }
}
