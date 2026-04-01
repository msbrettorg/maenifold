use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PipelineNodeKind {
    #[default]
    Workflow,
    Prompt,
    SequentialThinking,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(try_from = "PipelineNodeWire", into = "PipelineNodeWire")]
pub enum PipelineNode {
    Workflow { id: String, workflow_id: String, acp_client: Option<String>, prompt: Option<String> },
    Prompt { id: String, prompt: String, agent: String, tool_mode: String },
    SequentialThinking { id: String, topic: String, total_thoughts: u32, analysis_type: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PipelineNodeWire {
    #[serde(rename = "type", default)]
    kind: PipelineNodeKind,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")] workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] acp_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] agent: Option<String>,
    #[serde(default = "default_prompt_tool_mode", skip_serializing_if = "is_default_prompt_tool_mode")] tool_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")] topic: Option<String>,
    #[serde(default = "default_total_thoughts", skip_serializing_if = "is_default_total_thoughts")] total_thoughts: u32,
    #[serde(skip_serializing_if = "Option::is_none")] analysis_type: Option<String>,
}

impl PipelineNode {
    pub fn id(&self) -> &str { match self { Self::Workflow { id, .. } | Self::Prompt { id, .. } | Self::SequentialThinking { id, .. } => id } }
    pub fn workflow_id(&self) -> Option<&str> { match self { Self::Workflow { workflow_id, .. } => Some(workflow_id), _ => None } }
    pub fn acp_client(&self) -> Option<&str> { match self { Self::Workflow { acp_client, .. } => acp_client.as_deref(), Self::Prompt { agent, .. } => Some(agent), Self::SequentialThinking { .. } => None } }
    pub fn prompt(&self) -> Option<&str> { match self { Self::Workflow { prompt, .. } => prompt.as_deref(), Self::Prompt { prompt, .. } => Some(prompt), Self::SequentialThinking { .. } => None } }
    pub fn node_type_name(&self) -> &str { match self { Self::Workflow { .. } => "workflow", Self::Prompt { .. } => "prompt", Self::SequentialThinking { .. } => "sequentialThinking" } }
}

impl TryFrom<PipelineNodeWire> for PipelineNode {
    type Error = String;
    fn try_from(w: PipelineNodeWire) -> Result<Self, Self::Error> {
        let req = |v: Option<String>, name: &str| v.ok_or_else(|| format!("missing {name}"));
        Ok(match w.kind {
            PipelineNodeKind::Workflow => Self::Workflow { id: w.id, workflow_id: req(w.workflow_id, "workflowId")?, acp_client: w.acp_client, prompt: w.prompt },
            PipelineNodeKind::Prompt => Self::Prompt { id: w.id, prompt: req(w.prompt, "prompt")?, agent: req(w.agent, "agent")?, tool_mode: w.tool_mode },
            PipelineNodeKind::SequentialThinking => Self::SequentialThinking { id: w.id, topic: req(w.topic, "topic")?, total_thoughts: w.total_thoughts, analysis_type: w.analysis_type },
        })
    }
}

impl From<PipelineNode> for PipelineNodeWire {
    fn from(n: PipelineNode) -> Self {
        match n {
            PipelineNode::Workflow { id, workflow_id, acp_client, prompt } => Self { kind: PipelineNodeKind::Workflow, id, workflow_id: Some(workflow_id), acp_client, prompt, agent: None, tool_mode: default_prompt_tool_mode(), topic: None, total_thoughts: default_total_thoughts(), analysis_type: None },
            PipelineNode::Prompt { id, prompt, agent, tool_mode } => Self { kind: PipelineNodeKind::Prompt, id, workflow_id: None, acp_client: None, prompt: Some(prompt), agent: Some(agent), tool_mode, topic: None, total_thoughts: default_total_thoughts(), analysis_type: None },
            PipelineNode::SequentialThinking { id, topic, total_thoughts, analysis_type } => Self { kind: PipelineNodeKind::SequentialThinking, id, workflow_id: None, acp_client: None, prompt: None, agent: None, tool_mode: default_prompt_tool_mode(), topic: Some(topic), total_thoughts, analysis_type },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeState {
    pub session_id: String,
    pub status: NodeStatus,
    pub attempts: u32,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus { Pending, Ready, Active, Completed, Abandoned, Stuck }

fn default_prompt_tool_mode() -> String { "completion".to_string() }
fn is_default_prompt_tool_mode(v: &String) -> bool { v == "completion" }
fn default_total_thoughts() -> u32 { 5 }
fn is_default_total_thoughts(v: &u32) -> bool { *v == 5 }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn old_workflow_json_still_deserializes() {
        let node: PipelineNode = serde_json::from_value(json!({"id":"n1","workflowId":"wf","acpClient":"agent"})).unwrap();
        assert_eq!(node, PipelineNode::Workflow { id: "n1".into(), workflow_id: "wf".into(), acp_client: Some("agent".into()), prompt: None });
    }

    #[test]
    fn workflow_serialization_is_tagged() {
        let node = PipelineNode::Workflow { id: "n1".into(), workflow_id: "wf".into(), acp_client: None, prompt: Some("Summarize".into()) };
        assert_eq!(serde_json::to_value(node).unwrap(), json!({"type":"workflow","id":"n1","workflowId":"wf","prompt":"Summarize"}));
    }

    #[test]
    fn prompt_serialization_is_tagged() {
        let node = PipelineNode::Prompt { id: "n2".into(), prompt: "Review {{input}}".into(), agent: "claude".into(), tool_mode: "completion".into() };
        assert_eq!(serde_json::to_value(node).unwrap(), json!({"type":"prompt","id":"n2","prompt":"Review {{input}}","agent":"claude"}));
    }

    #[test]
    fn sequential_thinking_serialization_is_tagged() {
        let node = PipelineNode::SequentialThinking { id: "n3".into(), topic: "Tradeoffs".into(), total_thoughts: 5, analysis_type: Some("architecture".into()) };
        assert_eq!(serde_json::to_value(node).unwrap(), json!({"type":"sequentialThinking","id":"n3","topic":"Tradeoffs","analysisType":"architecture"}));
    }
}
