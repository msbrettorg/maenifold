use super::*;

#[test]
#[serial(config)]
fn pipeline_view_includes_assembled_prompt_for_active_and_completed_acp_nodes() {
    let (_tmp, context, _reset) = pipeline_test_context(OutputFormat::Json);
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "workflow-node".to_string(),
                workflow_id: "test-workflow-a".to_string(),
                acp_client: Some("agent-workflow".to_string()),
                prompt: Some("Summarize the upstream output".to_string()),
            
                timeout: None},
            prompt_node("prompt-node", "Draft operator guidance", "agent-prompt"),
            thinking_node(
                "thinking-node",
                "diagnose what happened",
                4,
                Some("diagnostic"),
                "agent-think",
            ),
            tool_node("tool-node", "ReadMemory", None, None),
        ],
        edges: vec![
            edge("workflow-node", "prompt-node", EdgeType::Success),
            edge("prompt-node", "thinking-node", EdgeType::Success),
            edge("thinking-node", "tool-node", EdgeType::Success),
        ],
    };
    let node_states = HashMap::from([
        (
            "workflow-node".to_string(),
            node_state(
                "workflow-session",
                NodeStatus::Completed,
                1,
                Some("workflow conclusion"),
            ),
        ),
        (
            "prompt-node".to_string(),
            node_state(
                "prompt-session",
                NodeStatus::Completed,
                1,
                Some("prompt answer"),
            ),
        ),
        (
            "thinking-node".to_string(),
            node_state("thinking-session", NodeStatus::Active, 1, None),
        ),
        (
            "tool-node".to_string(),
            node_state(
                "tool-session",
                NodeStatus::Completed,
                1,
                Some("tool output"),
            ),
        ),
    ]);

    let result = build_pipeline_view_result(
        &context,
        "pipeline-assembled-prompt",
        &definition,
        &node_states,
        Some("Keep the response concise"),
    )
    .expect("build pipeline view");
    let nodes = result
        .get("nodes")
        .and_then(Value::as_array)
        .expect("nodes array");

    assert_eq!(nodes[0]["assembled_prompt"], Value::Null);
    assert_eq!(
        nodes[1]["assembled_prompt"],
        "### From workflow-node\n\nworkflow conclusion\n\nKeep the response concise\n\nDraft operator guidance"
    );
    assert_eq!(
        nodes[2]["assembled_prompt"],
        "### From prompt-node\n\nprompt answer\n\nKeep the response concise\n\ndiagnose what happened"
    );
    assert_eq!(nodes[3]["assembled_prompt"], Value::Null);
}
