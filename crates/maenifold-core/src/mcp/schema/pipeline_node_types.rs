// T-01.9: Extracted from pipeline_params.rs to keep that file under the 300 LOC ceiling.
// Hosts the untagged `PipelineNodeSchema` enum, the four concrete node variant
// schemas (workflow / prompt / sequentialThinking / tool), and their per-variant
// type-marker enums consumed by serde for deserialization.
#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::pipeline_params::{PipelineAgentConfigSchema, PipelineNodeAdoptSchema};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PipelineNodeSchema {
    Workflow(PipelineWorkflowNodeSchema),
    Prompt(PipelinePromptNodeSchema),
    SequentialThinking(PipelineSequentialThinkingNodeSchema),
    Tool(PipelineToolNodeSchema),
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipelineJoinPolicySchema {
    All,
    Any,
    First,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineWorkflowNodeSchema {
    /// New node identifier
    pub id: String,
    /// Optional presentation-only label for UI surfaces
    #[serde(default)]
    pub display_label: Option<String>,
    /// Workflow node type
    #[serde(rename = "type")]
    pub kind: PipelineWorkflowNodeType,
    /// Workflow definition to run
    pub workflow_id: String,
    /// Optional ACP client
    #[serde(default)]
    pub acp_client: Option<String>,
    /// Optional per-node adopt configuration
    #[serde(default)]
    pub adopt: Option<PipelineNodeAdoptSchema>,
    /// Fan-in join policy.
    #[serde(default)]
    pub join_policy: Option<PipelineJoinPolicySchema>,
    /// Optional required role identifier for adopt validation
    #[serde(default)]
    pub requires_role: Option<String>,
    /// Optional prompt for workflow nodes
    #[serde(default)]
    pub prompt: Option<String>,
    /// Optional working directory for this node
    #[serde(default)]
    pub cwd: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    /// Optional maximum transient retries after the initial attempt. 0 means no retry.
    #[serde(default)]
    pub retry_max_attempts: Option<u32>,
    /// Optional minimum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_min_ms: Option<u64>,
    /// Optional maximum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_max_ms: Option<u64>,
    /// Optional jitter flag for retry delays.
    #[serde(default)]
    pub retry_jitter: Option<bool>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelineWorkflowNodeType {
    #[serde(rename = "workflow")]
    Workflow,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionRefSchema {
    /// ACP agent name for the session
    pub agent: String,
    /// ACP session ID to resume
    pub session_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelinePromptNodeSchema {
    /// New node identifier
    pub id: String,
    /// Optional presentation-only label for UI surfaces
    #[serde(default)]
    pub display_label: Option<String>,
    /// Node type 'prompt'
    #[serde(rename = "type")]
    pub kind: PipelinePromptNodeType,
    /// Prompt text for prompt nodes
    pub prompt: String,
    /// ACP agent for prompt nodes
    pub agent: String,
    /// Optional per-node adopt configuration
    #[serde(default)]
    pub adopt: Option<PipelineNodeAdoptSchema>,
    /// Fan-in join policy.
    #[serde(default)]
    pub join_policy: Option<PipelineJoinPolicySchema>,
    /// Optional required role identifier for adopt validation
    #[serde(default)]
    pub requires_role: Option<String>,
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
    /// Optional working directory for this node
    #[serde(default)]
    pub cwd: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    /// Optional maximum transient retries after the initial attempt. 0 means no retry.
    #[serde(default)]
    pub retry_max_attempts: Option<u32>,
    /// Optional minimum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_min_ms: Option<u64>,
    /// Optional maximum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_max_ms: Option<u64>,
    /// Optional jitter flag for retry delays.
    #[serde(default)]
    pub retry_jitter: Option<bool>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
    /// Optional session reference for resuming an existing ACP session.
    ///
    /// Validation that `sessionRef.agent` matches the node `agent` occurs in
    /// `workflow/pipeline/types_wire.rs` during `TryFrom<PipelineNodeWire>`.
    #[serde(default)]
    pub session_ref: Option<SessionRefSchema>,
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
    /// Optional presentation-only label for UI surfaces
    #[serde(default)]
    pub display_label: Option<String>,
    /// Node type 'sequentialThinking'
    #[serde(rename = "type")]
    pub kind: PipelineSequentialThinkingNodeType,
    /// Topic for sequential thinking nodes
    pub topic: String,
    /// Thought count for thinking nodes
    pub total_thoughts: u32,
    /// ACP agent for sequential thinking nodes
    pub agent: String,
    /// Optional per-node adopt configuration
    #[serde(default)]
    pub adopt: Option<PipelineNodeAdoptSchema>,
    /// Fan-in join policy.
    #[serde(default)]
    pub join_policy: Option<PipelineJoinPolicySchema>,
    /// Optional required role identifier for adopt validation
    #[serde(default)]
    pub requires_role: Option<String>,
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
    /// Optional working directory for this node
    #[serde(default)]
    pub cwd: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    /// Optional maximum transient retries after the initial attempt. 0 means no retry.
    #[serde(default)]
    pub retry_max_attempts: Option<u32>,
    /// Optional minimum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_min_ms: Option<u64>,
    /// Optional maximum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_max_ms: Option<u64>,
    /// Optional jitter flag for retry delays.
    #[serde(default)]
    pub retry_jitter: Option<bool>,
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
    /// Optional presentation-only label for UI surfaces
    #[serde(default)]
    pub display_label: Option<String>,
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
    /// Fan-in join policy.
    #[serde(default)]
    pub join_policy: Option<PipelineJoinPolicySchema>,
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
    /// Optional working directory for this node
    #[serde(default)]
    pub cwd: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    /// Optional maximum transient retries after the initial attempt. 0 means no retry.
    #[serde(default)]
    pub retry_max_attempts: Option<u32>,
    /// Optional minimum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_min_ms: Option<u64>,
    /// Optional maximum backoff delay in milliseconds.
    #[serde(default)]
    pub retry_backoff_max_ms: Option<u64>,
    /// Optional jitter flag for retry delays.
    #[serde(default)]
    pub retry_jitter: Option<bool>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}
#[derive(Deserialize, JsonSchema)]
pub enum PipelineToolNodeType {
    #[serde(rename = "tool")]
    Tool,
}
