pub mod http;
pub mod resources;
pub mod schema;
pub mod stdio_pool;
pub mod stdio_session;
pub mod tool_routing;
pub mod gateway_consolidation;
mod gateway_startup;
pub(crate) mod tool_utils;
pub use resources::*;
pub use tool_utils::{McpToolDefinition, list_mcp_tools};
use tool_utils::{format_tool_error, with_lock_retry};
pub(crate) fn localhost_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
    let origin_predicate = AllowOrigin::predicate(
        |origin: &axum::http::HeaderValue, _parts: &axum::http::request::Parts| {
            let Ok(s) = origin.to_str() else {
                return false;
            };
            s == "null"
                || s == "file://"
                || s == "http://localhost"
                || s.starts_with("http://localhost:")
                || s == "http://127.0.0.1"
                || s.starts_with("http://127.0.0.1:")
                || s == "http://[::1]"
                || s.starts_with("http://[::1]:")
        },
    );
    CorsLayer::new()
        .allow_origin(origin_predicate)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .expose_headers([axum::http::HeaderName::from_static("mcp-session-id")])
}

use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, JSONRPC_METHOD_NOT_FOUND, MaenifoldError,
    Result,
};
use crate::registry::ToolRegistry;
use crate::{mcp::{stdio_pool::StdioPool, tool_routing::ToolRoutingRegistry}, util::json::OutputFormat};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListResourcesResult,
    ListToolsResult, ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
    SubscribeRequestParams, Tool, UnsubscribeRequestParams,
};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};
pub struct MaenifoldMcpServer;
struct MaenifoldHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    gateway_pool: Option<Arc<StdioPool>>,
    gateway_routing: Option<Arc<ToolRoutingRegistry>>,
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
            gateway_pool: None,
            gateway_routing: None,
        })
    }
    fn set_gateway(&mut self, pool: Arc<StdioPool>, routing: Arc<ToolRoutingRegistry>) {
        self.gateway_pool = Some(pool);
        self.gateway_routing = Some(routing);
    }
}
impl ServerHandler for MaenifoldHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = match self.transport {
            TransportKind::Stdio => ServerCapabilities::builder().enable_tools().enable_resources()
                .enable_resources_subscribe().enable_resources_list_changed().build(),
            TransportKind::Http => ServerCapabilities::builder().enable_tools().enable_resources()
                .enable_resources_subscribe().build(),
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
        let mut tools = self.tools.clone();
        if let Some(routing) = &self.gateway_routing { tools.extend_from_slice(routing.gateway_tools()); }
        std::future::ready(Ok(ListToolsResult { tools, next_cursor: None, meta: None }))
    }
    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> std::result::Result<ListResourcesResult, rmcp::ErrorData> {
        tokio::task::spawn_blocking(|| with_lock_retry(self::resources::handle_list_resources))
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?
            .map_err(self::resources::map_resource_error)
    }
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl Future<Output = std::result::Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        let uri = request.uri.to_string();
        let gateway_pool = self.gateway_pool.clone();
        async move {
            if uri.starts_with("ui://")
                && self.gateway_routing.is_some()
                && let Some(pool) = gateway_pool
            {
                let mut last_err = None;
                for (server_name, _) in pool.all_gateway_tools() {
                    match pool.read_resource(&server_name, &uri).await {
                        Ok(result) => return Ok(result),
                        Err(e) => { last_err = Some(e); }
                    }
                }
                return Err(match last_err {
                    Some(MaenifoldError::JsonRpc { code, message, .. }) if code == -32002 => rmcp::ErrorData::resource_not_found(message, None),
                    Some(err) => rmcp::ErrorData::internal_error(format!("Gateway resource fetch failed for '{uri}': {err}"), None),
                    None => rmcp::ErrorData::resource_not_found(format!("No gateway server configured for '{uri}'"), None),
                });
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
        let tool_name_for_invoke = tool_name.clone();
        let native_arguments = arguments.clone();

        async move {
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
                    if let (Some(pool), Some(routing)) = (self.gateway_pool.clone(), self.gateway_routing.clone())
                        && let Some(server_name) = routing.lookup(&tool_name)
                    {
                        let server_name = server_name.to_string();
                        let gateway_result = pool.call_tool(&server_name, &tool_name, arguments).await;
                        return match gateway_result {
                            Ok(result) => Ok(result),
                            Err(err) => {
                                log_gateway_error(Some(&server_name), &tool_name, &err);
                                Err(map_gateway_error(&tool_name, err))
                            }
                        };
                    }
                    let err = MaenifoldError::JsonRpc {
                        code: JSONRPC_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: '{tool_name}'"),
                        data: Value::Null,
                    };
                    log_gateway_error(None, &tool_name, &err);
                    Err(map_gateway_error(&tool_name, err))
                }
                Err(err) => Ok(CallToolResult::error(vec![Content::text(format_tool_error(
                    &tool_name, &err,
                ))])),
            }
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
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) { error!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed"); }
            else { error!(server, tool, err = %message, code, "gateway tool call failed"); }
        }
        MaenifoldError::JsonRpc { code, message, data } => {
            if let Some(stderr_tail) = data.get("stderrTail").and_then(Value::as_str) { warn!(server, tool, err = %message, code, stderr_tail, "gateway tool call failed"); }
            else { warn!(server, tool, err = %message, code, "gateway tool call failed"); }
        }
        _ => warn!(server, tool, err = %err, "gateway tool call failed"),
    }
}
