use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, JSONRPC_INVALID_PARAMS, MaenifoldError,
};
use crate::mcp::domain_mapping;
use crate::mcp::gateway_catalog::{CommandTarget, GatewayCatalog};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::gateway_dispatch::validate_dispatch_parameters;
use crate::mcp::resource_routing::{ResourceRoutingRegistry, map_gateway_resource_error};
use crate::mcp::schema::gateway_tools::{
    GatewayExecuteParams, GatewayIndexParams, gateway_execute_tool, gateway_index_tool,
};
use crate::mcp::stdio_pool::{StdioPool, StdioPoolHealth};
use crate::mcp::tool_utils::format_tool_error;
use crate::registry::ToolRegistry;
use crate::util::json::OutputFormat;
use rmcp::model::{CallToolResult, Content, ReadResourceResult, Resource, Tool};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, warn};

#[derive(Clone)]
pub(crate) struct GatewayState {
    pool: Arc<StdioPool>,
    resource_routing: Arc<ResourceRoutingRegistry>,
    catalog: GatewayCatalog,
    consolidated_tools: Vec<ConsolidatedServerTool>,
    hidden_tool_domains: HashMap<String, String>,
}

impl GatewayState {
    pub(crate) fn new(
        pool: Arc<StdioPool>,
        resource_routing: Arc<ResourceRoutingRegistry>,
        catalog: GatewayCatalog,
        consolidated_tools: Vec<ConsolidatedServerTool>,
    ) -> Self {
        let mut hidden_tool_domains = HashMap::new();
        for server in &consolidated_tools {
            hidden_tool_domains.insert(legacy_facade_name(&server.server_name), server.server_name.clone());
            for tool in &server.tool_catalog {
                hidden_tool_domains.insert(tool.name.to_string(), server.server_name.clone());
            }
        }
        Self {
            pool,
            resource_routing,
            catalog,
            consolidated_tools,
            hidden_tool_domains,
        }
    }

    pub(crate) fn pool(&self) -> Arc<StdioPool> {
        Arc::clone(&self.pool)
    }

    #[allow(dead_code)]
    pub(crate) async fn startup_diagnostics(&self) -> StdioPoolHealth {
        self.pool.health().await
    }

    pub(crate) fn list_tools(&self) -> Vec<Tool> {
        vec![
            tool_from_definition(gateway_index_tool()),
            tool_from_definition(gateway_execute_tool()),
        ]
    }

    pub(crate) fn list_resources(&self) -> Vec<Resource> {
        self.resource_routing.gateway_resources().to_vec()
    }

    pub(crate) async fn read_resource(
        &self,
        uri: &str,
    ) -> std::result::Result<ReadResourceResult, rmcp::ErrorData> {
        let Some(server_name) = self.resource_routing.lookup(uri).map(str::to_string) else {
            return Err(rmcp::ErrorData::resource_not_found(
                format!("No gateway resource owner found for '{uri}'"),
                None,
            ));
        };
        self.pool
            .read_resource(&server_name, uri)
            .await
            .map_err(|err| map_gateway_resource_error(&server_name, uri, err))
    }

