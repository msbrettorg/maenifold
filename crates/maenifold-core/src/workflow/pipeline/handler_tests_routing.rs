use super::*;

#[test]
#[serial(config)]
fn pipeline_schema_learn_output_documents_all_new_operations() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);

    let output = pipeline(&context, &json!({ "learn": true })).expect("learn output");

    for keyword in [
        "run", "pause", "resume", "cancel", "rewind", "delete", "graft", "join", "fork",
    ] {
        assert!(
            output.contains(keyword),
            "expected help output to mention {keyword}: {output}"
        );
    }
}

#[test]
#[serial(config)]
fn pipeline_schema_routes_new_operation_keywords_to_handlers() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    for (payload, expected_error) in [
        (
            json!({ "sessionId": "pipeline-missing", "run": true }),
            "Session pipeline-missing doesn't exist",
        ),
        (
            json!({ "sessionId": "pipeline-missing", "pause": true }),
            "Pipeline pipeline-missing isn't running",
        ),
        (
            json!({ "sessionId": "pipeline-missing", "resume": true }),
            "Session pipeline-missing doesn't exist",
        ),
        (
            json!({ "sessionId": "pipeline-missing", "cancel": true }),
            "Pipeline pipeline-missing isn't running",
        ),
        (
            json!({ "sessionId": "pipeline-missing", "rewind": { "nodeIds": ["root"] } }),
            "Session pipeline-missing doesn't exist",
        ),
    ] {
        let error = pipeline(&context, &payload).expect_err("operation should route");
        assert_eq!(error.to_string(), expected_error);
    }

    let graft_pipeline_id = "pipeline-1735689600202";
    let graft_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600202-root".to_string(),
                status: NodeStatus::Active,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600202-child".to_string(),
                status: NodeStatus::Pending,
                attempts: 0,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, graft_pipeline_id, graft_states, 2);
    let graft_error = pipeline(
        &context,
        &json!({
            "sessionId": graft_pipeline_id,
            "graft": {
                "nodes": [{ "id": "new-node", "workflowId": "test-workflow-a" }],
                "edges": []
            }
        }),
    )
    .expect_err("graft should route");
    assert!(
        graft_error
            .to_string()
            .contains("Pipeline graft rejected: running pipelines")
    );

    let join_target_id = "pipeline-1735689600203";
    let join_source_id = "pipeline-1735689600204";
    let join_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600203-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
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
    let join_source_states = HashMap::from([
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
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, join_target_id, join_states, 2);
    seed_dispatch_pipeline(&context, join_source_id, join_source_states, 2);
    let join_error = pipeline(
        &context,
        &json!({
            "sessionId": join_target_id,
            "join": {
                "sourceSessionId": join_source_id,
                "edges": []
            }
        }),
    )
    .expect_err("join should route");
    assert!(
        join_error
            .to_string()
            .contains("Cannot join pipelines; conflicting node IDs")
    );

    let fork_pipeline_id = "pipeline-1735689600205";
    let fork_states = HashMap::from([
        (
            "root".to_string(),
            NodeState {
                session_id: "workflow-1735689600205-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
        (
            "child".to_string(),
            NodeState {
                session_id: "workflow-1735689600205-child".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        ),
    ]);
    seed_dispatch_pipeline(&context, fork_pipeline_id, fork_states, 2);
    let fork_error = pipeline(
        &context,
        &json!({
            "sessionId": fork_pipeline_id,
            "fork": {
                "fromNodeId": "missing",
                "newSessionId": "pipeline-forked"
            }
        }),
    )
    .expect_err("fork should route");
    assert!(
        fork_error
            .to_string()
            .contains("Pipeline doesn't contain node 'missing'")
    );
}

#[test]
#[serial(config)]
fn pipeline_tool_registration_includes_pipeline_aliases() {
    Config::reset_overrides();
    let registry = ToolRegistry::new();

    assert!(registry.contains("Pipeline"));
    assert!(registry.contains("pipeline"));
    assert!(registry.canonical_names().contains(&"Pipeline"));
}
