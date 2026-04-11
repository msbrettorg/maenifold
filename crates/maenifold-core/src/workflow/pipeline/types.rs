use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::types_wire::PipelineNodeWire;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDefinition {
    pub nodes: Vec<PipelineNode>,
    pub edges: Vec<PipelineEdge>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_options: Option<serde_json::Map<String, serde_json::Value>>,
}

impl PipelineDefinition {
    pub(crate) fn build_graph(&self) -> StableGraph<String, EdgeType> {
        let mut graph = StableGraph::with_capacity(self.nodes.len(), self.edges.len());
        let mut node_indices = HashMap::with_capacity(self.nodes.len());

        for node in &self.nodes {
            let node_index = graph.add_node(node.id().to_string());
            node_indices.insert(node.id(), node_index);
        }

        for edge in &self.edges {
            if let (Some(&source_index), Some(&target_index)) = (
                node_indices.get(edge.source.as_str()),
                node_indices.get(edge.target.as_str()),
            ) {
                graph.add_edge(source_index, target_index, edge.edge_type.clone());
            }
        }

        graph
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "PipelineNodeWire", into = "PipelineNodeWire")]
pub enum PipelineNode {
    Workflow {
        id: String,
        workflow_id: String,
        acp_client: Option<String>,
        prompt: Option<String>,
        timeout: Option<u64>,
    },
    Prompt {
        id: String,
        prompt: String,
        agent: String,
        tool_mode: String,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
    },
    SequentialThinking {
        id: String,
        topic: String,
        total_thoughts: u32,
        analysis_type: Option<String>,
        agent: String,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
    },
    Tool {
        id: String,
        tool_name: String,
        tool_args: Option<Value>,
        agent: Option<String>,
        model: Option<String>,
        thought_level: Option<String>,
        allowed_tools: Option<Vec<String>>,
        max_turns: Option<u32>,
        timeout: Option<u64>,
        agent_config: Option<AgentConfig>,
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

    pub fn node_type_name(&self) -> &str {
        match self {
            Self::Workflow { .. } => "workflow",
            Self::Prompt { .. } => "prompt",
            Self::SequentialThinking { .. } => "sequentialThinking",
            Self::Tool { .. } => "tool",
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type", default)]
    pub edge_type: EdgeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeType {
    Success,
    Always,
}

impl Default for EdgeType {
    fn default() -> Self {
        Self::Success
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    pub session_id: String,
    pub status: NodeStatus,
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_provenance: Option<ToolProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolProvenance {
    pub tool_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_args: Option<Value>,
    pub response_summary: String,
}

/// A snapshot of a completed predecessor node's output, captured at fork time
/// so the fork root begins with the same reasoning context it had in the
/// original pipeline.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PredecessorOutput {
    /// The node ID from the source pipeline (pre-fork naming).
    pub node_id: String,
    /// The output produced by the predecessor node.
    pub output: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Pending,
    Ready,
    Active,
    Completed,
    Abandoned,
    Stuck,
    Retrying,
}

#[cfg(test)]
mod tests {
    use super::{NodeState, NodeStatus};

    #[test]
    fn node_state_deserializes_without_optional_provenance_fields() {
        let node_state: NodeState = serde_yaml::from_str(
            r#"
sessionId: workflow-node-1
status: completed
attempts: 1
output: done
"#,
        )
        .expect("legacy node state should deserialize");

        assert_eq!(node_state.session_id, "workflow-node-1");
        assert_eq!(node_state.status, NodeStatus::Completed);
        assert_eq!(node_state.acp_session_id, None);
        assert_eq!(node_state.thinking_session_id, None);
        assert_eq!(node_state.tool_provenance, None);
        assert_eq!(node_state.output.as_deref(), Some("done"));
    }
}
