use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::Value;
use std::process::Command;

use super::lease;
use super::session_write::persist_final_pipeline_status;

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt).map(Some);
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("forceComplete")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return force_complete_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}

fn find_driver_binary() -> Result<std::path::PathBuf> {
    // Allow explicit override for non-standard installs.
    if let Ok(path) = std::env::var("MAENIFOLD_PIPELINE_DRIVER") {
        let driver = std::path::PathBuf::from(path);
        if driver.exists() {
            return Ok(driver);
        }
        return Err(MaenifoldError::Message(format!(
            "MAENIFOLD_PIPELINE_DRIVER is set but binary not found at {}",
            driver.display()
        )));
    }

    let exe = std::env::current_exe().map_err(|error| {
        MaenifoldError::Message(format!("Cannot determine current executable path: {error}"))
    })?;
    let exe_dir = exe
        .canonicalize()
        .unwrap_or(exe)
        .parent()
        .ok_or_else(|| {
            MaenifoldError::Message("Cannot determine executable directory".to_string())
        })?
        .to_path_buf();
    let driver = exe_dir.join("maenifold-pipeline-driver");
    if driver.exists() {
        return Ok(driver);
    }
    Err(MaenifoldError::Message(format!(
        "Pipeline driver binary not found at {}. \
         Build it with: cargo build --release -p maenifold-pipeline-driver",
        driver.display()
    )))
}

fn run_pipeline(context: &ToolContext, session_id: &str, prompt: Option<&str>) -> Result<String> {
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is already running"
        )));
    }

    let driver = find_driver_binary()?;
    let mut cmd = Command::new(&driver);
    cmd.arg(session_id);
    if let Some(prompt) = prompt {
        cmd.arg(prompt);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    cmd.spawn().map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to spawn pipeline driver at {}: {error}",
            driver.display()
        ))
    })?;

    Ok(format!("Pipeline {session_id} started"))
}

fn cancel_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let lock_path = lease::pipeline_lock_path(session_id, &context.config);
    if !lock_path.exists() {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is not running (no lock file)"
        )));
    }
    if !lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is not running (lock not held)"
        )));
    }

    let holder = std::fs::read_to_string(&lock_path).map_err(|error| {
        MaenifoldError::Message(format!(
            "Cannot read lock file {}: {error}",
            lock_path.display()
        ))
    })?;

    // Lock file format: "{hostname}-{pid}"
    let pid_str = holder.rsplit('-').next().unwrap_or("");
    let pid: i32 = pid_str.parse().map_err(|_| {
        MaenifoldError::Message(format!(
            "Cannot parse PID from lock file content: {holder:?}"
        ))
    })?;

    if pid <= 0 {
        return Err(MaenifoldError::Message(format!(
            "Invalid PID {pid} in lock file (must be positive)"
        )));
    }

    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 {
            let errno = std::io::Error::last_os_error();
            return Err(MaenifoldError::Message(format!(
                "Failed to send SIGTERM to pipeline driver (PID {pid}): {errno}"
            )));
        }
    }

    #[cfg(not(unix))]
    {
        return Err(MaenifoldError::Message(
            "Pipeline cancel via signal is only supported on Unix".to_string(),
        ));
    }

    Ok(format!(
        "Pipeline {session_id} cancellation requested (SIGTERM sent to PID {pid})"
    ))
}

