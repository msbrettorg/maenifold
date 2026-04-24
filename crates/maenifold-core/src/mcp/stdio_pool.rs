use crate::error::{MaenifoldError, Result};
use crate::mcp::stdio_session::StdioSession;
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use futures_util::stream::{FuturesUnordered, StreamExt};
use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock as StdRwLock, RwLockReadGuard};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::warn;

pub struct StdioPool {
    servers: RwLock<HashMap<String, ManagedServer>>,
    tools: StdRwLock<Vec<Tool>>,
}

#[derive(Debug, Clone)]
pub struct StdioServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout: Option<Duration>,
}

struct ManagedServer {
    config: StdioServerConfig,
    session: Arc<StdioSession>,
}

impl StdioPool {
    pub async fn start(servers: Vec<StdioServerConfig>) -> Result<Self> {
        let mut seen = HashSet::new();
        for config in &servers {
            if !seen.insert(&config.name) {
                return Err(MaenifoldError::Message(format!(
                    "Duplicate gateway server name: '{}'. Each server must have a unique name.",
                    config.name
                )));
            }
        }

        let mut tasks = JoinSet::new();
        for config in servers {
            tasks.spawn(async move {
                let name = config.name.clone();
                (name, config.clone(), start_session(&config).await)
            });
        }

        let mut started = HashMap::new();
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok((_, config, Ok(session))) => {
                    started.insert(
                        config.name.clone(),
                        ManagedServer {
                            config,
                            session: Arc::new(session),
                        },
                    );
                }
                Ok((name, _, Err(error))) => {
                    warn!(server = %name, error = %error, "failed to start stdio MCP server");
                }
                Err(error) => warn!(error = %error, "stdio MCP startup task failed"),
            }
        }

        let tools = match collect_tools(&started) {
            Ok(tools) => tools,
            Err(error) => {
                shutdown_servers(started).await;
                return Err(error);
            }
        };

        Ok(Self {
            servers: RwLock::new(started),
            tools: StdRwLock::new(tools),
        })
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<CallToolResult> {
        let session = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            server.session.clone()
        };
        let initial = {
            if session.is_usable() {
                Some(session.call_tool(tool_name, arguments.clone()).await)
            } else {
                None
            }
        };

        match initial {
            Some(Ok(result)) => Ok(result),
            Some(Err(error)) if is_prewrite_failure(&error) => {
                self.restart_server(server_name).await?;
                self.call_after_restart(server_name, tool_name, arguments).await
            }
            Some(Err(error)) => Err(error),
            None => {
                self.restart_server(server_name).await?;
                self.call_after_restart(server_name, tool_name, arguments).await
            }
        }
    }

    pub fn all_tools(&self) -> Vec<Tool> {
        lock_tools(&self.tools).clone()
    }

    pub async fn shutdown(&self) {
        let sessions = {
            let mut servers = self.servers.write().await;
            std::mem::take(&mut *servers)
                .into_values()
                .map(|server| server.session)
                .collect::<Vec<_>>()
        };
        *self.tools.write().unwrap_or_else(|error| error.into_inner()) = Vec::new();
        let mut shutdowns = FuturesUnordered::new();
        for session in sessions {
            shutdowns.push(async move { session.shutdown().await });
        }
        while shutdowns.next().await.is_some() {}
    }

    async fn call_after_restart(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<CallToolResult> {
        let session = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            server.session.clone()
        };
        session.call_tool(tool_name, arguments).await
    }

    async fn restart_server(&self, server_name: &str) -> Result<()> {
        let config = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            if server.session.is_usable() {
                return Ok(());
            }
            server.config.clone()
        };

        let mut replacement = Some(Arc::new(start_session(&config).await?));
        let outcome = {
            let mut servers = self.servers.write().await;
            match servers.get(server_name) {
                None => Err(missing_server(server_name)),
                Some(server) if server.session.is_usable() => Ok((None, None)),
                Some(_) => {
                    let Some(new_session) = replacement.as_ref() else {
                        return Err(MaenifoldError::Message(
                            "replacement stdio session missing".to_string(),
                        ));
                    };
                    ensure_replacement_tools(&servers, server_name, new_session.tools().as_ref())?;
                    let old = {
                        let server = servers
                            .get_mut(server_name)
                            .ok_or_else(|| missing_server(server_name))?;
                        std::mem::replace(
                            &mut server.session,
                            replacement.take().ok_or_else(|| {
                                MaenifoldError::Message(
                                    "replacement stdio session missing".to_string(),
                                )
                            })?,
                        )
                    };
                    Ok((Some(old), Some(collect_tools(&servers)?)))
                }
            }
        };

        let (old_session, tools) = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(session) = replacement {
                    session.shutdown().await;
                }
                return Err(error);
            }
        };

        if let Some(tools) = tools {
            *self.tools.write().unwrap_or_else(|error| error.into_inner()) = tools;
        }
        if let Some(session) = replacement {
            session.shutdown().await;
        }
        if let Some(session) = old_session {
            session.shutdown().await;
        }
        Ok(())
    }
}

async fn start_session(config: &StdioServerConfig) -> Result<StdioSession> {
    let entry = McpServerStdioConfig::new(config.name.clone(), config.command.clone())
        .args(config.args.clone())
        .env(
            config
                .env
                .iter()
                .map(|(name, value)| EnvVariable::new(name.clone(), value.clone()))
                .collect(),
        );
    StdioSession::start(&entry).await
}

fn collect_tools(servers: &HashMap<String, ManagedServer>) -> Result<Vec<Tool>> {
    let mut owners = HashMap::<String, String>::new();
    let mut tools = Vec::new();
    for (server_name, server) in servers {
        for tool in server.session.tools().iter() {
            let tool_name = tool.name.to_string();
            if let Some(other) = owners.insert(tool_name.clone(), server_name.clone()) {
                return Err(MaenifoldError::Message(format!(
                    "MCP tool name collision: '{tool_name}' provided by both '{other}' and '{server_name}'"
                )));
            }
            tools.push(tool.clone());
        }
    }
    Ok(tools)
}

fn ensure_replacement_tools(
    servers: &HashMap<String, ManagedServer>,
    replaced_server: &str,
    replacement_tools: &[Tool],
) -> Result<()> {
    let mut owners = HashMap::<String, String>::new();
    for (server_name, server) in servers {
        if server_name == replaced_server {
            continue;
        }
        for tool in server.session.tools().iter() {
            owners.insert(tool.name.to_string(), server_name.clone());
        }
    }
    for tool in replacement_tools {
        let tool_name = tool.name.to_string();
        if let Some(other) = owners.insert(tool_name.clone(), replaced_server.to_string()) {
            return Err(MaenifoldError::Message(format!(
                "MCP tool name collision: '{tool_name}' provided by both '{other}' and '{replaced_server}'"
            )));
        }
    }
    Ok(())
}

async fn shutdown_servers(servers: HashMap<String, ManagedServer>) {
    let mut shutdowns = FuturesUnordered::new();
    for server in servers.into_values() {
        shutdowns.push(async move { server.session.shutdown().await });
    }
    while shutdowns.next().await.is_some() {}
}

fn is_prewrite_failure(error: &MaenifoldError) -> bool {
    matches!(error, MaenifoldError::Message(message) if message == "MCP stdio session is not usable")
}

fn missing_server(server_name: &str) -> MaenifoldError {
    MaenifoldError::Message(format!("unknown MCP stdio server '{server_name}'"))
}

fn lock_tools(tools: &StdRwLock<Vec<Tool>>) -> RwLockReadGuard<'_, Vec<Tool>> {
    tools.read().unwrap_or_else(|error| error.into_inner())
}
