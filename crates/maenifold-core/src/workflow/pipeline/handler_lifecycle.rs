use crate::error::{MaenifoldError, Result};
use crate::markdown::write_markdown;
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use serde_json::{Map as JsonMap, Value, json};
use serde_yml::Value as YamlValue;
use std::process::Command;

use super::acp_reaper::{reap_acp_sessions, reaped_json};
use super::handler_wait::{await_terminal, parse_timeout_ms, wait_requested};
use super::lease;
use super::resource_notifications::emit_pipeline_resource_updated;
use super::session_io::{frontmatter_value, read_pipeline_session};
use super::session_write::persist_final_pipeline_status;
use super::session_write_files::persist_pipeline_runtime_context;
use super::{NodeState, PipelineDefinition};

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        let inputs = parse_inputs_payload(payload.get("inputs"))?;
        let started = run_pipeline(context, session_id, prompt, inputs.as_ref())?;
        if wait_requested(payload) {
            return await_terminal(context, session_id, parse_timeout_ms(payload)?).map(Some);
        }
        return Ok(Some(started));
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("forceComplete")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return force_complete_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}

fn find_driver_binary() -> Result<std::path::PathBuf> {
    // Allow explicit override for non-standard installs.
    if let Ok(path) = std::env::var("MAENIFOLD_PIPELINE_DRIVER") {
        let driver = std::path::PathBuf::from(path);
        if driver.exists() {
            return Ok(driver);
        }
        return Err(MaenifoldError::Message(format!(
            "MAENIFOLD_PIPELINE_DRIVER is set but binary not found at {}",
            driver.display()
        )));
    }

    let exe = std::env::current_exe().map_err(|error| {
        MaenifoldError::Message(format!("Cannot determine current executable path: {error}"))
    })?;
    let exe_dir = exe
        .canonicalize()
        .unwrap_or(exe)
        .parent()
        .ok_or_else(|| {
            MaenifoldError::Message("Cannot determine executable directory".to_string())
        })?
        .to_path_buf();
    let driver = exe_dir.join("maenifold-pipeline-driver");
    if driver.exists() {
        return Ok(driver);
    }
    Err(MaenifoldError::Message(format!(
        "Pipeline driver binary not found at {}. \
         Build it with: cargo build --release -p maenifold-pipeline-driver",
        driver.display()
    )))
}

fn run_pipeline(
    context: &ToolContext,
    session_id: &str,
    prompt: Option<&str>,
    inputs: Option<&JsonMap<String, Value>>,
) -> Result<String> {
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is already running. Use cancel to stop it, or wait for completion."
        )));
    }

    let prompt = persist_pipeline_runtime_context(context, session_id, prompt, inputs)?;
    let driver = find_driver_binary()?;
    let mut cmd = Command::new(&driver);
    cmd.arg(session_id);
    if !prompt.is_empty() {
        cmd.arg(prompt);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    cmd.spawn().map_err(|error| {
        MaenifoldError::Message(format!(
            "Failed to spawn pipeline driver at {}: {error}",
            driver.display()
        ))
    })?;

    Ok(format!("Pipeline {session_id} started"))
}

fn parse_inputs_payload(value: Option<&Value>) -> Result<Option<JsonMap<String, Value>>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => Ok(Some(map.clone())),
        Some(_) => Err(MaenifoldError::InvalidArgument(
            "inputs must be a JSON object".to_string(),
        )),
    }
}

