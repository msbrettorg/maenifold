mod cors;
pub(crate) mod domain_mapping;
pub(crate) mod gateway_batch;
pub(crate) mod gateway_batch_types;
pub(crate) mod gateway_catalog;
#[cfg(test)] mod gateway_catalog_domain_tests;
pub mod gateway_consolidation;
pub mod gateway_dispatch;
mod gateway_errors;
#[cfg(test)] mod gateway_experiment;
#[cfg(test)] mod gateway_experiment_fixtures;
#[cfg(test)] mod gateway_experiment_tests;
mod gateway_handler;
mod gateway_resolver;
mod gateway_resolver_types;
#[cfg(test)] mod gateway_resolver_tests;
#[cfg(test)] mod gateway_integration_tests;
mod gateway_schema_validation;
mod gateway_startup;
pub mod http;
mod resource_routing;
pub mod resources;
pub mod schema;
mod stdio_io;
pub mod stdio_pool;
mod stdio_pool_health;
mod stdio_pool_lifecycle;
mod stdio_pool_tools;
mod stdio_resource_reader;
pub mod stdio_session;
pub mod tool_routing;
pub(crate) mod tool_utils;
pub(crate) use cors::localhost_cors;
pub use resources::*;
pub use tool_utils::{McpToolDefinition, list_mcp_tools};
use tool_utils::{format_tool_error, with_lock_retry};
use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{JSONRPC_METHOD_NOT_FOUND, MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::{mcp::gateway_handler::GatewayState, util::json::OutputFormat};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, ReadResourceRequestParams, ReadResourceResult,
    ServerCapabilities, ServerInfo, SubscribeRequestParams, Tool, UnsubscribeRequestParams,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub struct MaenifoldMcpServer;
struct MaenifoldHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    gateway: Option<Arc<GatewayState>>,
    gateway_diagnostics: Option<Arc<crate::mcp::stdio_pool::StdioPool>>,
    gateway_startup_error: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransportKind {
    Stdio,
    Http,
}
impl MaenifoldHandler {
    fn new(transport: TransportKind) -> Result<Self> {
        let mcp_tools = list_mcp_tools(&ToolRegistry::new())?;
        let tools = mcp_tools
            .into_iter()
            .map(|t| {
                let schema: serde_json::Map<String, Value> = match t.input_schema {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                Tool::new(t.name, t.description, Arc::new(schema))
            })
            .collect();
        Ok(Self {
            transport,
            tools,
            gateway: None,
            gateway_diagnostics: None,
            gateway_startup_error: None,
        })
    }
    fn set_gateway(&mut self, gateway: Arc<GatewayState>) {
        self.gateway = Some(gateway);
    }
    fn set_gateway_diagnostics(&mut self, diagnostics: Arc<crate::mcp::stdio_pool::StdioPool>) {
        self.gateway_diagnostics = Some(diagnostics);
    }
    fn set_gateway_startup_error(&mut self, error: String) {
        self.gateway_startup_error = Some(error);
    }
}
impl ServerHandler for MaenifoldHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = match self.transport {
            TransportKind::Stdio => ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_resources_list_changed()
                .build(),
            TransportKind::Http => ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .build(),
        };
        ServerInfo::new(capabilities)
            .with_server_info(Implementation::new("maenifold", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Use maenifold tools to read, write, search, and organize memory files, \
                 inspect graph relationships, and run workflow or thinking sessions. \
                 Prefer exact tool names from tools/list and pass JSON arguments that \
                 match each tool schema.",
            )
    }
    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListToolsResult, rmcp::ErrorData>> + Send + '_
    {
        if let Some(error) = &self.gateway_startup_error {
            return std::future::ready(Err(rmcp::ErrorData::internal_error(
                format!("Gateway startup failed: {error}"),
                None,
            )));
        }
        let tools = if let Some(gateway) = &self.gateway {
            let mut tools: Vec<Tool> = self.tools.iter().filter(|tool| tool.name == "GetHelp").cloned().collect();
            tools.extend(gateway.list_tools());
            tools
        } else {
            self.tools.clone()
        };
        std::future::ready(Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        }))
    }
    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> std::result::Result<ListResourcesResult, rmcp::ErrorData> {
        let mut result =
            tokio::task::spawn_blocking(|| with_lock_retry(self::resources::handle_list_resources))
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
                .map_err(self::resources::map_resource_error)?;
        if let Some(gateway) = &self.gateway {
            result.resources.extend(gateway.list_resources());
        }
        Ok(result)
    }
    fn list_resource_templates(&self, _request: Option<rmcp::model::PaginatedRequestParams>, _context: rmcp::service::RequestContext<rmcp::RoleServer>) -> impl Future<Output = std::result::Result<ListResourceTemplatesResult, rmcp::ErrorData>> + Send + '_ {
        std::future::ready(
            self::resources::handle_list_resource_templates().map_err(self::resources::map_resource_error),
        )
    }
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let uri = request.uri.to_string();
        let gateway = self.gateway.clone();
        let gateway_diagnostics = self.gateway_diagnostics.clone();
        async move {
            if self::resources::is_gateway_uri(&uri) {
                return self::resources::handle_gateway_read_resource(&uri, gateway_diagnostics)
                    .await
                    .map_err(self::resources::map_resource_error);
            }
            if uri.starts_with("ui://")
                && let Some(gateway) = gateway
            {
                return gateway.read_resource(&uri).await;
            }
            tokio::task::spawn_blocking(move || {
                with_lock_retry(|| self::resources::handle_read_resource(&uri))
            })
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
            .map_err(self::resources::map_resource_error)
        }
    }
    fn subscribe(
        &self,
        request: SubscribeRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<(), rmcp::ErrorData>> + Send + '_ {
        std::future::ready(self::resources::handle_subscribe(&request.uri, &context))
    }
    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<(), rmcp::ErrorData>> + Send + '_ {
        std::future::ready(self::resources::handle_unsubscribe(&request.uri, &context))
    }
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        let tool_name = request.name.to_string();
        let arguments: Value = match request.arguments {
            Some(map) => Value::Object(map),
            None => json!({}),
        };
        async move {
            if let Some(gateway) = self.gateway.clone() {
                if tool_name != "GetHelp" {
                    if let Some(gateway_result) =
                        gateway.handle_tool_call(&tool_name, arguments).await?
                    {
                        return Ok(gateway_result);
                    }
                    return Err(unknown_tool_error(&tool_name));
                }
            }
            let tool_name_for_invoke = tool_name.clone();
            let native_arguments = arguments;
            let result = tokio::task::spawn_blocking(move || {
                ToolRegistry::global().try_invoke(
                    &tool_name_for_invoke,
                    &native_arguments,
                    OutputFormat::Json,
                )
            })
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

            match result {
                Ok(output) => Ok(CallToolResult::success(vec![Content::text(output)])),
                Err(MaenifoldError::UnknownTool(_)) => {
                    Err(unknown_tool_error(&tool_name))
                }
                Err(err) => Ok(CallToolResult::error(vec![Content::text(
                    format_tool_error(&tool_name, &err),
                )])),
            }
        }
    }
}
fn unknown_tool_error(tool_name: &str) -> rmcp::ErrorData {
    rmcp::ErrorData::new(rmcp::model::ErrorCode(JSONRPC_METHOD_NOT_FOUND as i32), format!("Unknown tool: '{tool_name}'"), Some(Value::Null))
}
impl MaenifoldMcpServer {
    pub fn list_tools() -> Result<Vec<McpToolDefinition>> { list_mcp_tools(ToolRegistry::global()) }
    pub async fn serve_stdio(shutdown: CancellationToken) -> Result<()> {
        let config = Config::current();
        initialize_runtime(&config)?;
        let handler = MaenifoldHandler::new(TransportKind::Stdio)?;
        let transport = rmcp::transport::io::stdio();
        let service = handler
            .serve(transport)
            .await
            .map_err(|e| MaenifoldError::Message(format!("MCP server init failed: {e}")))?;
        let asset_watcher = self::resources::start_asset_watcher(&config, service.peer().clone())?;
        let wait_result = tokio::select! {
            result = service.waiting() => {
                result
                    .map(|_| ())
                    .map_err(|e| MaenifoldError::Message(format!("MCP server error: {e}")))
            }
            _ = shutdown.cancelled() => Ok(())
        };
        asset_watcher.stop();
        wait_result?;
        Ok(())
    }
}

pub fn initialize_runtime(config: &Config) -> Result<()> {
    config.ensure_directories()?;
    AssetManager::initialize_assets(config)?;
    GraphDatabase::initialize_database(config)?;
    Ok(())
}
