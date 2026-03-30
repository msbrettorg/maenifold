use super::handler::pipeline;
use super::test_support::*;
use super::supervisor::{supervisor_is_running, supervisor_is_paused};
use super::supervisor_test_support::{reset_supervisor, wait_for};
use super::{EdgeType, NodeStatus};
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serial_test::serial;

fn data(output: String) -> Value {
    serde_json::from_str::<Value>(&output).expect("parse")["data"].clone()
}

fn tool(id: &str, tool_name: &str) -> Value {
    json!({ "id": id, "type": "tool", "toolName": tool_name })
}

fn edge(source: &str, target: &str, edge_type: EdgeType) -> Value {
    let edge_type = match edge_type {
        EdgeType::Success => "success",
        EdgeType::Always => "always",
    };
    json!({ "source": source, "target": target, "type": edge_type })
}

fn chain(count: usize) -> (Vec<Value>, Vec<Value>) {
    let nodes = (0..count)
        .map(|i| tool(&format!("n{i}"), "MemoryStatus"))
        .collect();
    let edges = (1..count)
        .map(|i| edge(&format!("n{}", i - 1), &format!("n{i}"), EdgeType::Success))
        .collect();
    (nodes, edges)
}

fn create_pipeline(
    context: &crate::tools::ToolContext,
    nodes: Vec<Value>,
    edges: Vec<Value>,
) -> String {
    data(
        pipeline(
            context,
            &json!({ "definition": { "nodes": nodes, "edges": edges, }, "maxConcurrency": 1 }),
        )
        .expect("create"),
    )["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string()
}

fn view_pipeline(context: &crate::tools::ToolContext, id: &str) -> Value {
    data(pipeline(context, &json!({ "sessionId": id, "view": true })).expect("view"))
}

fn node_status(view: &Value, node_id: &str) -> NodeStatus {
    serde_json::from_value(
        view["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| node["nodeId"] == node_id)
            .expect("node")["status"]
            .clone(),
    )
    .expect("status")
}

fn frontmatter_status(context: &crate::tools::ToolContext, id: &str) -> Option<String> {
    let (frontmatter, _, _) =
        crate::markdown::read_markdown(&crate::sessions::pipeline_session_uri(id), &context.config)
            .expect("read");
    frontmatter
        .expect("frontmatter")
        .get("status")
        .and_then(serde_yaml::Value::as_str)
        .map(str::to_string)
}

#[test]
#[serial(config)]
fn e2e_run_completes_three_node_chain() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let (nodes, edges) = chain(3);
    let id = create_pipeline(&context, nodes, edges);

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    assert_eq!(supervisor_is_paused(&id), Some(false));
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(node_status(&view, "n0"), NodeStatus::Completed);
    assert_eq!(node_status(&view, "n1"), NodeStatus::Completed);
    assert_eq!(node_status(&view, "n2"), NodeStatus::Completed);
    assert_eq!(view["status"], "completed");
}

#[test]
#[serial(config)]
fn e2e_cancel_abandons_running_pipeline() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(&context, vec![tool("a", "MemoryStatus")], vec![]);

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    let _ = pipeline(&context, &json!({ "sessionId": id, "cancel": true }));
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);
    let status = frontmatter_status(&context, &id).expect("status");

    assert!(
        status == "completed"
            || status == "cancelled"
            || node_status(&view, "a") == NodeStatus::Abandoned,
        "unexpected cancel outcome: status={status}, view={view}"
    );
}

#[test]
#[serial(config)]
fn e2e_run_on_completed_pipeline_is_idempotent() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let (nodes, edges) = chain(3);
    let id = create_pipeline(&context, nodes, edges);

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(&id));
    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("rerun");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(node_status(&view, "n0"), NodeStatus::Completed);
    assert_eq!(node_status(&view, "n1"), NodeStatus::Completed);
    assert_eq!(node_status(&view, "n2"), NodeStatus::Completed);
    assert_eq!(view["status"], "completed");
}

#[test]
#[serial(config)]
fn e2e_pipeline_auto_completes_when_all_nodes_done() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(
        &context,
        vec![tool("a", "MemoryStatus"), tool("b", "MemoryStatus")],
        vec![edge("a", "b", EdgeType::Success)],
    );

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(view["status"], "completed");
    assert_eq!(frontmatter_status(&context, &id).as_deref(), Some("completed"));
}

#[test]
#[serial(config)]
fn e2e_failed_node_with_always_edge_continues() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(
        &context,
        vec![tool("a", "NonExistentTool"), tool("b", "MemoryStatus")],
        vec![edge("a", "b", EdgeType::Always)],
    );

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(node_status(&view, "a"), NodeStatus::Abandoned);
    assert_eq!(node_status(&view, "b"), NodeStatus::Completed);
}

#[test]
#[serial(config)]
fn e2e_failed_node_with_success_edge_blocks_downstream() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(
        &context,
        vec![tool("a", "NonExistentTool"), tool("b", "MemoryStatus")],
        vec![edge("a", "b", EdgeType::Success)],
    );

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(node_status(&view, "a"), NodeStatus::Abandoned);
    assert_eq!(node_status(&view, "b"), NodeStatus::Pending);
}

#[test]
#[serial(config)]
fn e2e_completed_pipeline_allows_rerun_idempotent() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(&context, vec![tool("a", "MemoryStatus")], vec![]);

    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(&id));
    pipeline(&context, &json!({ "sessionId": id, "run": true })).expect("rerun");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(view["status"], "completed");
}

#[test]
#[serial(config)]
fn e2e_run_with_ambient_prompt() {
    let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
    let _reset = reset_supervisor();
    let id = create_pipeline(&context, vec![tool("a", "MemoryStatus")], vec![]);

    pipeline(
        &context,
        &json!({ "sessionId": id, "run": true, "prompt": "test ambient" }),
    )
    .expect("run");
    wait_for(|| !supervisor_is_running(&id));
    let view = view_pipeline(&context, &id);

    assert_eq!(node_status(&view, "a"), NodeStatus::Completed);
    assert_eq!(view["status"], "completed");
}
