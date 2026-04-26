pub(crate) mod config_tools;
mod cors;
pub(crate) mod domain_mapping;
pub(crate) mod gateway_batch;
pub(crate) mod gateway_batch_types;
pub(crate) mod gateway_catalog;
#[cfg(test)]
mod gateway_catalog_domain_tests;
pub mod gateway_consolidation;
pub mod gateway_dispatch;
mod gateway_errors;
#[cfg(test)]
mod gateway_experiment;
#[cfg(test)]
mod gateway_experiment_fixtures;
#[cfg(test)]
mod gateway_experiment_tests;
mod gateway_handler;
#[cfg(test)]
mod gateway_integration_tests;
mod gateway_intent;
#[cfg(test)]
mod gateway_intent_tests;
mod gateway_resolver;
#[cfg(test)]
mod gateway_resolver_tests;
mod gateway_resolver_types;
mod gateway_schema_validation;
mod gateway_startup;
mod mcp_tool_calls;
pub mod http;
mod resource_routing;
pub mod resources;
pub mod schema;
pub mod stdio_pool;
mod stdio_pool_health;
mod stdio_pool_lifecycle;
mod stdio_pool_tools;
mod stdio_process_group;
pub mod stdio_session;
mod stdio_session_ops;
pub mod tool_routing;
pub(crate) mod tool_utils;
use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::registry::ToolRegistry;
use crate::mcp::gateway_handler::GatewayState;
pub(crate) use cors::localhost_cors;
pub use resources::*;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, ReadResourceRequestParams, ReadResourceResult,
    ServerCapabilities, ServerInfo, SubscribeRequestParams, Tool, UnsubscribeRequestParams,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
pub use tool_utils::{McpToolDefinition, list_mcp_tools};
use tool_utils::with_lock_retry;
pub struct MaenifoldMcpServer;
struct MaenifoldHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    gateway: Option<Arc<GatewayState>>,
    gateway_diagnostics: Option<Arc<crate::mcp::stdio_pool::StdioPool>>,
    gateway_startup_error: Option<String>,
    config_mutation: Option<config_tools::ConfigMutationState>,
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
            config_mutation: None,
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
    fn set_config_mutation(&mut self, state: config_tools::ConfigMutationState) {
        self.config_mutation = Some(state);
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
            let mut tools: Vec<Tool> = self
                .tools
                .iter()
                .filter(|tool| {
                    tool.name == "GetHelp"
                        || tool.name == "ReadMcpResource"
                        || self::config_tools::is_config_mutation_tool(tool.name.as_ref())
                })
                .cloned()
                .collect();
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
    fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ListResourceTemplatesResult, rmcp::ErrorData>>
    + Send
    + '_ {
        std::future::ready(
            self::resources::handle_list_resource_templates()
                .map_err(self::resources::map_resource_error),
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
        let config_mutation = self.config_mutation.clone();
        let gateway = self.gateway.clone();
        async move {
            self::mcp_tool_calls::handle_tool_call(tool_name, arguments, config_mutation, gateway)
                .await
        }
    }
}
impl MaenifoldMcpServer {
    pub fn list_tools() -> Result<Vec<McpToolDefinition>> {
        list_mcp_tools(ToolRegistry::global())
    }
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
