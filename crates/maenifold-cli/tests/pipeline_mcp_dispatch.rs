mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::json;
fn single_node(node: serde_json::Value) -> serde_json::Value {
    json!({ "nodes": [node], "edges": [] })
}
fn workflow_node(id: &str) -> serde_json::Value {
    json!({ "id": id, "workflowId": "abductive-reasoning" })
}
fn prompt_node(id: &str) -> serde_json::Value {
    json!({ "id": id, "type": "prompt", "prompt": "Test prompt", "agent": "test-agent" })
}
fn thinking_node(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "sequentialThinking",
        "topic": "Test topic",
        "agent": "test-agent",
        "totalThoughts": 3
    })
}
fn node<'a>(view: &'a serde_json::Value, node_id: &str) -> &'a serde_json::Value {
    view["nodes"]
        .as_array()
        .expect("view nodes array")
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("missing node {node_id}"))
}
fn status(view: &serde_json::Value, node_id: &str) -> String {
    node(view, node_id)["status"].as_str().expect("node status").to_string()
}
fn active_ids(data: &serde_json::Value) -> Vec<&str> {
    data["active"]
        .as_array()
        .expect("active array")
        .iter()
        .map(|v| v.as_str().expect("active node id"))
        .collect()
}
fn dispatch_complete_cycle(
    node_def: serde_json::Value,
    node_id: &str,
    kind: Option<&str>,
    conclusion: &str,
) {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(node_def));
    let session_id = PipelineHarness::id(&create);
    assert!(PipelineHarness::is_ready(&create, node_id));
    let dispatch = p.dispatch(&session_id);
    assert_eq!(dispatch["dispatched"][0]["nodeId"], node_id);
    if let Some(kind) = kind {
        assert_eq!(dispatch["dispatched"][0]["type"], kind);
    } else {
        assert_eq!(dispatch["dispatched"][0]["workflowId"], "abductive-reasoning");
    }
    assert_eq!(active_ids(&dispatch), vec![node_id]);
    p.complete(&session_id, node_id, conclusion);
    let view = p.view(&session_id);
    assert_eq!(status(&view, node_id), "completed");
    assert_eq!(node(&view, node_id)["output"], conclusion);
    p.delete(&session_id);
}

#[test]
fn tc_2_01_workflow_node_dispatch_complete_cycle() {
    dispatch_complete_cycle(
        workflow_node("wf1"),
        "wf1",
        None,
        "Workflow completed successfully",
    );
}

#[test]
fn tc_2_02_prompt_node_dispatch_complete() {
    dispatch_complete_cycle(
        prompt_node("pr1"),
        "pr1",
        Some("prompt"),
        "Prompt completed successfully",
    );
}

#[test]
fn tc_2_03_sequential_thinking_node_dispatch_complete() {
    dispatch_complete_cycle(
        thinking_node("st1"),
        "st1",
        Some("sequentialThinking"),
        "Sequential thinking completed successfully",
    );
}

#[test]
fn tc_2_04_tool_node_dispatch_complete() {
    dispatch_complete_cycle(
        PipelineHarness::tool_node("tl1"),
        "tl1",
        Some("tool"),
        "Tool completed successfully",
    );
}

#[test]
fn tc_2_05_abandon_a_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(PipelineHarness::tool_node("tl1")));
    let session_id = PipelineHarness::id(&create);
    p.dispatch(&session_id);
    p.abandon(&session_id, "tl1");
    let view = p.view(&session_id);
    assert_eq!(status(&view, "tl1"), "abandoned");
    assert!(node(&view, "tl1")["output"].is_null());
    p.delete(&session_id);
}

#[test]
fn tc_2_06_complete_requires_non_empty_conclusion() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(PipelineHarness::tool_node("tl1")));
    let session_id = PipelineHarness::id(&create);
    p.dispatch(&session_id);
    let err = p.pipeline_err(json!({
        "sessionId": session_id,
        "nodeId": "tl1",
        "status": "completed",
        "conclusion": ""
    }));
    assert!(err.contains("Conclusion required"));
    p.abandon(&session_id, "tl1");
    p.delete(&session_id);
}

#[test]
fn tc_2_07_complete_requires_active_status() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(PipelineHarness::tool_node("tl1")));
    let session_id = PipelineHarness::id(&create);
    let err = p.pipeline_err(json!({
        "sessionId": session_id,
        "nodeId": "tl1",
        "status": "completed",
        "conclusion": "should fail"
    }));
    assert!(err.contains("must be active"));
    p.dispatch(&session_id);
    p.abandon(&session_id, "tl1");
    p.delete(&session_id);
}

#[test]
fn tc_2_08_abandon_requires_active_status() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(PipelineHarness::tool_node("tl1")));
    let session_id = PipelineHarness::id(&create);
    let err = p.pipeline_err(json!({
        "sessionId": session_id,
        "nodeId": "tl1",
        "status": "abandoned"
    }));
    assert!(err.contains("must be active"));
    p.dispatch(&session_id);
    p.abandon(&session_id, "tl1");
    p.delete(&session_id);
}

