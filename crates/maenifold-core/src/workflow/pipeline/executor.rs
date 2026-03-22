use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_yaml::Value as YamlValue;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;

use super::super::runner::{self, WorkflowRunStatus};
use super::dispatch::dispatch_pipeline;
use super::session_io::{derive_pipeline_node_states, frontmatter_value, read_pipeline_session};
use super::supervisor::supervisor_wait_if_paused;
use super::update::update_pipeline_node_status;
use super::{NodeStatus, PipelineDefinition};

const DEFAULT_MAX_CONCURRENCY: usize = 8;
const MAX_CONCLUSION_CHARS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionResult {
    pub completed: usize,
    pub abandoned: usize,
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    success: bool,
    data: T,
    error: Option<EnvelopeError>,
}

#[derive(Debug, Deserialize)]
struct EnvelopeError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchResult {
    dispatched: Vec<DispatchedNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DispatchedNode {
    node_id: String,
    #[allow(dead_code)]
    workflow_id: String,
}

pub(super) async fn execute_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: &str,
    cancel: CancellationToken,
) -> Result<ExecutionResult> {
    let json_context = ToolContext::new(context.config.clone(), OutputFormat::Json);
    let (definition, max_concurrency) = load_execution_config(&json_context, session_id)?;
    let nodes_by_id = definition
        .nodes
        .into_iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let mut join_set = JoinSet::new();
    let mut completed = 0usize;
    let mut abandoned = 0usize;
    let prompt = prompt.to_string();

    loop {
        if cancel.is_cancelled() {
            break;
        }

        supervisor_wait_if_paused(session_id, &cancel).await;
        if cancel.is_cancelled() {
            break;
        }

        let dispatched = parse_dispatch_result(dispatch_pipeline(&json_context, session_id)?)?;
        for dispatch in &dispatched {
            let permit = semaphore.clone().acquire_owned().await.map_err(|_| {
                MaenifoldError::Message(
                    "ERROR: Pipeline executor semaphore unexpectedly closed".to_string(),
                )
            })?;
            let node = nodes_by_id.get(&dispatch.node_id).ok_or_else(|| {
                MaenifoldError::Message(format!(
                    "ERROR: Pipeline definition missing node '{}'",
                    dispatch.node_id
                ))
            })?;
            let node_id = dispatch.node_id.clone();
            let agent = match &node.acp_client {
                Some(client) => client.clone(),
                None => {
                    drop(permit);
                    update_pipeline_node_status(
                        &json_context,
                        session_id,
                        &node_id,
                        "abandoned",
                        None,
                    )?;
                    abandoned += 1;
                    continue;
                }
            };
            let workflow_id = node.workflow_id.clone();
            let prompt = prompt.clone();
            let ctx = ToolContext::new(context.config.clone(), context.output_format);
            join_set.spawn(async move {
                let _permit = permit;
                let result = runner::run_workflow(&ctx, &workflow_id, &agent, &prompt).await;
                (node_id, result)
            });
        }

        if dispatched.is_empty() && join_set.is_empty() {
            break;
        }

        let Some(joined) = (tokio::select! {
            biased;
            _ = cancel.cancelled() => None,
            joined = join_set.join_next(), if !join_set.is_empty() => joined,
        }) else {
            break;
        };

        let (node_id, result) = joined.map_err(|error| {
            MaenifoldError::Message(format!("ERROR: Node task failed: {error}"))
        })?;

        match classify_workflow_result(result) {
            NodeOutcome::Completed(conclusion) => {
                update_pipeline_node_status(
                    &json_context,
                    session_id,
                    &node_id,
                    "completed",
                    Some(&conclusion),
                )?;
                completed += 1;
            }
            NodeOutcome::Abandoned => {
                update_pipeline_node_status(
                    &json_context,
                    session_id,
                    &node_id,
                    "abandoned",
                    None,
                )?;
                abandoned += 1;
            }
        }
    }

    let cancelled = cancel.is_cancelled();
    if cancelled {
        join_set.abort_all();
        drop(join_set);
        abandoned += abandon_active_nodes(&json_context, session_id)?;
    }

    Ok(ExecutionResult {
        completed,
        abandoned,
        status: derive_execution_status(&json_context, session_id, cancelled)?,
    })
}

