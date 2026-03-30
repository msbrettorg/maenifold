mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::json;
use std::{collections::BTreeSet, fs, path::Path};

fn chain() -> serde_json::Value {
    let nodes = ["A", "B", "C"]
        .into_iter()
        .map(PipelineHarness::tool_node)
        .collect::<Vec<_>>();
    json!({
        "nodes": nodes,
        "edges": [
            PipelineHarness::edge("A", "B"),
            PipelineHarness::edge("B", "C"),
        ]
    })
}

fn single(id: &str) -> serde_json::Value {
    json!({ "nodes": [PipelineHarness::tool_node(id)], "edges": [] })
}

fn diamond() -> serde_json::Value {
    let nodes = ["A", "B", "C", "D"]
        .into_iter()
        .map(PipelineHarness::tool_node)
        .collect::<Vec<_>>();
    json!({
        "nodes": nodes,
        "edges": [
            PipelineHarness::edge("A", "B"),
            PipelineHarness::edge("A", "C"),
            PipelineHarness::edge("B", "D"),
            PipelineHarness::edge("C", "D"),
        ]
    })
}

fn node_ids(data: &serde_json::Value) -> BTreeSet<String> {
    data["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .map(|n| n["nodeId"].as_str().expect("nodeId").to_string())
        .collect()
}

fn edge_pairs(data: &serde_json::Value) -> BTreeSet<(String, String)> {
    data["edges"]
        .as_array()
        .expect("edges array")
        .iter()
        .map(|e| {
            (
                e["source"].as_str().expect("source").to_string(),
                e["target"].as_str().expect("target").to_string(),
            )
        })
        .collect()
}

fn statuses(data: &serde_json::Value) -> Vec<String> {
    data["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .map(|n| n["status"].as_str().expect("status").to_string())
        .collect()
}

fn all_pending(data: &serde_json::Value) -> bool {
    statuses(data).into_iter().all(|s| s == "pending")
}

fn status_of(data: &serde_json::Value, node_id: &str) -> String {
    data["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|n| n["nodeId"].as_str() == Some(node_id))
        .and_then(|n| n["status"].as_str())
        .expect("node status")
        .to_string()
}

fn session_of(data: &serde_json::Value, suffix: &str) -> String {
    data["nodes"]
        .as_array()
        .expect("nodes array")
        .iter()
        .find(|n| n["nodeId"].as_str().is_some_and(|id| id.ends_with(suffix)))
        .and_then(|n| n["sessionId"].as_str())
        .expect("sessionId")
        .to_string()
}

fn find_file(root: &Path, name: &str) -> Option<std::path::PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

fn fork_err(p: &mut PipelineHarness, session_id: &str, fork: serde_json::Value) -> String {
    p.pipeline_err(json!({ "sessionId": session_id, "fork": fork }))
}

#[test]
fn tc_6_01_fork_from_root_creates_complete_copy() {
    let (_tmp, mut p) = PipelineHarness::new();
    let source = p.create(chain());
    let sid = PipelineHarness::id(&source);

    let fork = p.fork(&sid, json!({ "fromNodeId": "A" }));
    let fork_id = PipelineHarness::id(&fork);

    assert_ne!(fork_id, sid);
    assert_eq!(node_ids(&fork).len(), 3);
    assert!(node_ids(&fork)
        .iter()
        .all(|id| id.ends_with('A') || id.ends_with('B') || id.ends_with('C')));
    assert!(all_pending(&fork));
    assert_eq!(node_ids(&p.view(&sid)).len(), 3);
    assert!(all_pending(&p.view(&sid)));
}

#[test]
fn tc_6_02_fork_from_middle_downstream_subgraph_only() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(chain()));
    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");

    let fork = p.fork(&sid, json!({ "fromNodeId": "B" }));
    let ids = node_ids(&fork);

    assert_eq!(ids.len(), 2);
    assert!(ids.iter().all(|id| id.ends_with('B') || id.ends_with('C')));
    assert!(all_pending(&fork));
    let original = p.view(&sid);
    assert_eq!(status_of(&original, "A"), "completed");
    assert_eq!(status_of(&original, "B"), "pending");
    assert_eq!(status_of(&original, "C"), "pending");
}

#[test]
fn tc_6_03_fork_from_leaf_single_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(chain()));
    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");
    p.dispatch(&sid);
    p.complete(&sid, "B", "done-B");

    let fork = p.fork(&sid, json!({ "fromNodeId": "C" }));

    assert_eq!(node_ids(&fork).len(), 1);
    assert!(node_ids(&fork).iter().all(|id| id.ends_with('C')));
    assert!(fork["edges"].as_array().expect("edges").is_empty());
    assert!(all_pending(&fork));
}

