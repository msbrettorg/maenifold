use super::{MaenifoldHandler, TransportKind};
use crate::acp_config::{AgentsConfig, McpServerEntry};
use crate::config::Config;
use crate::error::Result;
use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use crate::mcp::tool_routing::ToolRoutingRegistry;
use rmcp::model::Tool;
use std::sync::Arc;
use tracing::info;

pub(super) async fn build_gateway(
    config: &Config,
) -> Option<(Arc<StdioPool>, Arc<ToolRoutingRegistry>)> {
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
            Ok(native_tools) => match ToolRoutingRegistry::build(&native_tools, &pool) {
                Ok(registry) => {
                    info!(count = pool.all_tools().len(), "gateway started with tools");
                    Some((Arc::new(pool), Arc::new(registry)))
                }
                Err(error) => {
                    tracing::error!(
                        err = %error,
                        "gateway tool routing failed, starting without gateway"
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
