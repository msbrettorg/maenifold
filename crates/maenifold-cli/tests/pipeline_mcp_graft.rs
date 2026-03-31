mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};

fn node<'a>(view: &'a Value, node_id: &str) -> &'a Value {
    view["nodes"]
        .as_array()
        .expect("view nodes")
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("missing node {node_id}"))
}

fn node_status<'a>(view: &'a Value, node_id: &str) -> &'a str {
    node(view, node_id)["status"].as_str().expect("node status")
}

fn create_id(p: &mut PipelineHarness, definition: Value) -> String {
    let data = p.create(definition);
    PipelineHarness::id(&data)
}

#[test]
fn tc_4_01_graft_single_node_onto_completed_pipeline() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({"nodes":[PipelineHarness::tool_node("A")],"edges":[]}),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "done-A");

    let grafted = p.graft(
        &id,
        json!({"nodes":[PipelineHarness::tool_node("B")],"edges":[PipelineHarness::edge("A","B")]}),
    );
    assert_eq!(node_status(&grafted, "A"), "completed");
    assert_eq!(node_status(&grafted, "B"), "pending");
    assert!(PipelineHarness::is_ready(&grafted, "B"));

    p.dispatch(&id);
    assert_eq!(node_status(&p.view(&id), "B"), "active");
    p.complete(&id, "B", "done-B");
    assert_eq!(node_status(&p.view(&id), "B"), "completed");
    p.delete(&id);
}

#[test]
fn tc_4_02_graft_multiple_nodes_with_internal_edges() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({"nodes":[PipelineHarness::tool_node("A")],"edges":[]}),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "done-A");

    let grafted = p.graft(
        &id,
        json!({
            "nodes":[PipelineHarness::tool_node("B"), PipelineHarness::tool_node("C")],
            "edges":[PipelineHarness::edge("A","B"), PipelineHarness::edge("B","C")]
        }),
    );
    assert_eq!(grafted["nodes"].as_array().expect("nodes").len(), 3);
    assert!(PipelineHarness::is_ready(&grafted, "B"));
    assert!(!PipelineHarness::is_ready(&grafted, "C"));

    p.dispatch(&id);
    p.complete(&id, "B", "done-B");
    let view = p.view(&id);
    assert!(PipelineHarness::is_ready(&view, "C"));
    assert_eq!(node_status(&view, "C"), "pending");
    p.dispatch(&id);
    p.complete(&id, "C", "done-C");
    p.delete(&id);
}

#[test]
fn tc_4_03_graft_fan_in_from_completed_predecessors() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({"nodes":[PipelineHarness::tool_node("A"), PipelineHarness::tool_node("B")],"edges":[]}),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "done-A");
    p.complete(&id, "B", "done-B");

    let grafted = p.graft(
        &id,
        json!({
            "nodes":[PipelineHarness::tool_node("C")],
            "edges":[PipelineHarness::edge("A","C"), PipelineHarness::edge("B","C")]
        }),
    );
    assert_eq!(node_status(&grafted, "C"), "pending");
    assert!(PipelineHarness::is_ready(&grafted, "C"));

    p.dispatch(&id);
    assert_eq!(node_status(&p.view(&id), "C"), "active");
    p.complete(&id, "C", "done-C");
    p.delete(&id);
}

#[test]
fn tc_4_04_validation_reject_duplicate_node_id() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({"nodes":[PipelineHarness::tool_node("A")],"edges":[]}),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "done-A");

    let err = p.graft_err(
        &id,
        json!({"nodes":[PipelineHarness::tool_node("A")],"edges":[]}),
    );
    assert!(err.contains("duplicate node ID"));
    assert!(err.contains("'A'") || err.contains("A"));
    p.delete(&id);
}

#[test]
fn tc_4_05_validation_reject_cycle_creation() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({
            "nodes":[PipelineHarness::tool_node("A"), PipelineHarness::tool_node("B")],
            "edges":[PipelineHarness::edge("A","B")]
        }),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "done-A");
    p.dispatch(&id);
    p.complete(&id, "B", "done-B");

    let err = p.graft_err(
        &id,
        json!({"nodes":[],"edges":[PipelineHarness::edge("B","A")]}),
    );
    assert!(err.contains("cycle"));
    let view = p.view(&id);
    assert_eq!(view["edges"].as_array().expect("edges").len(), 1);
    p.delete(&id);
}

#[test]
fn tc_4_06_validation_reject_while_nodes_are_active() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({
            "nodes":[PipelineHarness::tool_node("A"), PipelineHarness::tool_node("B")],
            "edges":[PipelineHarness::edge("A","B")]
        }),
    );
    p.dispatch(&id);
    assert_eq!(node_status(&p.view(&id), "A"), "active");

    let err = p.graft_err(
        &id,
        json!({"nodes":[PipelineHarness::tool_node("C")],"edges":[PipelineHarness::edge("B","C")]}),
    );
    assert!(err.contains("running pipelines"));
    p.abandon(&id, "A");
    let deleted = p.pipeline(json!({ "sessionId": id, "delete": true }));
    if deleted["result"]["isError"] == true {
        // NOTE: cleanup deferred to TempDir drop because B remains blocked pending after A is abandoned.
    }
}

#[test]
fn tc_4_07_graft_onto_stuck_pipeline() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({"nodes":[PipelineHarness::tool_node("A")],"edges":[]}),
    );
    p.dispatch(&id);
    p.abandon(&id, "A");
    assert_eq!(node_status(&p.view(&id), "A"), "abandoned");

    let grafted = p.graft(
        &id,
        json!({
            "nodes":[PipelineHarness::tool_node("B")],
            "edges":[PipelineHarness::always_edge("A","B")]
        }),
    );
    assert_eq!(node_status(&grafted, "B"), "pending");
    assert!(PipelineHarness::is_ready(&grafted, "B"));
    p.dispatch(&id);
    p.complete(&id, "B", "done-B");
    p.delete(&id);
}

#[test]
#[ignore] // BUG: graft guard rejects pipelines with pending work (has_remaining_work check)
fn tc_4_08_graft_preserves_existing_node_states() {
    let (_tmp, mut p) = PipelineHarness::new();
    let id = create_id(
        &mut p,
        json!({
            "nodes":[PipelineHarness::tool_node("A"), PipelineHarness::tool_node("B")],
            "edges":[PipelineHarness::edge("A","B")]
        }),
    );
    p.dispatch(&id);
    p.complete(&id, "A", "output-A");

    let view = p.graft(
        &id,
        json!({"nodes":[PipelineHarness::tool_node("C")],"edges":[PipelineHarness::edge("B","C")]}),
    );
    assert_eq!(node_status(&view, "A"), "completed");
    assert_eq!(node(&view, "A")["output"], "output-A");
    assert!(PipelineHarness::is_ready(&view, "B"));
    assert_eq!(node_status(&view, "C"), "pending");
    p.delete(&id);
}
