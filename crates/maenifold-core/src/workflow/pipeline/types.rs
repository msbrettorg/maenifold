use petgraph::stable_graph::StableGraph;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const EXPECTED_TYPES: &str = "workflow, prompt, sequentialThinking, tool";

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
        agent: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PipelineNodeWire {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    acp_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
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
            Self::Prompt { agent, .. }
            | Self::SequentialThinking { agent, .. }
            | Self::Tool { agent, .. } => Some(agent),
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
}

impl TryFrom<PipelineNodeWire> for PipelineNode {
    type Error = String;

    fn try_from(wire: PipelineNodeWire) -> Result<Self, Self::Error> {
        let node_id = wire.id.clone();

        let required = |value: Option<String>, name: &str| -> Result<String, String> {
            value.ok_or_else(|| format!("Node '{node_id}': missing '{name}' field"))
        };

        // Resolve node kind: detect missing `type` with backward-compat for old workflow JSON
        // (old JSON has no `type` field but always carries `workflowId`).
        let kind_owned = wire.kind.clone();
        let kind: &str = match kind_owned.as_deref() {
            None => {
                if wire.workflow_id.is_some() {
                    // Legacy workflow node submitted without `type` — accept as workflow.
                    "workflow"
                } else {
                    return Err(format!(
                        "Node '{node_id}': missing 'type' field. Expected one of: {EXPECTED_TYPES}"
                    ));
                }
            }
            Some(k) => k,
        };

        Ok(match kind {
            "workflow" => Self::Workflow {
                id: wire.id,
                workflow_id: required(wire.workflow_id, "workflowId")?,
                acp_client: wire.acp_client,
                prompt: wire.prompt,
            },
            "prompt" => Self::Prompt {
                id: wire.id,
                prompt: required(wire.prompt, "prompt")?,
                agent: required(wire.agent, "agent")?,
                tool_mode: wire.tool_mode,
            },
            "sequentialThinking" => Self::SequentialThinking {
                id: wire.id,
                topic: required(wire.topic, "topic")?,
                total_thoughts: wire.total_thoughts,
                analysis_type: wire.analysis_type,
                agent: required(wire.agent, "agent")?,
            },
            "tool" => Self::Tool {
                id: wire.id,
                tool_name: required(wire.tool_name, "toolName")?,
                tool_args: wire.tool_args,
                agent: required(wire.agent, "agent")?,
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
    fn from(node: PipelineNode) -> Self {
        match node {
            PipelineNode::Workflow {
                id,
                workflow_id,
                acp_client,
                prompt,
            } => Self {
                kind: Some("workflow".to_string()),
                id,
                workflow_id: Some(workflow_id),
                acp_client,
                prompt,
                agent: None,
                tool_mode: default_prompt_tool_mode(),
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: None,
                tool_args: None,
            },
            PipelineNode::Prompt {
                id,
                prompt,
                agent,
                tool_mode,
            } => Self {
                kind: Some("prompt".to_string()),
                id,
                workflow_id: None,
                acp_client: None,
                prompt: Some(prompt),
                agent: Some(agent),
                tool_mode,
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: None,
                tool_args: None,
            },
            PipelineNode::SequentialThinking {
                id,
                topic,
                total_thoughts,
                analysis_type,
                agent,
            } => Self {
                kind: Some("sequentialThinking".to_string()),
                id,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                agent: Some(agent),
                tool_mode: default_prompt_tool_mode(),
                topic: Some(topic),
                total_thoughts,
                analysis_type,
                tool_name: None,
                tool_args: None,
            },
            PipelineNode::Tool {
                id,
                tool_name,
                tool_args,
                agent,
            } => Self {
                kind: Some("tool".to_string()),
                id,
                workflow_id: None,
                acp_client: None,
                prompt: None,
                agent: Some(agent),
                tool_mode: default_prompt_tool_mode(),
                topic: None,
                total_thoughts: default_total_thoughts(),
                analysis_type: None,
                tool_name: Some(tool_name),
                tool_args,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type", default)]
    pub edge_type: EdgeType,
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
    pub output: Option<String>,
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
    use super::*;
    use serde_json::json;

    #[test]
    fn old_workflow_json_still_deserializes() {
        let node: PipelineNode =
            serde_json::from_value(json!({"id":"n1","workflowId":"wf","acpClient":"agent"}))
                .unwrap();
        assert_eq!(
            node,
            PipelineNode::Workflow {
                id: "n1".into(),
                workflow_id: "wf".into(),
                acp_client: Some("agent".into()),
                prompt: None,
            }
        );
    }

    #[test]
    fn workflow_serialization_is_tagged() {
        let node = PipelineNode::Workflow {
            id: "n1".into(),
            workflow_id: "wf".into(),
            acp_client: None,
            prompt: Some("Summarize".into()),
        };
        assert_eq!(
            serde_json::to_value(node).unwrap(),
            json!({"type":"workflow","id":"n1","workflowId":"wf","prompt":"Summarize"})
        );
    }

    #[test]
    fn prompt_serialization_is_tagged() {
        let node = PipelineNode::Prompt {
            id: "n2".into(),
            prompt: "Review {{input}}".into(),
            agent: "claude".into(),
            tool_mode: "disabled".into(),
        };
        assert_eq!(
            serde_json::to_value(node).unwrap(),
            json!({"type":"prompt","id":"n2","prompt":"Review {{input}}","agent":"claude"})
        );
    }

    #[test]
    fn sequential_thinking_serialization_is_tagged() {
        let node = PipelineNode::SequentialThinking {
            id: "n3".into(),
            topic: "Tradeoffs".into(),
            total_thoughts: 5,
            analysis_type: Some("architecture".into()),
            agent: "claude".into(),
        };
        assert_eq!(
            serde_json::to_value(node).unwrap(),
            json!({"type":"sequentialThinking","id":"n3","topic":"Tradeoffs","analysisType":"architecture","agent":"claude"})
        );
    }

    #[test]
    fn node_without_type_and_without_workflow_id_gives_clear_error() {
        let err =
            serde_json::from_value::<PipelineNode>(json!({"id": "n99"})).expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("n99"),
            "error should mention node id, got: {msg}"
        );
        assert!(
            msg.contains("missing 'type' field"),
            "error should mention missing type, got: {msg}"
        );
        assert!(
            msg.contains("workflow") && msg.contains("prompt") && msg.contains("tool"),
            "error should list expected types, got: {msg}"
        );
    }

    #[test]
    fn node_with_unknown_type_gives_clear_error() {
        let err = serde_json::from_value::<PipelineNode>(json!({"id": "n42", "type": "badType"}))
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("n42"),
            "error should mention node id, got: {msg}"
        );
        assert!(
            msg.contains("unknown type") && msg.contains("badType"),
            "error should mention unknown type value, got: {msg}"
        );
    }

    #[test]
    fn tool_node_deserializes() {
        let node: PipelineNode = serde_json::from_value(json!({
            "type": "tool",
            "id": "t1",
            "toolName": "pipeline",
            "toolArgs": { "sessionId": "pipeline-42" },
            "agent": "claude"
        }))
        .unwrap();
        assert_eq!(
            node,
            PipelineNode::Tool {
                id: "t1".into(),
                tool_name: "pipeline".into(),
                tool_args: Some(json!({"sessionId": "pipeline-42"})),
                agent: "claude".into(),
            }
        );
    }

    #[test]
    fn tool_node_serializes() {
        let node = PipelineNode::Tool {
            id: "t1".into(),
            tool_name: "pipeline".into(),
            tool_args: Some(json!({"sessionId": "pipeline-42"})),
            agent: "claude".into(),
        };
        assert_eq!(
            serde_json::to_value(node).unwrap(),
            json!({
                "type": "tool",
                "id": "t1",
                "toolName": "pipeline",
                "toolArgs": {"sessionId": "pipeline-42"},
                "agent": "claude"
            })
        );
    }
}
