use super::{
    ToolContext, env_allowlist_view, optional_bool, optional_path_buf, optional_string,
    optional_string_array, optional_u32, optional_value, reject_enabled_while_paused, render,
    required_string, resolve_maenifold_bin, schedule_store, task_value,
    validate_and_parse_schedule, validate_mcp_action,
};
use crate::error::{MaenifoldError, Result};
#[cfg(all(test, target_os = "macos"))]
use crate::schedule::LaunchctlBackend;
use crate::schedule::ScheduledTask;
use chrono::Utc;
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
#[cfg(all(test, target_os = "macos"))]
use std::path::PathBuf;
#[cfg(all(test, target_os = "macos"))]
use std::sync::Arc;
use tracing::warn;

pub(super) fn create_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let bin = resolve_maenifold_bin()?;
    create_task_with_bin(context, payload, &bin)
}

pub(super) fn create_task_with_bin(
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
) -> Result<String> {
    create_task_with_scheduler(
        context,
        payload,
        |task| super::schedule_os::sync_os_scheduler(task, bin),
        |task_id| super::schedule_os::uninstall_os_scheduler(task_id, bin),
    )
}

#[cfg(all(test, target_os = "macos"))]
pub(super) fn create_task_with_bin_for_test(
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<String> {
    create_task_with_scheduler(
        context,
        payload,
        |task| {
            super::schedule_os::sync_os_scheduler_for_test(
                task,
                bin,
                agents_dir.clone(),
                backend.clone(),
            )
        },
        |task_id| {
            super::schedule_os::uninstall_os_scheduler_for_test(
                task_id,
                bin,
                agents_dir.clone(),
                backend.clone(),
            )
        },
    )
}

#[cfg(all(test, unix, not(target_os = "macos")))]
pub(super) fn create_task_with_bin_for_test(
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
) -> Result<String> {
    create_task_with_bin(context, payload, bin)
}

fn create_task_with_scheduler<SyncFn, UninstallFn>(
    context: &ToolContext,
    payload: &Value,
    sync_scheduler: SyncFn,
    uninstall_scheduler: UninstallFn,
) -> Result<String>
where
    SyncFn: Fn(&ScheduledTask) -> Result<()>,
    UninstallFn: Fn(&str) -> Result<()>,
{
    let id = required_string(payload, "id")?;
    let parsed = validate_and_parse_schedule(required_string(payload, "schedule")?)?;
    let task = ScheduledTask {
        id: id.to_string(),
        name: optional_string(payload, "description")?.unwrap_or_else(|| id.to_string()),
        schedule: parsed.cron,
        schedule_human: Some(parsed.description),
        agent: required_string(payload, "agent")?.to_string(),
        prompt: required_string(payload, "prompt")?.to_string(),
        workdir: optional_path_buf(payload, "workdir")?,
        auto_grant_permissions: optional_bool(payload, "auto_grant_permissions")?.unwrap_or(false),
        allowed_tools: optional_string_array(payload, "allowed_tools")?.unwrap_or_default(),
        mcp_action: optional_string(payload, "mcp_action")?,
        mcp_parameters: optional_value(payload, "mcp_parameters")?,
        timeout_seconds: optional_u32(payload, "timeout_seconds")?.unwrap_or(300),
        enabled: optional_bool(payload, "enabled")?.unwrap_or(true),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    };
    let store = schedule_store(context);
    if store.exists(id) {
        return Err(MaenifoldError::Message(format!(
            "scheduled task '{id}' already exists"
        )));
    }
    validate_mcp_action(task.mcp_action.as_deref(), task.mcp_parameters.as_ref())?;
    reject_enabled_while_paused(context, task.enabled)?;
    if task.enabled {
        sync_scheduler(&task)?;
    }
    if let Err(error) = store.create(&task) {
        if task.enabled
            && let Err(rollback_error) = uninstall_scheduler(&task.id)
        {
            return Err(MaenifoldError::Message(format!(
                "failed to persist scheduled task '{}' after OS scheduler install; rollback uninstall also failed: {}; original store error: {}",
                task.id, rollback_error, error
            )));
        }
        return Err(MaenifoldError::Message(error.to_string()));
    }
    Ok(render(
        context.output_format,
        json!({"action":"create","task":task_value(&task),"envScrub":env_allowlist_view(),"message":format!("Scheduled task '{}' created.", id)}),
    ))
}

pub(super) fn update_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let bin = resolve_maenifold_bin()?;
    update_task_with_scheduler(context, payload, |task| {
        super::schedule_os::sync_os_scheduler(task, &bin)
    })
}

