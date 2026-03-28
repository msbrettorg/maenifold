use super::*;

#[test]
#[serial(config)]
fn pipeline_cancel_stops_supervisor_and_marks_pipeline_cancelled() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    super::super::supervisor::supervisor_clear_all();
    let _path_guard = install_acpx_stub(&context.config.maenifold_root);

    let pipeline_id = "pipeline-1735689600992";
    let workflow_session_id = "workflow-1735689600992-root";
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

    pipeline(&context, &json!({ "sessionId": pipeline_id, "run": true })).expect("start pipeline");
    let output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "cancel": true }),
    )
    .expect("cancel pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("status").and_then(Value::as_str),
        Some("cancelled")
    );
    assert_eq!(
        data.get("sessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert!(!super::super::supervisor::supervisor_is_running(
        pipeline_id
    ));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("cancelled")
    );
}

#[test]
#[serial(config)]
fn pipeline_pause_and_pipeline_resume_updates_status() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    super::super::supervisor::supervisor_clear_all();

    let pipeline_id = "pipeline-1735689600993";
    let workflow_session_id = "workflow-1735689600993-root";
    seed_pipeline_session_with_workflows(
        &context,
        pipeline_id,
        PipelineDefinition {
            nodes: vec![super::super::PipelineNode::Workflow {
                id: "root".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: None,
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
    super::super::supervisor::supervisor_spawn_test_task(pipeline_id).expect("spawn tracked task");
    assert!(super::super::supervisor::supervisor_is_running(pipeline_id));

    let pause_output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "pause": true }),
    )
    .expect("pause pipeline");
    let pause_response: Value = serde_json::from_str(&pause_output).expect("parse response");
    let pause_data = pause_response.get("data").expect("pause data");

    assert_eq!(
        pause_data.get("status").and_then(Value::as_str),
        Some("paused")
    );
    assert_eq!(
        pause_data.get("sessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert!(pause_data.get("pid").is_none() || pause_data.get("pid") == Some(&Value::Null));

    let read_pipeline_status = || {
        for _ in 0..20 {
            let (frontmatter, _, _) =
                read_markdown(&pipeline_session_uri(pipeline_id), &context.config)
                    .expect("read file");
            if let Some(frontmatter) = frontmatter {
                return frontmatter
                    .get("status")
                    .and_then(YamlValue::as_str)
                    .map(str::to_string);
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        None
    };
    assert_eq!(read_pipeline_status().as_deref(), Some("paused"));

    let resume_output = pipeline(
        &context,
        &json!({ "sessionId": pipeline_id, "resume": true }),
    )
    .expect("resume pipeline");
    let resume_response: Value = serde_json::from_str(&resume_output).expect("parse response");
    let resume_data = resume_response.get("data").expect("resume data");

    assert_eq!(
        resume_data.get("status").and_then(Value::as_str),
        Some("resumed")
    );
    assert_eq!(
        resume_data.get("sessionId").and_then(Value::as_str),
        Some(pipeline_id)
    );
    assert!(resume_data.get("pid").is_none() || resume_data.get("pid") == Some(&Value::Null));

    assert_eq!(read_pipeline_status().as_deref(), Some("active"));

    super::super::supervisor::supervisor_cancel(pipeline_id).expect("cleanup running process");
}