fn load_execution_config(
    context: &ToolContext,
    session_id: &str,
) -> Result<(PipelineDefinition, usize)> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let max_concurrency = frontmatter
        .get("maxConcurrency")
        .and_then(YamlValue::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_CONCURRENCY);
    Ok((definition, max_concurrency))
}

fn parse_dispatch_result(raw: String) -> Result<Vec<DispatchedNode>> {
    let response: Envelope<DispatchResult> = serde_json::from_str(&raw)?;
    if response.success {
        Ok(response.data.dispatched)
    } else {
        Err(MaenifoldError::Message(
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "ERROR: Pipeline dispatch failed".to_string()),
        ))
    }
}

enum NodeOutcome {
    Completed(String),
    Abandoned,
}

fn classify_workflow_result(result: Result<runner::WorkflowResult>) -> NodeOutcome {
    match result {
        Ok(wf) => match wf.status {
            WorkflowRunStatus::Completed => {
                let conclusion = wf.conclusion.unwrap_or_default();
                NodeOutcome::Completed(truncate_chars(&conclusion, MAX_CONCLUSION_CHARS))
            }
            WorkflowRunStatus::Failed(_) | WorkflowRunStatus::Cancelled => NodeOutcome::Abandoned,
        },
        Err(_) => NodeOutcome::Abandoned,
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn abandon_active_nodes(context: &ToolContext, session_id: &str) -> Result<usize> {
    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;
    let active_nodes = node_states
        .iter()
        .filter(|(_, state)| matches!(state.status, NodeStatus::Active))
        .map(|(node_id, _)| node_id.clone())
        .collect::<Vec<_>>();

    for node_id in &active_nodes {
        update_pipeline_node_status(context, session_id, node_id, "abandoned", None)?;
    }

    Ok(active_nodes.len())
}

fn derive_execution_status(
    context: &ToolContext,
    session_id: &str,
    cancelled: bool,
) -> Result<String> {
    if cancelled {
        return Ok("cancelled".to_string());
    }

    let (_, frontmatter, _) = read_pipeline_session(context, session_id)?;
    let node_states =
        derive_pipeline_node_states(context, &frontmatter_value(&frontmatter, "nodeStates")?)?;

    if node_states
        .values()
        .all(|state| matches!(state.status, NodeStatus::Completed))
    {
        Ok("completed".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Stuck))
    {
        Ok("stuck".to_string())
    } else if node_states
        .values()
        .any(|state| matches!(state.status, NodeStatus::Abandoned))
    {
        Ok("abandoned".to_string())
    } else {
        Ok("active".to_string())
    }
}

#[cfg(test)]
mod pipeline_executor_tests {
    use super::*;
    use crate::markdown::read_markdown;
    use crate::sessions::{pipeline_session_uri, workflow_session_uri};
    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::test_support::{
        pipeline_test_context, seed_pipeline_session_with_workflows,
    };
    use crate::workflow::pipeline::{EdgeType, NodeState, PipelineEdge, PipelineNode};
    use serde_json::{Value, json};
    use serde_yaml::Value as YamlValue;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;
    use tokio::time::{Instant, sleep};

    #[tokio::test]
    #[serial(config)]
    async fn executor_completes_single_node_pipeline() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-single";
        let workflow_session_id = "workflow-executor-single-root";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-root": { "text": "Single node completed [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("test-agent-root".to_string()),
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            )]),
            8,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 1,
                abandoned: 0,
                status: "completed".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert_eq!(node_states["root"].status, NodeStatus::Completed);
        assert_eq!(
            read_workflow_status(&context, workflow_session_id),
            Some("completed".to_string())
        );
        assert_eq!(
            read_workflow_conclusion(&context, workflow_session_id),
            Some("Single node completed [[step]]".to_string())
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_handles_node_failure() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-failure";
        let workflow_session_id = "workflow-executor-failure-root";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "failing-agent": { "error": "simulated failure" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![PipelineNode {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("failing-agent".to_string()),
                }],
                edges: vec![],
            },
            HashMap::from([(
                "root".to_string(),
                NodeState {
                    session_id: workflow_session_id.to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                },
            )]),
            8,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should handle failures");

        assert_eq!(
            result,
            ExecutionResult {
                completed: 0,
                abandoned: 1,
                status: "abandoned".to_string(),
            }
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert_eq!(node_states["root"].status, NodeStatus::Abandoned);
        assert_eq!(
            read_workflow_status(&context, workflow_session_id),
            Some("abandoned".to_string())
        );
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_respects_cancellation() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-cancel";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "delay": 5.0, "text": "A [[step]]" },
                "test-agent-b": { "delay": 5.0, "text": "B [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    PipelineNode {
                        id: "a".to_string(),
                        workflow_id: "test-workflow-a".to_string(),
                        acp_client: Some("test-agent-a".to_string()),
                    },
                    PipelineNode {
                        id: "b".to_string(),
                        workflow_id: "test-workflow-b".to_string(),
                        acp_client: Some("test-agent-b".to_string()),
                    },
                ],
                edges: vec![],
            },
            HashMap::from([
                (
                    "a".to_string(),
                    NodeState {
                        session_id: "workflow-executor-cancel-a".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                    },
                ),
                (
                    "b".to_string(),
                    NodeState {
                        session_id: "workflow-executor-cancel-b".to_string(),
                        status: NodeStatus::Pending,
                        attempts: 0,
                    },
                ),
            ]),
            2,
        );

        wait_for_stub_max(&context.config.maenifold_root, 0).await;

        let context = Arc::new(context);
        let cancel = CancellationToken::new();
        let cancel_for_task = cancel.clone();
        let task_context = context.clone();
        let handle = tokio::spawn(async move {
            execute_pipeline(
                &task_context,
                pipeline_id,
                "Execute the workflow",
                cancel_for_task,
            )
            .await
        });

        wait_for_stub_max(&context.config.maenifold_root, 2).await;
        cancel.cancel();

        let result = handle
            .await
            .expect("join executor task")
            .expect("executor should exit cleanly");

        assert_eq!(result.completed, 0);
        assert_eq!(result.abandoned, 2);
        assert_eq!(result.status, "cancelled");

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert!(matches!(node_states["a"].status, NodeStatus::Abandoned));
        assert!(matches!(node_states["b"].status, NodeStatus::Abandoned));
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_respects_max_concurrency() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-concurrency";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "delay": 0.2, "text": "A [[step]]" },
                "test-agent-b": { "delay": 0.2, "text": "B [[step]]" },
                "test-agent-c": { "delay": 0.2, "text": "C [[step]]" },
                "test-agent-d": { "delay": 0.2, "text": "D [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    pipeline_node("a", "test-workflow-a"),
                    pipeline_node("b", "test-workflow-b"),
                    pipeline_node("c", "test-workflow-a"),
                    pipeline_node("d", "test-workflow-b"),
                ],
                edges: vec![],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-concurrency-a"),
                pending_node_state("b", "workflow-executor-concurrency-b"),
                pending_node_state("c", "workflow-executor-concurrency-c"),
                pending_node_state("d", "workflow-executor-concurrency-d"),
            ]),
            2,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(result.completed, 4);
        assert_eq!(result.abandoned, 0);
        assert_eq!(result.status, "completed");
        assert_eq!(read_stub_max(&context.config.maenifold_root), 2);
    }

    #[tokio::test]
    #[serial(config)]
    async fn executor_cascades_through_dag() {
        let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
        let _path_guard = install_acpx_stub(&context.config.maenifold_root);
        let pipeline_id = "pipeline-executor-dag";

        write_workflow_definition(&context.config.assets_path, "test-workflow-a");
        write_workflow_definition(&context.config.assets_path, "test-workflow-b");

        write_stub_config(
            &context.config.maenifold_root,
            json!({
                "test-agent-a": { "text": "A complete [[step]]" },
                "test-agent-b": { "text": "B complete [[step]]" },
                "test-agent-c": { "text": "C complete [[step]]" }
            }),
        );

        seed_pipeline_session_with_workflows(
            &context,
            pipeline_id,
            PipelineDefinition {
                nodes: vec![
                    pipeline_node("a", "test-workflow-a"),
                    pipeline_node("b", "test-workflow-b"),
                    pipeline_node("c", "test-workflow-a"),
                ],
                edges: vec![
                    PipelineEdge {
                        source: "a".to_string(),
                        target: "b".to_string(),
                        edge_type: EdgeType::Success,
                    },
                    PipelineEdge {
                        source: "b".to_string(),
                        target: "c".to_string(),
                        edge_type: EdgeType::Success,
                    },
                ],
            },
            HashMap::from([
                pending_node_state("a", "workflow-executor-dag-a"),
                pending_node_state("b", "workflow-executor-dag-b"),
                pending_node_state("c", "workflow-executor-dag-c"),
            ]),
            3,
        );

        let result = execute_pipeline(
            &context,
            pipeline_id,
            "Execute the workflow",
            CancellationToken::new(),
        )
        .await
        .expect("executor should succeed");

        assert_eq!(result.completed, 3);
        assert_eq!(result.abandoned, 0);
        assert_eq!(result.status, "completed");

        let starts = read_stub_event_order(&context.config.maenifold_root);
        assert_eq!(
            starts,
            vec![
                "test-agent-a".to_string(),
                "test-agent-b".to_string(),
                "test-agent-c".to_string(),
            ]
        );

        let node_states = read_pipeline_node_states(&context, pipeline_id);
        assert!(
            node_states
                .values()
                .all(|state| matches!(state.status, NodeStatus::Completed))
        );
    }

    fn pipeline_node(id: &str, workflow_id: &str) -> PipelineNode {
        PipelineNode {
            id: id.to_string(),
            workflow_id: workflow_id.to_string(),
            acp_client: Some(format!("test-agent-{id}")),
        }
    }

    fn write_workflow_definition(assets_path: &Path, workflow_id: &str) {
        let workflows_dir = assets_path.join("workflows");
        fs::create_dir_all(&workflows_dir).expect("workflows dir");
        let definition = json!({
            "steps": [{
                "id": "step-1",
                "name": "Step 1",
                "description": "Execute [[task]]"
            }]
        });
        fs::write(
            workflows_dir.join(format!("{workflow_id}.json")),
            serde_json::to_vec_pretty(&definition).expect("workflow json"),
        )
        .expect("write workflow definition");
    }

    fn pending_node_state(id: &str, session_id: &str) -> (String, NodeState) {
        (
            id.to_string(),
            NodeState {
                session_id: session_id.to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
            },
        )
    }

    fn install_acpx_stub(root: &Path) -> PathGuard {
        let bin_dir = root.join("bin");
        fs::create_dir_all(&bin_dir).expect("create bin dir");
        let stub_path = bin_dir.join("acpx");
        fs::write(&stub_path, acpx_stub_script()).expect("write acpx stub");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&stub_path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&stub_path, permissions).expect("chmod");
        }

        prepend_path(&bin_dir)
    }

    fn acpx_stub_script() -> &'static str {
        r#"#!/usr/bin/env python3
