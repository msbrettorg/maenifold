use crate::error::{MaenifoldError, Result};
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use rmcp::model::{CallToolResult, InitializeResult, ListToolsResult, Tool};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex as StdMutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const LEGACY_PROTOCOL_VERSION: &str = "2024-11-05";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const STDERR_LIMIT: usize = 4096;
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);
const TERM_WAIT: Duration = Duration::from_secs(1);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
struct StartupState {
    io: (ChildStdin, BufReader<ChildStdout>),
    child: Child,
    stderr_task: tokio::task::JoinHandle<()>,
}
pub struct StdioSession {
    io: Mutex<(ChildStdin, BufReader<ChildStdout>)>,
    child: StdMutex<Option<Child>>,
    stderr_task: StdMutex<Option<tokio::task::JoinHandle<()>>>,
    tools: Arc<[Tool]>,
    next_id: AtomicU64,
    dead: AtomicBool,
    poisoned: AtomicBool,
}
impl StdioSession {
    pub async fn start(entry: &McpServerStdioConfig) -> Result<Self> {
        let mut state = spawn(entry)?;
        let startup = async {
            match request::<InitializeResult>(
                &mut state.io,
                1,
                json!({"protocolVersion":MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"maenifold-core","version":"0.1.0"}}),
            )
            .await? {
                result if is_supported_version(&result.protocol_version.to_string()) => {}
                result => return Err(MaenifoldError::Message(format!("unsupported MCP protocol version '{}'", result.protocol_version))),
            }
            send_message(&mut state.io.0, &json!({"jsonrpc":"2.0","method":"notifications/initialized"})).await?;
            list_tools(&mut state.io, 2).await
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
            tools: tools.into(),
            next_id: AtomicU64::new(100),
            dead: AtomicBool::new(false),
            poisoned: AtomicBool::new(false),
        })
    }
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        if !self.is_usable() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut io = self.io.lock().await;
        if self.is_poisoned() || !self.refresh_liveness() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let request = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        let response = match tokio::time::timeout(CALL_TIMEOUT, async {
            send_message(&mut io.0, &request).await?;
            read_response(&mut io.1, id).await
        }).await {
            Ok(result) => result,
            Err(_) => {
                let _ = send_message(&mut io.0, &json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id}})).await;
                self.poisoned.store(true, Ordering::SeqCst);
                return Err(MaenifoldError::Message(format!("timed out waiting for tools/call response for '{name}'")));
            }
        };
        match response {
            Ok(value) => parse_result(value),
            Err(error) if is_connection_error(&error) => {
                self.dead.store(true, Ordering::SeqCst);
                Err(error)
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
        if let Some(mut child) = lock_std(&self.child).take() {
            if !wait_for_exit(&mut child, SHUTDOWN_WAIT).await {
                terminate_child(&mut child).await;
            }
        }
        if let Some(task) = lock_std(&self.stderr_task).take() { let _ = task.await; }
    }
    fn refresh_liveness(&self) -> bool {
        if self.dead.load(Ordering::SeqCst) { return false; }
        let mut child = lock_std(&self.child);
        match child.as_mut().map(Child::try_wait) {
            Some(Ok(None)) => true,
            Some(Ok(Some(_))) | Some(Err(_)) | None => { self.dead.store(true, Ordering::SeqCst); false }
        }
    }
}
fn lock_std<T>(mutex: &StdMutex<T>) -> MutexGuard<'_, T> { mutex.lock().unwrap_or_else(|error| error.into_inner()) }
fn is_supported_version(version: &str) -> bool { matches!(version, MCP_PROTOCOL_VERSION | LEGACY_PROTOCOL_VERSION) }
fn is_connection_error(error: &MaenifoldError) -> bool { matches!(error, MaenifoldError::Message(msg) if msg.contains("connection error")) }
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
    let stderr_task = tokio::spawn(collect_stderr(stderr, Arc::new(Mutex::new(String::new()))));
    Ok(StartupState { io: (stdin, stdout), child, stderr_task })
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
async fn send_message(stdin: &mut ChildStdin, message: &Value) -> Result<()> {
    let mut body = serde_json::to_vec(message)?;
    body.push(b'\n');
    stdin.write_all(&body).await.map_err(connection_error)?;
    stdin.flush().await.map_err(connection_error)?;
    Ok(())
}
async fn request<T: DeserializeOwned>(io: &mut (ChildStdin, BufReader<ChildStdout>), id: u64, params: Value) -> Result<T> {
    send_message(&mut io.0, &json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":params})).await?;
    parse_result(read_response(&mut io.1, id).await?)
}
async fn list_tools(io: &mut (ChildStdin, BufReader<ChildStdout>), mut id: u64) -> Result<Vec<Tool>> {
    let mut tools = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let params = cursor.as_ref().map(|cursor| json!({"cursor":cursor}));
        let message = match params {
            Some(params) => json!({"jsonrpc":"2.0","id":id,"method":"tools/list","params":params}),
            None => json!({"jsonrpc":"2.0","id":id,"method":"tools/list"}),
        };
        send_message(&mut io.0, &message).await?;
        let result: ListToolsResult = parse_result(read_response(&mut io.1, id).await?)?;
        tools.extend(result.tools);
        if let Some(next) = result.next_cursor {
            cursor = Some(next);
            id += 1;
        } else {
            return Ok(tools);
        }
    }
}
async fn read_response<R: AsyncRead + Unpin>(reader: &mut BufReader<R>, id: u64) -> Result<Value> {
    loop {
        let mut line = Vec::new();
        loop {
            let chunk = reader.fill_buf().await.map_err(connection_error)?;
            if chunk.is_empty() { return Err(MaenifoldError::Message("connection error: unexpected EOF while reading MCP response".to_string())); }
            let take = chunk.iter().position(|&byte| byte == b'\n').map_or(chunk.len(), |idx| idx + 1);
            if line.len() + take > MAX_FRAME_BYTES {
                return Err(MaenifoldError::Message(format!(
                    "connection error: MCP response exceeded max frame size of {MAX_FRAME_BYTES} bytes"
                )));
            }
            line.extend_from_slice(&chunk[..take]);
            reader.consume(take);
            if line.last() == Some(&b'\n') {
                break;
            }
        }
        let message: Value = serde_json::from_slice(&line)?;
        if message.get("method").is_some() && message.get("id").is_none() {
            continue;
        }
        if message.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if let Some(error) = message.get("error").filter(|error| !error.is_null()) {
            let code = error.get("code").and_then(Value::as_i64).ok_or_else(|| MaenifoldError::Message(format!("JSON-RPC error missing integer code: {error}")))?;
            let message = error.get("message").and_then(Value::as_str).unwrap_or("unknown JSON-RPC error");
            let data = error.get("data").cloned().unwrap_or(Value::Null);
            return Err(MaenifoldError::JsonRpc { code, message: message.to_string(), data });
        }
        return Ok(message["result"].clone());
    }
}
fn parse_result<T: DeserializeOwned>(value: Value) -> Result<T> { serde_json::from_value(value).map_err(Into::into) }
fn connection_error(error: io::Error) -> MaenifoldError { MaenifoldError::Message(format!("connection error: {error}")) }
async fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            Err(_) => return false,
        }
        if tokio::time::Instant::now() >= deadline { return false; }
        tokio::time::sleep(POLL_INTERVAL).await;
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
