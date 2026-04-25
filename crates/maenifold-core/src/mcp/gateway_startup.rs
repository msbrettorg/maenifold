use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::{AgentsConfig, McpServerEntry};
use crate::config::Config;
use crate::error::Result;
use crate::mcp::gateway_consolidation::build_consolidated_tools;
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use crate::mcp::tool_routing::ToolRoutingRegistry;
use rmcp::model::Tool;
use std::sync::Arc;
use tracing::info;

pub(super) async fn build_gateway(
    config: &Config,
) -> Option<Arc<GatewayState>> {
    let agents_config = AgentsConfig::load(&config.maenifold_root);
    let gateway_servers: Vec<_> = agents_config
        .mcp_servers
        .iter()
        .filter(|server| server.is_gateway())
        .collect();
    if gateway_servers.is_empty() {
        return None;
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
                            info!(
                                upstream = pool.all_tools().len(),
                                facades = consolidated_tools.len(),
                                passthrough = passthrough_tools.len(),
                                "gateway started with consolidated tools"
                            );
                            Some(Arc::new(GatewayState::new(
                                Arc::new(pool),
                                Arc::new(registry),
                                consolidated_tools,
                                passthrough_tools,
                            )))
                        }
                        Err(error) => {
                            tracing::error!(
                                err = %error,
                                "gateway tool routing failed, starting without gateway"
                            );
                            None
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(
                        err = %error,
                        "gateway consolidation failed, starting without gateway"
                    );
                    None
                }
            },
            Err(error) => {
                tracing::error!(
                    err = %error,
                    "native tool registry load failed, starting without gateway"
                );
                None
            }
        },
        Err(error) => {
            tracing::error!(
                err = %error,
                "gateway pool startup failed, starting without gateway"
            );
            None
        }
    }
}

fn tool_registry_tools() -> Result<Vec<Tool>> {
    Ok(MaenifoldHandler::new(TransportKind::Http)?.tools)
}