#[cfg(all(test, target_os = "macos"))]
pub(super) fn update_task_for_test(
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<String> {
    update_task_with_scheduler(context, payload, |task| {
        super::schedule_os::sync_os_scheduler_for_test(
            task,
            bin,
            agents_dir.clone(),
            backend.clone(),
        )
    })
}

fn update_task_with_scheduler<SyncFn>(
    context: &ToolContext,
    payload: &Value,
    sync_scheduler: SyncFn,
) -> Result<String>
where
    SyncFn: Fn(&ScheduledTask) -> Result<()>,
{
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    let original = store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let mut task = original.clone();
    if let Some(schedule_input) = optional_string(payload, "schedule")? {
        let parsed = validate_and_parse_schedule(&schedule_input)?;
        task.schedule = parsed.cron;
        task.schedule_human = Some(parsed.description);
    }
    if let Some(agent) = optional_string(payload, "agent")? {
        task.agent = agent;
    }
    if let Some(prompt) = optional_string(payload, "prompt")? {
        task.prompt = prompt;
    }
    if payload.get("workdir").is_some() {
        task.workdir = optional_path_buf(payload, "workdir")?;
    }
    if let Some(timeout_seconds) = optional_u32(payload, "timeout_seconds")? {
        task.timeout_seconds = timeout_seconds;
    }
    if let Some(enabled) = optional_bool(payload, "enabled")? {
        task.enabled = enabled;
    }
    if let Some(auto_grant_permissions) = optional_bool(payload, "auto_grant_permissions")? {
        task.auto_grant_permissions = auto_grant_permissions;
    }
    if let Some(allowed_tools) = optional_string_array(payload, "allowed_tools")? {
        task.allowed_tools = allowed_tools;
    }
    if payload.get("mcp_action").is_some() {
        if payload.get("mcp_action").is_some_and(Value::is_null) {
            task.mcp_action = None;
            task.mcp_parameters = None;
        } else {
            task.mcp_action = optional_string(payload, "mcp_action")?;
        }
    }
    if payload.get("mcp_parameters").is_some() {
        task.mcp_parameters = match optional_value(payload, "mcp_parameters")? {
            Some(value) if value.is_null() => None,
            value => value,
        };
    }
    if let Some(description) = optional_string(payload, "description")? {
        task.name = description;
    }
    validate_mcp_action(task.mcp_action.as_deref(), task.mcp_parameters.as_ref())?;
    reject_enabled_while_paused(context, task.enabled)?;
    store
        .update(&task)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let task = store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if let Err(error) = sync_scheduler(&task) {
        // CAS-style rollback: restore only if the file still contains the failed update.
        // If another writer changed the task after our failed OS sync, avoid clobbering it.
        if let Err(rollback_error) = store.restore_snapshot_if_current(&original, &task) {
            warn!(
                task_id = id,
                error = %rollback_error,
                "failed to restore schedule config after OS scheduler update failed"
            );
        }
        return Err(error);
    }
    Ok(render(
        context.output_format,
        json!({"action":"update","task":task_value(&task),"envScrub":env_allowlist_view(),"message":format!("Scheduled task '{}' updated.", id)}),
    ))
}

pub(super) fn delete_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    match resolve_maenifold_bin()
        .and_then(|bin| super::schedule_os::uninstall_os_scheduler(id, &bin))
    {
        Ok(()) => {}
        Err(error) if super::schedule_os::is_not_installed_scheduler_error(&error) => {}
        Err(error) => {
            return Err(MaenifoldError::Message(format!(
                "failed to uninstall OS scheduler entry for '{id}'; schedule config was not deleted to avoid orphaning scheduler state: {error}"
            )));
        }
    };
    store
        .delete(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let message = format!("Scheduled task '{id}' deleted.");
    Ok(render(
        context.output_format,
        json!({"action":"delete","id":id,"message":message}),
    ))
}

pub(super) fn list_tasks(context: &ToolContext) -> Result<String> {
    let tasks = schedule_store(context)
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .into_iter()
        .map(|task| task_value(&task))
        .collect::<Vec<_>>();
    Ok(render(context.output_format, json!(tasks)))
}

pub(super) fn get_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let task = schedule_store(context)
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(render(context.output_format, task_value(&task)))
}

pub(super) fn run_now(context: &ToolContext, payload: &Value) -> Result<String> {
    let bin = resolve_maenifold_bin()?;
    run_now_with_bin(context, payload, &bin)
}

pub(super) fn run_now_with_bin(context: &ToolContext, payload: &Value, bin: &Path) -> Result<String> {
    let id = required_string(payload, "id")?;
    schedule_store(context)
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;

    let mut command = Command::new(bin);
    command
        .args(["schedule", "run", id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let child = command.spawn().map_err(|error| {
        MaenifoldError::Message(format!("failed to spawn detached scheduled run '{id}': {error}"))
    })?;
    let pid = child.id();
    // Reap the zombie when the child exits. setsid detaches the session/process
    // group but does NOT reparent the child while the daemon is alive, so we
    // must wait() somewhere. A background reaper thread takes ownership and
    // joins itself when the child exits, never blocking the daemon.
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });

    let started_at = Utc::now();
    Ok(render(
        context.output_format,
        json!({
            "action": "run_now",
            "id": id,
            "pid": pid,
            "started_at": started_at,
            "message": format!("Scheduled task '{}' spawned (pid {}).", id, pid)
        }),
    ))
}
