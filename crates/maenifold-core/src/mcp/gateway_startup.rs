use crate::acp_config::{AgentsConfig, McpServerEntry};
use crate::config::Config;
use crate::error::Result;
use crate::mcp::PendingRequestRegistry;
use crate::mcp::gateway_registry::GatewayRegistry;
use crate::mcp::gateway_runtime::GatewayState;
use crate::mcp::gateway_session::{GatewayServerConfig, HttpServerConfig};
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::resources;
use crate::mcp::stdio_pool::{StdioPool, StdioServerConfig};
use std::sync::Arc;
use tracing::info;

pub(super) enum GatewayStartup {
    Ready(Arc<GatewayState>),
    Failed {
        gateway: Arc<GatewayState>,
        diagnostics: Option<Arc<StdioPool>>,
    },
}

pub(super) async fn build_gateway(config: &Config) -> Result<GatewayStartup> {
    build_gateway_with_registry(config, Arc::new(PendingRequestRegistry::new())).await
}

pub(super) async fn build_gateway_with_registry(
    config: &Config,
    pending_registry: Arc<PendingRequestRegistry>,
) -> Result<GatewayStartup> {
    let agents_config = match AgentsConfig::load(&config.maenifold_root) {
        Ok(config) => config,
        Err(error) => {
            tracing::error!(err = %error, "gateway config load failed, starting degraded gateway");
            return degraded_gateway(
                config,
                pending_registry,
                format!("gateway config load failed: {error}"),
                None,
            )
            .await;
        }
    };
    let gateway_servers: Vec<_> = agents_config
        .mcp_servers
        .iter()
        .filter(|(_name, server)| server.is_gateway())
        .collect();
    if gateway_servers.is_empty() {
        return native_only_gateway(config, pending_registry).await;
    }

    let configs: Vec<GatewayServerConfig> = gateway_servers
        .iter()
        .map(|(name, entry)| match entry {
            McpServerEntry::Stdio {
                command,
                args,
                env,
                env_from_file,
                env_from_command,
                timeout,
                ..
            } => {
                let resolved_env = match super::stdio_env_resolver::resolve_mcp_server_env(
                    env,
                    env_from_file,
                    env_from_command,
                ) {
                    Ok(resolved_env) => resolved_env,
                    Err(error) => {
                        tracing::warn!(
                            server = %name,
                            err = %error,
                            "env resolution failed for mcpServer"
                        );
                        env.clone()
                    }
                };
                GatewayServerConfig::Stdio(StdioServerConfig {
                    name: (*name).clone(),
                    command: command.clone(),
                    args: args.clone(),
                    env: resolved_env,
                    timeout: timeout.map(std::time::Duration::from_secs),
                })
            }
            McpServerEntry::Http {
                url,
                headers,
                auth,
                timeout,
                ..
            } => GatewayServerConfig::Http(HttpServerConfig {
                name: (*name).clone(),
                url: url.clone(),
                headers: headers.clone(),
                auth: auth.clone(),
                timeout: timeout
                    .map(std::time::Duration::from_secs)
                    .unwrap_or(std::time::Duration::from_secs(60)),
            }),
        })
        .collect();

    match StdioPool::start(configs, Arc::clone(&pending_registry)).await {
        Ok(pool) => {
            let pool = Arc::new(pool);
            if pool.all_gateway_tools().is_empty() {
                let message = format!(
                    "all {} configured gateway server(s) started degraded or exposed no tools",
                    gateway_servers.len()
                );
                tracing::error!(err = %message, "gateway startup produced no upstream tools");
                return degraded_gateway(
                    config,
                    Arc::clone(&pending_registry),
                    message,
                    Some(pool),
                )
                .await;
            }
            match GatewayRegistry::build(pool.all_gateway_tools(), &pool.all_gateway_descriptions())
            {
                Ok(registry) => {
                    let resource_routing = match ResourceRoutingRegistry::build(pool.as_ref()) {
                        Ok(routing) => routing,
                        Err(error) => {
                            return degraded_gateway(
                                config,
                                Arc::clone(&pending_registry),
                                error.to_string(),
                                Some(Arc::clone(&pool)),
                            )
                            .await;
                        }
                    };
                    let domain_count = registry.domains().len();
                    let gateway = Arc::new(GatewayState::new(
                        Arc::clone(&pool),
                        Arc::new(resource_routing),
                        registry,
                        config.settings.gateway.intent_resolver.clone(),
                        config.maenifold_root.clone(),
                    ));
                    resources::set_gateway_registry(Arc::new(gateway.registry().clone()));
                    info!(
                        "Gateway registry indexed {domain_count} domains from native + upstream sources"
                    );
                    Ok(GatewayStartup::Ready(gateway))
                }
                Err(error) => {
                    tracing::error!(
                        err = %error,
                        "gateway registry build failed, starting without gateway"
                    );
                    degraded_gateway(
                        config,
                        Arc::clone(&pending_registry),
                        error.to_string(),
                        Some(Arc::clone(&pool)),
                    )
                    .await
                }
            }
        }
        Err(error) => {
            tracing::error!(
                err = %error,
                "gateway pool startup failed, starting without gateway"
            );
            degraded_gateway(config, pending_registry, error.to_string(), None).await
        }
    }
}

