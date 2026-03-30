mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};

// ── Local helpers ────────────────────────────────────────────────────

fn node<'a>(view: &'a Value, node_id: &str) -> &'a Value {
    view["nodes"]
        .as_array()
        .expect("view nodes")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("missing node {node_id}"))
}

fn status(view: &Value, node_id: &str) -> String {
    node(view, node_id)["status"]
        .as_str()
        .unwrap_or("unknown")
        .to_lowercase()
}

fn create_id(p: &mut PipelineHarness, definition: Value) -> String {
    PipelineHarness::id(&p.create(definition))
}

/// Dispatch and complete a chain of nodes in order.
fn complete_chain(p: &mut PipelineHarness, sid: &str, nodes: &[&str]) {
    for &n in nodes {
        p.dispatch(sid);
        p.complete(sid, n, &format!("done-{n}"));
    }
}

/// Build a linear chain definition: A→B→C→...
fn linear_chain(ids: &[&str]) -> Value {
    let nodes: Vec<Value> = ids.iter().map(|id| PipelineHarness::tool_node(id)).collect();
    let edges: Vec<Value> = ids.windows(2).map(|w| PipelineHarness::edge(w[0], w[1])).collect();
    json!({ "nodes": nodes, "edges": edges })
}

// ── Test Cases ───────────────────────────────────────────────────────

