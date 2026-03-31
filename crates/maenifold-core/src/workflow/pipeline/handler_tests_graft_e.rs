use super::*;

#[test]
#[serial(config)]
fn pipeline_graft_completed_nodes_succeed_despite_active_frontmatter() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600061";
    seed_dispatch_pipeline(
        &context,
        pipeline_id,
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600061-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600061-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "tail", "workflowId": "test-workflow-a", "acpClient": "agent-tail" }
                ],
                "edges": [
                    { "source": "child", "target": "tail" }
                ]
            }
        }),
    )
    .expect("completed nodes with active frontmatter should graft");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let nodes = response
        .get("data")
        .and_then(|data| data.get("nodes"))
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes.len(), 3);
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("tail".to_string()))
            && node.get("status") == Some(&Value::String("pending".to_string()))
    }));
}