async fn native_only_gateway(
    config: &Config,
    registry: Arc<PendingRequestRegistry>,
) -> Result<GatewayStartup> {
    let pool = Arc::new(StdioPool::start(Vec::<StdioServerConfig>::new(), registry).await?);
    let resource_routing = ResourceRoutingRegistry::build(pool.as_ref())?;
    let registry = GatewayRegistry::build(Vec::new(), &pool.all_gateway_descriptions())?;
    let domain_count = registry.domains().len();
    let gateway = Arc::new(GatewayState::new(
        pool,
        Arc::new(resource_routing),
        registry,
        config.settings.gateway.intent_resolver.clone(),
        config.maenifold_root.clone(),
    ));
    resources::set_gateway_registry(Arc::new(gateway.registry().clone()));
    info!(
        "Gateway registry indexed {domain_count} native domains (no upstream servers configured)"
    );
    Ok(GatewayStartup::Ready(gateway))
}

async fn degraded_gateway(
    config: &Config,
    registry: Arc<PendingRequestRegistry>,
    error: String,
    diagnostics: Option<Arc<StdioPool>>,
) -> Result<GatewayStartup> {
    let pool = match diagnostics {
        Some(pool) => pool,
        None => Arc::new(StdioPool::start(Vec::<StdioServerConfig>::new(), registry).await?),
    };
    let gateway = Arc::new(GatewayState::degraded(
        Arc::clone(&pool),
        config.settings.gateway.intent_resolver.clone(),
        config.maenifold_root.clone(),
        error.clone(),
    )?);
    resources::set_gateway_registry(Arc::new(gateway.registry().clone()));
    Ok(GatewayStartup::Failed {
        gateway,
        diagnostics: Some(pool),
    })
}

#[cfg(test)]
mod tests {
    use crate::acp_config::McpServerEntry;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn mcp_servers_schema_gateway_filter_iterates_object_keyed_pairs() {
        let servers: HashMap<String, McpServerEntry> = serde_json::from_value(json!({
            "workiq": {
                "command": "/usr/bin/env",
                "args": ["agency", "mcp", "workiq"],
                "gateway": true
            },
            "learn": {
                "url": "https://learn.microsoft.com/api/mcp",
                "gateway": true
            },
            "local-only": {
                "command": "/usr/bin/env",
                "args": ["local-only"]
            }
        }))
        .expect("object-keyed mcpServers fixture should deserialize");

        let mut gateway_names: Vec<&str> = servers
            .iter()
            .filter(|(_name, entry)| entry.is_gateway())
            .map(|(name, _entry)| name.as_str())
            .collect();
        gateway_names.sort_unstable();

        assert_eq!(gateway_names, vec!["learn", "workiq"]);
    }
}
