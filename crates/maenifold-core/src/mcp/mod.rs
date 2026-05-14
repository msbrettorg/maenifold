mod client_handler;
#[cfg(test)]
mod client_handler_tests;
mod cors;
pub(crate) mod discovery_tools;
pub(crate) mod domain_mapping;
mod elicitation_validation;
pub(crate) mod gateway_batch;
pub(crate) mod gateway_batch_types;
pub(crate) mod gateway_catalog;
#[cfg(test)]
mod gateway_catalog_domain_tests;
mod gateway_catalog_render;
mod gateway_cli;
pub mod gateway_consolidation;
pub mod gateway_dispatch;
mod gateway_errors;
#[cfg(test)]
mod gateway_experiment;
#[cfg(test)]
mod gateway_experiment_fixtures;
#[cfg(test)]
mod gateway_experiment_tests;
mod gateway_gate;
pub(crate) mod gateway_handler;
#[cfg(test)]
mod gateway_integration_tests;
mod gateway_intent;
#[cfg(test)]
mod gateway_intent_tests;
mod gateway_metadata;
#[cfg(test)]
mod gateway_metadata_tests;
mod gateway_resolver;
#[cfg(test)]
mod gateway_resolver_tests;
mod gateway_resolver_types;
mod gateway_schema_validation;
mod gateway_startup;
mod gateway_tasks;
#[cfg(test)]
mod gateway_tasks_tests;
#[path = "handler_impl.rs"]
mod handler_impl;
#[cfg(test)]
mod handler_impl_task_hooks_tests;
pub mod http;
pub(crate) mod loop_correlator;
mod mcp_tool_calls;
pub mod pending_requests;
#[cfg(test)]
mod pending_requests_tests;
mod resource_routing;
pub mod resources;
pub(crate) mod sampling_client_flow;
pub(crate) mod sampling_client_helpers;
#[cfg(test)]
mod sampling_dispatch_tests;
pub(crate) mod sampling_events;
pub(crate) mod sampling_session_registry;
#[cfg(test)]
mod sampling_session_registry_tests;
#[cfg(test)]
mod save_workflow_asset_gateway_tests;
pub mod schema;
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
pub mod tool_routing;
pub(crate) mod tool_utils;
use crate::assets::AssetManager;
use crate::config::Config;
use crate::db::GraphDatabase;
use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_handler::GatewayState;
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
pub(crate) use sampling_session_registry::SamplingSessionRegistry;
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
    gateway_startup_error: Option<String>,
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
