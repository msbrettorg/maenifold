mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::{Value, json};

fn wf(id: &str) -> serde_json::Value {
    json!({ "id": id, "workflowId": "abductive-reasoning" })
}

fn node<'a>(view: &'a Value, node_id: &str) -> &'a Value {
    view["nodes"]
        .as_array()
        .expect("view nodes")
        .iter()
        .find(|node| node["nodeId"].as_str() == Some(node_id))
        .unwrap_or_else(|| panic!("missing node {node_id}"))
}

fn drain_and_delete(p: &mut PipelineHarness, pipeline_id: &str) {
    loop {
        let view = p.view(pipeline_id);
        let active: Vec<String> = view["active"]
            .as_array()
            .expect("active array")
            .iter()
            .filter_map(|node| node.as_str().map(str::to_owned))
            .collect();

        if !active.is_empty() {
            for node_id in active {
                p.complete(pipeline_id, &node_id, &format!("completed {node_id}"));
            }
            continue;
        }

        if view["ready"]
            .as_array()
            .is_some_and(|ready| !ready.is_empty())
        {
            p.dispatch(pipeline_id);
            continue;
        }

        break;
    }

    p.delete(pipeline_id);
}

#[test]
fn tc_1_01_create_single_workflow_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let data = p.create(json!({ "nodes": [wf("root")], "edges": [] }));

    assert!(!PipelineHarness::id(&data).is_empty());
    assert_eq!(data["ready"], json!(["root"]));
    assert_eq!(data["nodes"].as_array().map(Vec::len), Some(1));
    assert_eq!(data["nodes"][0]["nodeId"], "root");

    drain_and_delete(&mut p, &PipelineHarness::id(&data));
}

#[test]
fn tc_1_02_create_all_four_node_types() {
    let (_tmp, mut p) = PipelineHarness::new();
    let data = p.create(json!({
        "nodes": [
            { "id": "wf", "type": "workflow", "workflowId": "abductive-reasoning" },
            { "id": "pr", "type": "prompt", "prompt": "Summarize findings", "agent": "test-agent" },
            { "id": "st", "type": "sequentialThinking", "topic": "Analyze results", "agent": "test-agent", "totalThoughts": 3 },
            { "id": "tl", "type": "tool", "toolName": "ReadMemory", "toolArgs": { "identifier": "test" } }
        ],
        "edges": [
            PipelineHarness::edge("wf", "pr"),
            PipelineHarness::edge("wf", "st"),
            { "source": "pr", "target": "tl", "type": "always" },
            { "source": "st", "target": "tl", "type": "always" }
        ]
    }));

    assert!(!PipelineHarness::id(&data).is_empty());
    assert_eq!(data["ready"], json!(["wf"]));
    assert_eq!(data["nodes"].as_array().map(Vec::len), Some(4));

    drain_and_delete(&mut p, &PipelineHarness::id(&data));
}

#[test]
fn tc_1_03_create_with_max_concurrency() {
    let (_tmp, mut p) = PipelineHarness::new();
    let data = p.create_with_concurrency(
        json!({ "nodes": [wf("a"), wf("b"), wf("c")], "edges": [] }),
        2,
    );
    let id = PipelineHarness::id(&data);

    let dispatch = p.dispatch(&id);
    let view = p.view(&id);

    assert_eq!(dispatch["dispatched"].as_array().map(Vec::len), Some(2));
    assert_eq!(view["active"].as_array().map(Vec::len), Some(2));

    drain_and_delete(&mut p, &id);
}

#[test]
fn tc_1_04_view_pipeline() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(json!({
        "nodes": [wf("root"), PipelineHarness::tool_node("tail")],
        "edges": [PipelineHarness::edge("root", "tail")]
    }));
    let id = PipelineHarness::id(&created);
    let view = p.view(&id);
    let node = node(&view, "root");

    assert_eq!(view["pipelineId"], id);
    assert!(view["status"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(view["edges"].as_array().map(Vec::len), Some(1));
    assert_eq!(view["ready"], json!(["root"]));
    assert_eq!(view["active"], json!([]));
    assert_eq!(view["completed"], json!([]));
    assert!(view["mermaid"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(node["nodeId"], "root");
    assert!(node["status"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(node["sessionId"].as_str().is_some_and(|s| !s.is_empty()));

    drain_and_delete(&mut p, &id);
}

#[test]
fn tc_1_05_list_pipelines() {
    let (_tmp, mut p) = PipelineHarness::new();
    let first = PipelineHarness::id(&p.create(json!({ "nodes": [wf("a")], "edges": [] })));
    let second =
        PipelineHarness::id(&p.create(json!({ "nodes": [wf("b"), wf("c")], "edges": [] })));
    let list = p.list();
    let items = list["pipelines"].as_array().expect("pipelines array");

    assert!(items.iter().any(|item| {
        item["pipelineId"] == first
            && item["status"].is_string()
            && item["completedCount"].is_number()
    }));
    assert!(items.iter().any(|item| {
        item["pipelineId"] == second && item["nodeCount"] == 2 && item["completedCount"].is_number()
    }));

    drain_and_delete(&mut p, &first);
    drain_and_delete(&mut p, &second);
}

#[test]
fn tc_1_06_delete_pipeline() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(json!({ "nodes": [wf("root")], "edges": [] }));
    let id = PipelineHarness::id(&created);

    p.dispatch(&id);
    p.complete(&id, "root", "completed root");
    let deleted = p.delete(&id);
    let err = p.pipeline_err(json!({ "sessionId": id, "view": true }));

    assert_eq!(deleted["deletedSessionId"], created["pipelineId"]);
    assert!(err.contains("pipeline-"));
}
