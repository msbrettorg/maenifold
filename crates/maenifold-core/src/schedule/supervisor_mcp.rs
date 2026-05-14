use super::build_payload;
use crate::error::MaenifoldError;
use crate::schedule::ScheduledTask;
use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Child;
use std::thread::{self, JoinHandle};
use tracing::warn;

pub(super) struct Invocation {
    pub(super) tool_name: String,
    pub(super) payload: String,
    pub(super) args: Vec<String>,
}

pub(super) struct ExecutionOutcome {
    pub(super) status: crate::schedule::RunStatus,
    pub(super) error: Option<String>,
    pub(super) stdout: String,
    pub(super) sidecar_path: std::path::PathBuf,
}

pub(super) fn write_runrecord_sidecar(record: &crate::schedule::RunRecord, path: &Path) {
    let result = serde_json::to_vec_pretty(record)
        .map_err(std::io::Error::other)
        .and_then(|payload| std::fs::write(path, payload));
    if let Err(error) = result {
        warn!(path = %path.display(), error = %error, "failed to write schedule run sidecar");
    }
}

pub(super) fn capture_stdout(
    child: &mut Child,
    mut log_file: File,
) -> Option<JoinHandle<std::io::Result<String>>> {
    child.stdout.take().map(|mut stdout| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes)?;
            log_file.write_all(&bytes)?;
            log_file.flush()?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        })
    })
}

pub(super) fn join_stdout(
    handle: Option<JoinHandle<std::io::Result<String>>>,
) -> Result<String> {
    match handle {
        Some(handle) => handle
            .join()
            .map_err(|_| MaenifoldError::InvalidArgument("stdout capture thread panicked".into()))?
            .context("failed to capture scheduled task stdout"),
        None => Ok(String::new()),
    }
}

pub(super) fn build_invocation(task: &ScheduledTask, workdir: Option<&Path>) -> Result<Invocation> {
    // Dual-path dispatch semantics:
    // - If task.mcp_action is set, fire maenifold tool GatewayExecute with the
    //   explicit allowlisted domain/command and JSON-object parameters.
    // - Otherwise, preserve the existing AcpExec prompt path. The two modes are
    //   mutually exclusive at fire time; mcp_action makes prompt ignored.
    let Some(action) = task.mcp_action.as_deref() else {
        let payload = build_payload(task, workdir)?;
        return Ok(Invocation {
            tool_name: "AcpExec".to_string(),
            args: vec![
                "tool".to_string(),
                "AcpExec".to_string(),
                payload.clone(),
                "--json".to_string(),
            ],
            payload,
        });
    };
    let (domain, command) = parse_mcp_action(action)?;
    let allowlist = crate::schedule::mcp_action_allowlist::default_allowlist();
    if !crate::schedule::mcp_action_allowlist::is_allowed(&allowlist, domain, command) {
        let allowed = allowlist
            .iter()
            .map(|(d, c)| format!("{}.{}", d, c))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(MaenifoldError::InvalidArgument(format!(
            "mcp_action '{}' is not on the schedule allowlist. Inference-capable commands \
             (AcpExec, GatewayIntent, Workflow, Pipeline, SequentialThinking, semantic SearchMemories) \
             cannot be scheduled — they must run under the desktop Sessions/ACP inspector. \
             Allowed actions: {}",
            action, allowed
        ))
        .into());
    }
    let parameters: Map<String, Value> = task
        .mcp_parameters
        .as_ref()
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let payload = json!({"domain": domain, "command": command, "parameters": parameters});
    let payload =
        serde_json::to_string(&payload).context("failed to serialize GatewayExecute payload")?;
    Ok(Invocation {
        tool_name: "GatewayExecute".to_string(),
        args: vec![
            "tool".to_string(),
            "GatewayExecute".to_string(),
            payload.clone(),
        ],
        payload,
    })
}

pub(super) fn parse_mcp_action(s: &str) -> std::result::Result<(&str, &str), MaenifoldError> {
    let (domain, command) = s.rsplit_once('.').ok_or_else(|| {
        MaenifoldError::InvalidArgument(
            "mcp_action must be in domain.command form (<domain>.<command>, e.g. maenifold.gate.gate_Status)"
                .into(),
        )
    })?;
    if domain.is_empty() || command.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "mcp_action domain and command must both be non-empty".into(),
        ));
    }
    Ok((domain, command))
}
