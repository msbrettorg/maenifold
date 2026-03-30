mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};

fn pipeline(nodes: Vec<Value>, edges: Vec<Value>) -> Value {
    json!({ "nodes": nodes, "edges": edges })
}

fn node<'a>(view: &'a Value, node_id: &str) -> &'a Value {
    view["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("missing node {node_id}"))
}

fn has_edge(view: &Value, source: &str, target: &str) -> bool {
    view["edges"].as_array().expect("edges array").iter().any(|edge| {
        edge["source"].as_str() == Some(source) && edge["target"].as_str() == Some(target)
    })
}

fn complete_ready_node(p: &mut PipelineHarness, session_id: &str, node_id: &str, output: &str) {
    p.dispatch(session_id);
    p.complete(session_id, node_id, output);
}

fn drain_and_delete(p: &mut PipelineHarness, session_id: &str) {
    loop {
        let view = p.view(session_id);
        let active: Vec<String> = view["active"]
            .as_array()
            .expect("active array")
            .iter()
            .filter_map(|node| node.as_str().map(str::to_owned))
            .collect();
        if !active.is_empty() {
            for node_id in active {
                p.complete(session_id, &node_id, &format!("cleanup-{node_id}"));
            }
        } else if view["ready"].as_array().is_some_and(|ready| !ready.is_empty()) {
            p.dispatch(session_id);
        } else {
            break;
        }
    }
    p.delete(session_id);
}

#[test]
fn tc_5_01_join_two_simple_pipelines() {
    let (_tmp, mut p) = PipelineHarness::new();
    let a = p.create(pipeline(vec![PipelineHarness::tool_node("A1")], vec![]));
    let b = p.create(pipeline(vec![PipelineHarness::tool_node("B1")], vec![]));
    let a_id = PipelineHarness::id(&a);
    let b_id = PipelineHarness::id(&b);

    complete_ready_node(&mut p, &a_id, "A1", "output-A1");
    complete_ready_node(&mut p, &b_id, "B1", "output-B1");
    let joined = p.join(
        &a_id,
        json!({
            "sourceSessionId": b_id,
            "edges": [PipelineHarness::edge("A1", "B1")]
        }),
    );

    assert_eq!(joined["pipelineId"], a_id);
    assert_eq!(joined["nodes"].as_array().expect("nodes").len(), 2);
    assert!(has_edge(&joined, "A1", "B1"));
    assert_eq!(node(&joined, "A1")["status"], "completed");
    assert_eq!(node(&joined, "B1")["status"], "completed");
    p.delete(&a_id);
}

#[test]
fn tc_5_02_join_preserves_node_states_from_both_pipelines() {
    let (_tmp, mut p) = PipelineHarness::new();
    let a_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("A1")], vec![])));
    let b_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("B1")], vec![])));

    complete_ready_node(&mut p, &a_id, "A1", "output-A1");
    complete_ready_node(&mut p, &b_id, "B1", "output-B1");
    let view = p.join(&a_id, json!({ "sourceSessionId": b_id, "edges": [] }));

    assert_eq!(node(&view, "A1")["status"], "completed");
    assert_eq!(node(&view, "A1")["output"], "output-A1");
    assert_eq!(node(&view, "B1")["status"], "completed");
    assert_eq!(node(&view, "B1")["output"], "output-B1");
    p.delete(&a_id);
}

#[test]
fn tc_5_03_join_with_multi_node_pipelines() {
    let (_tmp, mut p) = PipelineHarness::new();
    let a_id = PipelineHarness::id(&p.create(pipeline(
        vec![PipelineHarness::tool_node("A1"), PipelineHarness::tool_node("A2")],
        vec![PipelineHarness::edge("A1", "A2")],
    )));
    let b_id = PipelineHarness::id(&p.create(pipeline(
        vec![PipelineHarness::tool_node("B1"), PipelineHarness::tool_node("B2")],
        vec![PipelineHarness::edge("B1", "B2")],
    )));

    complete_ready_node(&mut p, &a_id, "A1", "a1");
    complete_ready_node(&mut p, &a_id, "A2", "a2");
    complete_ready_node(&mut p, &b_id, "B1", "b1");
    complete_ready_node(&mut p, &b_id, "B2", "b2");
    let view = p.join(
        &a_id,
        json!({
            "sourceSessionId": b_id,
            "edges": [PipelineHarness::edge("A2", "B1")]
        }),
    );

    assert_eq!(view["nodes"].as_array().expect("nodes").len(), 4);
    assert_eq!(node(&view, "A1")["status"], "completed");
    assert_eq!(node(&view, "A2")["status"], "completed");
    assert_eq!(node(&view, "B1")["status"], "completed");
    assert_eq!(node(&view, "B2")["status"], "completed");
    assert!(has_edge(&view, "A1", "A2"));
    assert!(has_edge(&view, "A2", "B1"));
    assert!(has_edge(&view, "B1", "B2"));
    p.delete(&a_id);
}

