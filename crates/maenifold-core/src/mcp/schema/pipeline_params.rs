#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code
use super::params::default_eight_u32;
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
    /// Block until the pipeline reaches a terminal status
    #[serde(default)]
    pub wait: Option<bool>,
    /// Maximum wait duration in milliseconds; omitted means unlimited
    #[serde(default, rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    /// Ambient prompt for autonomous pipeline execution
    #[serde(default)]
    pub prompt: Option<String>,
    /// Optional runtime inputs visible to pipeline execution. Run-time values override persisted inputs.
    #[serde(default)]
    pub inputs: Option<Map<String, Value>>,
    /// Optional gate task identifier used to attach lineage context
    #[serde(default)]
    pub gate_task_id: Option<String>,
    /// Cancel a running pipeline and stop future execution
    #[serde(default)]
    pub cancel: bool,
    /// Pause a running pipeline; in-flight nodes complete, no new nodes scheduled
    #[serde(default)]
    pub pause: bool,
    /// Resume a paused pipeline from its checkpoint
    #[serde(default)]
    pub resume: bool,
    /// Return current pipeline state
    #[serde(default)]
    pub view: bool,
    /// Compact one-row-per-node projection. Returns ≤4 KB for pipelines up to 50 nodes. Only valid with view=true.
    #[serde(default)]
    pub summary: bool,
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
    /// Restore the pipeline session from quarantine
    #[serde(default)]
    pub restore: bool,
    /// Quarantine date to restore from, formatted YYYY-MM-DD
    #[serde(default)]
    pub from_trash_date: Option<String>,
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
    /// Optional pipeline inputs persisted with the definition and session
    #[serde(default)]
    pub inputs: Option<Map<String, Value>>,
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
#[serde(untagged)]
pub enum PipelineForkSchema {
    /// Node identifier where the forked subgraph starts
    NodeId(String),
    /// Structured fork configuration with an optional session identifier override
    Config(PipelineForkConfigSchema),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineForkConfigSchema {
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
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PipelineEdgeSchema {
    /// Upstream node identifier
    pub source: String,
    /// Downstream node identifier
    pub target: String,
    /// Edge type controlling downstream activation
    #[serde(default, flatten)]
    #[schemars(default)]
    pub edge_type: PipelineEdgeType,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PipelineEdgeType {
    Success,
    Always,
    Conditional {
        /// JSON path within upstream output to inspect
        path: String,
        /// Expected JSON value at `path`
        equals: Value,
    },
}

impl Default for PipelineEdgeType {
    fn default() -> Self {
        Self::Success
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineAgentConfigSchema {
    /// Model override for the agent session
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for the agent session
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Arbitrary config options forwarded to the agent session
    #[serde(default)]
    pub config_options: Option<Map<String, Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineNodeAdoptSchema {
    /// Optional role asset identifier to adopt before dispatching.
    #[serde(default)]
    pub role: Option<String>,
    /// Optional color asset identifier to adopt before dispatching.
    #[serde(default)]
    pub color: Option<String>,
    /// Optional perspective asset identifier to adopt before dispatching.
    #[serde(default)]
    pub perspective: Option<String>,
}

// T-01.9: Re-export the node variant types from their dedicated module so existing
// `super::pipeline_params::*` consumers (notably `super::params`) keep working.
pub use super::pipeline_node_types::*;
