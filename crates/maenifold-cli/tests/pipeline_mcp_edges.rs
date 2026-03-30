mod pipeline_mcp_support;

use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};

fn definition(ids: &[&str], edges: Vec<Value>) -> Value {
    json!({
        "nodes": ids.iter().map(|id| PipelineHarness::tool_node(id)).collect::<Vec<_>>(),
        "edges": edges,
    })
}

fn ready_ids(data: &Value) -> Vec<String> {
    let mut ready = data["ready"]
        .as_array()
        .expect("ready array")
        .iter()
        .map(|id| id.as_str().expect("ready node id").to_string())
        .collect::<Vec<_>>();
    ready.sort();
    ready
}

fn assert_ready(data: &Value, expected: &[&str]) {
    let mut expected = expected.iter().map(|id| id.to_string()).collect::<Vec<_>>();
    expected.sort();
    assert_eq!(ready_ids(data), expected);
}

fn node_status(view: &Value, node_id: &str) -> String {
    view["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(node_id))
        .and_then(|node| node["status"].as_str())
        .expect("node status")
        .to_string()
}

fn delete_ok(p: &mut PipelineHarness, session_id: &str) {
    assert_eq!(p.delete(session_id)["deletedSessionId"], session_id);
}

#[test]
fn tc_3_01_success_edge_completed_predecessor_unblocks_downstream() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(&["a", "b"], vec![PipelineHarness::edge("a", "b")]));
    let id = PipelineHarness::id(&created);
    assert_ready(&created, &["a"]);
    p.dispatch(&id);
    assert_eq!(node_status(&p.view(&id), "a"), "active");
    p.complete(&id, "a", "done");
    assert_ready(&p.view(&id), &["b"]);
    p.dispatch(&id);
    assert_eq!(node_status(&p.view(&id), "b"), "active");
    p.complete(&id, "b", "done");
    assert_eq!(node_status(&p.view(&id), "b"), "completed");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_02_success_edge_abandoned_predecessor_blocks_downstream() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(&["a", "b"], vec![PipelineHarness::edge("a", "b")]));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.abandon(&id, "a");
    assert!(p.dispatch(&id)["dispatched"].as_array().expect("dispatched").is_empty());
    let view = p.view(&id);
    assert_eq!(node_status(&view, "b"), "pending");
    assert!(!PipelineHarness::is_ready(&view, "b"));
    p.restart(&id, "a");
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    p.dispatch(&id);
    p.abandon(&id, "b");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_03_always_edge_completed_predecessor_unblocks_downstream() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(&["a", "b"], vec![PipelineHarness::always_edge("a", "b")]));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    assert_ready(&p.view(&id), &["b"]);
    p.dispatch(&id);
    p.complete(&id, "b", "done");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_04_always_edge_abandoned_predecessor_still_unblocks() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(&["a", "b"], vec![PipelineHarness::always_edge("a", "b")]));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.abandon(&id, "a");
    let view = p.view(&id);
    assert_eq!(node_status(&view, "b"), "pending");
    assert!(PipelineHarness::is_ready(&view, "b"));
    p.dispatch(&id);
    p.abandon(&id, "b");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_05_diamond_pattern_with_all_success_edges() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(
        &["a", "b", "c", "d"],
        vec![
            PipelineHarness::edge("a", "b"),
            PipelineHarness::edge("a", "c"),
            PipelineHarness::edge("b", "d"),
            PipelineHarness::edge("c", "d"),
        ],
    ));
    let id = PipelineHarness::id(&created);
    assert_ready(&created, &["a"]);
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    assert_ready(&p.view(&id), &["b", "c"]);
    p.dispatch(&id);
    p.complete(&id, "b", "done");
    let mid = p.view(&id);
    assert_eq!(node_status(&mid, "d"), "pending");
    assert!(!PipelineHarness::is_ready(&mid, "d"));
    p.complete(&id, "c", "done");
    assert_ready(&p.view(&id), &["d"]);
    p.dispatch(&id);
    p.complete(&id, "d", "done");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_06_diamond_with_mixed_success_and_always_edges() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(
        &["a", "b", "c", "d"],
        vec![
            PipelineHarness::edge("a", "b"),
            PipelineHarness::edge("a", "c"),
            PipelineHarness::edge("b", "d"),
            PipelineHarness::always_edge("c", "d"),
        ],
    ));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    assert_ready(&p.view(&id), &["b", "c"]);
    p.dispatch(&id);
    p.complete(&id, "b", "done");
    p.abandon(&id, "c");
    assert_ready(&p.view(&id), &["d"]);
    p.dispatch(&id);
    p.abandon(&id, "d");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_07_chain_with_abandoned_middle_downstream_blocked_on_success() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(
        &["a", "b", "c"],
        vec![PipelineHarness::edge("a", "b"), PipelineHarness::edge("b", "c")],
    ));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    assert_ready(&p.view(&id), &["b"]);
    p.dispatch(&id);
    p.abandon(&id, "b");
    let view = p.view(&id);
    assert_eq!(node_status(&view, "c"), "pending");
    assert!(!PipelineHarness::is_ready(&view, "c"));
    p.restart(&id, "b");
    p.dispatch(&id);
    p.complete(&id, "b", "done");
    p.dispatch(&id);
    p.abandon(&id, "c");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_08_default_edge_type_is_success() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(&["a", "b"], vec![PipelineHarness::bare_edge("a", "b")]));
    let id = PipelineHarness::id(&created);
    p.dispatch(&id);
    p.abandon(&id, "a");
    let view = p.view(&id);
    assert_eq!(node_status(&view, "b"), "pending");
    assert!(!PipelineHarness::is_ready(&view, "b"));
    p.restart(&id, "a");
    p.dispatch(&id);
    p.complete(&id, "a", "done");
    p.dispatch(&id);
    p.abandon(&id, "b");
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_09_wide_fan_out_one_root_five_children() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(
        &["root", "c1", "c2", "c3", "c4", "c5"],
        vec![
            PipelineHarness::edge("root", "c1"),
            PipelineHarness::edge("root", "c2"),
            PipelineHarness::edge("root", "c3"),
            PipelineHarness::edge("root", "c4"),
            PipelineHarness::edge("root", "c5"),
        ],
    ));
    let id = PipelineHarness::id(&created);
    assert_ready(&created, &["root"]);
    p.dispatch(&id);
    p.complete(&id, "root", "done");
    assert_ready(&p.view(&id), &["c1", "c2", "c3", "c4", "c5"]);
    p.dispatch(&id);
    for child in ["c1", "c2", "c3", "c4", "c5"] {
        p.abandon(&id, child);
    }
    delete_ok(&mut p, &id);
}

#[test]
fn tc_3_10_wide_fan_in_three_parents_one_sink() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(definition(
        &["p1", "p2", "p3", "sink"],
        vec![
            PipelineHarness::edge("p1", "sink"),
            PipelineHarness::edge("p2", "sink"),
            PipelineHarness::edge("p3", "sink"),
        ],
    ));
    let id = PipelineHarness::id(&created);
    assert_ready(&created, &["p1", "p2", "p3"]);
    p.dispatch(&id);
    p.complete(&id, "p1", "done");
    p.complete(&id, "p2", "done");
    let mid = p.view(&id);
    assert_eq!(node_status(&mid, "sink"), "pending");
    assert!(!PipelineHarness::is_ready(&mid, "sink"));
    p.complete(&id, "p3", "done");
    assert_ready(&p.view(&id), &["sink"]);
    p.dispatch(&id);
    p.abandon(&id, "sink");
    delete_ok(&mut p, &id);
}
