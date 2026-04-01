use super::*;

#[test]
#[serial(config)]
fn pipeline_join_fork_join_merges_two_simple_dags() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600100";
    let source_id = "pipeline-1735689600200";

    seed_pipeline_session_with_workflows(
        &context,
        target_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "target-root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "target-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600100-target-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        3,
    );
    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "source-leaf".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "source-root".to_string(),
                target: "source-leaf".to_string(),
                edge_type: super::super::EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            (
                "source-root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-source-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 2,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "source-leaf".to_string(),
                NodeState {
                    session_id: "workflow-1735689600200-source-leaf".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        3,
    );

    let output = pipeline(
        &context,
        &json!({
            "sessionId": target_id,
            "join": {
                "sourceSessionId": source_id,
                "edges": [
                    { "source": "target-root", "target": "source-root" }
                ]
            }
        }),
    )
    .expect("join succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("pipelineId").and_then(Value::as_str),
        Some(target_id)
    );
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&Vec::new())
    );
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert_eq!(nodes.len(), 3);
    let source_root = nodes
        .iter()
        .find(|node| node.get("nodeId") == Some(&Value::String("source-root".to_string())))
        .expect("source root node");
    assert_eq!(
        source_root.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(source_root.get("attempts").and_then(Value::as_u64), Some(2));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(target_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("definition");
    assert_eq!(definition.nodes.len(), 3);
    assert_eq!(definition.edges.len(), 2);
    assert!(
        definition
            .edges
            .iter()
            .any(|edge| edge.source == "target-root" && edge.target == "source-root")
    );
}

#[test]
#[serial(config)]
fn pipeline_join_fork_join_rejects_node_id_collision() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600300";
    let source_id = "pipeline-1735689600400";

    for session_id in [target_id, source_id] {
        seed_pipeline_session_with_workflows(
            &context,
            session_id,
            PipelineDefinition {
                nodes: vec![super::super::PipelineNode::Workflow {
                    id: "shared".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                }],
                edges: vec![],
            },
            HashMap::from([(
                "shared".to_string(),
                NodeState {
                    session_id: format!("workflow-{session_id}-shared"),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            )]),
            2,
        );
    }

    let error = pipeline(
        &context,
        &json!({
            "sessionId": target_id,
            "join": {
                "sourceSessionId": source_id,
                "edges": []
            }
        }),
    )
    .expect_err("collision should fail");

    assert!(error.to_string().contains("conflicting node IDs: shared"));
}

#[test]
#[serial(config)]
fn pipeline_join_fork_join_rejects_cycle() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600500";
    let source_id = "pipeline-1735689600600";

    seed_pipeline_session_with_workflows(
        &context,
        target_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: Some("workflow-agent".to_string()),
                    prompt: None,

                    timeout: None,
                },
            ],
            edges: vec![super::super::PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: super::super::EdgeType::Success,
                mapping: None,
            }],
        },
        HashMap::from([
            (
                "a".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-a".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: "workflow-1735689600500-b".to_string(),
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
        source_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "c".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "c".to_string(),
            NodeState {
                session_id: "workflow-1735689600600-c".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );

    let error = pipeline(
        &context,
        &json!({
            "sessionId": target_id,
            "join": {
                "sourceSessionId": source_id,
                "edges": [
                    { "source": "b", "target": "c" },
                    { "source": "c", "target": "a" }
                ]
            }
        }),
    )
    .expect_err("cycle should fail");

    assert!(error.to_string().contains("contains a cycle"));
}
