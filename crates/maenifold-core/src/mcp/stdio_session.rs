use crate::error::{
    GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, MaenifoldError, Result,
};
use super::stdio_io::{parse_result, read_response, send_message, wait_for_exit};
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use rmcp::model::{CallToolResult, InitializeResult, ListToolsResult, ReadResourceResult, Tool};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex as StdMutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const LEGACY_PROTOCOL_VERSION: &str = "2024-11-05";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const STDERR_LIMIT: usize = 4096;
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);
const TERM_WAIT: Duration = Duration::from_secs(1);
struct StartupState {
    io: (ChildStdin, BufReader<ChildStdout>),
    child: Child,
    stderr: Arc<Mutex<String>>,
    stderr_task: tokio::task::JoinHandle<()>,
}
pub struct StdioSession {
    io: Mutex<(ChildStdin, BufReader<ChildStdout>)>,
    child: StdMutex<Option<Child>>,
    stderr_task: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    stderr: Arc<Mutex<String>>,
    tools: Arc<[Tool]>,
    next_id: AtomicU64,
    call_timeout: Duration,
    server_name: String,
    dead: AtomicBool,
    poisoned: AtomicBool,
}
impl StdioSession {
    pub async fn start(entry: &McpServerStdioConfig, call_timeout: Duration) -> Result<Self> {
        let mut state = spawn(entry)?;
        let startup = async {
            match request::<InitializeResult>(
                &mut state.io,
                1,
                json!({"protocolVersion":MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"maenifold-core","version":"0.1.0"}}),
                &entry.name,
            )
            .await? {
                result if is_supported_version(&result.protocol_version.to_string()) => {}
                result => return Err(MaenifoldError::Message(format!("unsupported MCP protocol version '{}'", result.protocol_version))),
            }
            send_message(&mut state.io.0, &json!({"jsonrpc":"2.0","method":"notifications/initialized"})).await?;
            list_tools(&mut state.io, 2, &entry.name).await
        };
        let tools = match tokio::time::timeout(STARTUP_TIMEOUT, startup).await {
            Ok(Ok(tools)) => tools,
            Ok(Err(error)) => { shutdown_startup(state).await; return Err(error); }
            Err(_) => { shutdown_startup(state).await; return Err(MaenifoldError::Message("MCP server startup timed out after 60s".to_string())); }
        };
        Ok(Self {
            io: Mutex::new(state.io),
            child: StdMutex::new(Some(state.child)),
            stderr_task: StdMutex::new(Some(state.stderr_task)),
            stderr: state.stderr,
            tools: tools.into(),
            next_id: AtomicU64::new(100),
            call_timeout,
            server_name: entry.name.to_string(),
            dead: AtomicBool::new(false),
            poisoned: AtomicBool::new(false),
        })
    }
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        self.do_request("tools/call", json!({"name": name, "arguments": arguments}), format!("tool '{name}'")).await
    }
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        self.do_request("resources/read", json!({"uri": uri}), format!("resource '{uri}'")).await
    }
    async fn do_request<T: DeserializeOwned>(&self, method: &str, params: Value, what: String) -> Result<T> {
        if !self.is_usable() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut io = self.io.lock().await;
        if self.is_poisoned() || !self.refresh_liveness() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let response = match tokio::time::timeout(self.call_timeout, async {
            send_message(&mut io.0, &request).await?;
            let (writer, reader) = &mut *io;
            read_response(writer, reader, id, &self.server_name).await
        }).await {
            Ok(result) => result,
            Err(_) => {
                let _ = send_message(&mut io.0, &json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id}})).await;
                self.poisoned.store(true, Ordering::SeqCst);
                return Err(self.gateway_error(GATEWAY_TIMEOUT_ERROR, format!("Gateway: {} {} timed out after {}s", self.server_name, what, self.call_timeout.as_secs())).await);
            }
        };
        match response {
            Ok(value) => parse_result(value),
            Err(error) if is_connection_error(&error) => {
                self.dead.store(true, Ordering::SeqCst);
                Err(self.gateway_error(JSONRPC_INTERNAL_ERROR, format!("Gateway: {} connection lost: {}", self.server_name, connection_detail(&error))).await)
            }
            Err(error) => Err(error),
        }
    }
    pub fn tools(&self) -> Arc<[Tool]> { Arc::clone(&self.tools) }
    pub fn is_usable(&self) -> bool { !self.is_poisoned() && self.refresh_liveness() }
    pub fn is_poisoned(&self) -> bool { self.poisoned.load(Ordering::SeqCst) }
    pub async fn shutdown(&self) {
        let mut io = self.io.lock().await;
        let _ = io.0.shutdown().await;
        drop(io);
        let child_opt = { lock_std(&self.child).take() };
        if let Some(mut child) = child_opt {
            if !wait_for_exit(&mut child, SHUTDOWN_WAIT).await {
                terminate_child(&mut child).await;
            }
        }
        let task_opt = { lock_std(&self.stderr_task).take() };
        if let Some(task) = task_opt { let _ = task.await; }
    }
    fn refresh_liveness(&self) -> bool {
        if self.dead.load(Ordering::SeqCst) { return false; }
        let mut child = lock_std(&self.child);
        match child.as_mut().map(Child::try_wait) {
            Some(Ok(None)) => true,
            Some(Ok(Some(_))) | Some(Err(_)) | None => { self.dead.store(true, Ordering::SeqCst); false }
        }
    }
    async fn gateway_error(&self, code: i64, message: String) -> MaenifoldError {
        MaenifoldError::JsonRpc { code, message, data: stderr_data(&self.stderr).await }
    }
}
fn lock_std<T>(mutex: &StdMutex<T>) -> MutexGuard<'_, T> { mutex.lock().unwrap_or_else(|error| error.into_inner()) }
fn is_supported_version(version: &str) -> bool { matches!(version, MCP_PROTOCOL_VERSION | LEGACY_PROTOCOL_VERSION) }
fn is_connection_error(error: &MaenifoldError) -> bool { matches!(error, MaenifoldError::Message(msg) if msg.contains("connection error")) }
fn connection_detail(error: &MaenifoldError) -> String {
    match error {
        MaenifoldError::Message(msg) => msg.strip_prefix("connection error: ").unwrap_or(msg).to_string(),
        _ => error.to_string(),
    }
}
fn spawn(entry: &McpServerStdioConfig) -> Result<StartupState> {
    let mut command = Command::new(&entry.command);
    command.args(&entry.args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    if !entry.env.is_empty() { command.envs(entry.env.iter().map(|entry: &EnvVariable| (entry.name.as_str(), entry.value.as_str()))); }
    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    let stdin = child.stdin.take().ok_or_else(|| {
        let _ = child.start_kill();
        MaenifoldError::Message("child stdin was not piped".to_string())
    })?;
    let stdout = BufReader::new(child.stdout.take().ok_or_else(|| {
        let _ = child.start_kill();
        MaenifoldError::Message("child stdout was not piped".to_string())
    })?);
    let stderr = child.stderr.take().ok_or_else(|| {
        let _ = child.start_kill();
        MaenifoldError::Message("child stderr was not piped".to_string())
    })?;
    let stderr_sink = Arc::new(Mutex::new(String::new()));
    let stderr_task = tokio::spawn(collect_stderr(stderr, Arc::clone(&stderr_sink)));
    Ok(StartupState { io: (stdin, stdout), child, stderr: stderr_sink, stderr_task })
}
async fn collect_stderr(stderr: ChildStderr, sink: Arc<Mutex<String>>) {
    let mut reader = BufReader::new(stderr);
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => { let mut sink = sink.lock().await; sink.push_str(&line); let overflow = sink.len().saturating_sub(STDERR_LIMIT); if overflow > 0 { sink.drain(..overflow); } }
        }
    }
}
async fn stderr_data(stderr: &Arc<Mutex<String>>) -> Value {
    let stderr = stderr.lock().await;
    let tail = tail_chars(stderr.trim(), 500);
    if tail.is_empty() { Value::Null } else { json!({ "stderrTail": tail }) }
}
fn tail_chars(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();
    if len <= max_chars { value.to_string() } else { value.chars().skip(len - max_chars).collect() }
}
async fn request<T: DeserializeOwned>(io: &mut (ChildStdin, BufReader<ChildStdout>), id: u64, params: Value, server_name: &str) -> Result<T> {
    send_message(&mut io.0, &json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":params})).await?;
    let (writer, reader) = io;
    parse_result(read_response(writer, reader, id, server_name).await?)
}
async fn list_tools(io: &mut (ChildStdin, BufReader<ChildStdout>), mut id: u64, server_name: &str) -> Result<Vec<Tool>> {
    let mut tools = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let params = cursor.as_ref().map(|cursor| json!({"cursor":cursor}));
        let message = match params {
            Some(params) => json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":params}),
            None => json!({"jsonrpc":"2.0","id":id,"method":"tools/list"}),
        };
        send_message(&mut io.0, &message).await?;
        let (writer, reader) = &mut *io;
        let result: ListToolsResult = parse_result(read_response(writer, reader, id, server_name).await?)?;
        tools.extend(result.tools);
        if let Some(next) = result.next_cursor {
            cursor = Some(next);
            id += 1;
        } else {
            return Ok(tools);
        }
    }
}
async fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id().and_then(|pid| i32::try_from(pid).ok()) {
        if unsafe { libc::getpgid(pid) } == pid {
            let _ = unsafe { libc::killpg(pid, libc::SIGTERM) };
            if wait_for_exit(child, TERM_WAIT).await { return; }
            let _ = unsafe { libc::killpg(pid, libc::SIGKILL) };
            let _ = wait_for_exit(child, TERM_WAIT).await;
            return;
        }
    }
    let _ = child.start_kill();
    let _ = wait_for_exit(child, TERM_WAIT).await;
}
async fn shutdown_startup(mut state: StartupState) {
    let _ = state.io.0.shutdown().await;
    if !wait_for_exit(&mut state.child, SHUTDOWN_WAIT).await {
        terminate_child(&mut state.child).await;
    }
    let _ = state.stderr_task.await;
}
#[cfg(test)] #[path = "stdio_session_tests.rs"] mod tests;