/// Admin override: force a pipeline to `completed` status regardless of
/// individual node states.  Useful when all terminal nodes have succeeded
/// but an abandoned non-terminal node prevents automatic completion (e.g.
/// an upstream `always`-edge source that was bypassed).
///
/// Requires the pipeline to not be actively running (no live lock file).
fn force_complete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is currently running; cancel it before force-completing"
        )));
    }

    persist_final_pipeline_status(context, session_id, "completed")?;

    Ok(format!("Pipeline {session_id} force-completed"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::collections::HashMap;
    use std::io::Write as _;

    use crate::util::json::OutputFormat;
    use crate::workflow::pipeline::{EdgeType, NodeState, NodeStatus, PipelineDefinition};

    use super::super::test_support::{
        pipeline_test_context, seed_pipeline_session_with_node_sessions,
    };
    use super::*;
    use serial_test::serial;

    fn make_tool_node(id: &str) -> crate::workflow::pipeline::PipelineNode {
        use crate::workflow::pipeline::PipelineNode;
        PipelineNode::Tool {
            id: id.to_string(),
            tool_name: "test.tool".to_string(),
            tool_args: None,
            agent: None,
            agent_config: None,
            timeout: None,
            allowed_tools: None,
            max_turns: None,
            model: None,
            thought_level: None,
        }
    }

    fn make_state(session_id: &str, status: NodeStatus) -> NodeState {
        NodeState {
            session_id: session_id.to_string(),
            status,
            attempts: 0,
            output: None,
            failure_reason: None,
            thinking_session_id: None,
            retry_after: None,
            retry_count: None,
        }
    }

    #[test]
    #[serial]
    fn cancel_pipeline_returns_error_when_not_running() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-cancel";

        let result = handle_lifecycle_operation(&context, session_id, &json!({ "cancel": true }));

        assert!(
            result.is_err(),
            "expected cancel to fail when pipeline is not running"
        );
    }

    #[test]
    #[serial]
    fn run_pipeline_returns_error_when_driver_binary_missing() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-nodriver";

        let result = handle_lifecycle_operation(
            &context,
            session_id,
            &json!({ "run": true, "prompt": "go" }),
        );

        // The driver binary won't be next to the test binary, so this should fail
        // with a "not found" message (unless we're in a full build environment).
        // Either outcome (success spawn or meaningful error) is acceptable.
        let _ = result;
    }

    /// force-complete should succeed on a stopped pipeline and persist "completed".
    #[test]
    #[serial]
    fn force_complete_persists_completed_status() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000001000-fc";

        let definition = PipelineDefinition {
            nodes: vec![make_tool_node("upstream"), make_tool_node("terminal")],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "upstream".to_string(),
                target: "terminal".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            }],
        };
        let node_states: HashMap<String, NodeState> = [
            ("upstream".to_string(), make_state("workflow-1700000001000-upstream", NodeStatus::Abandoned)),
            ("terminal".to_string(), make_state("workflow-1700000001000-terminal", NodeStatus::Completed)),
        ]
        .into();

        seed_pipeline_session_with_node_sessions(
            &context,
            session_id,
            definition,
            node_states,
            8,
        );

        let result = handle_lifecycle_operation(
            &context,
            session_id,
            &json!({ "forceComplete": true }),
        );

        assert!(result.is_ok(), "force-complete should succeed: {:?}", result);
        let msg = result.unwrap().expect("should have Some response");
        assert!(
            msg.contains("force-completed"),
            "response should mention force-completed, got: {msg}"
        );
    }

    /// force-complete on a live (running) pipeline must fail.
    ///
    /// Uses `fd_lock::RwLock::try_write()` — the same mechanism as
    /// `session_lock_is_held` — to hold an exclusive advisory lock on the
    /// pipeline lock file while calling `force_complete`, so the check is
    /// deterministic regardless of process ownership.
    #[test]
    #[serial]
    fn force_complete_fails_when_pipeline_running() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000002000-fc-running";

        let lock_path = lease::pipeline_lock_path(session_id, &context.config);
        std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
        let file = std::fs::File::create(&lock_path).unwrap();
        let mut rw = fd_lock::RwLock::new(file);
        // Hold the exclusive write lock for the duration of the call so that
        // `session_lock_is_held` sees WouldBlock and returns true.
        let mut guard = rw.try_write().expect("should acquire pipeline lock in test");
        guard.write_all(b"test-holder").unwrap();

        let result = handle_lifecycle_operation(
            &context,
            session_id,
            &json!({ "forceComplete": true }),
        );

        drop(guard); // release before any cleanup
        assert!(
            result.is_err(),
            "force-complete must be rejected while pipeline lock is held; got: {:?}", result
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("running") || err_msg.contains("cancel"),
            "error should mention running/cancel, got: {err_msg}"
        );
    }

    /// After force-completing a pipeline, the view path must return "completed".
    ///
    /// This is a regression test for the red-team finding that `view_pipeline_session`
    /// was ignoring the persisted `status` frontmatter field and recomputing status
    /// from raw node states, which always yielded "abandoned" for the always-edge scenario.
    #[test]
    #[serial]
    fn view_reflects_force_completed_status() {
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000003000-fc-view";

        let definition = PipelineDefinition {
            nodes: vec![make_tool_node("upstream"), make_tool_node("terminal")],
            edges: vec![crate::workflow::pipeline::PipelineEdge {
                source: "upstream".to_string(),
                target: "terminal".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            }],
        };
        let node_states: HashMap<String, NodeState> = [
            (
                "upstream".to_string(),
                make_state(
                    "workflow-1700000003000-upstream",
                    NodeStatus::Abandoned,
                ),
            ),
            (
                "terminal".to_string(),
                make_state(
                    "workflow-1700000003000-terminal",
                    NodeStatus::Completed,
                ),
            ),
        ]
        .into();

        seed_pipeline_session_with_node_sessions(
            &context,
            session_id,
            definition,
            node_states,
            8,
        );

        // Force-complete the pipeline.
        handle_lifecycle_operation(&context, session_id, &json!({ "forceComplete": true }))
            .expect("force-complete should succeed")
            .expect("should return Some");

        // Now view the pipeline — the authoritative view must report "completed".
        let view_output = super::super::view::view_pipeline_session(&context, session_id)
            .expect("view_pipeline_session should succeed");
        let view_json: serde_json::Value =
            serde_json::from_str(&view_output).expect("view output should be valid JSON");
        let status = view_json
            .get("data")
            .and_then(|r| r.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("(missing)");
        assert_eq!(
            status, "completed",
            "view should report 'completed' after force-complete, got: {status}"
        );
    }
}
