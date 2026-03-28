use std::path::PathBuf;

use thiserror::Error;

#[path = "driver/command.rs"]
mod command;
#[path = "driver/response.rs"]
mod response;

pub use response::{AcpxResponse, ToolCallRecord};

use command::{
    CommandSpec, build_command, cancel_spec, close_session_spec, create_session_spec, prompt_spec,
};
use response::parse_ndjson_response;

pub type Result<T> = std::result::Result<T, AcpxError>;

#[derive(Debug, Error)]
pub enum AcpxError {
    #[error("failed to resolve acpx binary with `which acpx`: {0}")]
    BinaryResolution(String),
    #[error("acpx binary not found on PATH")]
    BinaryNotFound,
    #[error("failed to spawn acpx command `{command}`: {source}")]
    SpawnFailed {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("acpx stdout pipe was not available")]
    MissingStdout,
    #[error("acpx stderr pipe was not available")]
    MissingStderr,
    #[error("invalid acpx NDJSON event: {0}")]
    InvalidEvent(String),
    #[error("acpx command timed out after {timeout_secs}s: {details}")]
    Timeout { timeout_secs: u64, details: String },
    #[error("acpx command failed with exit code {code:?}: {details}")]
    CommandFailed { code: Option<i32>, details: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct AcpxDriver {
    acpx_path: PathBuf,
    agent: String,
    timeout: u64,
    cwd: PathBuf,
}

struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

impl AcpxDriver {
    pub fn new(agent: String, timeout: u64, cwd: PathBuf) -> Result<Self> {
        let acpx_path = resolve_acpx_path()?;
        Ok(Self {
            acpx_path,
            agent,
            timeout,
            cwd,
        })
    }

    pub async fn create_session(&self, name: &str) -> Result<()> {
        self.run_status_only(create_session_spec(&self.agent, name))
            .await
    }

    pub async fn close_session(&self, name: &str) -> Result<()> {
        self.run_status_only(close_session_spec(&self.agent, name))
            .await
    }

    pub async fn prompt(&self, session: &str, prompt: &str) -> Result<AcpxResponse> {
        let spec = prompt_spec(&self.agent, self.timeout, session, prompt);
        let output = self.run_command(&spec).await?;

        if !output.status.success() {
            return Err(map_command_failure(
                output.status.code(),
                &output.stdout,
                &output.stderr,
                self.timeout,
            ));
        }

        parse_ndjson_response(&output.stdout)
    }

    pub async fn cancel(&self, session: &str) -> Result<()> {
        self.run_status_only(cancel_spec(&self.agent, session))
            .await
    }

    async fn run_status_only(&self, spec: CommandSpec) -> Result<()> {
        let output = self.run_command(&spec).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(map_command_failure(
                output.status.code(),
                &output.stdout,
                &output.stderr,
                self.timeout,
            ))
        }
    }

    async fn run_command(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        let mut command = build_command(&self.acpx_path, &self.cwd, spec);
        let command_display = spec.shell_string(&self.acpx_path);
        let env_summary = inherited_env_summary();

        eprintln!(
            "[acpx] running: {command_display} (cwd: {}, env: {env_summary})",
            self.cwd.display()
        );

        let output = command.output().await.map_err(|source| {
            eprintln!(
                "[acpx] spawn failed: {command_display} (cwd: {}, env: {env_summary}): {source}",
                self.cwd.display()
            );
            AcpxError::SpawnFailed {
                command: command_display.clone(),
                source,
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            eprintln!(
                "[acpx] command failed: {command_display} (cwd: {}, env: {env_summary}, status: {:?})",
                self.cwd.display(),
                output.status.code()
            );
            let stdout_trimmed = stdout.trim();
            let stderr_trimmed = stderr.trim();
            if !stdout_trimmed.is_empty() {
                eprintln!("[acpx] stdout: {stdout_trimmed}");
            }
            if !stderr_trimmed.is_empty() {
                eprintln!("[acpx] stderr: {stderr_trimmed}");
            }
            if stdout_trimmed.is_empty() && stderr_trimmed.is_empty() {
                eprintln!("[acpx] (no stdout or stderr output)");
            }
        }

        Ok(CommandOutput {
            status: output.status,
            stdout,
            stderr,
        })
    }

    #[cfg(test)]
    fn new_for_tests(agent: String, timeout: u64, cwd: PathBuf, acpx_path: PathBuf) -> Self {
        Self {
            acpx_path,
            agent,
            timeout,
            cwd,
        }
    }
}

fn resolve_acpx_path() -> Result<PathBuf> {
    let output = std::process::Command::new("which")
        .arg("acpx")
        .output()
        .map_err(|error| AcpxError::BinaryResolution(error.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(if stderr.trim().is_empty() {
            AcpxError::BinaryNotFound
        } else {
            AcpxError::BinaryResolution(stderr.trim().to_string())
        });
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err(AcpxError::BinaryNotFound);
    }

    Ok(PathBuf::from(path))
}

fn map_command_failure(
    code: Option<i32>,
    stdout: &str,
    stderr: &str,
    timeout_secs: u64,
) -> AcpxError {
    let details = merge_process_output(stdout, stderr);
    if output_indicates_timeout(&details) {
        AcpxError::Timeout {
            timeout_secs,
            details,
        }
    } else {
        AcpxError::CommandFailed { code, details }
    }
}

fn merge_process_output(stdout: &str, stderr: &str) -> String {
    let stdout = stdout.trim();
    let stderr = stderr.trim();
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("stdout: {stdout}; stderr: {stderr}"),
        (false, true) => format!("stdout: {stdout}"),
        (true, false) => format!("stderr: {stderr}"),
        (true, true) => "acpx exited without diagnostic output".to_string(),
    }
}

fn output_indicates_timeout(details: &str) -> bool {
    let normalized = details.to_ascii_lowercase();
    normalized.contains("timed out") || normalized.contains("timeout")
}

fn inherited_env_summary() -> String {
    format!(
        "PATH={}; HOME={}; XDG_CONFIG_HOME={}",
        env_var("PATH"),
        env_var("HOME"),
        env_var("XDG_CONFIG_HOME")
    )
}

fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| "<unset>".to_string())
}

#[cfg(test)]
#[path = "driver/tests.rs"]
mod tests;
