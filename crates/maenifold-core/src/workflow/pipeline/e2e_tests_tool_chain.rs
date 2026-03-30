use super::handler::pipeline;
use super::supervisor::supervisor_is_running;
use super::supervisor_test_support::{reset_supervisor, wait_for};
use super::test_support::*;
use crate::util::json::OutputFormat;
use serde_json::{json, Value};
use serial_test::serial;

fn tool(id: &str, tool_name: &str) -> Value {
    json!({ "id": id, "type": "tool", "toolName": tool_name })
}

fn tool_with_args(id: &str, tool_name: &str, tool_args: Value) -> Value {
    json!({ "id": id, "type": "tool", "toolName": tool_name, "toolArgs": tool_args })
}

fn create_pipeline_id(
    context: &crate::tools::ToolContext,
    definition: Value,
    max_concurrency: Option<u64>,
) -> String {
    let mut payload = json!({ "definition": definition });
    if let Some(value) = max_concurrency {
        payload
            .as_object_mut()
            .expect("payload object")
            .insert("maxConcurrency".to_string(), json!(value));
    }
    let result = pipeline(context, &payload).expect("create");
    serde_json::from_str::<Value>(&result).expect("parse create")["data"]["pipelineId"]
        .as_str()
        .expect("pipeline id")
        .to_string()
}

fn run_and_view(context: &crate::tools::ToolContext, pipeline_id: &str) -> Value {
    pipeline(context, &json!({ "sessionId": pipeline_id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(pipeline_id));
    let view = pipeline(context, &json!({ "sessionId": pipeline_id, "view": true })).expect("view");
    serde_json::from_str::<Value>(&view).expect("parse view")["data"].clone()
}

fn node<'a>(data: &'a Value, node_id: &str) -> &'a Value {
    data["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(node_id))
        .expect("node present")
}

#[test]
#[serial(config)]
fn e2e_single_tool_node_completes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({ "nodes": [tool("t1", "MemoryStatus")], "edges": [] }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(data["status"], "completed");
    assert_eq!(node(&data, "t1")["status"], "completed");
}

#[test]
#[serial(config)]
fn e2e_two_tool_nodes_in_chain() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({
            "nodes": [tool("a", "MemoryStatus"), tool("b", "MemoryStatus")],
            "edges": [{ "source": "a", "target": "b" }]
        }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(data["status"], "completed");
    assert_eq!(node(&data, "a")["status"], "completed");
    assert_eq!(node(&data, "b")["status"], "completed");
    assert!(node(&data, "b")["output"]
        .as_str()
        .is_some_and(|output| !output.is_empty()));
}

#[test]
#[serial(config)]
fn e2e_parallel_tool_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({
            "nodes": [tool("a", "MemoryStatus"), tool("b", "MemoryStatus")],
            "edges": []
        }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(data["status"], "completed");
    assert_eq!(node(&data, "a")["status"], "completed");
    assert_eq!(node(&data, "b")["status"], "completed");
}

#[test]
#[serial(config)]
fn e2e_tool_node_with_args() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({
            "nodes": [tool_with_args("t1", "GetHelp", json!({ "toolName": "Pipeline" }))],
            "edges": []
        }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);
    let output = node(&data, "t1")["output"].as_str().expect("tool output");

    assert_eq!(data["status"], "completed");
    assert_eq!(node(&data, "t1")["status"], "completed");
    assert!(output.contains("Pipeline"), "unexpected output: {output}");
}

#[test]
#[serial(config)]
fn e2e_tool_node_failure_abandons() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({ "nodes": [tool("t1", "NonExistentTool")], "edges": [] }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(node(&data, "t1")["status"], "abandoned");
}

#[test]
#[serial(config)]
fn e2e_diamond_dag_all_complete() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({
            "nodes": [
                tool("a", "MemoryStatus"),
                tool("b", "MemoryStatus"),
                tool("c", "MemoryStatus"),
                tool("d", "MemoryStatus")
            ],
            "edges": [
                { "source": "a", "target": "b" },
                { "source": "a", "target": "c" },
                { "source": "b", "target": "d" },
                { "source": "c", "target": "d" }
            ]
        }),
        None,
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(data["status"], "completed");
    assert_eq!(data["completed"], json!(["a", "b", "c", "d"]));
    for node_id in ["a", "b", "c", "d"] {
        assert_eq!(node(&data, node_id)["status"], "completed");
    }
}

#[test]
#[serial(config)]
fn e2e_max_concurrency_respected() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let pipeline_id = create_pipeline_id(
        &context,
        json!({
            "nodes": [
                tool("a", "MemoryStatus"),
                tool("b", "MemoryStatus"),
                tool("c", "MemoryStatus")
            ],
            "edges": []
        }),
        Some(1),
    );

    let data = run_and_view(&context, &pipeline_id);

    assert_eq!(data["status"], "completed");
    assert_eq!(data["completed"], json!(["a", "b", "c"]));
}
