use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_metadata;
use crate::mcp::MaenifoldClient;
use crate::mcp::PendingRequestRegistry;
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use rmcp::{
    Peer, RoleClient, ServiceExt,
    model::{ProtocolVersion, Tool},
    service::{RunningService, ServiceError},
    transport::TokioChildProcess,
};
use serde_json::Value;
use std::io;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, Command};
use tokio::sync::Mutex;

const MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2025_03_26;
const LEGACY_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2024_11_05;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const STDERR_LIMIT: usize = 4096;
const STDERR_JOIN_TIMEOUT: Duration = Duration::from_secs(1);
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

type RmcpClient = RunningService<RoleClient, MaenifoldClient>;

#[derive(Debug)]
pub struct StdioStartupError {
    pub error: MaenifoldError,
    pub stderr: Option<String>,
}

impl StdioStartupError {
    fn new(error: MaenifoldError, stderr: Option<String>) -> Self {
        Self { error, stderr }
    }
}

impl std::fmt::Display for StdioStartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for StdioStartupError {}

pub struct StdioSession {
    client: StdMutex<Option<RmcpClient>>,
    pub(super) peer: Peer<RoleClient>,
    stderr_task: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    pub(super) stderr: Arc<Mutex<String>>,
    tools: Arc<[Tool]>,
    initialize_description: Option<String>,
    pub(super) call_timeout: Duration,
    pub(super) server_name: String,
    process_group_id: Option<i32>,
    pub(super) dead: AtomicBool,
    pub(super) poisoned: AtomicBool,
}

impl StdioSession {
    pub async fn start(
        entry: &McpServerStdioConfig,
        call_timeout: Duration,
    ) -> std::result::Result<Self, StdioStartupError> {
        Self::start_with_registry(entry, call_timeout, Arc::new(PendingRequestRegistry::new())).await
    }

    pub async fn start_with_registry(
        entry: &McpServerStdioConfig,
        call_timeout: Duration,
        registry: Arc<PendingRequestRegistry>,
    ) -> std::result::Result<Self, StdioStartupError> {
        Self::start_with_startup_timeout_and_registry(entry, STARTUP_TIMEOUT, call_timeout, registry).await
    }

    pub async fn start_with_startup_timeout(
        entry: &McpServerStdioConfig,
        startup_timeout: Duration,
        call_timeout: Duration,
    ) -> std::result::Result<Self, StdioStartupError> {
        Self::start_with_startup_timeout_and_registry(entry, startup_timeout, call_timeout, Arc::new(PendingRequestRegistry::new())).await
    }

    pub async fn start_with_startup_timeout_and_registry(
        entry: &McpServerStdioConfig,
        startup_timeout: Duration,
        call_timeout: Duration,
        registry: Arc<PendingRequestRegistry>,
    ) -> std::result::Result<Self, StdioStartupError> {
        let (transport, stderr_task, stderr, process_group_id) =
            spawn(entry).map_err(|error| StdioStartupError::new(error, None))?;
        let startup = async move {
            let sampling_agent = None;
            let mut client = MaenifoldClient::new_with_sampling(registry, sampling_agent)
                .serve(transport)
                .await
                .map_err(to_message_error)?;
            let server_info = client.peer().peer_info().cloned().ok_or_else(|| {
                MaenifoldError::Message("MCP server did not return initialize result".to_string())
            })?;
            let protocol = server_info.protocol_version.clone();
            if !is_supported_version(&protocol) {
                let _ = client.close_with_timeout(SHUTDOWN_WAIT).await;
                super::stdio_process_group::terminate_process_group_id(process_group_id).await;
                return Err(MaenifoldError::Message(format!(
                    "unsupported MCP protocol version '{}'",
                    protocol
                )));
            }
            let tools = client
                .peer()
                .list_all_tools()
                .await
                .map_err(map_service_error)?;
            Ok((client, tools, server_info))
        };
        let (client, tools, server_info) =
            match tokio::time::timeout(startup_timeout, startup).await {
                Ok(Ok(started)) => started,
                Ok(Err(error)) => {
                    super::stdio_process_group::terminate_process_group_id(process_group_id).await;
                    let stderr = collect_failure_stderr(stderr_task, Arc::clone(&stderr)).await;
                    return Err(StdioStartupError::new(error, stderr));
                }
                Err(_) => {
                    super::stdio_process_group::terminate_process_group_id(process_group_id).await;
                    let stderr = collect_failure_stderr(stderr_task, Arc::clone(&stderr)).await;
                    return Err(StdioStartupError::new(
                        MaenifoldError::Message(format!(
                            "MCP server startup timed out after {}s",
                            startup_timeout.as_secs()
                        )),
                        stderr,
                    ));
                }
            };
        let peer = client.peer().clone();
        Ok(Self {
            client: StdMutex::new(Some(client)),
            peer,
            stderr_task: StdMutex::new(Some(stderr_task)),
            stderr,
            tools: tools.into(),
            initialize_description: gateway_metadata::upstream_initialize_description(&server_info),
            call_timeout,
            server_name: entry.name.to_string(),
            process_group_id,
            dead: AtomicBool::new(false),
            poisoned: AtomicBool::new(false),
        })
    }

