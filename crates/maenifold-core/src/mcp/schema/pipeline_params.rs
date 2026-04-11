#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code
use super::params::{default_edge_type, default_eight_u32};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineParams {
    /// Existing pipeline session identifier
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optional name prefix for the pipeline session ID. Format: {name}-{epoch}. Defaults to 'pipeline'. Must start with a letter, alphanumeric and hyphens only.
    #[serde(default)]
    pub name: Option<String>,
    /// Pipeline definition with nodes[] and edges[]
    #[serde(default)]
    pub definition: Option<PipelineDefinitionSchema>,
    /// Max concurrent active nodes
    #[serde(default = "default_eight_u32")]
    #[schemars(default = "default_eight_u32")]
    pub max_concurrency: u32,
    /// Start autonomous execution for an existing pipeline
    #[serde(default)]
    pub run: bool,
    /// Ambient prompt for autonomous pipeline execution
    #[serde(default)]
    pub prompt: Option<String>,
    /// Cancel a running pipeline and stop future execution
    #[serde(default)]
    pub cancel: bool,
    /// Return current pipeline state
    #[serde(default)]
    pub view: bool,
    /// Claim ready nodes as active
    #[serde(default)]
    pub dispatch: bool,
    /// List all pipeline sessions with status summaries
    #[serde(default)]
    pub list: bool,
    /// Rewind the specified nodeIds and all downstream nodes
    #[serde(default)]
    pub rewind: Option<PipelineRewindSchema>,
    /// Delete the pipeline session and all associated workflow sessions
    #[serde(default)]
    pub delete: bool,
    /// Add new nodes and edges onto an existing pipeline session
    #[serde(default)]
    pub graft: Option<PipelineGraftSchema>,
    /// Merge another pipeline session into this session using additional linking edges
    #[serde(default)]
    pub join: Option<PipelineJoinSchema>,
    /// Clone the selected node and downstream subgraph into a new pipeline session
    #[serde(default)]
    pub fork: Option<PipelineForkSchema>,
    /// Target node for status updates or restart
    #[serde(default)]
    pub node_id: Option<String>,
    /// Node status update
    #[serde(default)]
    pub status: Option<PipelineNodeStatusUpdate>,
    /// Conclusion text when completing a node
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Restart an abandoned/stuck node
    #[serde(default)]
    pub restart: bool,
    /// Admin override: force a pipeline to completed status. The pipeline must not be actively running.
    #[serde(default)]
    pub force_complete: bool,
    /// Return help documentation
    #[serde(default)]
    pub learn: bool,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineDefinitionSchema {
    /// Pipeline nodes to execute
    pub nodes: Vec<PipelineNodeSchema>,
    /// Pipeline edges connecting nodes
    pub edges: Vec<PipelineEdgeSchema>,
    /// Optional template identifier preserved across pipeline round-trips
    #[serde(default)]
    pub template_id: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineRewindSchema {
    /// Node identifiers to rewind before cascading downstream resets
    pub node_ids: Vec<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineGraftSchema {
    /// New pipeline nodes to append
    pub nodes: Vec<PipelineNodeSchema>,
    /// New pipeline edges to add
    pub edges: Vec<PipelineEdgeSchema>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineJoinSchema {
    /// Pipeline session identifier to merge into the target pipeline
    pub source_session_id: String,
    /// Edges that connect the source pipeline into the target pipeline
    pub edges: Vec<PipelineEdgeSchema>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineForkSchema {
    /// Node identifier where the forked subgraph starts
    pub from_node_id: String,
    /// Optional pipeline session identifier for the forked pipeline
    #[serde(default)]
    pub new_session_id: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipelineNodeStatusUpdate {
    Completed,
    Abandoned,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineEdgeSchema {
    /// Upstream node identifier
    pub source: String,
    /// Downstream node identifier
    pub target: String,
    /// Edge type controlling downstream activation
    #[serde(rename = "type", default = "default_edge_type")]
    #[schemars(default = "default_edge_type")]
    pub edge_type: PipelineEdgeType,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipelineEdgeType {
    Success,
    Always,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineAgentConfigSchema {
    /// Optional working directory for the agent session
    #[serde(default)]
    pub cwd: Option<String>,
    /// Model override for the agent session
    #[serde(default)]
    pub model: Option<String>,
    /// Session mode override for the agent session
    #[serde(default)]
    pub mode: Option<String>,
    /// Thought level override for the agent session
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Arbitrary config options forwarded to the agent session
    #[serde(default)]
    pub config_options: Option<Map<String, Value>>,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub enum PipelineNodeSchema {
    Workflow(PipelineWorkflowNodeSchema),
    Prompt(PipelinePromptNodeSchema),
    SequentialThinking(PipelineSequentialThinkingNodeSchema),
    Tool(PipelineToolNodeSchema),
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineWorkflowNodeSchema {
    /// New node identifier
    pub id: String,
    /// Workflow node type
    #[serde(rename = "type")]
    pub kind: PipelineWorkflowNodeType,
    /// Workflow definition to run
    pub workflow_id: String,
    /// Optional ACP client
    #[serde(default)]
    pub acp_client: Option<String>,
    /// Optional prompt for workflow nodes
    #[serde(default)]
    pub prompt: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelineWorkflowNodeType {
    #[serde(rename = "workflow")]
    Workflow,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelinePromptNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'prompt'
    #[serde(rename = "type")]
    pub kind: PipelinePromptNodeType,
    /// Prompt text for prompt nodes
    pub prompt: String,
    /// ACP agent for prompt nodes
    pub agent: String,
    /// Tool mode for prompt nodes
    #[serde(default)]
    pub tool_mode: Option<String>,
    /// Model override for prompt nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for prompt nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for prompt nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for prompt nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelinePromptNodeType {
    #[serde(rename = "prompt")]
    Prompt,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineSequentialThinkingNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'sequentialThinking'
    #[serde(rename = "type")]
    pub kind: PipelineSequentialThinkingNodeType,
    /// Topic for sequential thinking nodes
    pub topic: String,
    /// Thought count for thinking nodes
    pub total_thoughts: u32,
    /// Analysis type for thinking nodes
    #[serde(default)]
    pub analysis_type: Option<String>,
    /// ACP agent for sequential thinking nodes
    pub agent: String,
    /// Model override for sequential thinking nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for sequential thinking nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for sequential thinking nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for sequential thinking nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelineSequentialThinkingNodeType {
    #[serde(rename = "sequentialThinking")]
    SequentialThinking,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineToolNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'tool'
    #[serde(rename = "type")]
    pub kind: PipelineToolNodeType,
    /// MCP tool name for tool nodes
    pub tool_name: String,
    /// Tool arguments for tool nodes
    #[serde(default)]
    pub tool_args: Option<Map<String, Value>>,
    /// Optional ACP agent for tool nodes
    #[serde(default)]
    pub agent: Option<String>,
    /// Model override for tool nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for tool nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for tool nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for tool nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelineToolNodeType {
    #[serde(rename = "tool")]
    Tool,
}
