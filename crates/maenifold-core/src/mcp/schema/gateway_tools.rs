use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayIndexParams {
    pub domain: Option<String>,
    pub command: Option<String>,
    pub include_schemas: Option<bool>,
    pub include_examples: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayExecuteParams {
    pub domain: String,
    pub command: String,
    pub parameters: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayBatchParams {
    pub operation: String,
    pub items: Option<Vec<BatchItemParams>>,
    pub continue_on_error: Option<bool>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BatchItemParams {
    pub domain: String,
    pub command: String,
    pub parameters: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayIntentParams {
    pub intent: String,
    pub domain_hints: Option<Vec<String>>,
}

pub(crate) fn gateway_index_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayIndexParams>(
        "GatewayIndex",
        "Lists deterministic gateway domains and commands. Filter by domain and command to inspect exact schemas.",
    )
}

pub(crate) fn gateway_execute_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayExecuteParams>(
        "GatewayExecute",
        "Executes an exact gateway command by domain and command using the provided parameters object.",
    )
}

pub(crate) fn gateway_batch_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayBatchParams>(
        "GatewayBatch",
        "Submits multiple exact gateway commands sequentially or reads bounded in-memory batch status.",
    )
}

pub(crate) fn gateway_intent_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayIntentParams>(
        "GatewayIntent",
        "Resolves natural language to deterministic gateway candidates, auto-executing only safe single read operations when configured.",
    )
}
