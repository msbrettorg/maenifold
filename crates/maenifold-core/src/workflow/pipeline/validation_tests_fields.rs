use super::{PipelineDefinition, PipelineNode, validate_pipeline};

#[test]
#[allow(clippy::too_many_lines)]
fn validate_pipeline_rejects_invalid_type_specific_fields() {
    let cases = vec![
        (
            PipelineNode::Workflow {
                id: "workflow-node".to_string(),
                workflow_id: "   ".to_string(),
                acp_client: None,
                prompt: None,
            
                timeout: None},
            "workflow-node",
            "workflowId",
        ),
        (
            PipelineNode::Prompt {
                id: "prompt-node".to_string(),
                prompt: "   ".to_string(),
                agent: "prompt-agent".to_string(),
                tool_mode: "normal".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
            
                timeout: None},
            "prompt-node",
            "prompt",
        ),
        (
            PipelineNode::Prompt {
                id: "prompt-agent-node".to_string(),
                prompt: "Prompt text".to_string(),
                agent: "\n\t".to_string(),
                tool_mode: "normal".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
            
                timeout: None},
            "prompt-agent-node",
            "agent",
        ),
        (
            PipelineNode::SequentialThinking {
                id: "thinking-topic-node".to_string(),
                topic: "   ".to_string(),
                total_thoughts: 2,
                analysis_type: None,
                agent: "thinking-agent".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
            
                timeout: None},
            "thinking-topic-node",
            "topic",
        ),
        (
            PipelineNode::SequentialThinking {
                id: "thinking-agent-node".to_string(),
                topic: "Topic".to_string(),
                total_thoughts: 2,
                analysis_type: None,
                agent: "   ".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
            
                timeout: None},
            "thinking-agent-node",
            "agent",
        ),
        (
            PipelineNode::SequentialThinking {
                id: "thinking-count-node".to_string(),
                topic: "Topic".to_string(),
                total_thoughts: 0,
                analysis_type: None,
                agent: "thinking-agent".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
            
                timeout: None},
            "thinking-count-node",
            "totalThoughts",
        ),
    ];

    for (node, node_id, field) in cases {
        let definition = PipelineDefinition {
            nodes: vec![node],
            edges: vec![],
        };

        let error = validate_pipeline(&definition).expect_err("invalid node should fail");
        let message = error.to_string();
        assert!(
            message.contains(node_id),
            "expected error to mention node ID '{node_id}', got: {message}"
        );
        assert!(
            message.contains(field),
            "expected error to mention field '{field}', got: {message}"
        );
    }
}
