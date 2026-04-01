use super::*;

#[test]
#[serial(config)]
fn pipeline_delete_pending_pipeline_rejects_remaining_work() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600204";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600204-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600204-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

    let error = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "delete": true }),
    )
    .expect_err("pending pipeline delete should fail");

    assert!(
        error
            .to_string()
            .contains("pause or cancel it before deleting"),
        "unexpected error: {error}"
    );
    assert!(
        file_exists(&pipeline_session_uri(pipeline_id), &context.config)
            .expect("pipeline still exists")
    );
}
