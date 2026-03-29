use super::*;

#[test]
#[serial(config)]
fn pipeline_graft_rejects_cycles_without_mutation() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600052";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![
                super::super::PipelineNode::Workflow {
                    id: "a".to_string(),
                    workflow_id: "test-workflow-a".to_string(),
                    acp_client: None,
                    prompt: None,
                
                    timeout: None},
                super::super::PipelineNode::Workflow {
                    id: "b".to_string(),
                    workflow_id: "test-workflow-b".to_string(),
                    acp_client: None,
                    prompt: None,
                
                    timeout: None},
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
                    session_id: "workflow-1735689600052-a".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "b".to_string(),
                NodeState {
                    session_id: "workflow-1735689600052-b".to_string(),
                    status: NodeStatus::Completed,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "c", "workflowId": "test-workflow-a" }
                ],
                "edges": [
                    { "source": "b", "target": "c" },
                    { "source": "c", "target": "a" }
                ]
            }
        }),
    )
    .expect_err("cycle should fail");

    assert!(error.to_string().contains("cycle"));
    assert!(error.to_string().contains("a"));
    assert!(error.to_string().contains("b"));
    assert!(error.to_string().contains("c"));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
    assert_eq!(definition.nodes.len(), 2);
    assert_eq!(definition.edges.len(), 1);
    assert!(
        !file_exists(
            &workflow_session_uri("workflow-1735689600052-c"),
            &context.config
        )
        .expect("workflow file check")
    );
}

#[test]
#[serial(config)]
fn pipeline_graft_rejects_duplicate_node_ids() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600053";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None}],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600053-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let error = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "root", "workflowId": "test-workflow-b" }
                ],
                "edges": []
            }
        }),
    )
    .expect_err("duplicate node id should fail");

    assert!(error.to_string().contains("duplicate node ID 'root'"));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
    assert_eq!(definition.nodes.len(), 1);
}
