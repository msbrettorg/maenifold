use super::MCP_PROTOCOL_VERSION;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

const PROCESS_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const STDERR_JOIN_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(unix)]
const PROCESS_GROUP_TERM_WAIT: Duration = Duration::from_millis(200);
#[cfg(unix)]
const PROCESS_GROUP_POLL_INTERVAL: Duration = Duration::from_millis(10);

async fn send_stdio_message(stdin: &mut ChildStdin, message: &Value) -> Result<(), String> {
    let mut body = serde_json::to_vec(message)
        .map_err(|error| format!("failed to serialize message: {error}"))?;
    body.push(b'\n');
    stdin
        .write_all(&body)
        .await
        .map_err(|error| format!("failed to write message body: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("failed to flush stdin: {error}"))
}

async fn read_stdio_response<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    expected_id: u64,
) -> Result<Value, String> {
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .await
            .map_err(|error| format!("failed to read MCP line: {error}"))?;
        if bytes == 0 {
            return Err("unexpected EOF while reading MCP response".to_string());
        }
        let message: Value = serde_json::from_str(line.trim()).map_err(|error| {
            format!(
                "failed to parse MCP line as JSON: {error}; line={}",
                line.trim()
            )
        })?;
        if message.get("id").and_then(Value::as_u64) == Some(expected_id) {
            if let Some(error) = message.get("error").filter(|error| !error.is_null()) {
                return Err(format!("JSON-RPC error: {error}"));
            }
            return Ok(message);
        }
    }
}

async fn collect_stderr(
    stderr: tokio::process::ChildStderr,
    sink: Arc<Mutex<String>>,
) -> Result<(), String> {
    let mut reader = BufReader::new(stderr);
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .await
            .map_err(|error| format!("failed to read stderr: {error}"))?;
        if bytes == 0 {
            break;
        }
        sink.lock().await.push_str(&line);
    }
    Ok(())
}

#[cfg(unix)]
fn configure_new_process_group(command: &mut Command) {
    use std::io;
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(unix)]
async fn wait_for_child_exit(child: &mut Child, duration: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            Err(_) => return false,
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(PROCESS_GROUP_POLL_INTERVAL).await;
    }
}

#[cfg(unix)]
fn signal_process_group(leader_pid: i32, signal: i32) -> std::io::Result<()> {
    let result = unsafe { libc::killpg(leader_pid, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
async fn kill_process_group(child: &mut Child) -> bool {
    let leader_pid = match child.id().and_then(|pid| i32::try_from(pid).ok()) {
        Some(pid) => pid,
        None => return false,
    };

    match child.try_wait() {
        Ok(Some(_)) => return true,
        Ok(None) => {}
        Err(_) => return false,
    }

    let pgid = unsafe { libc::getpgid(leader_pid) };
    if pgid != leader_pid {
        eprintln!(
            "maenifold: MCP stdio child {leader_pid} is not a pg leader (pgid={pgid}); skipping killpg"
        );
        return false;
    }

    if let Err(error) = signal_process_group(leader_pid, libc::SIGTERM) {
        if error.raw_os_error() != Some(libc::ESRCH) {
            eprintln!("maenifold: Failed to SIGTERM MCP stdio process group {leader_pid}: {error}");
            return false;
        }
        return true;
    }

    if wait_for_child_exit(child, PROCESS_GROUP_TERM_WAIT).await {
        return true;
    }

    if let Err(error) = signal_process_group(leader_pid, libc::SIGKILL) {
        if error.raw_os_error() != Some(libc::ESRCH) {
            eprintln!("maenifold: Failed to SIGKILL MCP stdio process group {leader_pid}: {error}");
            return false;
        }
    }
    let _ = wait_for_child_exit(child, PROCESS_GROUP_TERM_WAIT).await;
    true
}

#[cfg(not(unix))]
async fn kill_process_group(_child: &mut Child) -> bool {
    false
}

async fn shutdown_child(
    child: &mut Child,
    stderr_task: tokio::task::JoinHandle<Result<(), String>>,
    stderr_text: Arc<Mutex<String>>,
) -> Result<String, String> {
    if child
        .try_wait()
        .map_err(|error| format!("failed to inspect child process: {error}"))?
        .is_none()
    {
        if !kill_process_group(child).await {
            if let Err(error) = child.start_kill() {
                eprintln!("maenifold: Failed to kill MCP stdio child: {error}");
            }
        }
        timeout(PROCESS_SHUTDOWN_TIMEOUT, child.wait())
            .await
            .map_err(|_| "timed out waiting for child process to exit".to_string())?
            .map_err(|error| format!("failed to wait for child process: {error}"))?;
    }
    match timeout(STDERR_JOIN_TIMEOUT, stderr_task).await {
        Ok(Ok(Ok(()))) | Err(_) => {}
        Ok(Ok(Err(error))) => return Err(error),
        Ok(Err(error)) => return Err(format!("stderr task join failed: {error}")),
    }
    Ok(stderr_text.lock().await.clone())
}

pub(super) async fn discover_stdio_tools(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    timeout_duration: Duration,
) -> Result<Vec<Value>, String> {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if !env.is_empty() {
        cmd.envs(env);
    }

    #[cfg(unix)]
    configure_new_process_group(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|error| format!("failed to spawn '{command} {}': {error}", args.join(" ")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "child stdin was not piped".to_string())?;
    let stdout: ChildStdout = child
        .stdout
        .take()
        .ok_or_else(|| "child stdout was not piped".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "child stderr was not piped".to_string())?;
    let stderr_text = Arc::new(Mutex::new(String::new()));
    let stderr_task = tokio::spawn(collect_stderr(stderr, Arc::clone(&stderr_text)));
    let mut stdout = BufReader::new(stdout);

    let run = async {
        send_stdio_message(&mut stdin, &json!({"jsonrpc":"2.0","id":1u64,"method":"initialize","params":{"protocolVersion":MCP_PROTOCOL_VERSION,"capabilities":{},"clientInfo":{"name":"maenifold-core","version":"0.1.0"}}})).await?;
        read_stdio_response(&mut stdout, 1).await?;
        send_stdio_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        )
        .await?;
        send_stdio_message(
            &mut stdin,
            &json!({"jsonrpc":"2.0","id":2u64,"method":"tools/list"}),
        )
        .await?;
        let payload = read_stdio_response(&mut stdout, 2).await?;
        payload
            .get("result")
            .and_then(|result| result.get("tools"))
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| "tools/list response missing result.tools".to_string())
    };

    let result = timeout(timeout_duration, run)
        .await
        .map_err(|_| "timed out waiting for stdio server".to_string())?;
    let stderr_output = shutdown_child(&mut child, stderr_task, stderr_text).await?;
    result.map_err(|error| {
        if stderr_output.trim().is_empty() {
            error
        } else {
            format!("{error}\nstderr:\n{}", stderr_output.trim())
        }
    })
}
