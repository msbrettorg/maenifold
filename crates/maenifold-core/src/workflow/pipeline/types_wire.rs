use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{AgentConfig, JoinPolicy, NodeAdopt, PipelineNode, RetryPolicy, SessionRef};

const EXPECTED_TYPES: &str = "workflow, prompt, sequentialThinking, tool";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adopt: Option<NodeAdopt>,
    #[serde(
        default,
        rename = "joinPolicy",
        skip_serializing_if = "is_default_join_policy"
    )]
    join_policy: Option<JoinPolicy>,
    #[serde(
        default,
        rename = "requiresRole",
        skip_serializing_if = "Option::is_none"
    )]
    requires_role: Option<String>,
    #[serde(
        default,
        rename = "sessionRef",
        skip_serializing_if = "Option::is_none"
    )]
    session_ref: Option<SessionRef>,
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
    #[serde(
        default,
        rename = "retryMaxAttempts",
        skip_serializing_if = "Option::is_none"
    )]
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
    #[serde(
        default,
        rename = "retryJitter",
        skip_serializing_if = "Option::is_none"
    )]
    retry_jitter: Option<bool>,
}

impl TryFrom<PipelineNodeWire> for PipelineNode {
    type Error = String;

    #[allow(clippy::too_many_lines)]
    fn try_from(wire: PipelineNodeWire) -> Result<Self, Self::Error> {
        let node_id = wire.id.clone();
        let retry_policy = parse_retry_policy(&wire, &node_id)?;
        let join_policy = wire.join_policy.clone();

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
                adopt: wire.adopt,
                requires_role: wire.requires_role,
                retry_policy,
                join_policy,
            },
            "prompt" => {
                let prompt = required(wire.prompt, "prompt")?;
                let agent = required(wire.agent, "agent")?;
                if let Some(session_ref) = &wire.session_ref
                    && session_ref.agent != agent
                {
                    return Err(format!(
                        "Node '{}': sessionRef.agent ('{}') must match node agent ('{}')",
                        node_id, session_ref.agent, agent
                    ));
                }

                Self::Prompt {
                    id: wire.id,
                    display_label: wire.display_label,
                    prompt,
                    agent,
                    tool_mode: wire.tool_mode,
                    model: wire.model,
                    thought_level: wire.thought_level,
                    allowed_tools: wire.allowed_tools,
                    max_turns: wire.max_turns,
                    cwd: wire.cwd,
                    timeout: wire.timeout,
                    agent_config: wire.agent_config,
                    adopt: wire.adopt,
                    requires_role: wire.requires_role,
                    retry_policy,
                    session_ref: wire.session_ref,
                    join_policy,
                }
            }
            "sequentialThinking" => Self::SequentialThinking {
                id: wire.id,
                display_label: wire.display_label,
                topic: required(wire.topic, "topic")?,
                total_thoughts: wire.total_thoughts,
                agent: required(wire.agent, "agent")?,
                model: wire.model,
                thought_level: wire.thought_level,
                allowed_tools: wire.allowed_tools,
                max_turns: wire.max_turns,
                cwd: wire.cwd,
                timeout: wire.timeout,
                agent_config: wire.agent_config,
                adopt: wire.adopt,
                requires_role: wire.requires_role,
                retry_policy,
                join_policy,
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
                join_policy,
            },
            other => {
                return Err(format!(
                    "Node '{node_id}': unknown type '{other}'. Expected one of: {EXPECTED_TYPES}"
                ));
            }
        })
    }
}
fn parse_retry_policy(
    wire: &PipelineNodeWire,
    node_id: &str,
) -> Result<Option<RetryPolicy>, String> {
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

pub(super) fn default_prompt_tool_mode() -> String {
    "disabled".to_string()
}

fn is_default_prompt_tool_mode(value: &String) -> bool {
    value == "disabled"
}
fn is_default_join_policy(value: &Option<JoinPolicy>) -> bool {
    matches!(value, None | Some(JoinPolicy::All))
}
pub(super) fn default_total_thoughts() -> u32 {
    5
}

fn is_default_total_thoughts(value: &u32) -> bool {
    *value == 5
}

#[path = "types_wire_from.rs"]
mod types_wire_from;

#[cfg(test)]
#[path = "types_wire_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "types_wire_edge_tests.rs"]
mod edge_tests;
