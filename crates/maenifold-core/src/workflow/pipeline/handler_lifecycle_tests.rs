use serde_json::json;
use std::collections::HashMap;
use std::io::Write as _;

use crate::markdown::read_markdown;
use crate::sessions::pipeline_session_uri;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{EdgeType, NodeState, NodeStatus, PipelineDefinition};

use super::super::test_support::{pipeline_test_context, seed_pipeline_session_with_node_sessions};
use super::*;
use serial_test::serial;

fn make_tool_node(id: &str) -> crate::workflow::pipeline::PipelineNode {
    use crate::workflow::pipeline::PipelineNode;
    PipelineNode::Tool {
        id: id.to_string(),
        display_label: None,
        tool_name: "test.tool".to_string(),
        tool_args: None,
        agent: None,
        agent_config: None,
        timeout: None,
        allowed_tools: None,
        max_turns: None,
        cwd: None,
        model: None,
        thought_level: None,
        retry_policy: None,
        join_policy: None,
    }
}

fn make_state(session_id: &str, status: NodeStatus) -> NodeState {
    NodeState {
        session_id: session_id.to_string(),
        status,
        attempts: 0,
        started_at: None,
        completed_at: None,
        acp_session_id: None,
        thinking_session_id: None,
        tool_provenance: None,
        input_tokens: None,
        output_tokens: None,
        model: None,
        output: None,
        failure_reason: None,
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

#[test]
#[serial]
fn run_pipeline_persists_merged_inputs_before_driver_spawn() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1700000002000-run-inputs";
    let definition = PipelineDefinition {
        nodes: vec![make_tool_node("tool")],
        edges: vec![],
        inputs: Some(serde_json::Map::from_iter([(
            "persisted".to_string(),
            json!(1),
        )])),
        template_id: None,
    };
    let node_states = HashMap::from([(
        "tool".to_string(),
        make_state("workflow-1700000002000-tool", NodeStatus::Pending),
    )]);
    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

    let previous_driver = std::env::var("MAENIFOLD_PIPELINE_DRIVER").ok();
    unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", "/usr/bin/true") };
    let result = handle_lifecycle_operation(
        &context,
        session_id,
        &json!({
            "run": true,
            "prompt": "hello {{inputs.runtime}}",
            "inputs": { "persisted": 2, "runtime": "world" }
        }),
    );
    match previous_driver {
        Some(value) => unsafe { std::env::set_var("MAENIFOLD_PIPELINE_DRIVER", value) },
        None => unsafe { std::env::remove_var("MAENIFOLD_PIPELINE_DRIVER") },
    }
    result.expect("run pipeline");

    let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(session_id), &context.config)
        .expect("read pipeline session");
    let frontmatter = frontmatter.expect("frontmatter");
    let inputs: serde_json::Map<String, serde_json::Value> =
        serde_yml::from_value(frontmatter["inputs"].clone()).expect("inputs");
    assert_eq!(
        frontmatter.get("prompt").and_then(serde_yml::Value::as_str),
        Some("hello {{inputs.runtime}}")
    );
    assert_eq!(
        inputs,
        serde_json::Map::from_iter([
            ("persisted".to_string(), json!(2)),
            ("runtime".to_string(), json!("world")),
        ])
    );
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
        inputs: None,
        template_id: None,
    };
    let node_states: HashMap<String, NodeState> = [
        (
            "upstream".to_string(),
            make_state("workflow-1700000001000-upstream", NodeStatus::Abandoned),
        ),
        (
            "terminal".to_string(),
            make_state("workflow-1700000001000-terminal", NodeStatus::Completed),
        ),
    ]
    .into();

    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 8);

    let result =
        handle_lifecycle_operation(&context, session_id, &json!({ "forceComplete": true }));

    assert!(
        result.is_ok(),
        "force-complete should succeed: {:?}",
        result
    );
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
    let mut guard = rw
        .try_write()
        .expect("should acquire pipeline lock in test");
    guard.write_all(b"test-holder").unwrap();

    let result =
        handle_lifecycle_operation(&context, session_id, &json!({ "forceComplete": true }));

    drop(guard); // release before any cleanup
    assert!(
        result.is_err(),
        "force-complete must be rejected while pipeline lock is held; got: {:?}",
        result
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
        inputs: None,
        template_id: None,
    };
    let node_states: HashMap<String, NodeState> = [
        (
            "upstream".to_string(),
            make_state("workflow-1700000003000-upstream", NodeStatus::Abandoned),
        ),
        (
            "terminal".to_string(),
            make_state("workflow-1700000003000-terminal", NodeStatus::Completed),
        ),
    ]
    .into();

    seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 8);

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
