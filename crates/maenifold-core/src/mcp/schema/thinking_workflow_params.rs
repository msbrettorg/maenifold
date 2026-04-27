#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::params::{default_recent_activity_filter, default_ten_u32, default_zero_u32};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubmachineType {
    Sequential,
    Workflow,
    Gate,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequentialThinkingParams {
    /// Main thought content; must include [[WikiLinks]] unless cancelling
    #[serde(default)]
    pub response: Option<String>,
    /// Whether another thought is needed after this one
    #[serde(default)]
    pub next_thought_needed: bool,
    /// Current thought number
    #[serde(default = "default_zero_u32")]
    #[schemars(default = "default_zero_u32")]
    pub thought_number: u32,
    /// Current estimate for total thoughts
    #[serde(default = "default_zero_u32")]
    #[schemars(default = "default_zero_u32")]
    pub total_thoughts: u32,
    /// Existing sequential thinking session identifier
    #[serde(default)]
    pub session_id: Option<String>,
    /// Cancel the session instead of adding a thought
    #[serde(default)]
    pub cancel: bool,
    /// Optional meta-thoughts; include [[WikiLinks]] when provided
    #[serde(default)]
    pub thoughts: Option<String>,
    /// Marks this thought as a revision of prior thinking
    #[serde(default)]
    pub is_revision: bool,
    /// Thought number that this revision updates
    #[serde(default)]
    pub revises_thought: Option<u32>,
    /// Thought number where this branch diverges
    #[serde(default)]
    pub branch_from_thought: Option<u32>,
    /// Branch identifier for multi-agent coordination
    #[serde(default)]
    pub branch_id: Option<String>,
    /// Whether more thoughts are needed than originally estimated
    #[serde(default)]
    pub needs_more_thoughts: bool,
    /// Optional parent workflow session identifier
    #[serde(default)]
    pub parent_workflow_id: Option<String>,
    /// Child session or task identifier to register as a submachine
    #[serde(default)]
    pub submachine_session_id: Option<String>,
    /// Type of child submachine to register; defaults to sequential when omitted
    #[serde(default)]
    pub submachine_type: Option<SubmachineType>,
    /// Structured ConfessionReport used when completing the session
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkflowIdValue {
    String(String),
    List(Vec<String>),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Completed,
    Cancelled,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowParams {
    /// Existing workflow session identifier to continue
    #[serde(default)]
    pub session_id: Option<String>,
    /// Workflow identifier or list of workflow identifiers to start
    #[serde(default)]
    pub workflow_id: Option<WorkflowIdValue>,
    /// Response to the current step; include [[WikiLinks]]
    #[serde(default)]
    pub response: Option<String>,
    /// Ambient or meta-thoughts; include [[WikiLinks]]
    #[serde(default)]
    pub thoughts: Option<String>,
    /// Terminal status for the workflow session
    #[serde(default)]
    pub status: Option<WorkflowStatus>,
    /// Structured ConfessionReport used when completing the session
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Display the workflow queue status
    #[serde(default)]
    pub view: bool,
    /// Workflow identifier or identifiers to append to the queue
    #[serde(default)]
    pub append: Option<String>,
    /// SequentialThinking session identifier registered as a submachine
    #[serde(default)]
    pub submachine_session_id: Option<String>,
    /// Type of submachine to register; defaults to sequential when omitted
    #[serde(default)]
    pub submachine_type: Option<SubmachineType>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RecentActivityFilter {
    Thinking,
    Memory,
    Assumptions,
    All,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentActivityParams {
    /// Maximum number of results to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub limit: u32,
    /// Activity filter
    #[serde(default = "default_recent_activity_filter")]
    #[schemars(default = "default_recent_activity_filter")]
    pub filter: RecentActivityFilter,
    /// Positive timespan such as 24.00:00:00
    #[serde(default)]
    pub timespan: Option<String>,
    /// Include full H2 section content instead of headers only
    #[serde(default)]
    pub include_content: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}
