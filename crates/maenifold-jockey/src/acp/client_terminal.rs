use agent_client_protocol::schema::{self as acp};
use dashmap::DashMap;
use std::sync::OnceLock;

struct TerminalHandle {
    child: tokio::sync::Mutex<tokio::process::Child>,
    output: tokio::sync::Mutex<String>,
    output_byte_limit: Option<u64>,
}

static TERMINAL_HANDLES: OnceLock<DashMap<String, TerminalHandle>> = OnceLock::new();
fn terminal_handles() -> &'static DashMap<String, TerminalHandle> {
    TERMINAL_HANDLES.get_or_init(DashMap::new)
}

pub(crate) async fn shutdown_terminals() {
    let ids: Vec<String> = terminal_handles()
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for id in ids {
        if let Some((_, handle)) = terminal_handles().remove(&id) {
            match tokio::time::timeout(std::time::Duration::from_millis(500), handle.child.lock())
                .await
            {
                Ok(mut child) => {
                    let _ = child.kill().await;
                    let _ =
                        tokio::time::timeout(std::time::Duration::from_millis(500), child.wait())
                            .await;
                }
                Err(_) => {}
            }
        }
    }
}

pub(crate) async fn create_terminal(
    args: acp::CreateTerminalRequest,
) -> acp::Result<acp::CreateTerminalResponse> {
    let mut cmd = tokio::process::Command::new(&args.command);
    cmd.args(&args.args);
    if let Some(cwd) = &args.cwd {
        cmd.current_dir(cwd);
    }
    for env_var in &args.env {
        cmd.env(&env_var.name, &env_var.value);
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());
    let child = cmd
        .spawn()
        .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;
    let terminal_id = acp::TerminalId::from(uuid::Uuid::new_v4().to_string());
    terminal_handles().insert(
        terminal_id.to_string(),
        TerminalHandle {
            child: tokio::sync::Mutex::new(child),
            output: tokio::sync::Mutex::new(String::new()),
            output_byte_limit: args.output_byte_limit,
        },
    );
    Ok(acp::CreateTerminalResponse::new(terminal_id))
}

pub(crate) async fn terminal_output(
    args: acp::TerminalOutputRequest,
) -> acp::Result<acp::TerminalOutputResponse> {
    let key = args.terminal_id.to_string();
    let handle = terminal_handles().get(&key).ok_or_else(|| {
        acp::Error::new(acp::ErrorCode::InvalidParams.into(), "terminal not found")
    })?;
    let byte_limit = handle.output_byte_limit;
    let mut child = handle.child.lock().await;
    let mut accumulated = handle.output.lock().await;
    let mut new_chunk = String::new();
    if let Some(stdout) = child.stdout.as_mut() {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 4096];
        match tokio::time::timeout(std::time::Duration::from_millis(100), stdout.read(&mut buf))
            .await
        {
            Ok(Ok(n)) if n > 0 => new_chunk.push_str(&String::from_utf8_lossy(&buf[..n])),
            _ => {}
        }
    }
    accumulated.push_str(&new_chunk);
    let truncated = if let Some(limit) = byte_limit {
        let limit = limit as usize;
        if accumulated.len() > limit {
            // Keep only the last `limit` bytes (tail of output)
            let overflow = accumulated.len() - limit;
            accumulated.drain(..overflow);
            true
        } else {
            false
        }
    } else {
        false
    };
    let output = accumulated.clone();
    drop(accumulated);
    let exit_status = child
        .try_wait()
        .ok()
        .flatten()
        .map(|s| acp::TerminalExitStatus::new().exit_code(s.code().map(|c| c as u32)));
    Ok(acp::TerminalOutputResponse::new(output, truncated).exit_status(exit_status))
}

pub(crate) async fn wait_for_terminal_exit(
    args: acp::WaitForTerminalExitRequest,
) -> acp::Result<acp::WaitForTerminalExitResponse> {
    let key = args.terminal_id.to_string();
    let handle = terminal_handles().get(&key).ok_or_else(|| {
        acp::Error::new(acp::ErrorCode::InvalidParams.into(), "terminal not found")
    })?;
    let mut child = handle.child.lock().await;
    let status = child
        .wait()
        .await
        .map_err(|e| acp::Error::new(acp::ErrorCode::InternalError.into(), e.to_string()))?;
    Ok(acp::WaitForTerminalExitResponse::new(
        acp::TerminalExitStatus::new().exit_code(status.code().map(|c| c as u32)),
    ))
}

pub(crate) async fn kill_terminal(
    args: acp::KillTerminalRequest,
) -> acp::Result<acp::KillTerminalResponse> {
    let key = args.terminal_id.to_string();
    if let Some(handle) = terminal_handles().get(&key) {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
    }
    Ok(acp::KillTerminalResponse::new())
}

pub(crate) async fn release_terminal(
    args: acp::ReleaseTerminalRequest,
) -> acp::Result<acp::ReleaseTerminalResponse> {
    let key = args.terminal_id.to_string();
    if let Some((_, handle)) = terminal_handles().remove(&key) {
        let mut child = handle.child.lock().await;
        let _ = child.kill().await;
    }
    Ok(acp::ReleaseTerminalResponse::new())
}
