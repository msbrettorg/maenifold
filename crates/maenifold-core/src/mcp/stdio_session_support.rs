use crate::error::{MaenifoldError, Result};
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use rmcp::{service::ServiceError, transport::TokioChildProcess};
use serde_json::Value;
#[cfg(unix)]
use std::io;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, Command};
use tokio::sync::Mutex;

const STDERR_LIMIT: usize = 4096;

pub(super) fn spawn(
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

pub(super) async fn collect_failure_stderr(
    stderr_task: tokio::task::JoinHandle<()>,
    stderr: Arc<Mutex<String>>,
    join_timeout: Duration,
) -> Option<String> {
    let _ = tokio::time::timeout(join_timeout, stderr_task).await;
    let stderr = stderr.lock().await.trim().to_string();
    (!stderr.is_empty()).then_some(stderr)
}

pub(super) fn lock_std<T>(mutex: &StdMutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

pub(super) fn to_message_error(error: impl std::fmt::Display) -> MaenifoldError {
    MaenifoldError::Message(error.to_string())
}

pub(super) fn map_service_error(error: ServiceError) -> MaenifoldError {
    match error {
        ServiceError::McpError(error) => MaenifoldError::JsonRpc {
            code: i64::from(error.code.0),
            message: error.message.to_string(),
            data: error.data.unwrap_or(Value::Null),
        },
        other => MaenifoldError::Message(other.to_string()),
    }
}
