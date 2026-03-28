use super::*;

#[test]
#[serial(config)]
fn pipeline_rewind_rejects_running_pipeline() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    super::super::supervisor::supervisor_clear_all();
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);

    let pipeline_id = "pipeline-1735689600063";
    let workflow_session_id = "workflow-1735689600063-root";
    write_acpx_stub_config(
        &context.config.maenifold_root,
        json!({
            "runner-agent": { "delay": 30.0, "text": "still running" }
        }),
    );
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("runner-agent".to_string()),
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: workflow_session_id.to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        )]),
        1,
    );
    set_pipeline_status(&context, pipeline_id, "pending");

    pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "run": true, "prompt": "keep running" }),
    )
    .expect("start pipeline");
    assert!(super::super::supervisor::supervisor_is_running(pipeline_id));
    super::super::supervisor::supervisor_pause(pipeline_id).expect("pause running process");
    assert_eq!(
        super::super::supervisor::supervisor_is_paused(pipeline_id),
        Some(true)
    );
    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["root"] } }),
    )
    .expect_err("running pipeline rewind should fail");

    assert!(
        error.to_string().contains("running") || error.to_string().contains("pause"),
        "unexpected error: {error}"
    );

    super::super::supervisor::supervisor_cancel(pipeline_id).expect("cancel running process");
    super::super::supervisor::supervisor_clear_all();
}

#[test]
#[serial(config)]
fn pipeline_rewind_rejects_unknown_node_id() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600064";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("runner-agent".to_string()),
                prompt: None,
            }],
            edges: vec![],
        },
        HashMap::from([(
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600064-root".to_string(),
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
        &json!({ "sessionId": pipeline_id, "rewind": { "nodeIds": ["missing"] } }),
    )
    .expect_err("missing node should fail");

    assert!(
        error.to_string().contains("Unknown node ID: missing"),
        "unexpected error: {error}"
    );
}
