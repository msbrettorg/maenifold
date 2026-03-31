use super::handler::pipeline;
use super::session_io::frontmatter_value;
use super::test_support::*;
use super::update::update_pipeline_node_status;
use super::{EdgeType, NodeState, NodeStatus, PipelineDefinition, PipelineEdge, PipelineNode};
use crate::markdown::read_markdown;
use crate::sessions::{pipeline_session_uri, workflow_session_uri};
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;

#[rustfmt::skip]
fn node_state(id: &str, status: NodeStatus, attempts: u32) -> NodeState {
    NodeState { session_id: id.to_string(), status, attempts, thinking_session_id: None, output: None }
}

#[rustfmt::skip]
fn response_data(output: String) -> Value { serde_json::from_str::<Value>(&output).expect("parse")["data"].clone() }

#[rustfmt::skip]
fn workflow_node(id: &str, workflow_id: &str) -> PipelineNode {
    PipelineNode::Workflow { id: id.to_string(), workflow_id: workflow_id.to_string(), acp_client: None, prompt: None, timeout: None }
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s01_s02_complete_active_node_sets_conclusion_and_unblocks_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602100", HashMap::from([
        ("root".into(), node_state("workflow-1735689602100-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602100-child", NodeStatus::Pending, 0)),
    ]), 8);
    let view = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602100","nodeId":"root","status":"completed","conclusion":"done [[pipeline]]"})).expect("complete"));
    assert_eq!(view["status"], "active");
    assert_eq!(view["ready"], json!(["child"]));
    assert_eq!(view["completed"], json!(["root"]));
    let (frontmatter, _, _) = read_markdown(&workflow_session_uri("workflow-1735689602100-root"), &context.config).expect("read root");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(frontmatter.get("status").and_then(serde_yaml::Value::as_str), Some("completed"));
    assert_eq!(frontmatter.get("conclusion").and_then(serde_yaml::Value::as_str), Some("done [[pipeline]]"));
}

// BUG: status-conclusion-truncation — update_pipeline_node_status persists conclusions verbatim without 4096-char cap
#[rustfmt::skip]
#[test]
#[ignore]
#[serial(config)]
fn s03_s16_long_conclusion_is_truncated_to_4096_chars() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602101", HashMap::from([
        ("root".into(), node_state("workflow-1735689602101-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602101-child", NodeStatus::Pending, 0)),
    ]), 8);
    let long = format!("{}[[pipeline]]", "x".repeat(5000));
    pipeline(&context, &json!({"sessionId":"pipeline-1735689602101","nodeId":"root","status":"completed","conclusion":long})).expect("complete");
    let (frontmatter, _, _) = read_markdown(&workflow_session_uri("workflow-1735689602101-root"), &context.config).expect("read");
    let fm = frontmatter.expect("frontmatter");
    let stored = fm.get("conclusion").and_then(serde_yaml::Value::as_str).expect("conclusion");
    assert_eq!(stored.chars().count(), 4096);
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s04_s05_abandon_blocks_success_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let view = response_data(pipeline(&context, &json!({"definition":{"nodes":[
        {"id":"root","workflowId":"test-workflow-a"},{"id":"child","workflowId":"test-workflow-b"}],
        "edges":[{"source":"root","target":"child","type":"success"}]}})).expect("create"));
    let id = view["pipelineId"].as_str().expect("id").to_string();
    let dispatch = response_data(pipeline(&context, &json!({"sessionId":id,"dispatch":true})).expect("dispatch"));
    assert_eq!(dispatch["active"], json!(["root"]));
    let after = response_data(pipeline(&context, &json!({"sessionId":id,"nodeId":"root","status":"abandoned"})).expect("abandon"));
    assert_eq!(after["ready"], json!([]));
    assert_eq!(after["active"], json!([]));
    assert_eq!(after["completed"], json!([]));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s06_abandon_always_edge_activates_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(&context, "pipeline-1735689602102", PipelineDefinition {
        nodes: vec![workflow_node("root", "test-workflow-a"), workflow_node("cleanup", "test-workflow-b")],
        edges: vec![PipelineEdge { source: "root".into(), target: "cleanup".into(), edge_type: EdgeType::Always, mapping: None }],
    }, HashMap::from([
        ("root".into(), node_state("workflow-1735689602102-root", NodeStatus::Active, 0)),
        ("cleanup".into(), node_state("workflow-1735689602102-cleanup", NodeStatus::Pending, 0)),
    ]), 8);
    let after = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602102","nodeId":"root","status":"abandoned"})).expect("abandon"));
    assert_eq!(after["ready"], json!(["cleanup"]));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s07_s08_complete_requires_nonempty_conclusion() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602103", HashMap::from([
        ("root".into(), node_state("workflow-1735689602103-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602103-child", NodeStatus::Pending, 0)),
    ]), 8);
    for conclusion in [None, Some("   ")] {
        let error = pipeline(&context, &json!({"sessionId":"pipeline-1735689602103","nodeId":"root","status":"completed","conclusion":conclusion})).expect_err("missing conclusion");
        assert!(error.to_string().contains("Conclusion required"), "{error}");
    }
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s09_status_update_non_active_node_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602104", HashMap::from([
        ("root".into(), node_state("workflow-1735689602104-root", NodeStatus::Pending, 0)),
        ("child".into(), node_state("workflow-1735689602104-child", NodeStatus::Pending, 0)),
    ]), 8);
    let error = pipeline(&context, &json!({"sessionId":"pipeline-1735689602104","nodeId":"root","status":"completed","conclusion":"x [[pipeline]]"})).expect_err("inactive");
    assert!(error.to_string().contains("must be active to transition to completed"));
}

