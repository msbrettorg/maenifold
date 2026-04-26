use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::{AgentsConfig, McpServerEntry};
use crate::config::Config;
use crate::error::Result;
use crate::mcp::gateway_catalog::GatewayCatalog;
use crate::mcp::gateway_consolidation::{build_consolidated_tools, gateway_tool_counts};
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::resources;
use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use crate::mcp::tool_routing::ToolRoutingRegistry;
use rmcp::model::Tool;
use std::sync::Arc;
use tracing::{info, warn};

pub(super) enum GatewayStartup {
    Disabled,
    Ready(Arc<GatewayState>),
    Failed {
        error: String,
        diagnostics: Option<Arc<StdioPool>>,
    },
}

pub(super) async fn build_gateway(config: &Config) -> Result<GatewayStartup> {
    let agents_config = match AgentsConfig::load(&config.maenifold_root) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(err = %error, "gateway config load failed, starting without gateway");
            return Ok(GatewayStartup::Disabled);
        }
    };
    let gateway_servers: Vec<_> = agents_config
        .mcp_servers
        .iter()
        .filter(|server| server.is_gateway())
        .collect();
    if gateway_servers.is_empty() {
        return native_only_gateway().await;
    }

    let configs: Vec<StdioServerConfig> = gateway_servers
        .iter()
        .map(|entry| match entry {
            McpServerEntry::Stdio {
                name,
                command,
                args,
                env,
                timeout,
                ..
            } => StdioServerConfig {
                name: name.clone(),
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
                timeout: timeout.map(std::time::Duration::from_secs),
            },
            _ => unreachable!("is_gateway filters to Stdio only"),
        })
        .collect();

    match StdioPool::start(configs).await {
        Ok(pool) => {
            let pool = Arc::new(pool);
            if pool.all_gateway_tools().is_empty() {
                let message = format!(
                    "all {} configured gateway server(s) started degraded or exposed no tools",
                    gateway_servers.len()
                );
                tracing::error!(err = %message, "gateway startup produced no upstream tools");
                return Ok(GatewayStartup::Failed {
                    error: message,
                    diagnostics: Some(pool),
                });
            }
            match tool_registry_tools() {
                Ok(native_tools) => match build_consolidated_tools(pool.all_gateway_tools(), 4) {
                    Ok((consolidated_tools, passthrough_tools)) => {
                        match ToolRoutingRegistry::build(&native_tools, pool.as_ref()) {
                            Ok(registry) => {
                                let resource_routing =
                                    ResourceRoutingRegistry::build(pool.as_ref())?;
                                let catalog = GatewayCatalog::build(
                                    &consolidated_tools,
                                    &passthrough_tools,
                                    &registry,
                                )?;
                                resources::set_gateway_catalog(Arc::new(catalog.clone()));
                                let counts = gateway_tool_counts(
                                    &consolidated_tools,
                                    &passthrough_tools,
                                    &native_tools,
                                );
                                let total = counts.total;
                                let facades = counts.facades;
                                let passthrough = counts.passthrough;
                                info!(
                                    "Gateway tools: {total} total ({facades} facades, {passthrough} passthrough)"
                                );
                                if total > 50 {
                                    warn!(
                                        "Gateway exposes {total} tools — consider consolidation threshold"
                                    );
                                }
                                Ok(GatewayStartup::Ready(Arc::new(GatewayState::new(
                                    Arc::clone(&pool),
                                    Arc::new(resource_routing),
                                    catalog,
                                    consolidated_tools,
                                ))))
                            }
                            Err(error) => {
                                tracing::error!(
                                    err = %error,
                                    "gateway tool routing failed, starting without gateway"
                                );
                                Ok(GatewayStartup::Failed {
                                    error: error.to_string(),
                                    diagnostics: Some(Arc::clone(&pool)),
                                })
                            }
                        }
                    }
                    Err(error) => {
                        tracing::error!(
                            err = %error,
                            "gateway consolidation failed, starting without gateway"
                        );
                        Ok(GatewayStartup::Failed {
                            error: error.to_string(),
                            diagnostics: Some(Arc::clone(&pool)),
                        })
                    }
                },
                Err(error) => {
                    tracing::error!(
                        err = %error,
                        "native tool registry load failed, starting without gateway"
                    );
                    Ok(GatewayStartup::Failed {
                        error: error.to_string(),
                        diagnostics: Some(Arc::clone(&pool)),
                    })
                }
            }
        }
        Err(error) => {
            tracing::error!(
                err = %error,
                "gateway pool startup failed, starting without gateway"
            );
            Ok(GatewayStartup::Failed {
                error: error.to_string(),
                diagnostics: None,
            })
        }
    }
}

async fn native_only_gateway() -> Result<GatewayStartup> {
    let pool = Arc::new(StdioPool::start(Vec::new()).await?);
    let native_tools = tool_registry_tools()?;
    let routing = ToolRoutingRegistry::build(&native_tools, pool.as_ref())?;
    let resource_routing = ResourceRoutingRegistry::build(pool.as_ref())?;
    let catalog = GatewayCatalog::build(&[], &[], &routing)?;
    resources::set_gateway_catalog(Arc::new(catalog.clone()));
    info!(
        "Gateway tools: {} native commands indexed (no upstream servers configured)",
        native_tools.len()
    );
    Ok(GatewayStartup::Ready(Arc::new(GatewayState::new(
        pool,
        Arc::new(resource_routing),
        catalog,
        Vec::new(),
    ))))
}

fn tool_registry_tools() -> Result<Vec<Tool>> {
    Ok(MaenifoldHandler::new(TransportKind::Http)?.tools)
}
