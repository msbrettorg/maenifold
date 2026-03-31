use super::*;

#[test]
#[serial(config)]
fn pipeline_view_includes_prompt_node_prompt_and_agent_fields() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600053";
    let prompt_session_id = "workflow-1735689600053-prompt";
    let definition = PipelineDefinition {
        nodes: vec![super::super::PipelineNode::Prompt {
            id: "prompt".to_string(),
            prompt: "Summarize the workflow result".to_string(),
            agent: "prompt-agent".to_string(),
            tool_mode: "default".to_string(),
            agent_config: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,

            timeout: None,
        }],
        edges: vec![],
    };
    let node_state = NodeState {
        session_id: prompt_session_id.to_string(),
        status: NodeStatus::Pending,
        attempts: 0,
        thinking_session_id: None,
        output: None,
    };
    let node_states = BTreeMap::from([(
        "prompt".to_string(),
        serde_yaml::to_value(&node_state).expect("serialize node state"),
    )]);
    write_pipeline_session(&context, pipeline_id, &definition, &node_states, 1)
        .expect("write prompt pipeline session");

    let mut workflow_frontmatter =
        crate::workflow::initial_workflow_frontmatter(prompt_session_id, &[], "pending");
    workflow_frontmatter.insert(
        "pipelineId".to_string(),
        YamlValue::String(pipeline_id.to_string()),
    );
    workflow_frontmatter.insert(
        "nodeId".to_string(),
        YamlValue::String("prompt".to_string()),
    );
    write_markdown(
        &workflow_session_uri(prompt_session_id),
        Some(&workflow_frontmatter),
        "# Workflow Session\n",
        &context.config,
    )
    .expect("write prompt workflow session");
    assert!(
        file_exists(&workflow_session_uri(prompt_session_id), &context.config)
            .expect("check prompt workflow session"),
        "expected prompt workflow session to exist"
    );

    let view_output = pipeline(&context, &json!({ "sessionId": pipeline_id, "view": true }))
        .expect("view prompt pipeline");
    let view_response: Value = serde_json::from_str(&view_output).expect("parse view response");
    let prompt_node = view_response
        .get("data")
        .and_then(|data| data.get("nodes"))
        .and_then(Value::as_array)
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|node| node.get("nodeId") == Some(&Value::String("prompt".to_string())))
        })
        .expect("prompt node");

    assert_eq!(
        prompt_node.get("type").and_then(Value::as_str),
        Some("prompt")
    );
    assert_eq!(
        prompt_node.get("prompt").and_then(Value::as_str),
        Some("Summarize the workflow result")
    );
    assert_eq!(
        prompt_node.get("agent").and_then(Value::as_str),
        Some("prompt-agent")
    );
    assert_eq!(
        prompt_node.get("status").and_then(Value::as_str),
        Some("pending")
    );
    assert_eq!(prompt_node.get("attempts").and_then(Value::as_u64), Some(0));
    assert_eq!(prompt_node.get("output"), Some(&Value::Null));
}

#[test]
#[serial(config)]
fn pipeline_create_defaults_max_concurrency_to_eight() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                { "id": "solo", "workflowId": "test-workflow-a", "acpClient": "agent-solo" }
            ],
            "edges": []
        }
    });

    let output = pipeline(&context, &payload).expect("create pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let pipeline_id = response
        .get("data")
        .and_then(|data| data.get("pipelineId"))
        .and_then(Value::as_str)
        .expect("pipeline id");
    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read file");
    let frontmatter = frontmatter.expect("frontmatter");

    assert_eq!(
        frontmatter
            .get("maxConcurrency")
            .and_then(YamlValue::as_u64),
        Some(8)
    );
}

#[test]
#[serial(config)]
fn pipeline_create_rejects_non_positive_max_concurrency() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);

    for max_concurrency in [json!(0), json!(-1)] {
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "solo", "workflowId": "test-workflow-a", "acpClient": "agent-solo" }
                ],
                "edges": []
            },
            "maxConcurrency": max_concurrency
        });

        let error = pipeline(&context, &payload).expect_err("invalid maxConcurrency should fail");
        assert!(
            error
                .to_string()
                .contains("maxConcurrency must be a positive integer"),
            "unexpected error: {error}"
        );
    }
}

#[test]
#[serial(config)]
fn pipeline_learn_returns_help_text() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Markdown);

    let output = pipeline(&context, &json!({ "learn": true })).expect("learn output");

    assert!(output.contains("# Pipeline"));
    assert!(output.contains("Creates and manages pipeline sessions composed of workflow nodes."));
}

#[test]
#[serial(config)]
fn pipeline_create_rejects_invalid_definition() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                { "id": "a", "workflowId": "test-workflow-a", "acpClient": "agent-a" },
                { "id": "b", "workflowId": "test-workflow-b", "acpClient": "agent-b" }
            ],
            "edges": [
                { "source": "a", "target": "b" },
                { "source": "b", "target": "a", "type": "always" }
            ]
        }
    });

    let error = pipeline(&context, &payload).expect_err("cycle should fail");

    assert!(error.to_string().contains("contains a cycle"));
}

#[test]
#[serial(config)]
fn pipeline_create_surfaces_type_specific_validation_errors() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                {
                    "id": "bad-prompt",
                    "type": "prompt",
                    "prompt": "   ",
                    "agent": "prompt-agent"
                }
            ],
            "edges": []
        }
    });

    let error = pipeline(&context, &payload).expect_err("invalid prompt node should fail");
    let message = error.to_string();

    assert!(
        message.contains("bad-prompt"),
        "unexpected error: {message}"
    );
    assert!(message.contains("prompt"), "unexpected error: {message}");
}

#[test]
#[serial(config)]
fn pipeline_create_rejects_tool_node_with_empty_tool_name() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                {
                    "id": "bad-tool",
                    "type": "tool",
                    "toolName": "   ",
                    "agent": "prompt-agent"
                }
            ],
            "edges": []
        }
    });

    let error = pipeline(&context, &payload).expect_err("invalid tool node should fail");
    let message = error.to_string();

    assert!(message.contains("bad-tool"), "unexpected error: {message}");
    assert!(message.contains("toolName"), "unexpected error: {message}");
}