#[test]
fn tc_5_04_validation_join_rejects_conflicting_node_ids() {
    let (_tmp, mut p) = PipelineHarness::new();
    let target_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("shared-id")], vec![])));
    let source_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("shared-id")], vec![])));

    let err = p.join_err(&target_id, json!({ "sourceSessionId": source_id, "edges": [] }));

    assert!(err.contains("conflicting node IDs"));
    assert!(err.contains("shared-id"));
    drain_and_delete(&mut p, &target_id);
    drain_and_delete(&mut p, &source_id);
}

#[test]
fn tc_5_05_validation_join_rejects_if_target_has_active_nodes() {
    let (_tmp, mut p) = PipelineHarness::new();
    let target_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("A1")], vec![])));
    let source_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("B1")], vec![])));

    p.dispatch(&target_id);
    let err = p.join_err(&target_id, json!({ "sourceSessionId": source_id, "edges": [] }));

    assert!(err.contains("target pipeline"));
    assert!(err.contains("running"));
    p.abandon(&target_id, "A1");
    drain_and_delete(&mut p, &source_id);
    p.delete(&target_id);
}

#[test]
fn tc_5_06_validation_join_rejects_if_source_has_active_nodes() {
    let (_tmp, mut p) = PipelineHarness::new();
    let target_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("A1")], vec![])));
    let source_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("B1")], vec![])));

    complete_ready_node(&mut p, &target_id, "A1", "done");
    p.dispatch(&source_id);
    let err = p.join_err(&target_id, json!({ "sourceSessionId": source_id, "edges": [] }));

    assert!(err.contains("source pipeline"));
    assert!(err.contains(&source_id));
    p.abandon(&source_id, "B1");
    p.delete(&target_id);
    p.delete(&source_id);
}

#[test]
fn tc_5_07_join_creating_new_scheduling_dependencies() {
    let (_tmp, mut p) = PipelineHarness::new();
    let target_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("A1")], vec![])));
    let source_id = PipelineHarness::id(&p.create(pipeline(
        vec![PipelineHarness::tool_node("B1"), PipelineHarness::tool_node("B2")],
        vec![PipelineHarness::edge("B1", "B2")],
    )));

    complete_ready_node(&mut p, &target_id, "A1", "done-A1");
    complete_ready_node(&mut p, &source_id, "B1", "done-B1");
    let view = p.join(
        &target_id,
        json!({
            "sourceSessionId": source_id,
            "edges": [PipelineHarness::edge("A1", "B1")]
        }),
    );

    assert_eq!(node(&view, "B2")["status"], "pending");
    assert!(PipelineHarness::is_ready(&view, "B2"));
    complete_ready_node(&mut p, &target_id, "B2", "done-B2");
    p.delete(&target_id);
}

#[test]
fn tc_5_08_validation_join_that_creates_cycle() {
    let (_tmp, mut p) = PipelineHarness::new();
    let target_id = PipelineHarness::id(&p.create(pipeline(
        vec![PipelineHarness::tool_node("A1"), PipelineHarness::tool_node("A2")],
        vec![PipelineHarness::edge("A1", "A2")],
    )));
    let source_id = PipelineHarness::id(&p.create(pipeline(vec![PipelineHarness::tool_node("B1")], vec![])));

    complete_ready_node(&mut p, &target_id, "A1", "a1");
    complete_ready_node(&mut p, &target_id, "A2", "a2");
    complete_ready_node(&mut p, &source_id, "B1", "b1");
    let err = p.join_err(
        &target_id,
        json!({
            "sourceSessionId": source_id,
            "edges": [
                PipelineHarness::edge("A2", "B1"),
                PipelineHarness::edge("B1", "A1")
            ]
        }),
    );

    assert!(err.contains("cycle"));
    p.delete(&target_id);
    p.delete(&source_id);
}
