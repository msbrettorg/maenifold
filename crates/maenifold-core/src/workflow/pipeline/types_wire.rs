use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{AgentConfig, PipelineNode, RetryPolicy};

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
    cwd: Option<std::path::PathBuf>,
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
    #[serde(default, rename = "retryMaxAttempts", skip_serializing_if = "Option::is_none")]
    retry_max_attempts: Option<u32>,
    #[serde(
        default,
        rename = "retryBackoffMinMs",
        skip_serializing_if = "Option::is_none"
    )]
    retry_backoff_min_ms: Option<u64>,
    #[serde(
        default,
        rename = "retryBackoffMaxMs",
        skip_serializing_if = "Option::is_none"
    )]
    retry_backoff_max_ms: Option<u64>,
    #[serde(default, rename = "retryJitter", skip_serializing_if = "Option::is_none")]
    retry_jitter: Option<bool>,
}

impl TryFrom<PipelineNodeWire> for PipelineNode {
    type Error = String;

    #[allow(clippy::too_many_lines)]
    fn try_from(wire: PipelineNodeWire) -> Result<Self, Self::Error> {
        let node_id = wire.id.clone();
        let retry_policy = parse_retry_policy(&wire, &node_id)?;

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
                acp_client: wire.acp_client,
                prompt: wire.prompt,
                cwd: wire.cwd,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
                retry_policy,
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
                cwd: wire.cwd,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
                retry_policy,
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
                cwd: wire.cwd,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
                retry_policy,
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
                cwd: wire.cwd,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
                retry_policy,
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
                cwd,
                timeout,
                agent_config,
                retry_policy,
            } => Self {
                kind: Some("workflow".to_string()),
                id,
                display_label,
                workflow_id: Some(workflow_id),
                acp_client,
                prompt,
                cwd,
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
                retry_max_attempts: retry_policy.as_ref().map(|policy| policy.max_attempts),
                retry_backoff_min_ms: retry_policy.as_ref().map(|policy| policy.backoff_min_ms),
                retry_backoff_max_ms: retry_policy.as_ref().map(|policy| policy.backoff_max_ms),
                retry_jitter: retry_policy.as_ref().map(|policy| policy.jitter),
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
                cwd,
                timeout,
                agent_config,
                retry_policy,
            } => Self {
                kind: Some("prompt".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: Some(prompt),
                cwd,
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
                retry_max_attempts: retry_policy.as_ref().map(|policy| policy.max_attempts),
                retry_backoff_min_ms: retry_policy.as_ref().map(|policy| policy.backoff_min_ms),
                retry_backoff_max_ms: retry_policy.as_ref().map(|policy| policy.backoff_max_ms),
                retry_jitter: retry_policy.as_ref().map(|policy| policy.jitter),
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
                cwd,
                timeout,
                agent_config,
                retry_policy,
            } => Self {
                kind: Some("sequentialThinking".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                cwd,
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
                retry_max_attempts: retry_policy.as_ref().map(|policy| policy.max_attempts),
                retry_backoff_min_ms: retry_policy.as_ref().map(|policy| policy.backoff_min_ms),
                retry_backoff_max_ms: retry_policy.as_ref().map(|policy| policy.backoff_max_ms),
                retry_jitter: retry_policy.as_ref().map(|policy| policy.jitter),
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
                cwd,
                timeout,
                agent_config,
                retry_policy,
            } => Self {
                kind: Some("tool".to_string()),
                id,
                display_label,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                cwd,
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
                retry_max_attempts: retry_policy.as_ref().map(|policy| policy.max_attempts),
                retry_backoff_min_ms: retry_policy.as_ref().map(|policy| policy.backoff_min_ms),
                retry_backoff_max_ms: retry_policy.as_ref().map(|policy| policy.backoff_max_ms),
                retry_jitter: retry_policy.as_ref().map(|policy| policy.jitter),
            },
        }
    }
}

