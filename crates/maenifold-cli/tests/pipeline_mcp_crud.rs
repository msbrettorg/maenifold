mod pipeline_mcp_support;
use pipeline_mcp_support::PipelineHarness;
use serde_json::json;

fn wf(id: &str) -> serde_json::Value {
    json!({ "id": id, "workflowId": "abductive-reasoning" })
}

#[test]
fn tc_1_1_create_single_workflow_node() {
    let (_tmp, mut p) = PipelineHarness::new();
    let data = p.create(json!({ "nodes": [wf("root")], "edges": [] }));

    assert!(!PipelineHarness::id(&data).is_empty());
    assert_eq!(data["ready"], json!(["root"]));
    assert_eq!(data["nodes"].as_array().map(Vec::len), Some(1));
    assert_eq!(data["nodes"][0]["nodeId"], "root");
}

#[test]
fn tc_1_2_create_all_four_node_types() {
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
}

#[test]
fn tc_1_3_create_with_max_concurrency() {
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
}

#[test]
fn tc_1_4_view_pipeline() {
    let (_tmp, mut p) = PipelineHarness::new();
    let created = p.create(json!({
        "nodes": [wf("root"), PipelineHarness::tool_node("tail")],
        "edges": [PipelineHarness::edge("root", "tail")]
    }));
    let id = PipelineHarness::id(&created);
    let view = p.view(&id);
    let node = &view["nodes"][0];

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
}

#[test]
fn tc_1_5_list_pipelines() {
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
}

#[test]
fn tc_1_6_delete_pipeline() {
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
