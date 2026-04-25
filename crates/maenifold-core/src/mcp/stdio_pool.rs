use crate::error::{JSONRPC_INVALID_PARAMS, MaenifoldError, Result};
use crate::mcp::stdio_pool_health::{lock_state, RestartState};
use crate::mcp::stdio_pool_tools::{
    collect_gateway_tools, ensure_replacement_tools, shutdown_servers,
};
use crate::mcp::stdio_session::StdioSession;
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use futures_util::stream::{FuturesUnordered, StreamExt};
use rmcp::model::{CallToolResult, ReadResourceResult, Tool};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock, RwLockReadGuard};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinSet;
use tracing::warn;
pub use crate::mcp::stdio_pool_health::{StdioPoolHealth, StdioServerHealth};
pub struct StdioPool {
    pub(super) servers: RwLock<HashMap<String, ManagedServer>>,
    tools: StdRwLock<Vec<Tool>>,
    gateway_tools: StdRwLock<Vec<(String, Vec<Tool>)>>,
}
#[derive(Debug, Clone)]
pub struct StdioServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout: Option<Duration>,
}
pub(super) struct ManagedServer {
    pub(super) config: StdioServerConfig,
    pub(super) session: Option<Arc<StdioSession>>,
    pub(super) restart_lock: Arc<Mutex<()>>,
    pub(super) restart_state: Arc<StdMutex<RestartState>>,
}
impl ManagedServer {
    fn running(config: StdioServerConfig, session: StdioSession) -> Self {
        Self {
            config,
            session: Some(Arc::new(session)),
            restart_lock: Arc::new(Mutex::new(())),
            restart_state: Arc::new(StdMutex::new(RestartState::new())),
        }
    }
    fn degraded(config: StdioServerConfig, error: String) -> Self {
        let server = Self::configured(config);
        lock_state(&server.restart_state).record_failure(error);
        server
    }
    fn configured(config: StdioServerConfig) -> Self {
        Self {
            config,
            session: None,
            restart_lock: Arc::new(Mutex::new(())),
            restart_state: Arc::new(StdMutex::new(RestartState::new())),
        }
    }
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
                    started.insert(config.name.clone(), ManagedServer::running(config, session));
                }
                Ok((name, config, Err(error))) => {
                    warn!(server = %name, error = %error, "failed to start stdio MCP server");
                    started.insert(name, ManagedServer::degraded(config, error.to_string()));
                }
                Err(error) => warn!(error = %error, "stdio MCP startup task failed"),
            }
        }
        let (tools, gateway_tools) = match collect_gateway_tools(&started) {
            Ok(tool_state) => tool_state,
            Err(error) => {
                shutdown_servers(started).await;
                return Err(error);
            }
        };
        Ok(Self {
            servers: RwLock::new(started),
            tools: StdRwLock::new(tools),
            gateway_tools: StdRwLock::new(gateway_tools),
        })
    }
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<CallToolResult> {
        let t = tool_name.to_string();
        self.with_restart(server_name, move |s| {
            let t = t.clone();
            let a = arguments.clone();
            Box::pin(async move { s.call_tool(&t, a).await })
        }).await
    }
    pub async fn read_resource(&self, server_name: &str, uri: &str) -> Result<ReadResourceResult> {
        let u = uri.to_string();
        self.with_restart(server_name, move |s| {
            let u = u.clone();
            Box::pin(async move { s.read_resource(&u).await })
        }).await
    }
    pub fn all_tools(&self) -> Vec<Tool> { lock_tools(&self.tools).clone() }
    pub fn all_gateway_tools(&self) -> Vec<(String, Vec<Tool>)> {
        self.gateway_tools
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub async fn shutdown(&self) {
        let sessions = {
            let mut servers = self.servers.write().await;
            std::mem::take(&mut *servers)
                .into_values()
                .filter_map(|server| server.session)
                .collect::<Vec<_>>()
        };
        *self.tools.write().unwrap_or_else(|error| error.into_inner()) = Vec::new();
        *self.gateway_tools
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Vec::new();
        let mut shutdowns = FuturesUnordered::new();
        for session in sessions {
            shutdowns.push(async move { session.shutdown().await });
        }
        while shutdowns.next().await.is_some() {}
    }
    async fn with_restart<T: Send>(
        &self,
        server_name: &str,
        f: impl Fn(Arc<StdioSession>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    ) -> Result<T> {
        let session = {
            let servers = self.servers.read().await;
            servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?
                .session
                .clone()
        };
        let initial = if let Some(session) = session.filter(|session| session.is_usable()) {
            Some(f(session).await)
        } else {
            None
        };
        if let Some(result) = initial {
            match result {
                Ok(value) => return Ok(value),
                Err(error) if !is_prewrite_failure(&error) => return Err(error),
                Err(_) => {}
            }
        }
        self.restart_server(server_name).await?;
        let session = {
            let servers = self.servers.read().await;
            servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?
                .session
                .clone()
                .ok_or_else(|| MaenifoldError::SessionUnusable)?
        };
        f(session).await
    }
    async fn restart_server(&self, server_name: &str) -> Result<()> {
        let (config, restart_lock, restart_state) = {
            let servers = self.servers.read().await;
            let server = servers
                .get(server_name)
                .ok_or_else(|| missing_server(server_name))?;
            (
                server.config.clone(),
                server.restart_lock.clone(),
                server.restart_state.clone(),
            )
        };
        let _guard = restart_lock.lock().await;
        {
            let servers = self.servers.read().await;
            let server = servers.get(server_name).ok_or_else(|| missing_server(server_name))?;
            if server.session.as_ref().is_some_and(|session| session.is_usable()) {
                return Ok(());
            }
        }
        if let Some(message) = lock_state(&restart_state).backoff_error(server_name) {
            return Err(MaenifoldError::Message(message));
        }
        let mut replacement = match start_session(&config).await {
            Ok(session) => Some(Arc::new(session)),
            Err(error) => {
                lock_state(&restart_state).record_failure(error.to_string());
                return Err(error);
            }
        };
        lock_state(&restart_state).record_success();
        let outcome = {
            let mut servers = self.servers.write().await;
            match servers.get(server_name) {
                None => Err(missing_server(server_name)),
                Some(server)
                    if server.session.as_ref().is_some_and(|session| session.is_usable()) =>
                {
                    Ok((None, None))
                }
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
                        std::mem::replace(&mut server.session, Some(replacement.take().ok_or_else(|| {
                                 MaenifoldError::Message(
                                     "replacement stdio session missing".to_string(),
                                 )
                             })?))
                    };
                    Ok((Some(old), Some(collect_gateway_tools(&servers)?)))
                }
            }
        };
        let (old_session, tool_state) = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(session) = replacement {
                    session.shutdown().await;
                }
                lock_state(&restart_state).record_failure(error.to_string());
                return Err(error);
            }
        };
        if let Some((tools, gateway_tools)) = tool_state {
            *self.tools.write().unwrap_or_else(|error| error.into_inner()) = tools;
            *self
                .gateway_tools
                .write()
                .unwrap_or_else(|error| error.into_inner()) = gateway_tools;
        }
        if let Some(session) = replacement {
            session.shutdown().await;
        }
        if let Some(Some(session)) = old_session {
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
    StdioSession::start(&entry, config.timeout.unwrap_or(Duration::from_secs(60))).await
}
fn is_prewrite_failure(error: &MaenifoldError) -> bool {
    matches!(error, MaenifoldError::SessionUnusable)
}
fn missing_server(server_name: &str) -> MaenifoldError {
    MaenifoldError::JsonRpc {
        code: JSONRPC_INVALID_PARAMS,
        message: format!("Gateway: unknown server '{server_name}'"),
        data: Value::Null,
    }
}
fn lock_tools(tools: &StdRwLock<Vec<Tool>>) -> RwLockReadGuard<'_, Vec<Tool>> { tools.read().unwrap_or_else(|error| error.into_inner()) }
#[cfg(test)] #[path = "stdio_pool_tests.rs"] mod tests;