    pub fn tools(&self) -> Arc<[Tool]> {
        Arc::clone(&self.tools)
    }

    pub fn initialize_description(&self) -> Option<String> {
        self.initialize_description.clone()
    }

    pub fn is_usable(&self) -> bool {
        !self.is_poisoned() && self.refresh_liveness()
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned.load(Ordering::SeqCst)
    }

    pub async fn shutdown(&self) {
        let client = { lock_std(&self.client).take() };
        if let Some(mut client) = client {
            let _ = client.close_with_timeout(SHUTDOWN_WAIT).await;
        }
        super::stdio_process_group::terminate_process_group_id(self.process_group_id).await;
        let task = { lock_std(&self.stderr_task).take() };
        if let Some(task) = task {
            let _ = tokio::time::timeout(STDERR_JOIN_TIMEOUT, task).await;
        }
    }

    pub(super) fn refresh_liveness(&self) -> bool {
        if self.dead.load(Ordering::SeqCst) {
            return false;
        }
        let usable = lock_std(&self.client)
            .as_ref()
            .is_some_and(|client| !client.is_closed() && !client.peer().is_transport_closed());
        if !usable {
            self.dead.store(true, Ordering::SeqCst);
        }
        usable
    }

    pub(super) async fn terminate_process_group(&self) {
        super::stdio_process_group::terminate_process_group_id(self.process_group_id).await;
    }
}

fn is_supported_version(version: &ProtocolVersion) -> bool {
    version == &MCP_PROTOCOL_VERSION || version == &LEGACY_PROTOCOL_VERSION
}

fn spawn(
    entry: &McpServerStdioConfig,
) -> Result<(
    TokioChildProcess,
    tokio::task::JoinHandle<()>,
    Arc<Mutex<String>>,
    Option<i32>,
)> {
    let mut command = Command::new(&entry.command);
    command.args(&entry.args);
    if !entry.env.is_empty() {
        command.envs(
            entry
                .env
                .iter()
                .map(|entry: &EnvVariable| (entry.name.as_str(), entry.value.as_str())),
        );
    }
    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let (transport, stderr) = TokioChildProcess::builder(command)
        .stderr(Stdio::piped())
        .spawn()?;
    let process_group_id = transport.id().and_then(|pid| i32::try_from(pid).ok());
    let stderr =
        stderr.ok_or_else(|| MaenifoldError::Message("child stderr was not piped".to_string()))?;
    let sink = Arc::new(Mutex::new(String::new()));
    let task = tokio::spawn(collect_stderr(stderr, Arc::clone(&sink)));
    Ok((transport, task, sink, process_group_id))
}

async fn collect_stderr(stderr: ChildStderr, sink: Arc<Mutex<String>>) {
    let mut reader = BufReader::new(stderr);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let mut sink = sink.lock().await;
                sink.push_str(&line);
                let overflow = sink.len().saturating_sub(STDERR_LIMIT);
                if overflow > 0 {
                    sink.drain(..overflow);
                }
            }
        }
    }
}

async fn collect_failure_stderr(
    stderr_task: tokio::task::JoinHandle<()>,
    stderr: Arc<Mutex<String>>,
) -> Option<String> {
    let _ = tokio::time::timeout(STDERR_JOIN_TIMEOUT, stderr_task).await;
    let stderr = stderr.lock().await.trim().to_string();
    (!stderr.is_empty()).then_some(stderr)
}

fn lock_std<T>(mutex: &StdMutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn to_message_error(error: impl std::fmt::Display) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}

fn map_service_error(error: ServiceError) -> MaenifoldError {
    match error {
        ServiceError::McpError(error) => MaenifoldError::JsonRpc {
            code: i64::from(error.code.0),
            message: error.message.to_string(),
            data: error.data.unwrap_or(Value::Null),
        },
        other => MaenifoldError::Message(other.to_string()),
    }
}

#[cfg(test)]
#[path = "stdio_session_tests.rs"]
mod tests;
