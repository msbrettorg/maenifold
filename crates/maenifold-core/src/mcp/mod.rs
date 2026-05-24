mod client_handler;
#[cfg(test)]
mod client_handler_tests;
#[cfg(test)]
mod config_tools;
#[cfg(test)]
mod config_tools_tests;
mod cors;
pub(crate) mod domain_mapping;
mod elicitation_validation;
pub(crate) mod gateway_batch;
pub(crate) mod gateway_batch_types;
mod gateway_cli;
mod gateway_direct_inference_resolver;
#[cfg(test)]
mod gateway_experiment;
#[cfg(test)]
mod gateway_experiment_fixtures;
#[cfg(test)]
mod gateway_experiment_tests;
mod gateway_gate;
mod gateway_intent;
#[cfg(test)]
mod gateway_intent_tests;
pub(crate) mod gateway_registry;
mod gateway_resolver;
#[cfg(test)]
mod gateway_resolver_tests;
mod gateway_resolver_types;
pub(crate) mod gateway_runtime;
mod gateway_schema_validation;
mod gateway_session;
mod gateway_startup;
mod gateway_tasks;
#[cfg(test)]
mod gateway_tasks_tests;
#[path = "handler_impl.rs"]
mod handler_impl;
#[cfg(test)]
mod handler_impl_task_hooks_tests;
pub mod http;
mod http_oauth;
mod http_oauth_client;
mod http_session;
mod mcp_tool_calls;
pub mod pending_requests;
mod resource_routing;
pub mod resources;
#[cfg(test)]
mod save_workflow_asset_gateway_tests;
pub mod schema;
pub(crate) mod stdio_env_resolver;
#[cfg(test)]
mod stdio_env_resolver_tests;
pub mod stdio_pool;
mod stdio_pool_health;
mod stdio_pool_lifecycle;
mod stdio_pool_prompts;
mod stdio_pool_tools;
mod stdio_process_group;
pub mod stdio_session;
mod stdio_session_ops;
mod stdio_session_support;
mod task_hooks;
pub(crate) mod tool_utils;
use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_runtime::GatewayState;
use crate::registry::ToolRegistry;
pub use client_handler::MaenifoldClient;
pub(crate) use cors::localhost_cors;
pub(crate) use gateway_cli::{gateway_cli_help, invoke_gateway_cli_tool};
pub use pending_requests::PendingRequestRegistry;
pub use resources::*;
use rmcp::ServiceExt;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Implementation, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, ReadResourceRequestParams, ReadResourceResult,
    ServerCapabilities, ServerInfo, SubscribeRequestParams, Tool, UnsubscribeRequestParams,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tool_utils::with_lock_retry;
pub use tool_utils::{McpToolDefinition, list_mcp_tools};
pub struct MaenifoldMcpServer;
struct MaenifoldHandler {
    transport: TransportKind,
    tools: Vec<Tool>,
    gateway: Option<Arc<GatewayState>>,
    gateway_diagnostics: Option<Arc<crate::mcp::stdio_pool::StdioPool>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransportKind {
    Stdio,
    Http,
}
impl MaenifoldHandler {
    fn new(transport: TransportKind) -> Result<Self> {
        let tools = list_mcp_tools(&ToolRegistry::new())?
            .into_iter()
            .map(|t| {
                let schema: serde_json::Map<String, Value> = match t.input_schema {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };
                let mut tool = Tool::new(t.name, t.description, Arc::new(schema));
                tool.execution = t.execution;
                tool
            })
            .collect();
        Ok(Self {
            transport,
            tools,
            gateway: None,
            gateway_diagnostics: None,
        })
    }
    fn set_gateway(&mut self, gateway: Arc<GatewayState>) {
        self.gateway = Some(gateway);
    }
    fn set_gateway_diagnostics(&mut self, diagnostics: Arc<crate::mcp::stdio_pool::StdioPool>) {
        self.gateway_diagnostics = Some(diagnostics);
    }
}
impl MaenifoldMcpServer {
    pub fn list_tools() -> Result<Vec<McpToolDefinition>> {
        list_mcp_tools(ToolRegistry::global())
    }
    pub async fn serve_stdio(shutdown: CancellationToken) -> Result<()> {
        let config = Config::current();
        initialize_runtime(&config)?;
        let pending_registry = Arc::new(PendingRequestRegistry::new());
        let gateway_startup =
            self::gateway_startup::build_gateway_with_registry(&config, pending_registry).await?;
        let (gateway, gateway_diagnostics) = match gateway_startup {
            self::gateway_startup::GatewayStartup::Ready(gateway) => {
                let diagnostics = Some(gateway.pool());
                (Some(gateway), diagnostics)
            }
            self::gateway_startup::GatewayStartup::Failed {
                gateway,
                diagnostics,
            } => {
                let diagnostics = diagnostics.or_else(|| Some(gateway.pool()));
                (Some(gateway), diagnostics)
            }
        };
        let mut handler = MaenifoldHandler::new(TransportKind::Stdio)?;
        if let Some(gateway) = gateway {
            handler.set_gateway(gateway);
        }
        if let Some(diagnostics) = gateway_diagnostics {
            handler.set_gateway_diagnostics(diagnostics);
        }
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
