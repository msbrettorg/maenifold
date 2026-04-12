use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{AgentConfig, PipelineNode};

const EXPECTED_TYPES: &str = "workflow, prompt, sequentialThinking, tool";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct PipelineNodeWire {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    id: String,
    #[serde(
        default,
        rename = "displayLabel",
        skip_serializing_if = "Option::is_none"
    )]
    display_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acp_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    #[serde(
        default,
        rename = "agentConfig",
        skip_serializing_if = "Option::is_none"
    )]
    agent_config: Option<AgentConfig>,
    #[serde(
        default = "default_prompt_tool_mode",
        skip_serializing_if = "is_default_prompt_tool_mode"
    )]
    tool_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    topic: Option<String>,
    #[serde(
        default = "default_total_thoughts",
        skip_serializing_if = "is_default_total_thoughts"
    )]
    total_thoughts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_name: Option<String>,
    #[serde(rename = "toolArgs", skip_serializing_if = "Option::is_none")]
    tool_args: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(
        default,
        rename = "thoughtLevel",
        skip_serializing_if = "Option::is_none"
    )]
    thought_level: Option<String>,
    #[serde(
        default,
        rename = "allowedTools",
        skip_serializing_if = "Option::is_none"
    )]
    allowed_tools: Option<Vec<String>>,
    #[serde(default, rename = "maxTurns", skip_serializing_if = "Option::is_none")]
    max_turns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
}

impl TryFrom<PipelineNodeWire> for PipelineNode {
    type Error = String;

    #[allow(clippy::too_many_lines)]
    fn try_from(wire: PipelineNodeWire) -> Result<Self, Self::Error> {
        let node_id = wire.id.clone();

        let required = |value: Option<String>, name: &str| -> Result<String, String> {
            value.ok_or_else(|| format!("Node '{node_id}': missing '{name}' field"))
        };

        let kind_owned = wire.kind.clone();
        let kind: &str = match kind_owned.as_deref() {
            None => {
                if wire.workflow_id.is_some() {
                    "workflow"
                } else {
                    return Err(format!(
                        "Node '{node_id}': missing 'type' field. Expected one of: {EXPECTED_TYPES}"
                    ));
                }
            }
            Some(kind) => kind,
        };

        Ok(match kind {
            "workflow" => Self::Workflow {
                id: wire.id,
                display_label: wire.display_label,
                workflow_id: required(wire.workflow_id, "workflowId")?,
                acp_client: Some(required(wire.acp_client, "acpClient")?),
                prompt: wire.prompt,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
            },
            "prompt" => Self::Prompt {
                id: wire.id,
                display_label: wire.display_label,
                prompt: required(wire.prompt, "prompt")?,
                agent: required(wire.agent, "agent")?,
                tool_mode: wire.tool_mode,
                model: wire.model,
                thought_level: wire.thought_level,
                allowed_tools: wire.allowed_tools,
                max_turns: wire.max_turns,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
            },
            "sequentialThinking" => Self::SequentialThinking {
                id: wire.id,
                display_label: wire.display_label,
                topic: required(wire.topic, "topic")?,
                total_thoughts: wire.total_thoughts,
                analysis_type: wire.analysis_type,
                agent: required(wire.agent, "agent")?,
                model: wire.model,
                thought_level: wire.thought_level,
                allowed_tools: wire.allowed_tools,
                max_turns: wire.max_turns,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
            },
            "tool" => Self::Tool {
                id: wire.id,
                display_label: wire.display_label,
                tool_name: required(wire.tool_name, "toolName")?,
                tool_args: wire.tool_args,
                agent: wire.agent,
                model: wire.model,
                thought_level: wire.thought_level,
                allowed_tools: wire.allowed_tools,
                max_turns: wire.max_turns,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
            },
            other => {
                return Err(format!(
                    "Node '{node_id}': unknown type '{other}'. Expected one of: {EXPECTED_TYPES}"
                ));
            }
        })
    }
}

impl From<PipelineNode> for PipelineNodeWire {
    #[allow(clippy::too_many_lines)]
    fn from(node: PipelineNode) -> Self {
        match node {
            PipelineNode::Workflow {
                id,
                display_label,
                workflow_id,
                acp_client,
                prompt,
                timeout,
                agent_config,
            } => Self {
                kind: Some("workflow".to_string()),
                id,
                display_label,
                workflow_id: Some(workflow_id),
                acp_client,
                prompt,
                agent: None,
                agent_config,
                tool_mode: default_prompt_tool_mode(),
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: None,
                tool_args: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                timeout,
            },
            PipelineNode::Prompt {
                id,
                display_label,
                prompt,
                agent,
                tool_mode,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
                agent_config,
            } => Self {
                kind: Some("prompt".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: Some(prompt),
                agent: Some(agent),
                agent_config,
                tool_mode,
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: None,
                tool_args: None,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
            },
            PipelineNode::SequentialThinking {
                id,
                display_label,
                topic,
                total_thoughts,
                analysis_type,
                agent,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
                agent_config,
            } => Self {
                kind: Some("sequentialThinking".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                agent: Some(agent),
                agent_config,
                tool_mode: default_prompt_tool_mode(),
                topic: Some(topic),
                total_thoughts,
                analysis_type,
                tool_name: None,
                tool_args: None,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
            },
            PipelineNode::Tool {
                id,
                display_label,
                tool_name,
                tool_args,
                agent,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
                agent_config,
            } => Self {
                kind: Some("tool".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                agent,
                agent_config,
                tool_mode: default_prompt_tool_mode(),
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: Some(tool_name),
                tool_args,
                model,
                thought_level,
                allowed_tools,
                max_turns,
                timeout,
            },
        }
    }
}

fn default_prompt_tool_mode() -> String {
    "disabled".to_string()
}

fn is_default_prompt_tool_mode(value: &String) -> bool {
    value == "disabled"
}

fn default_total_thoughts() -> u32 {
    5
}

fn is_default_total_thoughts(value: &u32) -> bool {
    *value == 5
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::types::PipelineNode;

    #[test]
    fn tool_node_deserialization_preserves_timeout() {
        let node: PipelineNode = serde_json::from_value(json!({
            "id": "tool",
            "type": "tool",
            "toolName": "GetConfig",
            "timeout": 21
        }))
        .expect("deserialize tool node");

        match node {
            PipelineNode::Tool { timeout, .. } => assert_eq!(timeout, Some(21)),
            other => panic!("expected tool node, got {other:?}"),
        }
    }

    #[test]
    fn tool_node_serialization_emits_timeout() {
        let node = PipelineNode::Tool {
            id: "tool".to_string(),
            display_label: None,
            tool_name: "GetConfig".to_string(),
            tool_args: None,
            agent: None,
            model: None,
            thought_level: None,
            allowed_tools: None,
            max_turns: None,
            timeout: Some(21),
            agent_config: None,
        };

        let value = serde_json::to_value(node).expect("serialize tool node");

        assert_eq!(value["timeout"], json!(21));
    }

    #[test]
    fn prompt_node_round_trips_display_label() {
        let node: PipelineNode = serde_json::from_value(json!({
            "id": "prompt",
            "type": "prompt",
            "displayLabel": "Prompt label",
            "prompt": "Say hi",
            "agent": "agent"
        }))
        .expect("deserialize prompt node");

        match &node {
            PipelineNode::Prompt { display_label, .. } => {
                assert_eq!(display_label.as_deref(), Some("Prompt label"));
            }
            other => panic!("expected prompt node, got {other:?}"),
        }

        let value = serde_json::to_value(node).expect("serialize prompt node");
        assert_eq!(value["displayLabel"], json!("Prompt label"));
    }
}