import fcntl
import json
import os
import sys
import time

ROOT = os.getcwd()
CONFIG_PATH = os.path.join(ROOT, ".executor-stub-config.json")
STATE_PATH = os.path.join(ROOT, ".executor-stub-state.json")
LOCK_PATH = os.path.join(ROOT, ".executor-stub.lock")

def read_json(path, default):
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except Exception:
        return default

def write_state(state):
    with open(STATE_PATH, "w", encoding="utf-8") as handle:
        json.dump(state, handle)

def mutate_state(mutator):
    with open(LOCK_PATH, "a+", encoding="utf-8") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = read_json(STATE_PATH, {"current": 0, "max": 0, "events": []})
        mutator(state)
        write_state(state)
        fcntl.flock(lock, fcntl.LOCK_UN)

args = sys.argv[1:]
prompt = args[-1] if args else ""
session = ""
agent = ""
if "-s" in args:
    index = args.index("-s")
    if index + 1 < len(args):
        session = args[index + 1]
    if index > 0:
        agent = args[index - 1]

config = read_json(CONFIG_PATH, {})
behavior = config.get(session) or config.get(agent) or config.get("*") or {}

mutate_state(lambda state: (
    state.__setitem__("current", state.get("current", 0) + 1),
    state.__setitem__("max", max(state.get("max", 0), state.get("current", 0))),
    state.setdefault("events", []).append({"event": "start", "session": session, "agent": agent, "prompt": prompt})
))

