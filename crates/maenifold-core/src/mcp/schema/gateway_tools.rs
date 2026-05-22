use super::support::derived_tool_definition;
use crate::mcp::McpToolDefinition;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayIndexParams {
    /// Optional gateway domain to filter the catalog to (e.g. `maenifold.gate`,
    /// `maenifold.memory`, `playwright`). Omit to list every domain.
    pub domain: Option<String>,
    /// Optional command name to inspect a single command within `domain`.
    /// Requires `domain` to be set; otherwise the call errors.
    pub command: Option<String>,
    /// When true, includes each command's JSON Schema so the caller can build
    /// a valid `parameters` payload before `GatewayExecute`.
    pub include_schemas: Option<bool>,
    /// When true, includes worked example invocations alongside each command.
    pub include_examples: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayExecuteParams {
    /// Gateway domain that owns the command (e.g. `maenifold.gate`,
    /// `maenifold.memory`, upstream server names like `playwright` or
    /// `azure-mcp-server`). Look these up via `GatewayIndex`.
    pub domain: String,
    /// Exact command name within `domain`, taken verbatim from `GatewayIndex`
    /// (case-sensitive, no fuzzy matching).
    pub command: String,
    /// JSON object holding every parameter the command's schema declares as
    /// required, plus any optional ones you want to override. Use
    /// `GatewayIndex` with `includeSchemas: true` to see the schema.
    pub parameters: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayBatchParams {
    /// Batch operation: `"submit"` to dispatch a new batch of gateway calls,
    /// or `"status"` to poll an existing one by `batchId`.
    pub operation: String,
    /// Required for `operation: "submit"`. Array of exact gateway calls; each
    /// item has the same `{domain, command, parameters}` shape as
    /// `GatewayExecute`. An empty array is accepted and returns immediately.
    pub items: Option<Vec<BatchItemParams>>,
    /// Optional for `operation: "submit"`. When true (default false), keeps
    /// executing remaining items after one fails. When false, stops on the
    /// first failure.
    pub continue_on_error: Option<bool>,
    /// Required for `operation: "status"`. The id returned by a previous
    /// `submit` call.
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BatchItemParams {
    /// Gateway domain (see `GatewayExecute.domain`).
    pub domain: String,
    /// Exact command name (see `GatewayExecute.command`).
    pub command: String,
    /// Parameter object for this command (see `GatewayExecute.parameters`).
    pub parameters: Map<String, Value>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GatewayIntentParams {
    /// Natural-language description of what you want to accomplish. The
    /// resolver maps this to ranked `{domain, command, parametersDraft}`
    /// candidates from the live catalog. Read-only candidates may auto-execute
    /// when one clearly wins AND its draft already includes every required
    /// parameter; otherwise you get back candidates to review and pass to
    /// `GatewayExecute`.
    pub intent: String,
    /// Optional list of domain names to bias the resolver toward (e.g.
    /// `["maenifold.gate"]`). Passed to the LLM as additional context.
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
        "Translates a natural-language request into likely gateway domain+command candidates when the exact command is unknown. Prefer GatewayIndex for deterministic cold-start discovery and schema inspection. Each returned candidate includes `missingRequiredParameters`; auto-execute is suppressed whenever the resolver's parameters_draft does not satisfy the command's required-param set, so callers always see what they still need to fill before GatewayExecute.",
    )
}
