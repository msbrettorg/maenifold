use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineDefinition {
    pub nodes: Vec<PipelineNode>,
    pub edges: Vec<PipelineEdge>,
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
