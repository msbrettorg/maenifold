use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::{AgentsConfig, McpServerEntry};
use crate::config::Config;
use crate::error::Result;
use crate::mcp::gateway_consolidation::{build_consolidated_tools, gateway_tool_counts};
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use crate::mcp::tool_routing::ToolRoutingRegistry;
use rmcp::model::Tool;
use std::sync::Arc;
use tracing::{info, warn};

pub(super) async fn build_gateway(config: &Config) -> Result<Option<Arc<GatewayState>>> {
    let agents_config = match AgentsConfig::load(&config.maenifold_root) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(err = %error, "gateway config load failed, starting without gateway");
            return Ok(None);
        }
    };
    let gateway_servers: Vec<_> = agents_config
        .mcp_servers
        .iter()
        .filter(|server| server.is_gateway())
        .collect();
    if gateway_servers.is_empty() {
        return Ok(None);
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
        Ok(pool) => match tool_registry_tools() {
            Ok(native_tools) => match build_consolidated_tools(pool.all_gateway_tools(), 4) {
                Ok((consolidated_tools, passthrough_tools)) => {
                    match ToolRoutingRegistry::build(&native_tools, &pool) {
                        Ok(registry) => {
                            let resource_routing = ResourceRoutingRegistry::build(&pool)?;
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
                            Ok(Some(Arc::new(GatewayState::new(
                                Arc::new(pool),
                                Arc::new(registry),
                                Arc::new(resource_routing),
                                consolidated_tools,
                                passthrough_tools,
                            ))))
                        }
                        Err(error) => {
                            tracing::error!(
                                err = %error,
                                "gateway tool routing failed, starting without gateway"
                            );
                            Ok(None)
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(
                        err = %error,
                        "gateway consolidation failed, starting without gateway"
                    );
                    Ok(None)
                }
            },
            Err(error) => {
                tracing::error!(
                    err = %error,
                    "native tool registry load failed, starting without gateway"
                );
                Ok(None)
            }
        },
        Err(error) => {
            tracing::error!(
                err = %error,
                "gateway pool startup failed, starting without gateway"
            );
            Ok(None)
        }
    }
}

fn tool_registry_tools() -> Result<Vec<Tool>> {
    Ok(MaenifoldHandler::new(TransportKind::Http)?.tools)
}
