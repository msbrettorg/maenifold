mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::json;

fn wf(id: &str) -> serde_json::Value {
    json!({ "id": id, "workflowId": "abductive-reasoning" })
}

fn err_create(p: &mut PipelineHarness, definition: serde_json::Value) -> String {
    p.pipeline_err(json!({ "definition": definition }))
}

#[test]
fn tc_1_07_reject_empty_nodes() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(&mut p, json!({ "nodes": [], "edges": [] }));
    assert!(err.contains("must contain at least one node"));
}

#[test]
fn tc_1_08_reject_duplicate_ids() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({ "nodes": [wf("dup"), wf("dup")], "edges": [] }),
    );
    assert!(err.to_lowercase().contains("duplicate node"));
}

#[test]
fn tc_1_09_reject_cycle() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [wf("a"), wf("b"), wf("c")],
            "edges": [PipelineHarness::edge("a", "b"), PipelineHarness::edge("b", "c"), PipelineHarness::edge("c", "a")]
        }),
    );
    assert!(err.to_lowercase().contains("cycle"));
}

#[test]
fn tc_1_10_reject_unknown_edge_source() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [wf("a")],
            "edges": [{ "source": "missing", "target": "a", "type": "success" }]
        }),
    );
    assert!(err.to_lowercase().contains("unknown source node"));
}

#[test]
fn tc_1_11_reject_unknown_edge_target() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [wf("a")],
            "edges": [{ "source": "a", "target": "missing", "type": "success" }]
        }),
    );
    assert!(err.to_lowercase().contains("unknown target node"));
}

#[test]
fn tc_1_12_reject_duplicate_edge() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [wf("a"), wf("b")],
            "edges": [PipelineHarness::edge("a", "b"), { "source": "a", "target": "b", "type": "always" }]
        }),
    );
    assert!(err.to_lowercase().contains("duplicate edge"));
}

#[test]
fn tc_1_13_reject_blank_workflow_id() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({ "nodes": [{ "id": "wf", "workflowId": "  " }], "edges": [] }),
    );
    assert!(err.contains("workflowId"));
}

#[test]
fn tc_1_14_reject_blank_prompt() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [{ "id": "pr", "type": "prompt", "prompt": "", "agent": "test-agent" }],
            "edges": []
        }),
    );
    assert!(err.contains("prompt"));
}

#[test]
fn tc_1_15_reject_blank_agent_on_prompt() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [{ "id": "pr", "type": "prompt", "prompt": "go", "agent": "" }],
            "edges": []
        }),
    );
    assert!(err.contains("agent"));
}

#[test]
fn tc_1_16_reject_zero_total_thoughts() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [{ "id": "st", "type": "sequentialThinking", "topic": "Analyze", "agent": "test-agent", "totalThoughts": 0 }],
            "edges": []
        }),
    );
    assert!(err.contains("totalThoughts"));
}

#[test]
fn tc_1_17_reject_blank_tool_name() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [{ "id": "tl", "type": "tool", "toolName": "  " }],
            "edges": []
        }),
    );
    assert!(err.contains("toolName"));
}

#[test]
fn tc_1_18_reject_unknown_node_type() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = err_create(
        &mut p,
        json!({
            "nodes": [{ "id": "weird", "type": "banana" }],
            "edges": []
        }),
    );
    assert!(err.to_lowercase().contains("unknown type"));
}

#[test]
fn tc_1_19_delete_while_active() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(json!({ "nodes": [wf("root")], "edges": [] }));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);

    let err = p.pipeline_err(json!({ "sessionId": id, "delete": true }));
    assert!(err.to_lowercase().contains("before deleting"));

    p.abandon(&id, "root");
    p.delete(&id);
}

#[test]
fn tc_1_20_reject_max_concurrency_zero() {
    let (_tmp, mut p) = PipelineHarness::new();
    let err = p.pipeline_err(json!({
        "definition": { "nodes": [wf("root")], "edges": [] },
        "maxConcurrency": 0
    }));
    assert!(err.contains("maxConcurrency"));
}
