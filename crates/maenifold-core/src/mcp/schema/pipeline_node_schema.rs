use super::params::PipelineNodeSchema;
use super::pipeline_node_schema_data_variants::{
    sequential_thinking_variant_schema, tool_variant_schema,
};
use super::pipeline_node_schema_workflow_variants::{
    prompt_variant_schema, workflow_variant_schema,
};
use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde_json::json;

/// Custom `JsonSchema` for `PipelineNodeSchema` that produces a **flat `oneOf`** with every
/// variant schema fully inlined — no `$ref` pointers into a `definitions` block.
///
/// This is required for compatibility with GPT-5.4 and other OpenAI models whose schema
/// validator rejects externally-tagged enum wrappers like `{"workflow": {...}}` and rejects
/// schemas where `$ref` appears alongside sibling keywords inside a `oneOf` variant.
///
/// Each variant is a plain JSON Schema object with:
/// - `type: "object"`
/// - `additionalProperties: false`
/// - `required: [...]` listing all mandatory fields
/// - `properties: {...}` with all fields inlined (including `agentConfig`)
///
/// The `type` property inside every variant acts as a discriminator
/// (`"enum": ["workflow"]`, `"enum": ["prompt"]`, etc.) so agents can unambiguously
/// identify which variant they are targeting.
impl JsonSchema for PipelineNodeSchema {
    /// Returning `false` forces schemars to inline this schema wherever it appears
    /// (e.g. as array `items`) instead of emitting a `$ref` to `definitions`.
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        "PipelineNodeSchema".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        serde_json::from_value(node_schema_value())
            .expect("PipelineNodeSchema value is valid JSON Schema")
    }
}

/// Shared inline schema for the `agentConfig` property that appears in prompt,
/// sequentialThinking, and tool node variants.
//
// T-01.9: `pub(super)` so the variant modules can compose this helper inline.
pub(super) fn agent_config_schema() -> serde_json::Value {
    json!({
        "description": "Agent configuration for pipeline node execution",
        "type": "object",
        "additionalProperties": false,
            "properties": {
                "model": {
                    "type": "string",
                    "description": "Model override for the agent session"
                },
            "mode": {
                "type": "string",
                "description": "Session mode override for the agent session"
            },
            "thoughtLevel": {
                "type": "string",
                "description": "Thought level override for the agent session"
            },
            "configOptions": {
                "type": "object",
                "description": "Arbitrary config options forwarded to the agent session",
                "additionalProperties": true
            }
        }
    })
}

// T-01.9: `pub(super)` so the variant modules can compose this helper inline.
pub(super) fn node_adopt_schema() -> serde_json::Value {
    json!({
        "description": "Per-node adoption configuration applied before prompt dispatch",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "role": {
                "type": "string",
                "description": "Optional role asset identifier to adopt"
            },
            "color": {
                "type": "string",
                "description": "Optional color asset identifier to adopt"
            },
            "perspective": {
                "type": "string",
                "description": "Optional perspective asset identifier to adopt"
            }
        }
    })
}

pub(super) fn join_policy_schema() -> serde_json::Value {
    json!({
        "type": "string",
        "enum": ["all", "any", "first"],
        "description": "Fan-in join policy. 'all' (default) waits for every inbound predecessor; 'any' fires when at least one completes and the rest are terminal; 'first' fires as soon as any completes."
    })
}

// T-01.9: Variant bodies were extracted to sibling modules; this entry point
// just composes the `oneOf` array so the file stays well under the 300 LOC ceiling.
fn node_schema_value() -> serde_json::Value {
    json!({
        "oneOf": [
            workflow_variant_schema(),
            prompt_variant_schema(),
            sequential_thinking_variant_schema(),
            tool_variant_schema(),
        ]
    })
}
