use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;
use thiserror::Error;
use tokio::process::Command;

pub type Result<T> = std::result::Result<T, AcpxError>;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallRecord {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub arguments: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AcpxResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub error: Option<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandSpec {
    args: Vec<String>,
}

impl CommandSpec {
    fn shell_string(&self, binary: &Path) -> String {
        let mut parts = vec![binary.display().to_string()];
        parts.extend(self.args.iter().cloned());
        parts.join(" ")
    }
}

#[derive(Debug, Clone)]
pub struct AcpxDriver {
    acpx_path: PathBuf,
    agent: String,
    timeout: u64,
    cwd: PathBuf,
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
        let spec = self.create_session_spec(name);
        self.run_status_only(spec).await
    }

    pub async fn close_session(&self, name: &str) -> Result<()> {
        let spec = self.close_session_spec(name);
        self.run_status_only(spec).await
    }

    pub async fn prompt(&self, session: &str, prompt: &str) -> Result<AcpxResponse> {
        let spec = self.prompt_spec(session, prompt);
        let mut command = self.build_command(&spec);
        let command_display = spec.shell_string(&self.acpx_path);
        let output = command
            .output()
            .await
            .map_err(|source| AcpxError::SpawnFailed {
                command: command_display.clone(),
                source,
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(map_command_failure(
                output.status.code(),
                &stdout,
                &stderr,
                self.timeout,
            ));
        }

        parse_ndjson_response(&stdout)
    }

    pub async fn cancel(&self, session: &str) -> Result<()> {
        let spec = self.cancel_spec(session);
        self.run_status_only(spec).await
    }

    fn create_session_spec(&self, name: &str) -> CommandSpec {
        CommandSpec {
            args: vec![
                self.agent.clone(),
                "sessions".to_string(),
                "new".to_string(),
                "--name".to_string(),
                name.to_string(),
            ],
        }
    }

    fn close_session_spec(&self, name: &str) -> CommandSpec {
        CommandSpec {
            args: vec![
                self.agent.clone(),
                "sessions".to_string(),
                "close".to_string(),
                name.to_string(),
            ],
        }
    }

    fn prompt_spec(&self, session: &str, prompt: &str) -> CommandSpec {
        CommandSpec {
            args: vec![
                "--format".to_string(),
                "json".to_string(),
                "--timeout".to_string(),
                self.timeout.to_string(),
                "--approve-all".to_string(),
                self.agent.clone(),
                "-s".to_string(),
                session.to_string(),
                prompt.to_string(),
            ],
        }
    }

    fn cancel_spec(&self, session: &str) -> CommandSpec {
        CommandSpec {
            args: vec![
                self.agent.clone(),
                "cancel".to_string(),
                "-s".to_string(),
                session.to_string(),
            ],
        }
    }

    fn build_command(&self, spec: &CommandSpec) -> Command {
        let mut command = Command::new(&self.acpx_path);
        command
            .args(&spec.args)
            .current_dir(&self.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }

    async fn run_status_only(&self, spec: CommandSpec) -> Result<()> {
        let mut command = self.build_command(&spec);
        let command_display = spec.shell_string(&self.acpx_path);
        let output = command
            .output()
            .await
            .map_err(|source| AcpxError::SpawnFailed {
                command: command_display,
                source,
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok(())
        } else {
            Err(map_command_failure(
                output.status.code(),
                &stdout,
                &stderr,
                self.timeout,
            ))
        }
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

fn parse_ndjson_response(stdout: &str) -> Result<AcpxResponse> {
    let mut response = AcpxResponse::default();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let event: Value = serde_json::from_str(line)
            .map_err(|error| AcpxError::InvalidEvent(format!("{error}; line={line}")))?;

        match event.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(chunk) = extract_text_chunk(&event) {
                    response.text.push_str(&chunk);
                }
            }
            Some("tool_call") => response.tool_calls.push(parse_tool_call_record(&event)),
            Some("error") => {
                response.error = extract_error_text(&event).or_else(|| Some(line.to_string()));
            }
            _ => {}
        }
    }

    Ok(response)
}

fn parse_tool_call_record(event: &Value) -> ToolCallRecord {
    ToolCallRecord {
        id: extract_optional_string(event, &["id"]),
        name: extract_optional_string(event, &["name", "tool_name"]),
        title: extract_optional_string(event, &["title"]),
        status: extract_optional_string(event, &["status"]),
        arguments: event
            .get("arguments")
            .cloned()
            .or_else(|| event.get("input").cloned())
            .or_else(|| event.get("data").cloned()),
    }
}

fn extract_text_chunk(event: &Value) -> Option<String> {
    extract_optional_string(event, &["text"]).or_else(|| {
        event.get("data").and_then(|data| match data {
            Value::String(text) => Some(text.clone()),
            Value::Object(_) => extract_optional_string(data, &["text", "content", "message"]),
            _ => None,
        })
    })
}

fn extract_error_text(event: &Value) -> Option<String> {
    extract_optional_string(event, &["error", "message", "text"]).or_else(|| {
        event.get("data").and_then(|data| match data {
            Value::String(text) => Some(text.clone()),
            Value::Object(_) => extract_optional_string(data, &["error", "message", "text"]),
            _ => None,
        })
    })
}

fn extract_optional_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|candidate| match candidate {
            Value::String(text) => Some(text.clone()),
            _ => None,
        })
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn test_driver() -> AcpxDriver {
        AcpxDriver::new_for_tests(
            "codex".to_string(),
            300,
            PathBuf::from("/tmp/worktree"),
            PathBuf::from("/usr/local/bin/acpx"),
        )
    }

    #[test]
    fn acpx_driver_parses_ndjson_text_tool_calls_and_errors() {
        let stdout = concat!(
            "{\"type\":\"text\",\"text\":\"Hello\"}\n",
            "{\"type\":\"tool_call\",\"id\":\"call-1\",\"name\":\"ReadMemory\",\"title\":\"Read memory\",\"status\":\"completed\",\"arguments\":{\"uri\":\"memory://foo\"}}\n",
            "{\"type\":\"text\",\"data\":{\"text\":\" world\"}}\n",
            "{\"type\":\"error\",\"message\":\"agent reported a warning\"}\n"
        );

        let response = parse_ndjson_response(stdout).expect("NDJSON should parse");

        assert_eq!(response.text, "Hello world");
        assert_eq!(response.error.as_deref(), Some("agent reported a warning"));
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(
            response.tool_calls[0],
            ToolCallRecord {
                id: Some("call-1".to_string()),
                name: Some("ReadMemory".to_string()),
                title: Some("Read memory".to_string()),
                status: Some("completed".to_string()),
                arguments: Some(json!({"uri": "memory://foo"})),
            }
        );
    }

    #[test]
    fn acpx_driver_builds_expected_prompt_command_args() {
        let driver = test_driver();
        let spec = driver.prompt_spec("workflow-42", "review changes");

        assert_eq!(
            spec.args,
            vec![
                "--format",
                "json",
                "--timeout",
                "300",
                "--approve-all",
                "codex",
                "-s",
                "workflow-42",
                "review changes",
            ]
        );
    }

    #[test]
    fn acpx_driver_builds_expected_session_and_cancel_commands() {
        let driver = test_driver();

        assert_eq!(
            driver.create_session_spec("workflow-42").args,
            vec!["codex", "sessions", "new", "--name", "workflow-42"]
        );
        assert_eq!(
            driver.close_session_spec("workflow-42").args,
            vec!["codex", "sessions", "close", "workflow-42"]
        );
        assert_eq!(
            driver.cancel_spec("workflow-42").args,
            vec!["codex", "cancel", "-s", "workflow-42"]
        );
    }

    #[test]
    fn acpx_driver_maps_timeout_failures() {
        let error = map_command_failure(Some(124), "", "request timed out after 300 seconds", 300);

        match error {
            AcpxError::Timeout {
                timeout_secs,
                details,
            } => {
                assert_eq!(timeout_secs, 300);
                assert!(details.contains("timed out"));
            }
            other => panic!("expected timeout error, got {other:?}"),
        }
    }

    #[test]
    fn acpx_driver_maps_non_zero_exit_codes() {
        let error = map_command_failure(Some(2), "", "permission denied", 300);

        match error {
            AcpxError::CommandFailed { code, details } => {
                assert_eq!(code, Some(2));
                assert!(details.contains("permission denied"));
            }
            other => panic!("expected command failure, got {other:?}"),
        }
    }

    #[test]
    fn acpx_driver_rejects_invalid_ndjson_lines() {
        let error = parse_ndjson_response("not-json\n").expect_err("invalid NDJSON must fail");
        assert!(matches!(error, AcpxError::InvalidEvent(_)));
    }

    #[test]
    fn acpx_driver_resolves_binary_from_path() {
        let temp = tempdir().expect("tempdir");
        let acpx_path = temp.path().join("acpx");
        std::fs::write(&acpx_path, "#!/bin/sh\nexit 0\n").expect("write acpx stub");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&acpx_path)
                .expect("metadata")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&acpx_path, permissions).expect("chmod");
        }

        let original_path = std::env::var_os("PATH");
        let combined_path = match &original_path {
            Some(existing) => {
                let mut paths = vec![temp.path().to_path_buf()];
                paths.extend(std::env::split_paths(existing));
                std::env::join_paths(paths).expect("join PATH")
            }
            None => std::env::join_paths([temp.path()]).expect("join PATH"),
        };
        // SAFETY: this unit test mutates PATH in a narrow scope and restores it before returning.
        unsafe {
            std::env::set_var("PATH", &combined_path);
        }
        let resolved = resolve_acpx_path().expect("acpx should resolve");
        match original_path {
            Some(path) => {
                // SAFETY: this restores PATH to its original value before the test exits.
                unsafe {
                    std::env::set_var("PATH", path);
                }
            }
            None => {
                // SAFETY: this restores the absence of PATH before the test exits.
                unsafe {
                    std::env::remove_var("PATH");
                }
            }
        }

        assert_eq!(resolved, acpx_path);
    }
}
