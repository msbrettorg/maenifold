#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code
use super::params::default_eight_u32;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

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
    /// Update an existing pipeline definition in place instead of creating a new session
    #[serde(default)]
    pub update: bool,
    /// Force destructive operations such as deleting a completed, active, or unreadable pipeline
    #[serde(default)]
    pub force: bool,
    /// Optional human-readable description persisted with the pipeline session
    #[serde(default)]
    pub description: Option<String>,
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
    /// Optional JSON Schema validating pipeline-level inputs at definition time
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<Value>,
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
    /// Data propagation mode: 'context' (default), 'binding', or 'none'
    #[serde(default)]
    pub mode: Option<PipelineEdgeDataModeSchema>,
    /// Explicit downstream-input to upstream-output field mapping (target path -> source path)
    #[serde(default)]
    pub mapping: Option<HashMap<String, String>>,
    /// Binding pairs ({from, to}) merged into the edge mapping
    #[serde(default)]
    pub bindings: Option<Vec<PipelineEdgeBindingSchema>>,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PipelineEdgeDataModeSchema {
    Context,
    Binding,
    None,
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineEdgeBindingSchema {
    /// Source path within upstream node output
    pub from: String,
    /// Target path within downstream node input
    pub to: String,
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

#[cfg(test)]
mod t600_9_schema_tests {
    //! Blue-team (T-600.9) focused serde coverage for the gateway-schema parity
    //! extensions on `PipelineParams` / `PipelineDefinitionSchema` / `PipelineEdgeSchema`.
    //! These exercise ONLY the fields the SWE added under SDD T-600.9.
    use super::*;
    use serde_json::json;

    /// Positive: a full payload carrying top-level `description`, a `definition`
    /// with `inputSchema`, and an edge with `type`/`mode`/`mapping`/`bindings`
    /// deserializes Ok and every new field lands in its struct field.
    #[test]
    fn deserializes_description_input_schema_and_edge_extensions() {
        let payload = json!({
            "description": "parity round-trip pipeline",
            "definition": {
                "nodes": [
                    { "id": "n1", "type": "tool", "toolName": "GetHelp" }
                ],
                "edges": [
                    {
                        "source": "n1",
                        "target": "n2",
                        "type": "success",
                        "mode": "binding",
                        "mapping": { "out.x": "in.y" },
                        "bindings": [ { "from": "a.b", "to": "c.d" } ]
                    }
                ],
                "inputSchema": { "type": "object" }
            }
        });

        let params: PipelineParams =
            serde_json::from_value(payload).expect("payload should deserialize Ok");

        assert_eq!(
            params.description.as_deref(),
            Some("parity round-trip pipeline"),
            "top-level description must land in PipelineParams::description"
        );

        let definition = params.definition.expect("definition must be present");
        assert!(
            definition.input_schema.is_some(),
            "inputSchema must map to PipelineDefinitionSchema::input_schema"
        );

        let edge = definition
            .edges
            .first()
            .expect("definition must contain the edge");
        assert!(
            matches!(edge.edge_type, PipelineEdgeType::Success),
            "edge `type: success` must flatten into edge_type"
        );
        assert!(
            matches!(edge.mode, Some(PipelineEdgeDataModeSchema::Binding)),
            "edge `mode: binding` must map to PipelineEdgeDataModeSchema::Binding"
        );
        let mapping = edge.mapping.as_ref().expect("edge mapping must be present");
        assert_eq!(
            mapping.get("out.x").map(String::as_str),
            Some("in.y"),
            "string->string mapping entry must round-trip"
        );
        let bindings = edge
            .bindings
            .as_ref()
            .expect("edge bindings must be present");
        assert_eq!(bindings.len(), 1, "one binding expected");
        assert_eq!(bindings[0].from, "a.b", "binding.from must round-trip");
        assert_eq!(bindings[0].to, "c.d", "binding.to must round-trip");
    }

    /// Positive: top-level `update` and `force` booleans deserialize into the
    /// new bool fields.
    #[test]
    fn deserializes_top_level_update_and_force_booleans() {
        let payload = json!({ "update": true, "force": true });
        let params: PipelineParams =
            serde_json::from_value(payload).expect("update/force payload should deserialize Ok");
        assert!(
            params.update,
            "update:true must map to PipelineParams::update"
        );
        assert!(params.force, "force:true must map to PipelineParams::force");
    }

    /// Negative: an unknown top-level field is rejected by `deny_unknown_fields`.
    #[test]
    fn rejects_unknown_top_level_field() {
        let payload = json!({ "bogusField": 1 });
        let result: Result<PipelineParams, _> = serde_json::from_value(payload);
        assert!(
            result.is_err(),
            "deny_unknown_fields must reject an unknown top-level field"
        );
    }

    /// Negative: the parity tightening to `HashMap<String, String>` means a
    /// mapping with a non-string value must be rejected. The rest of the payload
    /// is valid so the bad mapping value is the only failure source.
    #[test]
    fn rejects_mapping_with_non_string_value() {
        let payload = json!({
            "definition": {
                "nodes": [
                    { "id": "n1", "type": "tool", "toolName": "GetHelp" }
                ],
                "edges": [
                    {
                        "source": "n1",
                        "target": "n2",
                        "type": "success",
                        "mapping": { "x": 5 }
                    }
                ]
            }
        });
        let result: Result<PipelineParams, _> = serde_json::from_value(payload);
        assert!(
            result.is_err(),
            "mapping value 5 (non-string) must be rejected by HashMap<String, String>"
        );
    }
}
