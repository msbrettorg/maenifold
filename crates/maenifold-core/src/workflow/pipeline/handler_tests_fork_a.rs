use super::*;

#[test]
#[serial(config)]
fn pipeline_join_fork_fork_rejects_existing_new_session_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let source_id = "pipeline-1735689600698";
    let existing_id = "pipeline-1735689600699";

    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "child".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "root".to_string(),
                target: "child".to_string(),
                edge_type: super::super::EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600698-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600698-child".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );
    seed_pipeline_session_with_workflows(
        &context,
        existing_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "existing".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "existing".to_string(),
            NodeState {
                session_id: "workflow-1735689600699-existing".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        1,
    );

    let error = pipeline(
        &context,
        &json!({
            "sessionId": source_id,
            "fork": {
                "fromNodeId": "root",
                "newSessionId": existing_id
            }
        }),
    )
    .expect_err("fork should reject existing session id");

    assert!(
        error.to_string().contains("already exists"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_join_fork_fork_from_middle_includes_all_downstream() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let source_id = "pipeline-1735689600700";
    let new_id = "pipeline-1735689600701";

    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "root".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "middle".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "left".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "right".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
            ],
            edges: vec![
                super::super::PipelineEdge {
                    source: "root".to_string(),
                    target: "middle".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "middle".to_string(),
                    target: "left".to_string(),
                    edge_type: super::super::EdgeType::Success,
                    mapping: None,
                },
                super::super::PipelineEdge {
                    source: "middle".to_string(),
                    target: "right".to_string(),
                    edge_type: super::super::EdgeType::Always,
                    mapping: None,
                },
            ],
        },
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "middle".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-middle".to_string(),
                    status: NodeStatus::Active,
                    attempts: 2,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "left".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-left".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 3,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "right".to_string(),
                NodeState {
                    session_id: "workflow-1735689600700-right".to_string(),
                    status: NodeStatus::Stuck,
                    attempts: 4,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        5,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": source_id,
            "fork": {
                "fromNodeId": "middle",
                "newSessionId": new_id
            }
        }),
    )
    .expect("fork succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let node_ids = nodes
        .iter()
        .map(|node| {
            node.get("nodeId")
                .and_then(Value::as_str)
                .expect("node id")
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 3);
    assert!(!node_ids.iter().any(|id| id.ends_with("root")));
    assert!(node_ids.iter().any(|id| id.ends_with("middle")));
    assert!(node_ids.iter().any(|id| id.ends_with("left")));
    assert!(node_ids.iter().any(|id| id.ends_with("right")));
    assert!(
        nodes
            .iter()
            .all(|node| node.get("status").and_then(Value::as_str) == Some("pending"))
    );
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String(format!("fork-{new_id}-middle"))])
    );

    let (frontmatter, _, _) = read_markdown(&pipeline_session_uri(new_id), &context.config)
        .expect("read forked pipeline");
    let frontmatter = frontmatter.expect("frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("definition");
    assert_eq!(definition.nodes.len(), 3);
    assert_eq!(definition.edges.len(), 2);
    assert!(
        definition
            .nodes
            .iter()
            .all(|node| node.id().starts_with(&format!("fork-{new_id}-")))
    );
}
