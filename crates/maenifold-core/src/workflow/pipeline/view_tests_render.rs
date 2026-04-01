use super::*;

#[test]
#[serial(config)]
fn pipeline_view_returns_structured_json_and_mermaid() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let session_id = "pipeline-1735689600000";
    let definition = seed_pipeline_view_session(&context, session_id, "pipeline");

    let output = pipeline(&context, &json!({ "sessionId": session_id, "view": true }))
        .expect("view pipeline");
    let response: Value = serde_json::from_str(&output).expect("parse response");
    let data = response.get("data").expect("response data");

    assert_eq!(
        data.get("pipelineId").and_then(Value::as_str),
        Some(session_id)
    );
    assert_eq!(data.get("status").and_then(Value::as_str), Some("active"));
    assert_eq!(
        data.get("ready").and_then(Value::as_array),
        Some(&vec![Value::String("child".to_string())])
    );
    assert_eq!(
        data.get("active").and_then(Value::as_array),
        Some(&vec![Value::String("cleanup".to_string())])
    );
    assert_eq!(
        data.get("completed").and_then(Value::as_array),
        Some(&vec![Value::String("root".to_string())])
    );

    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");
    assert_eq!(nodes.len(), definition.nodes.len());
    assert_eq!(nodes[0]["nodeId"], "root");
    assert_eq!(nodes[0]["type"], "workflow");
    assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
    assert_eq!(nodes[0]["sessionId"], "workflow-1735689600000-root");
    assert_eq!(nodes[0]["status"], "completed");
    assert_eq!(nodes[0]["attempts"], 1);
    assert_eq!(nodes[1]["status"], "active");
    assert_eq!(nodes[2]["status"], "pending");

    let edges = data
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges array");
    assert_eq!(edges.len(), definition.edges.len());
    assert_eq!(edges[0]["type"], "success");
    assert_eq!(edges[1]["type"], "always");

    let mermaid = data
        .get("mermaid")
        .and_then(Value::as_str)
        .expect("mermaid string");
    assert!(mermaid.starts_with("graph LR\n"));
    assert!(mermaid.contains("root[\"✅ wf: test-workflow-a\"]"));
    assert!(mermaid.contains("cleanup[\"🔄 wf: test-workflow-b\"]"));
    assert!(mermaid.contains("child[\"⏳ wf: test-workflow-b\"]"));
    assert!(mermaid.contains("root --> child"));
    assert!(mermaid.contains("root -.-> cleanup"));
    assert_eq!(data.get("running").and_then(Value::as_bool), Some(false));
    assert!(data.get("pid").is_none() || data.get("pid") == Some(&Value::Null));
    assert!(data.get("attempts").is_none() || data.get("attempts") == Some(&Value::Null));
}