fn parse_retry_policy(wire: &PipelineNodeWire, node_id: &str) -> Result<Option<RetryPolicy>, String> {
    let has_retry_field = wire.retry_max_attempts.is_some()
        || wire.retry_backoff_min_ms.is_some()
        || wire.retry_backoff_max_ms.is_some()
        || wire.retry_jitter.is_some();

    if !has_retry_field {
        return Ok(None);
    }

    let max_attempts = wire.retry_max_attempts.ok_or_else(|| {
        format!("Node '{node_id}': retryMaxAttempts is required when retry fields are present")
    })?;
    let backoff_min_ms = wire.retry_backoff_min_ms.ok_or_else(|| {
        format!("Node '{node_id}': retryBackoffMinMs is required when retry fields are present")
    })?;
    let backoff_max_ms = wire.retry_backoff_max_ms.ok_or_else(|| {
        format!("Node '{node_id}': retryBackoffMaxMs is required when retry fields are present")
    })?;
    let jitter = wire.retry_jitter.ok_or_else(|| {
        format!("Node '{node_id}': retryJitter is required when retry fields are present")
    })?;

    if backoff_min_ms > backoff_max_ms {
        return Err(format!(
            "Node '{node_id}': retryBackoffMinMs ({backoff_min_ms}) must be <= retryBackoffMaxMs ({backoff_max_ms})"
        ));
    }

    Ok(Some(RetryPolicy {
        max_attempts,
        backoff_min_ms,
        backoff_max_ms,
        jitter,
    }))
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

    use super::{super::types::PipelineNode, PipelineNodeWire};

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
            PipelineNode::Tool { timeout, .. }
 => assert_eq!(timeout, Some(21)),
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
            cwd: None,
            timeout: Some(21),
            agent_config: None,
            retry_policy: None,
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
            PipelineNode::Prompt { display_label, .. }
 => {
                assert_eq!(display_label.as_deref(), Some("Prompt label"));
            }
            other => panic!("expected prompt node, got {other:?}"),
        }

        let value = serde_json::to_value(node).expect("serialize prompt node");
        assert_eq!(value["displayLabel"], json!("Prompt label"));
    }

    #[test]
    fn prompt_node_round_trips_cwd() {
        let node: PipelineNode = serde_json::from_value(json!({
            "id": "prompt",
            "type": "prompt",
            "prompt": "Say hi",
            "agent": "agent",
            "cwd": "/tmp/workspace"
        }))
        .expect("deserialize prompt node");

        match &node {
            PipelineNode::Prompt { cwd, .. }
 => {
                assert_eq!(
                    cwd.as_deref().and_then(|path| path.to_str()),
                    Some("/tmp/workspace")
                );
            }
            other => panic!("expected prompt node, got {other:?}"),
        }

        let value = serde_json::to_value(node).expect("serialize prompt node");
        assert_eq!(value["cwd"], json!("/tmp/workspace"));
    }

    #[test]
    fn test_workflow_wire_round_trip_with_agent_config() {
        let json = r#"{
            "id": "workflow-node",
            "type": "workflow",
            "workflowId": "wf-123",
            "acpClient": "acp-client",
            "prompt": "Run workflow",
            "timeout": 30,
            "agentConfig": {
                "model": "gpt-5",
                "mode": "chat",
                "thoughtLevel": "high",
                "configOptions": {
                    "temperature": 0.2,
                    "stream": true
                }
            }
        }"#;

        let wire: PipelineNodeWire = serde_json::from_str(json).expect("deserialize workflow wire");
        let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");
        let wire = PipelineNodeWire::from(node);
        let value = serde_json::to_value(&wire).expect("serialize workflow wire");

        assert_eq!(value["workflowId"], json!("wf-123"));
        assert_eq!(value["acpClient"], json!("acp-client"));
        assert_eq!(value["agentConfig"]["model"], json!("gpt-5"));
        assert_eq!(value["agentConfig"]["thoughtLevel"], json!("high"));
    }

    #[test]
    fn test_workflow_wire_without_agent_config() {
        let json = r#"{
            "id": "workflow-node",
            "type": "workflow",
            "workflowId": "wf-123",
            "acpClient": "acp-client"
        }"#;

        let wire: PipelineNodeWire =
            serde_json::from_str(json).expect("deserialize workflow wire without agent config");
        let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");

        match node {
            PipelineNode::Workflow { agent_config, .. }
 => assert!(agent_config.is_none()),
            other => panic!("expected workflow node, got {other:?}"),
        }
    }

    #[test]
    fn test_workflow_wire_without_acp_client() {
        let json = r#"{
            "id": "workflow-node",
            "type": "workflow",
            "workflowId": "wf-123"
        }"#;

        let wire: PipelineNodeWire =
            serde_json::from_str(json).expect("deserialize workflow wire without acp client");
        let node = PipelineNode::try_from(wire).expect("convert workflow wire to domain node");

        match node {
            PipelineNode::Workflow { acp_client, .. }
 => assert!(acp_client.is_none()),
            other => panic!("expected workflow node, got {other:?}"),
        }
    }
}
