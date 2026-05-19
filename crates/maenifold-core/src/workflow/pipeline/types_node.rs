use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::types::{AgentConfig, JoinPolicy, NodeAdopt, RetryPolicy};
use super::types_wire::PipelineNodeWire;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRef {
    pub agent: String,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "PipelineNodeWire", into = "PipelineNodeWire")]
pub enum PipelineNode {
    Workflow {
        id: String,
        display_label: Option<String>,
        workflow_id: String,
        acp_client: Option<String>,
        prompt: Option<String>,
        cwd: Option<PathBuf>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
        adopt: Option<NodeAdopt>,
        requires_role: Option<String>,
        retry_policy: Option<RetryPolicy>,
        join_policy: Option<JoinPolicy>,
        input_schema: Option<Value>,
        output_schema: Option<Value>,
    },
    Prompt {
        id: String,
        display_label: Option<String>,
        prompt: String,
        agent: String,
        tool_mode: String,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        cwd: Option<PathBuf>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
        adopt: Option<NodeAdopt>,
        requires_role: Option<String>,
        retry_policy: Option<RetryPolicy>,
        session_ref: Option<SessionRef>,
        join_policy: Option<JoinPolicy>,
        input_schema: Option<Value>,
        output_schema: Option<Value>,
    },
    SequentialThinking {
        id: String,
        display_label: Option<String>,
        topic: String,
        total_thoughts: u32,
        agent: String,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        cwd: Option<PathBuf>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
        adopt: Option<NodeAdopt>,
        requires_role: Option<String>,
        retry_policy: Option<RetryPolicy>,
        join_policy: Option<JoinPolicy>,
        input_schema: Option<Value>,
        output_schema: Option<Value>,
    },
    Tool {
        id: String,
        display_label: Option<String>,
        tool_name: String,
        tool_args: Option<Value>,
        agent: Option<String>,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        cwd: Option<PathBuf>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
        retry_policy: Option<RetryPolicy>,
        join_policy: Option<JoinPolicy>,
    },
}

impl PipelineNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Workflow { id, .. }
            | Self::Prompt { id, .. }
            | Self::SequentialThinking { id, .. }
            | Self::Tool { id, .. } => id,
        }
    }

    pub fn workflow_id(&self) -> Option<&str> {
        match self {
            Self::Workflow { workflow_id, .. } => Some(workflow_id),
            Self::Prompt { .. } | Self::SequentialThinking { .. } | Self::Tool { .. } => None,
        }
    }

    pub fn display_label(&self) -> Option<&str> {
        match self {
            Self::Workflow { display_label, .. }
            | Self::Prompt { display_label, .. }
            | Self::SequentialThinking { display_label, .. }
            | Self::Tool { display_label, .. } => display_label.as_deref(),
        }
    }

    pub fn acp_client(&self) -> Option<&str> {
        match self {
            Self::Workflow { acp_client, .. } => acp_client.as_deref(),
            Self::Prompt { agent, .. } | Self::SequentialThinking { agent, .. } => Some(agent),
            Self::Tool { agent, .. } => agent.as_deref(),
        }
    }

    pub fn required_execution_target_field(&self) -> Option<&'static str> {
        match self {
            Self::Workflow { .. } => Some("acpClient"),
            Self::Prompt { .. } | Self::SequentialThinking { .. } => Some("agent"),
            Self::Tool { .. } => None,
        }
    }

    pub fn has_execution_target(&self) -> bool {
        self.acp_client()
            .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn prompt(&self) -> Option<&str> {
        match self {
            Self::Workflow { prompt, .. } => prompt.as_deref(),
            Self::Prompt { prompt, .. } => Some(prompt),
            Self::SequentialThinking { .. } | Self::Tool { .. } => None,
        }
    }

    pub fn output_schema(&self) -> Option<&Value> {
        match self {
            Self::Workflow { output_schema, .. }
            | Self::Prompt { output_schema, .. }
            | Self::SequentialThinking { output_schema, .. } => output_schema.as_ref(),
            Self::Tool { .. } => None,
        }
    }

    pub fn input_schema(&self) -> Option<&Value> {
        match self {
            Self::Workflow { input_schema, .. }
            | Self::Prompt { input_schema, .. }
            | Self::SequentialThinking { input_schema, .. } => input_schema.as_ref(),
            Self::Tool { .. } => None,
        }
    }

    pub fn cwd(&self) -> Option<&Path> {
        match self {
            Self::Workflow { cwd, .. }
            | Self::Prompt { cwd, .. }
            | Self::SequentialThinking { cwd, .. }
            | Self::Tool { cwd, .. } => cwd.as_deref(),
        }
    }

    pub fn node_type_name(&self) -> &str {
        match self {
            Self::Workflow { .. } => "workflow",
            Self::Prompt { .. } => "prompt",
            Self::SequentialThinking { .. } => "sequentialThinking",
            Self::Tool { .. } => "tool",
        }
    }

    pub fn retry_policy(&self) -> Option<&RetryPolicy> {
        match self {
            Self::Workflow { retry_policy, .. }
            | Self::Prompt { retry_policy, .. }
            | Self::SequentialThinking { retry_policy, .. }
            | Self::Tool { retry_policy, .. } => retry_policy.as_ref(),
        }
    }

    pub fn join_policy(&self) -> JoinPolicy {
        match self {
            Self::Workflow { join_policy, .. }
            | Self::Prompt { join_policy, .. }
            | Self::SequentialThinking { join_policy, .. }
            | Self::Tool { join_policy, .. } => join_policy.clone().unwrap_or_default(),
        }
    }

    pub fn adopt(&self) -> Option<&NodeAdopt> {
        match self {
            Self::Workflow { adopt, .. }
            | Self::Prompt { adopt, .. }
            | Self::SequentialThinking { adopt, .. } => adopt.as_ref(),
            Self::Tool { .. } => None,
        }
    }

    pub fn requires_role(&self) -> Option<&str> {
        match self {
            Self::Workflow { requires_role, .. }
            | Self::Prompt { requires_role, .. }
            | Self::SequentialThinking { requires_role, .. } => requires_role.as_deref(),
            Self::Tool { .. } => None,
        }
    }

    /// Returns the workflow_id for Workflow nodes, or a synthetic
    /// `__<nodeType>__` identifier for non-Workflow node types.
    pub fn effective_workflow_id(&self) -> String {
        match self.workflow_id() {
            Some(id) => id.to_string(),
            None => format!("__{}__", self.node_type_name()),
        }
    }
}