// Tests internal status validation directly — handler-level coverage exists in handler_tests_state_matrix
// These tests intentionally call update_pipeline_node_status directly.
#[rustfmt::skip]
#[test]
#[serial(config)]
fn s10_unsupported_status_returns_error() {
    // status validation only fires after the Active guard — node must be Active
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602110", HashMap::from([
        ("root".into(), node_state("workflow-1735689602110-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602110-child", NodeStatus::Pending, 0)),
    ]), 8);
    let err = update_pipeline_node_status(&context, "pipeline-1735689602110", "root", "invalid_status", None, None).expect_err("unsupported");
    assert!(err.to_string().contains("Unsupported pipeline node status"));
}

// Tests internal status validation directly — handler-level coverage exists in handler_tests_state_matrix
// These tests intentionally call update_pipeline_node_status directly.
#[rustfmt::skip]
#[test]
#[serial(config)]
fn s11_status_update_nonexistent_node_returns_error() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602111", HashMap::from([
        ("root".into(), node_state("workflow-1735689602111-root", NodeStatus::Pending, 0)),
        ("child".into(), node_state("workflow-1735689602111-child", NodeStatus::Pending, 0)),
    ]), 8);
    let err = update_pipeline_node_status(&context, "pipeline-1735689602111", "ghost", "completed", Some("x [[pipeline]]"), None).expect_err("missing node");
    assert!(err.to_string().contains("missing node 'ghost'"));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s12_all_nodes_completed_marks_pipeline_completed() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602105", HashMap::from([
        ("root".into(), node_state("workflow-1735689602105-root", NodeStatus::Completed, 1)),
        ("child".into(), node_state("workflow-1735689602105-child", NodeStatus::Active, 1)),
    ]), 8);
    let after = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602105","nodeId":"child","status":"completed","conclusion":"child [[pipeline]]"})).expect("complete"));
    assert_eq!(after["status"], "completed");
    assert_eq!(after["completed"], json!(["root", "child"]));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s13_s14_abandoned_branch_leaves_active_pipeline_with_no_ready_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602106", HashMap::from([
        ("root".into(), node_state("workflow-1735689602106-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602106-child", NodeStatus::Pending, 0)),
    ]), 8);
    let after = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602106","nodeId":"root","status":"abandoned"})).expect("abandon"));
    assert_eq!(after["status"], "active");
    assert_eq!(after["ready"], json!([]));
    assert_eq!(after["active"], json!([]));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s15_double_complete_same_node_fails_on_second_attempt() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_dispatch_pipeline(&context, "pipeline-1735689602107", HashMap::from([
        ("root".into(), node_state("workflow-1735689602107-root", NodeStatus::Active, 1)),
        ("child".into(), node_state("workflow-1735689602107-child", NodeStatus::Pending, 0)),
    ]), 8);
    pipeline(&context, &json!({"sessionId":"pipeline-1735689602107","nodeId":"root","status":"completed","conclusion":"first [[pipeline]]"})).expect("first");
    let err = pipeline(&context, &json!({"sessionId":"pipeline-1735689602107","nodeId":"root","status":"completed","conclusion":"second [[pipeline]]"})).expect_err("second");
    assert!(err.to_string().contains("must be active to transition to completed"));
}

#[rustfmt::skip]
#[test]
#[serial(config)]
fn s17_complete_diamond_branches_before_sink_becomes_ready() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    seed_pipeline_session_with_workflows(&context, "pipeline-1735689602108", PipelineDefinition {
        nodes: vec![workflow_node("left", "test-workflow-a"), workflow_node("right", "test-workflow-a"), workflow_node("sink", "test-workflow-b")],
        edges: vec![
            PipelineEdge { source: "left".into(), target: "sink".into(), edge_type: EdgeType::Success, mapping: None },
            PipelineEdge { source: "right".into(), target: "sink".into(), edge_type: EdgeType::Success, mapping: None },
        ],
    }, HashMap::from([
        ("left".into(), node_state("workflow-1735689602108-left", NodeStatus::Active, 1)),
        ("right".into(), node_state("workflow-1735689602108-right", NodeStatus::Active, 1)),
        ("sink".into(), node_state("workflow-1735689602108-sink", NodeStatus::Pending, 0)),
    ]), 8);
    let first = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602108","nodeId":"left","status":"completed","conclusion":"left [[pipeline]]"})).expect("left"));
    assert_eq!(first["ready"], json!([]));
    let second = response_data(pipeline(&context, &json!({"sessionId":"pipeline-1735689602108","nodeId":"right","status":"completed","conclusion":"right [[pipeline]]"})).expect("right"));
    assert_eq!(second["ready"], json!(["sink"]));
    let (frontmatter, _, _) = read_markdown(&pipeline_session_uri("pipeline-1735689602108"), &context.config).expect("read pipeline");
    let node_states: HashMap<String, NodeState> = frontmatter_value(&frontmatter.expect("frontmatter"), "nodeStates").expect("states");
    assert_eq!(node_states["left"].status, NodeStatus::Completed);
    assert_eq!(node_states["right"].status, NodeStatus::Completed);
    assert_eq!(node_states["sink"].status, NodeStatus::Pending);
}
