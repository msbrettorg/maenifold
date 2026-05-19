use std::collections::HashMap;

use serde_json::json;

use super::{EdgeDataMode, EdgeType, PipelineDefinition, PipelineEdge, PipelineNode, validate_pipeline};

#[test]
fn tool_output_schema_allows_binding_validation_to_prompt_input_schema() {
    let definition = PipelineDefinition {
        nodes: vec![
            PipelineNode::Tool {
                id: "memory-status".to_string(),
                display_label: None,
                tool_name: "GatewayExecute".to_string(),
                tool_args: None,
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                retry_policy: None,
                join_policy: None,
                input_schema: None,
                output_schema: Some(json!({
                    "type": "object",
                    "required": ["data"],
                    "properties": { "data": { "type": "string" } }
                })),
            },
            PipelineNode::Prompt {
                id: "inspect-memory".to_string(),
                display_label: None,
                prompt: "inspect".to_string(),
                agent: "copilot".to_string(),
                tool_mode: "disabled".to_string(),
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                cwd: None,
                timeout: None,
                agent_config: None,
                adopt: None,
                requires_role: None,
                retry_policy: None,
                session_ref: None,
                join_policy: None,
                input_schema: Some(json!({
                    "type": "object",
                    "required": ["memory_status_text"],
                    "properties": { "memory_status_text": { "type": "string" } }
                })),
                output_schema: None,
            },
        ],
        edges: vec![PipelineEdge {
            source: "memory-status".to_string(),
            target: "inspect-memory".to_string(),
            edge_type: EdgeType::Success,
            mode: EdgeDataMode::Binding,
            mapping: Some(HashMap::from([(
                "inputs.memory_status_text".to_string(),
                "data".to_string(),
            )])),
        }],
        inputs: None,
        input_schema: None,
        template_id: None,
    };

    validate_pipeline(&definition).expect("tool schema binding should validate");
}
