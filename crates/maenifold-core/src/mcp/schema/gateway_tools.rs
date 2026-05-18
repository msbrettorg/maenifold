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
        "Start here on cold start. Lists the live gateway domains and commands currently exposed by Maenifold, including gateway-backed MCP server domains. Use domain/command filters plus includeSchemas or includeExamples to inspect exact call shapes before GatewayExecute.",
    )
}

pub(crate) fn gateway_execute_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayExecuteParams>(
        "GatewayExecute",
        "Runs one exact gateway command by domain and command with the provided parameters object. Use it after GatewayIndex has identified the target domain, command, schema, and safety class. It does not infer intent or choose commands from natural language.",
    )
}

pub(crate) fn gateway_batch_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayBatchParams>(
        "GatewayBatch",
        "Submits a batch of exact gateway calls, or polls status for one. Required: operation='submit'|'status'. For submit: items=[{domain, command, parameters}] and continueOnError: bool. For status: batchId. Example: {operation:'submit', continueOnError:false, items:[{domain:'maenifold.gate', command:'gate_Status', parameters:{root:'/repo'}}]}. Use only after GatewayIndex/GetHelp identifies each call.",
    )
}

pub(crate) fn gateway_intent_tool() -> McpToolDefinition {
    derived_tool_definition::<GatewayIntentParams>(
        "GatewayIntent",
        "Translates a natural-language request into likely gateway domain+command candidates when the exact command is unknown. Prefer GatewayIndex for deterministic cold-start discovery and schema inspection. Review candidates before executing anything with GatewayExecute.",
    )
}
