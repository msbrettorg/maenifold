use super::*;

#[test]
#[serial(config)]
fn pipeline_run_rejects_pipeline_with_active_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600990";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
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
                    session_id: "workflow-1735689600990-root".to_string(),
                    status: NodeStatus::Active,
                    attempts: 1,
                    thinking_session_id: None,
                    output: None,
                },
            ),
            (
                "child".to_string(),
                NodeState {
                    session_id: "workflow-1735689600990-child".to_string(),
                    status: NodeStatus::Pending,
                    attempts: 0,
                    thinking_session_id: None,
                    output: None,
                },
            ),
        ]),
        1,
    );

    let error = pipeline(&context, &json!({ "sessionId": pipeline_id, "run": true }))
        .expect_err("run should fail");

    assert!(
        error.to_string().contains("already has active nodes"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_run_starts_supervisor_and_reports_running_state() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    super::super::supervisor::supervisor_clear_all();
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);

    let pipeline_id = "pipeline-1735689600991";
    let workflow_session_id = "workflow-1735689600991-root";
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

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "run": true, "prompt": "ship it" }),
    )
    .expect("run pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(data.get("status").and_then(Value::as_str), Some("started"));
    assert_eq!(
        data.get("sessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert!(data.get("pid").is_none() || data.get("pid") == Some(&Value::Null));
    assert!(super::super::supervisor::supervisor_is_running(pipeline_id));

    let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view running pipeline");
    let view_response: Value = serde_json::from_str(&view_output).expect("parse response");
    let view_data = view_response.get("data").expect("view data");
    assert_eq!(
        view_data.get("running").and_then(Value::as_bool),
        Some(true)
    );
    assert!(view_data.get("pid").is_none() || view_data.get("pid") == Some(&Value::Null));
    assert!(view_data.get("attempts").is_none() || view_data.get("attempts") == Some(&Value::Null));

    super::super::supervisor::supervisor_cancel(pipeline_id).expect("cancel running process");
}