#[test]
#[serial(config)]
fn pipeline_view_renders_mixed_node_types() {
    let definition = PipelineDefinition {
        nodes: vec![
            workflow_node("workflow-node", "test-workflow-a"),
            prompt_node(
                "prompt-node",
                "Summarize the pipeline status and propose next steps for the operator",
                "agent-prompt",
            ),
            thinking_node(
                "thinking-node",
                "pipeline triage",
                3,
                Some("diagnostic"),
                "agent-think",
            ),
            tool_node(
                "tool-node",
                "ReadMemory",
                Some(json!({"identifier": "memory://build/test"})),
                None,
            ),
        ],
        edges: vec![
            edge("workflow-node", "prompt-node", EdgeType::Success),
            edge("prompt-node", "thinking-node", EdgeType::Always),
            edge("thinking-node", "tool-node", EdgeType::Success),
        ],
    };
    let node_states = HashMap::from([
        (
            "workflow-node".to_string(),
            node_state("workflow-session", NodeStatus::Completed, 1, None),
        ),
        (
            "prompt-node".to_string(),
            node_state(
                "prompt-session",
                NodeStatus::Active,
                2,
                Some("prompt output"),
            ),
        ),
        (
            "thinking-node".to_string(),
            node_state("thinking-session", NodeStatus::Ready, 0, None),
        ),
        (
            "tool-node".to_string(),
            node_state("tool-session", NodeStatus::Pending, 0, None),
        ),
    ]);

    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let result = build_pipeline_view_result(
        &context,
        "pipeline-mixed-nodes",
        &definition,
        &node_states,
        Some("stay focused"),
    )
    .expect("build pipeline view");
    let nodes = result
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes[0]["type"], "workflow");
    assert_eq!(nodes[0]["workflowId"], "test-workflow-a");
    assert_eq!(nodes[0]["agent"], "workflow-agent");
    assert_eq!(nodes[0]["prompt"], Value::Null);
    assert!(nodes[0].get("topic").is_none());
    assert_eq!(nodes[0]["status"], "completed");
    assert_eq!(nodes[0]["attempts"], 1);
    assert_eq!(nodes[0]["output"], Value::Null);
    assert_eq!(nodes[0]["workflow_steps"], Value::Null);

    assert_eq!(nodes[1]["type"], "prompt");
    assert_eq!(nodes[1]["agent"], "agent-prompt");
    assert_eq!(
        nodes[1]["prompt"],
        "Summarize the pipeline status and propose next steps for the operator"
    );
    assert!(nodes[1].get("workflowId").is_none());
    assert!(nodes[1].get("topic").is_none());
    assert_eq!(nodes[1]["status"], "active");
    assert_eq!(nodes[1]["attempts"], 2);
    assert_eq!(nodes[1]["output"], "prompt output");
    assert_eq!(nodes[1]["assembled_prompt"], Value::Null);
    assert_eq!(nodes[1]["workflow_steps"], Value::Null);

    assert_eq!(nodes[2]["type"], "sequentialThinking");
    assert_eq!(nodes[2]["agent"], "agent-think");
    assert_eq!(nodes[2]["topic"], "pipeline triage");
    assert_eq!(nodes[2]["totalThoughts"], 3);
    assert_eq!(nodes[2]["analysisType"], "diagnostic");
    assert!(nodes[2].get("workflowId").is_none());
    assert_eq!(nodes[2]["status"], "ready");
    assert_eq!(nodes[2]["attempts"], 0);
    assert_eq!(nodes[2]["output"], Value::Null);
    assert_eq!(nodes[2]["assembled_prompt"], Value::Null);
    assert_eq!(nodes[2]["workflow_steps"], Value::Null);

    assert_eq!(nodes[3]["type"], "tool");
    assert_eq!(nodes[3]["agent"], Value::Null);
    assert_eq!(nodes[3]["toolName"], "ReadMemory");
    assert_eq!(
        nodes[3]["toolArgs"],
        json!({"identifier": "memory://build/test"})
    );
    assert!(nodes[3].get("workflowId").is_none());
    assert_eq!(nodes[3]["status"], "pending");
    assert_eq!(nodes[3]["attempts"], 0);
    assert_eq!(nodes[3]["output"], Value::Null);
    assert_eq!(nodes[3]["assembled_prompt"], Value::Null);
    assert_eq!(nodes[3]["workflow_steps"], Value::Null);

    let mermaid = result
        .get("mermaid")
        .and_then(Value::as_str)
        .expect("mermaid string");
    assert!(mermaid.contains("workflow-node[\"✅ wf: test-workflow-a\"]"));
    assert!(
        mermaid.contains("prompt-node(\"🔄 prompt: Summarize the pipeline status and propos...\")")
    );
    assert!(mermaid.contains("thinking-node{{\"🟡 think: pipeline triage\"}}"));
    assert!(mermaid.contains("tool-node>⏳ tool: ReadMemory]"));
    assert!(mermaid.contains("workflow-node --> prompt-node"));
    assert!(mermaid.contains("prompt-node -.-> thinking-node"));
    assert!(mermaid.contains("thinking-node --> tool-node"));
}
