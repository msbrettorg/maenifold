use super::{EdgeType, PipelineDefinition, PipelineEdge, PipelineNode};

pub(super) fn sample_definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "a".to_string(),
                workflow_id: "workflow-a".to_string(),
                acp_client: Some("client-a".to_string()),
                prompt: None,

                timeout: None,
                retry_policy: None,
            },
            PipelineNode::Workflow {
                id: "b".to_string(),
                workflow_id: "workflow-b".to_string(),
                acp_client: Some("client-b".to_string()),
                prompt: None,

                timeout: None,
                retry_policy: None,
            },
            PipelineNode::Workflow {
                id: "c".to_string(),
                workflow_id: "workflow-c".to_string(),
                acp_client: Some("client-c".to_string()),
                prompt: None,

                timeout: None,
                retry_policy: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "a".to_string(),
                target: "b".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "b".to_string(),
                target: "c".to_string(),
                edge_type: EdgeType::Always,
                mapping: None,
            },
        ],
    }
}

pub(super) fn mixed_definition() -> PipelineDefinition {
    PipelineDefinition {
        nodes: vec![
            PipelineNode::Workflow {
                id: "workflow".to_string(),
                workflow_id: "workflow-a".to_string(),
                acp_client: Some("workflow-agent".to_string()),
                prompt: None,

                timeout: None,
                retry_policy: None,
            },
            PipelineNode::Prompt {
                id: "prompt".to_string(),
                prompt: "Summarize the workflow output".to_string(),
                agent: "prompt-agent".to_string(),
                tool_mode: "normal".to_string(),
                agent_config: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,

                timeout: None,
                retry_policy: None,
            },
            PipelineNode::SequentialThinking {
                id: "thinking".to_string(),
                topic: "Pipeline validation".to_string(),
                total_thoughts: 3,
                analysis_type: Some("analysis".to_string()),
                agent: "thinking-agent".to_string(),
                agent_config: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,

                timeout: None,
                retry_policy: None,
            },
        ],
        edges: vec![
            PipelineEdge {
                source: "workflow".to_string(),
                target: "prompt".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
            PipelineEdge {
                source: "prompt".to_string(),
                target: "thinking".to_string(),
                edge_type: EdgeType::Success,
                mapping: None,
            },
        ],
    }
}