    pub(crate) async fn handle_tool_call(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> std::result::Result<Option<CallToolResult>, rmcp::ErrorData> {
        if tool_name == "GatewayIndex" {
            return self.handle_gateway_index(arguments).map(Some);
        }
        if tool_name == "GatewayExecute" {
            return self.handle_gateway_execute(arguments).await.map(Some);
        }

        if let Some(domain) = domain_mapping::tool_domain(tool_name) {
            return Err(rmcp::ErrorData::invalid_params(
                hidden_tool_message(tool_name, domain),
                None,
            ));
        }
        if let Some(domain) = self.hidden_tool_domains.get(tool_name) {
            return Err(rmcp::ErrorData::invalid_params(
                hidden_tool_message(tool_name, domain),
                None,
            ));
        }

        Ok(None)
    }

    fn handle_gateway_index(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayIndexParams = serde_json::from_value(arguments)
            .map_err(|err| map_gateway_error("GatewayIndex", invalid_params(err)))?;
        let index = self
            .catalog
            .render_index(
                args.domain.as_deref(),
                args.command.as_deref(),
                args.include_schemas.unwrap_or(false),
                args.include_examples.unwrap_or(false),
            )
            .map_err(|err| map_gateway_error("GatewayIndex", err))?;
        Ok(CallToolResult::success(vec![Content::text(render_json(index))]))
    }

    async fn handle_gateway_execute(
        &self,
        arguments: Value,
    ) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
        let args: GatewayExecuteParams = serde_json::from_value(arguments)
            .map_err(|err| map_gateway_error("GatewayExecute", invalid_params(err)))?;
        let parameters = Value::Object(args.parameters);
        let _idempotency_key = args.idempotency_key.as_deref();
        match self
            .catalog
            .resolve(&args.domain, &args.command)
            .map_err(|err| map_gateway_error("GatewayExecute", err))?
        {
            CommandTarget::Native { tool_name } => {
                invoke_native(tool_name, parameters).await
            }
            CommandTarget::Upstream {
                server_name,
                tool_name,
            } => {
                let server = self.consolidated_server(&server_name)?;
                validate_dispatch_parameters(server, &tool_name, &parameters)
                    .map_err(|err| map_gateway_error(&tool_name, err))?;
                match self.pool.call_tool(&server_name, &tool_name, parameters).await {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        log_gateway_error(Some(&server_name), &tool_name, &err);
                        Err(map_gateway_error(&tool_name, err))
                    }
                }
            }
        }
    }

    fn consolidated_server(
        &self,
        server_name: &str,
    ) -> std::result::Result<&ConsolidatedServerTool, rmcp::ErrorData> {
        self.consolidated_tools
            .iter()
            .find(|server| server.server_name == server_name)
            .ok_or_else(|| {
                rmcp::ErrorData::internal_error(
                    format!("Gateway catalog resolved missing server '{server_name}'"),
                    None,
                )
            })
    }
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

async fn invoke_native(
    tool_name: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let display_name = tool_name.clone();
    let result = tokio::task::spawn_blocking(move || {
        ToolRegistry::global().try_invoke(&tool_name, &parameters, OutputFormat::Json)
    })
    .await
    .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;
    match result {
        Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(format_tool_error(
            &display_name,
            &err,
        ))])),
    }
}

fn tool_from_definition(definition: crate::mcp::McpToolDefinition) -> Tool {
    let schema = match definition.input_schema {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    Tool::new(definition.name, definition.description, Arc::new(schema))
}

fn invalid_params(error: serde_json::Error) -> MaenifoldError {
    MaenifoldError::GatewayInvalidParams(error.to_string())
}

fn hidden_tool_message(tool_name: &str, domain: &str) -> String {
    if tool_name.starts_with("gateway_") {
        return format!(
            "Gateway facade '{tool_name}' is retired. Use GatewayIndex with domain='{domain}' to inspect commands, then GatewayExecute."
        );
    }
    format!(
        "Tool '{tool_name}' is routed under domain '{domain}'. Use GatewayExecute with domain='{domain}', command='{tool_name}'."
    )
}

fn legacy_facade_name(server_name: &str) -> String {
    let sanitized: String = server_name
        .chars()
        .flat_map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec!['_']
            }
        })
        .take(60)
        .collect();
    format!("gateway_{sanitized}")
}

fn map_gateway_error(tool: &str, err: MaenifoldError) -> rmcp::ErrorData {
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(i32::try_from(code).unwrap_or(JSONRPC_INTERNAL_ERROR as i32)),
            message,
            Some(data),
        ),
        MaenifoldError::GatewayInvalidParams(message) => rmcp::ErrorData::new(
            rmcp::model::ErrorCode(JSONRPC_INVALID_PARAMS as i32),
            message,
            Some(Value::Null),
        ),
        _ => rmcp::ErrorData::internal_error(format_tool_error(tool, &err), None),
    }
}

fn log_gateway_error(server: Option<&str>, tool: &str, err: &MaenifoldError) {
    let server = server.unwrap_or("");
    match err {
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } if *code == JSONRPC_INTERNAL_ERROR || *code == GATEWAY_TIMEOUT_ERROR => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                error!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                error!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        MaenifoldError::JsonRpc {
            code,
            message,
            data,
        } => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) {
                warn!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed");
            } else {
                warn!(server, tool, err = %message, code, "gateway tool call failed");
            }
        }
        _ => warn!(server, tool, err = %err, "gateway tool call failed"),
    }
}
