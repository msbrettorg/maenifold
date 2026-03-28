use super::*;
use crate::thinking::sequential_session_uri;

#[test]
#[serial(config)]
fn pipeline_delete_completed_pipeline_removes_session_and_workflow_files() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600200";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600200-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600200-child".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "delete": true }),
    )
    .expect("delete completed pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("deletedSessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert_eq!(data.get("cleanedUpFiles").and_then(Value::as_u64), Some(3));
    assert_eq!(data.get("skippedFiles").and_then(Value::as_u64), Some(0));
    assert!(
        !file_exists(&pipeline_session_uri(pipeline_id), &context.config)
            .expect("pipeline removed")
    );
    assert!(
        !file_exists(
            &workflow_session_uri("workflow-1735689600200-root"),
            &context.config,
        )
        .expect("root workflow removed")
    );
    assert!(
        !file_exists(
            &workflow_session_uri("workflow-1735689600200-child"),
            &context.config,
        )
        .expect("child workflow removed")
    );
}

#[test]
#[serial(config)]
fn list_pipelines_returns_empty_when_no_pipelines() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    let output = pipeline(&context, &json!({ "list": true })).expect("list should succeed");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let pipelines = data
        .get("pipelines")
        .and_then(Value::as_array)
        .expect("pipelines array");

    assert!(
        pipelines.is_empty(),
        "expected empty list, got {pipelines:?}"
    );
}

#[test]
#[serial(config)]
fn list_pipelines_returns_seeded_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600001";
    seed_pipeline_view_session(&context, session_id, "pipeline");

    let output = pipeline(&context, &json!({ "list": true })).expect("list should succeed");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let pipelines = data
        .get("pipelines")
        .and_then(Value::as_array)
        .expect("pipelines array");

    assert_eq!(pipelines.len(), 1);
    let entry = &pipelines[0];
    assert_eq!(
        entry.get("pipelineId").and_then(Value::as_str),
        Some(session_id)
    );
    assert_eq!(entry.get("nodeCount").and_then(Value::as_u64), Some(3));
    assert_eq!(entry.get("completedCount").and_then(Value::as_u64), Some(1));
}

#[test]
#[serial(config)]
fn pipeline_delete_running_pipeline_rejects_active_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600201";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600201-root".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600201-child".to_string(),
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
    .expect_err("running pipeline delete should fail");

    assert!(
        error
            .to_string()
            .contains("pause or cancel it before deleting"),
        "unexpected error: {error}"
    );
}

#[test]
#[serial(config)]
fn pipeline_delete_skips_missing_workflow_session_files() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600202";
    let missing_workflow_id = "workflow-1735689600202-child";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600202-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: missing_workflow_id.to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);
    delete_file(&workflow_session_uri(missing_workflow_id), &context.config)
        .expect("remove child workflow session");

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "delete": true }),
    )
    .expect("delete pipeline with missing workflow session");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("deletedSessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert_eq!(data.get("cleanedUpFiles").and_then(Value::as_u64), Some(2));
    assert_eq!(data.get("skippedFiles").and_then(Value::as_u64), Some(1));
    assert!(
        !file_exists(&pipeline_session_uri(pipeline_id), &context.config)
            .expect("pipeline removed")
    );
    assert!(
        !file_exists(
            &workflow_session_uri("workflow-1735689600202-root"),
            &context.config,
        )
        .expect("root workflow removed")
    );
    assert!(
        !file_exists(&workflow_session_uri(missing_workflow_id), &context.config)
            .expect("missing workflow still absent")
    );
}

#[test]
#[serial(config)]
fn pipeline_delete_completed_pipeline_removes_sequential_thinking_session_file() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600203";
    let thinking_session_id = "session-1735689600203-4242";
    let node_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600203-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: Some(thinking_session_id.to_string()),
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600203-child".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, pipeline_id, node_states, 2);

    let thinking_uri = sequential_session_uri(thinking_session_id);
    write_markdown(
        &thinking_uri,
        None,
        "# Sequential Thinking Session\n",
        &context.config,
    )
    .expect("write thinking session");

    assert!(file_exists(&thinking_uri, &context.config).expect("thinking session exists"));
    assert!(
        !file_exists(&workflow_session_uri(thinking_session_id), &context.config)
            .expect("no workflow shadow session")
    );

    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "delete": true }),
    )
    .expect("delete completed pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("deletedSessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert_eq!(data.get("cleanedUpFiles").and_then(Value::as_u64), Some(4));
    assert_eq!(data.get("skippedFiles").and_then(Value::as_u64), Some(0));
    assert!(!file_exists(&thinking_uri, &context.config).expect("thinking session removed"));
}
