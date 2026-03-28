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
    },
    Prompt {
        id: String,
        prompt: String,
        agent: String,
        tool_mode: String,
    },
    SequentialThinking {
        id: String,
        topic: String,
        total_thoughts: u32,
        analysis_type: Option<String>,
        agent: String,
    },
    Tool {
        id: String,
        tool_name: String,
        tool_args: Option<Value>,
        agent: Option<String>,
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
    pub thinking_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
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
}
