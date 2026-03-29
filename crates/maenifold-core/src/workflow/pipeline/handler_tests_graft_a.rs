use super::*;

#[test]
#[serial(config)]
fn pipeline_graft_completed_pipeline_adds_node_and_persists_sessions() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600051";
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
                session_id: "workflow-1735689600051-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let output = pipeline(
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
    )
    .expect("graft succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    let child = nodes
        .iter()
        .find(|node| node.get("nodeId") == Some(&Value::String("child".to_string())))
        .expect("child node");
    let child_session_id = child
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("child session id");

    assert_eq!(nodes.len(), 2);
    assert_eq!(
        child.get("workflowId").and_then(Value::as_str),
        Some("test-workflow-b")
    );
    assert_eq!(child.get("status").and_then(Value::as_str), Some("pending"));
    assert!(
        file_exists(&workflow_session_uri(child_session_id), &context.config)
            .expect("workflow exists")
    );

    let (frontmatter, _, _) =
        read_markdown(&pipeline_session_uri(pipeline_id), &context.config).expect("read pipeline");
    let frontmatter = frontmatter.expect("pipeline frontmatter");
    let definition: PipelineDefinition =
        frontmatter_value(&frontmatter, "definition").expect("pipeline definition");
    assert_eq!(definition.nodes.len(), 2);
    assert_eq!(definition.edges.len(), 1);
    assert_eq!(
        frontmatter.get("status").and_then(YamlValue::as_str),
        Some("active")
    );
}

#[test]
#[serial(config)]
fn pipeline_graft_accepts_new_node_types() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let pipeline_id = "pipeline-1735689600051-mixed";
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
                session_id: "workflow-1735689600051-mixed-root".to_string(),
                status: NodeStatus::Completed,
                attempts: 1,
                thinking_session_id: None,
                output: None,
            },
        )]),
        2,
    );
    set_pipeline_status(&context, pipeline_id, "completed");

    let output = pipeline(
        &context,
        &json!({
            "sessionId": pipeline_id,
            "graft": {
                "nodes": [
                    {
                        "id": "prompt",
                        "type": "prompt",
                        "prompt": "Summarize the root output",
                        "agent": "prompt-agent"
                    },
                    {
                        "id": "thinking",
                        "type": "sequentialThinking",
                        "topic": "Review the root result",
                        "totalThoughts": 2,
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
        }),
    )
    .expect("graft mixed nodes succeeds");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let nodes = response
        .get("data")
        .and_then(|data| data.get("nodes"))
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes.len(), 4);
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("prompt".to_string()))
            && node.get("agent") == Some(&Value::String("prompt-agent".to_string()))
    }));
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("thinking".to_string()))
            && node.get("agent") == Some(&Value::String("thinking-agent".to_string()))
            && node.get("topic") == Some(&Value::String("Review the root result".to_string()))
    }));
    assert!(nodes.iter().any(|node| {
        node.get("nodeId") == Some(&Value::String("tool".to_string()))
            && node.get("agent") == Some(&Value::String("prompt-agent".to_string()))
            && node.get("toolName") == Some(&Value::String("ReadMemory".to_string()))
            && node.get("toolArgs") == Some(&json!({"identifier": "memory://build/test"}))
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
            if id == "thinking" && *total_thoughts == 2
    )));
    assert!(definition.nodes.iter().any(|node| matches!(
        node,
        super::super::PipelineNode::Tool { id, tool_name, tool_args, .. }
            if id == "tool"
                && tool_name == "ReadMemory"
                && tool_args.as_ref() == Some(&json!({"identifier": "memory://build/test"}))
    )));
}
