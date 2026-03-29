use super::*;

#[test]
#[serial(config)]
fn pipeline_graft_pending_pipeline_rejects_remaining_work() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600058";
    seed_dispatch_pipeline(
        &context,
        pipeline_id,
        HashMap::from([
            (
                "root".to_string(),
                NodeState {
                    session_id: "workflow-1735689600058-root".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600058-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
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
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "tail", "workflowId": "test-workflow-a" }
                ],
                "edges": [
                    { "source": "child", "target": "tail" }
                ]
            }
        }),
    )
    .expect_err("pending graft should fail");

    assert!(error.to_string().contains("running pipelines"));
}

#[test]
#[serial(config)]
fn pipeline_graft_duplicate_node_id_wins_over_missing_workflow_definition() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600059";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,

                timeout: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600059-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
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
                    { "id": "root", "workflowId": "missing-workflow" }
                ],
                "edges": []
            }
        }),
    )
    .expect_err("duplicate node id should be reported first");

    assert!(error.to_string().contains("duplicate node ID 'root'"));
    assert!(!error.to_string().contains("missing-workflow"));
}

#[test]
#[serial(config)]
fn pipeline_graft_persists_pipeline_only_after_new_workflow_sessions_exist() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600060";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
                prompt: None,

                timeout: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600060-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let workflow_root = context.config.memory_path.join("thinking").join("workflow");
    let original_permissions = fs::metadata(&workflow_root)
        .expect("workflow root metadata")
        .permissions();
    let mut read_only_permissions = original_permissions.clone();
    read_only_permissions.set_readonly(true);
    fs::set_permissions(&workflow_root, read_only_permissions)
        .expect("set workflow root read-only");

    let result = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    { "id": "child", "workflowId": "test-workflow-b" }
                ],
                "edges": [
                    { "source": "root", "target": "child" }
                ]
            }
        }),
    );

    fs::set_permissions(&workflow_root, original_permissions)
        .expect("restore workflow root permissions");

    let error = result.expect_err("workflow session write should fail");
    assert!(!error.to_string().is_empty(), "expected an I/O error");

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");

    assert_eq!(definition.nodes.len(), 1);
    assert_eq!(definition.edges.len(), 0);
}