fn cancel_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    let lock_path = lease::pipeline_lock_path(session_id, &context.config);
    if !lock_path.exists() {
        return cancel_orphan_pipeline(context, session_id, "no lock file");
    }
    if !lease::session_lock_is_held(session_id, &context.config) {
        return cancel_orphan_pipeline(context, session_id, "lock not held");
    }

    let holder = std::fs::read_to_string(&lock_path).map_err(|error| {
        MaenifoldError::Message(format!(
            "Cannot read lock file {}: {error}",
            lock_path.display()
        ))
    })?;

    // Lock file format: "{hostname}-{pid}"
    let pid_str = holder.rsplit('-').next().unwrap_or("");
    let pid: i32 = pid_str.parse().map_err(|_| {
        MaenifoldError::Message(format!(
            "Cannot parse PID from lock file content: {holder:?}"
        ))
    })?;

    if pid <= 0 {
        return Err(MaenifoldError::Message(format!(
            "Invalid PID {pid} in lock file (must be positive)"
        )));
    }

    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };
        if result != 0 {
            let errno = std::io::Error::last_os_error();
            return Err(MaenifoldError::Message(format!(
                "Failed to send SIGTERM to pipeline driver (PID {pid}): {errno}"
            )));
        }
    }

    #[cfg(not(unix))]
    {
        return Err(MaenifoldError::Message(
            "Pipeline cancel via signal is only supported on Unix".to_string(),
        ));
    }

    Ok(format!(
        "Pipeline {session_id} cancellation requested (SIGTERM sent to PID {pid})"
    ))
}

fn cancel_orphan_pipeline(context: &ToolContext, session_id: &str, reason: &str) -> Result<String> {
    let (uri, mut frontmatter, pipeline_content) = read_pipeline_session(context, session_id)?;
    let definition: PipelineDefinition = frontmatter_value(&frontmatter, "definition")?;
    let node_states: std::collections::HashMap<String, NodeState> =
        frontmatter_value(&frontmatter, "nodeStates")?;
    let reaped = reap_acp_sessions(&definition, &node_states);
    if frontmatter.get("status").and_then(YamlValue::as_str) != Some("running") {
        let result = json!({
            "cancelled": false,
            "reaped": reaped_json(reaped),
            "message": format!("Pipeline {session_id} is not running ({reason}); ACP cleanup performed"),
        });
        return Ok(match context.output_format {
            OutputFormat::Json => success(result),
            OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
        });
    }

    let cancelled_nodes = cancel_active_or_retrying_nodes(&mut frontmatter)?;
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("cancelled".to_string()),
    );
    write_markdown(&uri, Some(&frontmatter), &pipeline_content, &context.config)?;
    emit_pipeline_resource_updated(session_id);

    let result = json!({
        "cancelled": true,
        "cancelledOrphan": true,
        "cancelledNodes": cancelled_nodes,
        "reaped": reaped_json(reaped),
        "message": format!("Pipeline {session_id} cancelled ({reason}); ACP cleanup performed"),
    });
    Ok(match context.output_format {
        OutputFormat::Json => success(result),
        OutputFormat::Markdown => serde_json::to_string_pretty(&result)?,
    })
}

fn cancel_active_or_retrying_nodes(
    frontmatter: &mut crate::markdown::Frontmatter,
) -> Result<usize> {
    let Some(node_states) = frontmatter
        .get_mut("nodeStates")
        .and_then(YamlValue::as_mapping_mut)
    else {
        return Err(MaenifoldError::Message(
            "Pipeline session is missing nodeStates".to_string(),
        ));
    };
    let mut cancelled_nodes = 0;
    for node in node_states.values_mut() {
        let Some(node) = node.as_mapping_mut() else {
            continue;
        };
        let status_key = YamlValue::String("status".to_string());
        let status = node.get(&status_key).and_then(YamlValue::as_str);
        if matches!(status, Some("active" | "retrying")) {
            node.insert(status_key, YamlValue::String("cancelled".to_string()));
            cancelled_nodes += 1;
        }
    }
    Ok(cancelled_nodes)
}

/// Admin override: force a pipeline to `completed` status regardless of
/// individual node states.  Useful when all terminal nodes have succeeded
/// but an abandoned non-terminal node prevents automatic completion (e.g.
/// an upstream `always`-edge source that was bypassed).
///
/// Requires the pipeline to not be actively running (no live lock file).
fn force_complete_pipeline(context: &ToolContext, session_id: &str) -> Result<String> {
    if lease::session_lock_is_held(session_id, &context.config) {
        return Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is currently running; cancel it before force-completing"
        )));
    }

    persist_final_pipeline_status(context, session_id, "completed")?;

    Ok(format!("Pipeline {session_id} force-completed"))
}

#[cfg(test)]
#[path = "handler_lifecycle_tests.rs"]
mod tests;
