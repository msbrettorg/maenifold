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
    pub idempotency_key: Option<String>,
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