#[test]
fn tc_6_04_fork_with_explicit_new_session_id() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(single("A")));

    let fork = p.fork(
        &sid,
        json!({ "fromNodeId": "A", "newSessionId": "my-custom-fork" }),
    );

    assert_eq!(PipelineHarness::id(&fork), "my-custom-fork");
    assert_eq!(node_ids(&p.view("my-custom-fork")).len(), 1);
}

#[test]
fn tc_6_05_fork_rejects_nonexistent_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(single("A")));
    let err = fork_err(&mut p, &sid, json!({ "fromNodeId": "nonexistent" }));
    assert!(err.contains("doesn't contain node") || err.contains("not found"));
}

#[test]
fn tc_6_06_fork_rejects_duplicate_session_id() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid_x = PipelineHarness::id(&p.create(single("A")));
    let sid_y = PipelineHarness::id(&p.create(single("B")));

    let err = fork_err(
        &mut p,
        &sid_x,
        json!({ "fromNodeId": "A", "newSessionId": sid_y }),
    );
    assert!(err.contains("already exists"));
}

#[test]
fn tc_6_07_fork_preserves_branching_topology() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(diamond()));

    let fork = p.fork(&sid, json!({ "fromNodeId": "A" }));
    let fork_id = PipelineHarness::id(&fork);
    let prefix = format!("fork-{fork_id}-");

    assert_eq!(node_ids(&fork).len(), 4);
    assert_eq!(
        edge_pairs(&fork),
        BTreeSet::from([
            (format!("{prefix}A"), format!("{prefix}B")),
            (format!("{prefix}A"), format!("{prefix}C")),
            (format!("{prefix}B"), format!("{prefix}D")),
            (format!("{prefix}C"), format!("{prefix}D")),
        ])
    );
}

#[test]
fn tc_6_08_fork_from_node_with_completed_predecessors_captures_context() {
    let (tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(chain()));
    p.dispatch(&sid);
    p.complete(&sid, "A", "context-from-A");

    let fork = p.fork(&sid, json!({ "fromNodeId": "B" }));
    let fork_id = PipelineHarness::id(&fork);
    let fork_root = format!("fork-{fork_id}-B");
    let workflow_session = session_of(&fork, "B");
    let workflow_file =
        find_file(tmp.path(), &format!("{workflow_session}.md")).expect("workflow file");
    let body = fs::read_to_string(workflow_file).expect("read workflow");

    assert!(p.view(&fork_id)["ready"] == json!([fork_root]));
    assert!(body.contains("## Predecessor Context"));
    assert!(body.contains("### From A"));
    assert!(body.contains("context-from-A"));

    p.dispatch(&fork_id);
    assert_eq!(
        status_of(&p.view(&fork_id), &format!("fork-{fork_id}-B")),
        "active"
    );
}

#[test]
fn tc_6_09_source_pipeline_unaffected_by_fork_operations() {
    let (_tmp, mut p) = PipelineHarness::new();
    let sid = PipelineHarness::id(&p.create(json!({
        "nodes": [PipelineHarness::tool_node("A"), PipelineHarness::tool_node("B")],
        "edges": [PipelineHarness::edge("A", "B")]
    })));
    p.dispatch(&sid);
    p.complete(&sid, "A", "done-A");
    p.dispatch(&sid);

    let fork = p.fork(&sid, json!({ "fromNodeId": "A" }));
    let fork_id = PipelineHarness::id(&fork);
    p.complete(&sid, "B", "done-B");

    assert_eq!(status_of(&p.view(&sid), "B"), "completed");
    assert!(all_pending(&p.view(&fork_id)));
}