#[test]
fn tc_7_01_rewind_single_leaf_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, linear_chain(&["A", "B"]));
    complete_chain(&mut p, &sid, &["A", "B"]);

    p.rewind(&sid, json!({ "nodeIds": ["B"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "completed");
    assert_eq!(status(&v, "B"), "pending");
}

#[test]
fn tc_7_02_rewind_cascades_downstream() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, linear_chain(&["A", "B", "C", "D"]));
    complete_chain(&mut p, &sid, &["A", "B", "C", "D"]);

    p.rewind(&sid, json!({ "nodeIds": ["B"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "completed");
    assert_eq!(status(&v, "B"), "pending");
    assert_eq!(status(&v, "C"), "pending");
    assert_eq!(status(&v, "D"), "pending");
}

#[test]
fn tc_7_03_rewind_root_resets_everything() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, linear_chain(&["A", "B", "C"]));
    complete_chain(&mut p, &sid, &["A", "B", "C"]);

    p.rewind(&sid, json!({ "nodeIds": ["A"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "pending");
    assert_eq!(status(&v, "B"), "pending");
    assert_eq!(status(&v, "C"), "pending");
}

#[test]
fn tc_7_04_rewind_resets_attempts_counter() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, json!({
        "nodes": [PipelineHarness::tool_node("A")],
        "edges": []
    }));

    // Dispatch and abandon → attempts=1
    p.dispatch(&sid);
    p.abandon(&sid, "A");
    // Restart → attempts=2, dispatch and abandon again
    p.restart(&sid, "A");
    p.dispatch(&sid);
    p.abandon(&sid, "A");

    p.rewind(&sid, json!({ "nodeIds": ["A"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "pending");
    assert_eq!(node(&v, "A")["attempts"], 0);
}

#[test]
fn tc_7_05_rewind_clears_output() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, json!({
        "nodes": [PipelineHarness::tool_node("A")],
        "edges": []
    }));

    p.dispatch(&sid);
    p.complete(&sid, "A", "my-output");
    let v = p.view(&sid);
    assert_eq!(node(&v, "A")["output"], "my-output");

    p.rewind(&sid, json!({ "nodeIds": ["A"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "pending");
    assert!(
        node(&v, "A")["output"].is_null()
            || node(&v, "A")["output"].as_str() == Some(""),
        "output should be null or empty after rewind"
    );
}

#[test]
fn tc_7_06_rewind_allocates_new_session_id() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, json!({
        "nodes": [PipelineHarness::tool_node("A")],
        "edges": []
    }));

    let v_before = p.view(&sid);
    let session_before = node(&v_before, "A")["sessionId"]
        .as_str()
        .expect("sessionId before")
        .to_string();

    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");
    p.rewind(&sid, json!({ "nodeIds": ["A"] }));

    let v_after = p.view(&sid);
    let session_after = node(&v_after, "A")["sessionId"]
        .as_str()
        .expect("sessionId after");
    assert_ne!(
        session_before, session_after,
        "rewind must allocate a fresh sessionId"
    );
}

#[test]
fn tc_7_07_rewind_multiple_nodes_simultaneously() {
    let (_tmp, mut p) = PipelineHarness::new();
    // A→C, B→C (two roots, one sink)
    let sid = create_id(&mut p, json!({
        "nodes": [
            PipelineHarness::tool_node("A"),
            PipelineHarness::tool_node("B"),
            PipelineHarness::tool_node("C"),
        ],
        "edges": [
            PipelineHarness::edge("A", "C"),
            PipelineHarness::edge("B", "C"),
        ]
    }));

    // Complete A and B (both roots ready), then C
    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");
    p.complete(&sid, "B", "done-B");
    p.dispatch(&sid);
    p.complete(&sid, "C", "done-C");

    p.rewind(&sid, json!({ "nodeIds": ["A", "B"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "pending");
    assert_eq!(status(&v, "B"), "pending");
    assert_eq!(status(&v, "C"), "pending", "C downstream of both A and B");
}

#[test]
fn tc_7_08_rewind_diamond_partial_reset() {
    let (_tmp, mut p) = PipelineHarness::new();
    // Diamond: A→B, A→C, B→D, C→D
    let sid = create_id(&mut p, json!({
        "nodes": [
            PipelineHarness::tool_node("A"),
            PipelineHarness::tool_node("B"),
            PipelineHarness::tool_node("C"),
            PipelineHarness::tool_node("D"),
        ],
        "edges": [
            PipelineHarness::edge("A", "B"),
            PipelineHarness::edge("A", "C"),
            PipelineHarness::edge("B", "D"),
            PipelineHarness::edge("C", "D"),
        ]
    }));

    // Complete A, then B and C in parallel, then D
    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");
    p.dispatch(&sid);
    p.complete(&sid, "B", "done-B");
    p.complete(&sid, "C", "done-C");
    p.dispatch(&sid);
    p.complete(&sid, "D", "done-D");

    // Rewind only B
    p.rewind(&sid, json!({ "nodeIds": ["B"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "completed");
    assert_eq!(status(&v, "B"), "pending");
    assert_eq!(status(&v, "C"), "completed");
    assert_eq!(status(&v, "D"), "pending", "D is downstream of B");
}

#[test]
fn tc_7_09_rewind_preserves_upstream_state() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, linear_chain(&["A", "B", "C"]));

    // Complete with distinct outputs
    p.dispatch(&sid);
    p.complete(&sid, "A", "outA");
    p.dispatch(&sid);
    p.complete(&sid, "B", "outB");
    p.dispatch(&sid);
    p.complete(&sid, "C", "outC");

    // Rewind only C (leaf)
    p.rewind(&sid, json!({ "nodeIds": ["C"] }));
    let v = p.view(&sid);
    assert_eq!(status(&v, "A"), "completed");
    assert_eq!(node(&v, "A")["output"], "outA");
    assert_eq!(status(&v, "B"), "completed");
    assert_eq!(node(&v, "B")["output"], "outB");
    assert_eq!(status(&v, "C"), "pending");
    assert!(
        node(&v, "C")["output"].is_null()
            || node(&v, "C")["output"].as_str() == Some(""),
        "C output should be cleared"
    );
}

#[test]
fn tc_7_10_validation_rewind_nonexistent_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = create_id(&mut p, json!({
        "nodes": [PipelineHarness::tool_node("A")],
        "edges": []
    }));

    let err = p.rewind_err(&sid, json!({ "nodeIds": ["nonexistent"] }));
    assert!(
        err.to_lowercase().contains("unknown")
            || err.to_lowercase().contains("not found")
            || err.to_lowercase().contains("nonexistent"),
        "Expected error about unknown node, got: {err}"
    );
}