#[test]
fn tc_2_09_linear_chain_predecessor_context_flows_downstream() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(json!({
        "nodes": [
            PipelineHarness::tool_node("a"),
            PipelineHarness::tool_node("b"),
            PipelineHarness::tool_node("c")
        ],
        "edges": [
            PipelineHarness::edge("a", "b"),
            PipelineHarness::edge("b", "c")
        ]
    }));
    let session_id = PipelineHarness::id(&create);
    assert_eq!(create["ready"], json!(["a"]));
    assert_eq!(p.dispatch(&session_id)["dispatched"][0]["nodeId"], "a");
    p.complete(&session_id, "a", "Output from A");
    let after_a = p.view(&session_id);
    assert!(PipelineHarness::is_ready(&after_a, "b"));
    assert!(!PipelineHarness::is_ready(&after_a, "c"));
    assert_eq!(p.dispatch(&session_id)["dispatched"][0]["nodeId"], "b");
    p.complete(&session_id, "b", "Output from B");
    let after_b = p.view(&session_id);
    assert!(PipelineHarness::is_ready(&after_b, "c"));
    assert_eq!(p.dispatch(&session_id)["dispatched"][0]["nodeId"], "c");
    p.complete(&session_id, "c", "Output from C");
    let final_view = p.view(&session_id);
    assert_eq!(final_view["status"], "completed");
    assert_eq!(PipelineHarness::count_status(&final_view, "completed"), 3);
    p.delete(&session_id);
}

#[test]
fn tc_2_10_parallel_roots_multiple_nodes_ready_at_once() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(json!({
        "nodes": [
            PipelineHarness::tool_node("a"),
            PipelineHarness::tool_node("b"),
            PipelineHarness::tool_node("c")
        ],
        "edges": []
    }));
    let session_id = PipelineHarness::id(&create);
    assert!(PipelineHarness::is_ready(&create, "a"));
    assert!(PipelineHarness::is_ready(&create, "b"));
    assert!(PipelineHarness::is_ready(&create, "c"));
    let dispatch = p.dispatch(&session_id);
    assert_eq!(dispatch["dispatched"].as_array().expect("dispatched").len(), 3);
    assert_eq!(active_ids(&dispatch).len(), 3);
    for id in ["a", "b", "c"] {
        p.complete(&session_id, id, &format!("Output from {id}"));
    }
    let view = p.view(&session_id);
    assert_eq!(view["status"], "completed");
    assert_eq!(PipelineHarness::count_status(&view, "completed"), 3);
    p.delete(&session_id);
}

#[test]
fn tc_2_11_max_concurrency_limits_dispatch_count() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create_with_concurrency(json!({
        "nodes": [
            PipelineHarness::tool_node("a"),
            PipelineHarness::tool_node("b"),
            PipelineHarness::tool_node("c"),
            PipelineHarness::tool_node("d")
        ],
        "edges": []
    }), 2);
    let session_id = PipelineHarness::id(&create);
    let first = p.dispatch(&session_id);
    let first_ids: Vec<String> = first["dispatched"]
        .as_array()
        .expect("first dispatch")
        .iter()
        .map(|n| n["nodeId"].as_str().expect("node id").to_string())
        .collect();
    assert_eq!(first_ids.len(), 2);
    assert_eq!(active_ids(&first).len(), 2);
    p.complete(&session_id, &first_ids[0], "done one");
    let second = p.dispatch(&session_id);
    assert_eq!(second["dispatched"].as_array().expect("second dispatch").len(), 1);
    assert_eq!(active_ids(&second).len(), 2);
    let second_new = second["dispatched"][0]["nodeId"].as_str().expect("new node");
    p.complete(&session_id, &first_ids[1], "done two");
    p.complete(&session_id, second_new, "done three");
    let third = p.dispatch(&session_id);
    assert_eq!(third["dispatched"].as_array().expect("third dispatch").len(), 1);
    p.complete(
        &session_id,
        third["dispatched"][0]["nodeId"].as_str().expect("final node"),
        "done four",
    );
    let view = p.view(&session_id);
    assert_eq!(view["status"], "completed");
    p.delete(&session_id);
}

#[test]
fn tc_2_12_unsupported_status_value_error() {
    let (_tmp, mut p) = PipelineHarness::new();
    let create = p.create(single_node(PipelineHarness::tool_node("tl1")));
    let session_id = PipelineHarness::id(&create);
    p.dispatch(&session_id);
    let err = p.pipeline_err(json!({
        "sessionId": session_id,
        "nodeId": "tl1",
        "status": "stuck"
    }));
    assert!(
        err.contains("Unsupported pipeline node status")
            || err.contains("Invalid parameter combination")
    );
    p.abandon(&session_id, "tl1");
    p.delete(&session_id);
}
