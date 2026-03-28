use super::*;

#[test]
#[serial(config)]
fn pipeline_join_fork_join_rejects_running_target_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600601";
    let source_id = "pipeline-1735689600602";

    seed_pipeline_session_with_workflows(
        &context,
        target_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "target-root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "target-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600601-target-root".to_string(),
                status: NodeStatus::Ready,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "source-root".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "source-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600602-source-root".to_string(),
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
                "edges": []
            }
        }),
    )
    .expect_err("running target should fail");

    assert!(error.to_string().contains("target pipeline"));
    assert!(error.to_string().contains("before join"));
}

#[test]
#[serial(config)]
fn pipeline_join_fork_join_rejects_running_source_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600603";
    let source_id = "pipeline-1735689600604";

    seed_pipeline_session_with_workflows(
        &context,
        target_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "target-root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "target-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600603-target-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "source-root".to_string(),
                workflow_id: "test-workflow-b".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "source-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600604-source-root".to_string(),
                status: NodeStatus::Active,
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
                "edges": []
            }
        }),
    )
    .expect_err("running source should fail");

    assert!(error.to_string().contains("source pipeline"));
    assert!(error.to_string().contains(source_id));
}

#[test]
#[serial(config)]
fn pipeline_join_fork_join_rejects_duplicate_bridge_edge() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let target_id = "pipeline-1735689600605";
    let source_id = "pipeline-1735689600606";

    seed_pipeline_session_with_workflows(
        &context,
        target_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "target-root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "target-root".to_string(),
            NodeState {
                session_id: "workflow-1735689600605-target-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    seed_pipeline_session_with_workflows(
        &context,
        source_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "source-root".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                },
                super::super::PipelineNode::Workflow {
                    id: "source-leaf".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
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
                    session_id: "workflow-1735689600606-source-root".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "source-leaf".to_string(),
                NodeState {
                    session_id: "workflow-1735689600606-source-leaf".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );

    let error = pipeline(
        &context,
        &json!({
            "sessionId": target_id,
            "join": {
                "sourceSessionId": source_id,
                "edges": [
                    { "source": "source-root", "target": "source-leaf", "type": "always" }
                ]
            }
        }),
    )
    .expect_err("duplicate bridge edge should fail");

    assert!(
        error
            .to_string()
            .contains("duplicate edge 'source-root -> source-leaf'")
    );
}
