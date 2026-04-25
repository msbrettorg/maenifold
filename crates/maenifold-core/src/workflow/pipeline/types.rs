use chrono::{DateTime, Utc};
use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub use super::types_node::{PipelineNode, SessionRef};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDefinition {
    pub nodes: Vec<PipelineNode>,
    pub edges: Vec<PipelineEdge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeAdopt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perspective: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    /// Maximum transient retries after the initial attempt.
    ///
    /// This budget does not include the first execution attempt. For example,
    /// `max_attempts = 2` permits the initial attempt plus two retries.
    pub max_attempts: u32,
    pub backoff_min_ms: u64,
    pub backoff_max_ms: u64,
    pub jitter: bool,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdge {
    pub source: String,
    pub target: String,
    #[serde(default, flatten)]
    pub edge_type: EdgeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<HashMap<String, String>>,
}

/// Edge predicate determining whether a downstream node may run.
///
/// `Success` requires the upstream node to have completed; `Always` accepts
/// any terminal upstream status (completed, abandoned, or stuck);
/// `Conditional` requires the upstream to have completed AND the upstream's
/// JSON output to have a value at `path` equal to `equals`.
///
/// Wire format is internally tagged on the `type` field of `PipelineEdge`:
/// `{"type":"success"}`, `{"type":"always"}`,
/// `{"type":"conditional","path":"verdict","equals":"FAIL"}`.
///
/// Note: `Eq` is intentionally NOT derived because `serde_json::Value`
/// contains floating point numbers and is not `Eq`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EdgeType {
    Success,
    Always,
    Conditional {
        path: String,
        equals: serde_json::Value,
    },
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
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_provenance: Option<ToolProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
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
        assert_eq!(node_state.started_at, None);
        assert_eq!(node_state.completed_at, None);
        assert_eq!(node_state.acp_session_id, None);
        assert_eq!(node_state.thinking_session_id, None);
        assert_eq!(node_state.tool_provenance, None);
        assert_eq!(node_state.input_tokens, None);
        assert_eq!(node_state.output_tokens, None);
        assert_eq!(node_state.model, None);
        assert_eq!(node_state.output.as_deref(), Some("done"));
    }
}