try:
    delay = float(behavior.get("delay", 0))
    if delay > 0:
        time.sleep(delay)

    if "stderr" in behavior:
        print(behavior["stderr"], file=sys.stderr)

    if "error" in behavior:
        print(json.dumps({"type": "error", "message": behavior["error"]}))
    else:
        print(json.dumps({"type": "text", "text": behavior.get("text", "")}))

    sys.exit(int(behavior.get("exitCode", 0)))
finally:
    mutate_state(lambda state: (
        state.__setitem__("current", max(0, state.get("current", 0) - 1)),
        state.setdefault("events", []).append({"event": "end", "session": session})
    ))
"#
    }

    fn write_stub_config(root: &Path, config: Value) {
        fs::write(
            root.join(".executor-stub-config.json"),
            serde_json::to_vec_pretty(&config).expect("serialize stub config"),
        )
        .expect("write stub config");
    }

    async fn wait_for_stub_max(root: &Path, expected: u64) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if read_stub_max(root) >= expected {
                return;
            }
            sleep(Duration::from_millis(25)).await;
        }
        panic!("stub max concurrency never reached {expected}");
    }

    fn read_stub_state(root: &Path) -> Value {
        let state_path = root.join(".executor-stub-state.json");
        if !state_path.exists() {
            return json!({
                "current": 0,
                "max": 0,
                "events": []
            });
        }

        serde_json::from_slice(&fs::read(&state_path).expect("read stub state"))
            .expect("parse stub state")
    }

    fn read_stub_max(root: &Path) -> u64 {
        read_stub_state(root)
            .get("max")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    fn read_stub_event_order(root: &Path) -> Vec<String> {
        read_stub_state(root)
            .get("events")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|event| event.get("event").and_then(Value::as_str) == Some("start"))
            .filter_map(|event| {
                event
                    .get("agent")
                    .and_then(Value::as_str)
                    .filter(|agent| !agent.is_empty())
                    .map(str::to_string)
            })
            .collect()
    }

    fn read_pipeline_node_states(
        context: &ToolContext,
        pipeline_id: &str,
    ) -> HashMap<String, NodeState> {
        let (frontmatter, _, _) =
            read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                .expect("read pipeline");
        let frontmatter = frontmatter.expect("pipeline frontmatter");
        frontmatter_value(&frontmatter, "nodeStates").expect("node states")
    }

    fn read_workflow_status(context: &ToolContext, session_id: &str) -> Option<String> {
        let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
            .expect("read workflow");
        frontmatter
            .expect("workflow frontmatter")
            .get("status")
            .and_then(YamlValue::as_str)
            .map(str::to_string)
    }

    fn read_workflow_conclusion(context: &ToolContext, session_id: &str) -> Option<String> {
        let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
            .expect("read workflow");
        frontmatter
            .expect("workflow frontmatter")
            .get("conclusion")
            .and_then(YamlValue::as_str)
            .map(str::to_string)
    }

    fn prepend_path(path: &Path) -> PathGuard {
        let original = std::env::var_os("PATH");
        let updated = match &original {
            Some(existing) => {
                let mut parts = vec![path.to_path_buf()];
                parts.extend(std::env::split_paths(existing));
                std::env::join_paths(parts).expect("join PATH")
            }
            None => std::env::join_paths([path]).expect("join PATH"),
        };

        // SAFETY: tests serialize PATH mutation with #[serial(config)] and restore it on drop.
        unsafe {
            std::env::set_var("PATH", &updated);
        }

        PathGuard { original }
    }

    struct PathGuard {
        original: Option<OsString>,
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(path) => {
                    // SAFETY: this restores PATH to its pre-test value.
                    unsafe {
                        std::env::set_var("PATH", path);
                    }
                }
                None => {
                    // SAFETY: this restores the pre-test absence of PATH.
                    unsafe {
                        std::env::remove_var("PATH");
                    }
                }
            }
        }
    }
}
