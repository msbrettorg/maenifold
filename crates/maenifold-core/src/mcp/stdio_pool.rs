use crate::error::{JSONRPC_INVALID_PARAMS, MaenifoldError, Result};
use crate::mcp::stdio_pool_health::{RestartState, lock_state};
pub use crate::mcp::stdio_pool_health::{ServerStartupStatus, StdioPoolHealth, StdioServerHealth};
use crate::mcp::stdio_pool_tools::{collect_gateway_tools, shutdown_servers};
use crate::mcp::stdio_session::{StdioSession, StdioStartupError};
use crate::mcp::PendingRequestRegistry;
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use futures_util::stream::{FuturesUnordered, StreamExt};
use rmcp::model::{CallToolResult, ReadResourceResult, Tool};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex as StdMutex, RwLock as StdRwLock, RwLockReadGuard};
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinSet;
use tracing::warn;
pub struct StdioPool {
    pub(super) servers: RwLock<HashMap<String, ManagedServer>>,
    pub(super) tools: StdRwLock<Vec<Tool>>,
    pub(super) gateway_tools: StdRwLock<Vec<(String, Vec<Tool>)>>,
    pub(super) gateway_descriptions: StdRwLock<BTreeMap<String, String>>,
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
    fn degraded(config: StdioServerConfig, error: String, stderr: Option<String>) -> Self {
        let server = Self::configured(config);
        lock_state(&server.restart_state).record_failure(error, stderr);
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
        Self::start_with_registry(servers, Arc::new(PendingRequestRegistry::new())).await
    }

    pub async fn start_with_registry(
        servers: Vec<StdioServerConfig>,
        registry: Arc<PendingRequestRegistry>,
    ) -> Result<Self> {
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
            let registry = Arc::clone(&registry);
            tasks.spawn(async move {
                let name = config.name.clone();
                (
                    name,
                    config.clone(),
                    start_session_with_registry(&config, registry).await,
                )
            });
        }
        let mut started = HashMap::new();
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok((_, config, Ok(session))) => {
                    started.insert(config.name.clone(), ManagedServer::running(config, session));
                }
                Ok((name, config, Err(error))) => {
                    warn!(server = %name, error = %error.error, "failed to start stdio MCP server");
                    started.insert(
                        name,
                        ManagedServer::degraded(config, error.error.to_string(), error.stderr),
                    );
                }
                Err(error) => warn!(error = %error, "stdio MCP startup task failed"),
            }
        }
        let (tools, gateway_tools, gateway_descriptions) = match collect_gateway_tools(&started) {
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
            gateway_descriptions: StdRwLock::new(gateway_descriptions),
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
        })
        .await
    }
    pub async fn read_resource(&self, server_name: &str, uri: &str) -> Result<ReadResourceResult> {
        let u = uri.to_string();
        self.with_restart(server_name, move |s| {
            let u = u.clone();
            Box::pin(async move { s.read_resource(&u).await })
        })
        .await
    }
    pub fn all_tools(&self) -> Vec<Tool> {
        lock_tools(&self.tools).clone()
    }
    pub fn all_gateway_tools(&self) -> Vec<(String, Vec<Tool>)> {
        self.gateway_tools
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub fn all_gateway_descriptions(&self) -> BTreeMap<String, String> {
        self.gateway_descriptions
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
        *self
            .tools
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Vec::new();
        *self
            .gateway_tools
            .write()
            .unwrap_or_else(|error| error.into_inner()) = Vec::new();
        *self
            .gateway_descriptions
            .write()
            .unwrap_or_else(|error| error.into_inner()) = BTreeMap::new();
        let mut shutdowns = FuturesUnordered::new();
        for session in sessions {
            shutdowns.push(async move { session.shutdown().await });
        }
        while shutdowns.next().await.is_some() {}
    }
    async fn with_restart<T: Send>(
        &self,
        server_name: &str,
        f: impl Fn(
            Arc<StdioSession>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
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
}
pub(super) async fn start_session(
    config: &StdioServerConfig,
) -> std::result::Result<StdioSession, StdioStartupError> {
    start_session_with_registry(config, Arc::new(PendingRequestRegistry::new())).await
}

pub(super) async fn start_session_with_registry(
    config: &StdioServerConfig,
    registry: Arc<PendingRequestRegistry>,
) -> std::result::Result<StdioSession, StdioStartupError> {
    let entry = McpServerStdioConfig::new(config.name.clone(), config.command.clone())
        .args(config.args.clone())
        .env(
            config
                .env
                .iter()
                .map(|(name, value)| EnvVariable::new(name.clone(), value.clone()))
                .collect(),
        );
    StdioSession::start_with_registry(
        &entry,
        config.timeout.unwrap_or(Duration::from_secs(60)),
        registry,
    )
    .await
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
fn lock_tools(tools: &StdRwLock<Vec<Tool>>) -> RwLockReadGuard<'_, Vec<Tool>> {
    tools.read().unwrap_or_else(|error| error.into_inner())
}
#[cfg(test)]
#[path = "stdio_pool_tests.rs"]
mod tests;
