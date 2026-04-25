use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, MaenifoldError, Result,
};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::gateway_dispatch::{DispatchResult, dispatch_facade_call};
use crate::mcp::stdio_pool::StdioPool;
use crate::mcp::tool_routing::ToolRoutingRegistry;
use crate::mcp::tool_utils::format_tool_error;
use rmcp::model::{CallToolResult, Content, Tool};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::{error, warn};

#[derive(Clone)]
pub(crate) struct GatewayState {
    pool: Arc<StdioPool>,
    routing: Arc<ToolRoutingRegistry>,
    consolidated_tools: Vec<ConsolidatedServerTool>,
    passthrough_tools: Vec<Tool>,
}

impl GatewayState {
    pub(crate) fn new(
        pool: Arc<StdioPool>,
        routing: Arc<ToolRoutingRegistry>,
        consolidated_tools: Vec<ConsolidatedServerTool>,
        passthrough_tools: Vec<Tool>,
    ) -> Self {
        Self {
            pool,
            routing,
            consolidated_tools,
            passthrough_tools,
        }
    }

    pub(crate) fn pool(&self) -> Arc<StdioPool> {
        Arc::clone(&self.pool)
    }

    pub(crate) fn list_tools(&self) -> Vec<Tool> {
        let mut tools = self
            .consolidated_tools
            .iter()
            .map(|server| server.facade_tool.clone())
            .collect::<Vec<_>>();
        tools.extend(self.passthrough_tools.iter().cloned());
        tools
    }

    pub(crate) async fn handle_tool_call(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> std::result::Result<Option<CallToolResult>, rmcp::ErrorData> {
        if let Some(server) = self.find_facade(tool_name) {
            return self
                .handle_facade_call(server, tool_name, arguments)
                .await
                .map(Some);
        }

        if let Some(server_name) = self.routing.lookup(tool_name).map(str::to_string) {
            let gateway_result = self.pool.call_tool(&server_name, tool_name, arguments).await;
            return match gateway_result {
                Ok(result) => Ok(Some(result)),
                Err(err) => {
                    log_gateway_error(Some(&server_name), tool_name, &err);
                    Err(map_gateway_error(tool_name, err))
                }
            };
        }

        Ok(None)
    }

    fn find_facade(&self, tool_name: &str) -> Option<&ConsolidatedServerTool> {
        self.consolidated_tools
            .iter()
            .find(|server| server.facade_tool.name.to_string() == tool_name)
    }

    async fn handle_facade_call(
        &self,
        server: &ConsolidatedServerTool,
        facade_name: &str,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args = parse_facade_arguments(facade_name, arguments)
            .map_err(|err| map_gateway_error(facade_name, err))?;
        let dispatch = dispatch_facade_call(
            server,
            args.intent.as_deref(),
            args.command.as_deref(),
            args.learn,
            args.parameters,
        )
        .map_err(|err| map_gateway_error(facade_name, err))?;

        match dispatch {
            DispatchResult::CallTool {
                server_name,
                tool_name,
                parameters,
            } => {
                let gateway_result = self.pool.call_tool(&server_name, &tool_name, parameters).await;
                match gateway_result {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        log_gateway_error(Some(&server_name), &tool_name, &err);
                        Err(map_gateway_error(&tool_name, err))
                    }
                }
            }
            DispatchResult::LearnCatalog(text) => Ok(CallToolResult::success(vec![Content::text(text)])),
            DispatchResult::LearnSchema(schema) => Ok(CallToolResult::success(vec![Content::text(
                render_json(schema),
            )])),
            DispatchResult::Candidates(candidates) => Ok(CallToolResult::success(vec![Content::text(
                render_candidates(candidates),
            )])),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FacadeArguments {
    intent: Option<String>,
    command: Option<String>,
    learn: bool,
    parameters: Value,
}

impl Default for FacadeArguments {
    fn default() -> Self {
        Self {
            intent: None,
            command: None,
            learn: false,
            parameters: Value::Object(Map::new()),
        }
    }
}

fn parse_facade_arguments(tool_name: &str, arguments: Value) -> Result<FacadeArguments> {
    let mut parsed: FacadeArguments = serde_json::from_value(arguments).map_err(|error| {
        MaenifoldError::Message(format!(
            "Invalid arguments for gateway facade '{tool_name}': {error}"
        ))
    })?;
    if parsed.parameters.is_null() {
        parsed.parameters = Value::Object(Map::new());
    }
    Ok(parsed)
}

fn render_candidates(candidates: Vec<(String, String)>) -> String {
    let lines = candidates
        .into_iter()
        .map(|(name, description)| format!("- {name}: {description}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Matching commands:\n{lines}")
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

fn map_gateway_error(tool: &str, err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        MaenifoldError::JsonRpc { code, message, data } => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(i32::try_from(code).unwrap_or(JSONRPC_INTERNAL_ERROR as i32)),
            message,
            Some(data),
        ),
        _ => rmcp::ErrorData::internal_error(format_tool_error(tool, &err), None),
    }
}

fn log_gateway_error(server: Option<&str>, tool: &str, err: &MaenifoldError) {
    let server = server.unwrap_or("");
    match err {
        MaenifoldError::JsonRpc { code, message, data }
            if *code == JSONRPC_INTERNAL_ERROR || *code == GATEWAY_TIMEOUT_ERROR =>
        {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                error!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                error!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        MaenifoldError::JsonRpc { code, message, data } => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                warn!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                warn!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        _ => warn!(server, tool, err = %err, "gateway tool call failed"),
    }
}
