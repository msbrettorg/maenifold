use crate::error::MaenifoldError;
use crate::mcp::MaenifoldClient;
use crate::mcp::PendingRequestRegistry;
use crate::mcp::stdio_session_support::{
    collect_failure_stderr, lock_std, map_service_error, spawn, to_message_error,
};
use agent_client_protocol_schema::McpServerStdio as McpServerStdioConfig;
use rmcp::{
    Peer, RoleClient, ServiceExt,
    model::{Prompt, ProtocolVersion, Tool},
    service::RunningService,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::Mutex;

const MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2025_03_26;
const LEGACY_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2024_11_05;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
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
    prompts: Arc<[Prompt]>,
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
        Self::start_with_registry(entry, call_timeout, Arc::new(PendingRequestRegistry::new()))
            .await
    }

    pub async fn start_with_registry(
        entry: &McpServerStdioConfig,
        call_timeout: Duration,
        registry: Arc<PendingRequestRegistry>,
    ) -> std::result::Result<Self, StdioStartupError> {
        Self::start_with_startup_timeout_and_registry(
            entry,
            STARTUP_TIMEOUT,
            call_timeout,
            registry,
        )
        .await
    }

    pub async fn start_with_startup_timeout(
        entry: &McpServerStdioConfig,
        startup_timeout: Duration,
        call_timeout: Duration,
    ) -> std::result::Result<Self, StdioStartupError> {
        Self::start_with_startup_timeout_and_registry(
            entry,
            startup_timeout,
            call_timeout,
            Arc::new(PendingRequestRegistry::new()),
        )
        .await
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
            let mut client = MaenifoldClient::new_with_server_name(registry, entry.name.as_str())
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
            let prompts = if server_info.capabilities.prompts.is_some() {
                match client.peer().list_all_prompts().await {
                    Ok(prompts) => prompts,
                    Err(error) => {
                        tracing::debug!(server = %entry.name, error = %error, "failed to list MCP prompts");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            };
            Ok((client, tools, prompts, server_info))
        };
        let (client, tools, prompts, server_info) =
            match tokio::time::timeout(startup_timeout, startup).await {
                Ok(Ok(started)) => started,
                Ok(Err(error)) => {
                    super::stdio_process_group::terminate_process_group_id(process_group_id).await;
                    let stderr = collect_failure_stderr(
                        stderr_task,
                        Arc::clone(&stderr),
                        STDERR_JOIN_TIMEOUT,
                    )
                    .await;
                    return Err(StdioStartupError::new(error, stderr));
                }
                Err(_) => {
                    super::stdio_process_group::terminate_process_group_id(process_group_id).await;
                    let stderr = collect_failure_stderr(
                        stderr_task,
                        Arc::clone(&stderr),
                        STDERR_JOIN_TIMEOUT,
                    )
                    .await;
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
            prompts: prompts.into(),
            initialize_description: crate::mcp::gateway_registry::upstream_initialize_description(
                &server_info,
            ),
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

    pub fn prompts(&self) -> Arc<[Prompt]> {
        Arc::clone(&self.prompts)
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

#[cfg(test)]
#[path = "stdio_session_tests.rs"]
mod tests;
