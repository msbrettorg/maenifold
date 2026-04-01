use super::handler::pipeline;
use super::supervisor::supervisor_is_running;
use super::supervisor_test_support::{reset_supervisor, wait_for};
use super::test_support::*;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;

fn tool(id: &str) -> Value {
    json!({ "id": id, "type": "tool", "toolName": "MemoryStatus" })
}

fn create_and_get_id(
    context: &crate::tools::ToolContext,
    nodes: Vec<Value>,
    edges: Vec<Value>,
) -> String {
    let result = pipeline(
        context,
        &json!({ "definition": { "nodes": nodes, "edges": edges } }),
    )
    .expect("create");
    let parsed: Value = serde_json::from_str(&result).expect("parse");
    parsed["data"]["pipelineId"]
        .as_str()
        .expect("id")
        .to_string()
}

fn run_to_completion(context: &crate::tools::ToolContext, id: &str) {
    pipeline(context, &json!({ "sessionId": id, "run": true })).expect("run");
    wait_for(|| !supervisor_is_running(id));
}

fn view_pipeline(context: &crate::tools::ToolContext, id: &str) -> Value {
    let result = pipeline(context, &json!({ "sessionId": id, "view": true })).expect("view");
    serde_json::from_str::<Value>(&result).expect("parse")
}

fn node_status(data: &Value, node_id: &str) -> String {
    data["data"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(node_id) || n["id"].as_str() == Some(node_id))
        .expect("node")["status"]
        .as_str()
        .expect("status")
        .to_string()
}

fn node_ids(data: &Value) -> Vec<String> {
    data["data"]["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| {
            n["nodeId"]
                .as_str()
                .or_else(|| n["id"].as_str())
                .expect("node id")
                .to_string()
        })
        .collect()
}

#[test]
#[serial(config)]
fn e2e_graft_onto_completed_then_run_again() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(
        &context,
        vec![tool("a"), tool("b")],
        vec![json!({ "source": "a", "target": "b" })],
    );

    run_to_completion(&context, &id);
    pipeline(
        &context,
        &json!({
            "sessionId": id,
            "graft": {
                "nodes": [tool("c"), tool("d")],
                "edges": [
                    { "source": "b", "target": "c" },
                    { "source": "c", "target": "d" }
                ]
            }
        }),
    )
    .expect("graft");

    run_to_completion(&context, &id);
    let data = view_pipeline(&context, &id);
    assert_eq!(node_status(&data, "c"), "completed");
    assert_eq!(node_status(&data, "d"), "completed");
}

#[test]
#[serial(config)]
fn e2e_rewind_completed_node_then_rerun() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(
        &context,
        vec![tool("a"), tool("b")],
        vec![json!({ "source": "a", "target": "b" })],
    );

    run_to_completion(&context, &id);
    pipeline(
        &context,
        &json!({ "sessionId": id, "rewind": { "nodeIds": ["b"] } }),
    )
    .expect("rewind");

    let rewound = view_pipeline(&context, &id);
    assert_eq!(node_status(&rewound, "b"), "pending");

    run_to_completion(&context, &id);
    let data = view_pipeline(&context, &id);
    assert_eq!(node_status(&data, "b"), "completed");
}

#[test]
#[serial(config)]
fn e2e_rewind_cascades_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(
        &context,
        vec![tool("a"), tool("b"), tool("c")],
        vec![
            json!({ "source": "a", "target": "b" }),
            json!({ "source": "b", "target": "c" }),
        ],
    );

    run_to_completion(&context, &id);
    pipeline(
        &context,
        &json!({ "sessionId": id, "rewind": { "nodeIds": ["b"] } }),
    )
    .expect("rewind");

    let rewound = view_pipeline(&context, &id);
    assert_eq!(node_status(&rewound, "a"), "completed");
    assert_eq!(node_status(&rewound, "b"), "pending");
    assert_eq!(node_status(&rewound, "c"), "pending");

    run_to_completion(&context, &id);
    let data = view_pipeline(&context, &id);
    for node_id in ["a", "b", "c"] {
        assert_eq!(node_status(&data, node_id), "completed");
    }
}

#[test]
#[serial(config)]
fn e2e_rewind_preserves_sibling_in_diamond() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(
        &context,
        vec![tool("a"), tool("b"), tool("c"), tool("d")],
        vec![
            json!({ "source": "a", "target": "b" }),
            json!({ "source": "a", "target": "c" }),
            json!({ "source": "b", "target": "d" }),
            json!({ "source": "c", "target": "d" }),
        ],
    );

    run_to_completion(&context, &id);
    pipeline(
        &context,
        &json!({ "sessionId": id, "rewind": { "nodeIds": ["b"] } }),
    )
    .expect("rewind");

    let rewound = view_pipeline(&context, &id);
    assert_eq!(node_status(&rewound, "a"), "completed");
    assert_eq!(node_status(&rewound, "b"), "pending");
    assert_eq!(node_status(&rewound, "c"), "completed");
    assert_eq!(node_status(&rewound, "d"), "pending");

    run_to_completion(&context, &id);
    let data = view_pipeline(&context, &id);
    for node_id in ["a", "b", "c", "d"] {
        assert_eq!(node_status(&data, node_id), "completed");
    }
}

#[test]
#[serial(config)]
fn e2e_fork_from_completed_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(
        &context,
        vec![tool("a"), tool("b"), tool("c")],
        vec![
            json!({ "source": "a", "target": "b" }),
            json!({ "source": "b", "target": "c" }),
        ],
    );

    run_to_completion(&context, &id);
    let forked = pipeline(
        &context,
        &json!({ "sessionId": id, "fork": { "fromNodeId": "b" } }),
    )
    .expect("fork");
    let forked_id =
        serde_json::from_str::<Value>(&forked).expect("parse fork")["data"]["pipelineId"]
            .as_str()
            .expect("fork id")
            .to_string();

    let fork_view = view_pipeline(&context, &forked_id);
    let fork_nodes = node_ids(&fork_view);
    assert_eq!(fork_nodes.len(), 2);
    assert!(fork_nodes.iter().any(|n| n.ends_with("-b")));
    assert!(fork_nodes.iter().any(|n| n.ends_with("-c")));

    let original = view_pipeline(&context, &id);
    assert_eq!(node_ids(&original), vec!["a", "b", "c"]);
    for node_id in ["a", "b", "c"] {
        assert_eq!(node_status(&original, node_id), "completed");
    }
}

#[test]
#[serial(config)]
fn e2e_full_lifecycle_create_run_graft_rewind_run() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let _sup_reset = reset_supervisor();
    let id = create_and_get_id(&context, vec![tool("a")], vec![]);

    run_to_completion(&context, &id);
    pipeline(
        &context,
        &json!({
            "sessionId": id,
            "graft": {
                "nodes": [tool("b")],
                "edges": [{ "source": "a", "target": "b" }]
            }
        }),
    )
    .expect("graft");
    run_to_completion(&context, &id);

    let after_graft = view_pipeline(&context, &id);
    assert_eq!(node_status(&after_graft, "a"), "completed");
    assert_eq!(node_status(&after_graft, "b"), "completed");

    pipeline(
        &context,
        &json!({ "sessionId": id, "rewind": { "nodeIds": ["b"] } }),
    )
    .expect("rewind");
    run_to_completion(&context, &id);

    let after_rerun = view_pipeline(&context, &id);
    assert_eq!(node_status(&after_rerun, "a"), "completed");
    assert_eq!(node_status(&after_rerun, "b"), "completed");
}
