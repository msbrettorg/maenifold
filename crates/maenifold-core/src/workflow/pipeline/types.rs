use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
            let node_index = graph.add_node(node.id.clone());
            node_indices.insert(node.id.as_str(), node_index);
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineNode {
    pub id: String,
    pub workflow_id: String,
    pub acp_client: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type", default)]
    pub edge_type: EdgeType,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    pub session_id: String,
    pub status: NodeStatus,
    pub attempts: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Pending,
    Ready,
    Active,
    Completed,
    Abandoned,
    Stuck,
}
