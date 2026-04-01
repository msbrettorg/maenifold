use super::*;

#[test]
#[serial(config)]
fn pipeline_create_persists_pipeline_and_workflow_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                { "id": "root", "workflowId": "test-workflow-a", "acpClient": "agent-root" },
                { "id": "child", "workflowId": "test-workflow-b", "acpClient": "agent-b" }
            ],
            "edges": [
                { "source": "root", "target": "child" }
            ]
        },
        "maxConcurrency": 4
    });

    let output = pipeline(&context, &payload).expect("create pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let pipeline_id = data
        .get("pipelineId")
        .and_then(Value::as_str)
        .expect("pipeline id");
    let ready = data
        .get("ready")
        .and_then(Value::as_array)
        .expect("ready array");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    assert!(pipeline_id.starts_with("pipeline-"));
    assert_eq!(ready, &vec![Value::String("root".to_string())]);
    assert_eq!(nodes.len(), 2);

    let pipeline_uri = pipeline_session_uri(pipeline_id);
    let (pipeline_frontmatter, pipeline_content, _) =
        read_markdown(&pipeline_uri, &context.config).expect("read pipeline session");
    let pipeline_frontmatter = pipeline_frontmatter.expect("pipeline frontmatter");

    assert_eq!(
        pipeline_frontmatter.get("type").and_then(YamlValue::as_str),
        Some("pipeline")
    );
    assert_eq!(
        pipeline_frontmatter
            .get("status")
            .and_then(YamlValue::as_str),
        Some("active")
    );
    assert_eq!(
        pipeline_frontmatter
            .get("maxConcurrency")
            .and_then(YamlValue::as_u64),
        Some(4)
    );
    assert_eq!(
        pipeline_frontmatter
            .get("first")
            .and_then(YamlValue::as_u64),
        Some(0)
    );
    assert_eq!(
        pipeline_frontmatter.get("last").and_then(YamlValue::as_u64),
        Some(0)
    );
    assert_eq!(pipeline_content, "# Pipeline Session\n");

    let definition = pipeline_frontmatter
        .get("definition")
        .and_then(YamlValue::as_mapping)
        .expect("definition mapping");
    let definition_nodes = definition
        .get(YamlValue::String("nodes".to_string()))
        .and_then(YamlValue::as_sequence)
        .expect("definition nodes");
    assert_eq!(definition_nodes.len(), 2);

    let node_states = pipeline_frontmatter
        .get("nodeStates")
        .and_then(YamlValue::as_mapping)
        .expect("node states");
    assert_eq!(node_states.len(), 2);

    for node_value in nodes {
        let node_id = node_value
            .get("nodeId")
            .and_then(Value::as_str)
            .expect("node id");
        let workflow_id = node_value
            .get("workflowId")
            .and_then(Value::as_str)
            .expect("workflow id");
        let session_id = node_value
            .get("sessionId")
            .and_then(Value::as_str)
            .expect("session id");

        assert!(session_id.starts_with("workflow-"));
        assert!(session_id.ends_with(&format!("-{node_id}")));

        let state = node_states
            .get(YamlValue::String(node_id.to_string()))
            .and_then(YamlValue::as_mapping)
            .expect("node state");
        assert_eq!(
            state
                .get(YamlValue::String("status".to_string()))
                .and_then(YamlValue::as_str),
            Some("pending")
        );
        assert_eq!(
            state
                .get(YamlValue::String("attempts".to_string()))
                .and_then(YamlValue::as_u64),
            Some(0)
        );
        assert_eq!(
            state
                .get(YamlValue::String("sessionId".to_string()))
                .and_then(YamlValue::as_str),
            Some(session_id)
        );

        let workflow_uri = workflow_session_uri(session_id);
        let (workflow_frontmatter, workflow_content, _) =
            read_markdown(&workflow_uri, &context.config).expect("read workflow session");
        let workflow_frontmatter = workflow_frontmatter.expect("workflow frontmatter");

        assert_eq!(
            workflow_frontmatter.get("type").and_then(YamlValue::as_str),
            Some("workflow")
        );
        assert_eq!(
            workflow_frontmatter
                .get("status")
                .and_then(YamlValue::as_str),
            Some("pending")
        );
        assert_eq!(
            workflow_frontmatter
                .get("pipelineId")
                .and_then(YamlValue::as_str),
            Some(pipeline_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("nodeId")
                .and_then(YamlValue::as_str),
            Some(node_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("workflowId")
                .and_then(YamlValue::as_str),
            Some(workflow_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("queue")
                .and_then(YamlValue::as_sequence)
                .and_then(|queue| queue.first())
                .and_then(YamlValue::as_str),
            Some(workflow_id)
        );
        assert_eq!(
            workflow_frontmatter
                .get("currentWorkflow")
                .and_then(YamlValue::as_u64),
            Some(0)
        );
        assert_eq!(
            workflow_frontmatter
                .get("currentStep")
                .and_then(YamlValue::as_u64),
            Some(0)
        );
        assert_eq!(
            workflow_frontmatter
                .get("phase")
                .and_then(YamlValue::as_str),
            Some("running")
        );
        assert_eq!(workflow_content, "# Workflow Session\n");
    }
}

#[test]
#[serial(config)]
fn pipeline_create_accepts_mixed_node_types() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let payload = json!({
        "definition": {
            "nodes": [
                { "id": "root", "workflowId": "test-workflow-a", "acpClient": "agent-root" },
                {
                    "id": "prompt",
                    "type": "prompt",
                    "prompt": "Summarize the workflow result",
                    "agent": "prompt-agent"
                },
                {
                    "id": "thinking",
                    "type": "sequentialThinking",
                    "topic": "Review the workflow output",
                    "totalThoughts": 3,
                    "agent": "thinking-agent"
                },
                {
                    "id": "tool",
                    "type": "tool",
                    "toolName": "ReadMemory",
                    "toolArgs": { "identifier": "memory://build/test" },
                    "agent": "prompt-agent"
                }
            ],
            "edges": [
                { "source": "root", "target": "prompt" },
                { "source": "prompt", "target": "thinking" },
                { "source": "thinking", "target": "tool" }
            ]
        }
    });

    let output = pipeline(&context, &payload).expect("create mixed pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let pipeline_id = data
        .get("pipelineId")
        .and_then(Value::as_str)
        .expect("pipeline id");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes.len(), 4);
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("root".to_string()))
            && node.get("workflowId") == Some(&Value::String("test-workflow-a".to_string()))
    }));
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("prompt".to_string()))
            && node.get("workflowId") == Some(&Value::Null)
    }));
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("thinking".to_string()))
            && node.get("workflowId") == Some(&Value::Null)
    }));
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("tool".to_string()))
            && node.get("workflowId") == Some(&Value::Null)
    }));

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");

    assert_eq!(definition.nodes.len(), 4);
    assert!(definition.nodes.iter().any(
        |node| matches!(node, super::super::PipelineNode::Prompt { id, .. } if id == "prompt")
    ));
    assert!(definition.nodes.iter().any(|node| matches!(
        node,
        super::super::PipelineNode::SequentialThinking { id, total_thoughts, .. }
            if id == "thinking" && *total_thoughts == 3
    )));
    assert!(definition.nodes.iter().any(|node| matches!(
        node,
        super::super::PipelineNode::Tool { id, tool_name, tool_args, .. }
            if id == "tool"
                && tool_name == "ReadMemory"
                && tool_args.as_ref() == Some(&json!({"identifier": "memory://build/test"}))
    )));
}
