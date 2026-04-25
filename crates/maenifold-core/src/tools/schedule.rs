use super::context::{ToolContext, render, required_string};
use crate::error::{MaenifoldError, Result};
use crate::schedule::{ScheduleStore, ScheduledTask, Supervisor, parse_schedule};
#[cfg(target_os = "macos")]
use crate::schedule::LaunchdManager;
#[cfg(target_os = "linux")]
use crate::schedule::SystemdManager;
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub fn schedule(context: &ToolContext, payload: &Value) -> Result<String> {
    let action = required_string(payload, "action")?;
    match action {
        "create" => create_task(context, payload),
        "update" => update_task(context, payload),
        "delete" => delete_task(context, payload),
        "list" => list_tasks(context),
        "get" => get_task(context, payload),
        "run_now" => run_now(context, payload),
        "logs" => get_logs(context, payload),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown action '{}'. Available: create, update, delete, list, get, run_now, logs",
            action
        ))),
    }
}

fn create_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let parsed = parse_schedule(required_string(payload, "schedule")?)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let task = ScheduledTask {
        id: id.to_string(),
        name: optional_string(payload, "description")?.unwrap_or_else(|| id.to_string()),
        schedule: parsed.cron,
        schedule_human: Some(parsed.description),
        agent: required_string(payload, "agent")?.to_string(),
        prompt: required_string(payload, "prompt")?.to_string(),
        workdir: None,
        timeout_seconds: 300,
        skip_if_running: true,
        enabled: optional_bool(payload, "enabled")?.unwrap_or(true),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    };
    let store = schedule_store(context);
    store
        .create(&task)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    sync_os_scheduler(&task, &resolve_maenifold_bin()?)?;
    Ok(render(
        context.output_format,
        json!({"action":"create","task":task_value(&task),"message":format!("Scheduled task '{}' created.", id)}),
    ))
}

fn update_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    let mut task = store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if let Some(schedule_input) = optional_string(payload, "schedule")? {
        let parsed = parse_schedule(&schedule_input)
            .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
        task.schedule = parsed.cron;
        task.schedule_human = Some(parsed.description);
    }
    if let Some(agent) = optional_string(payload, "agent")? {
        task.agent = agent;
    }
    if let Some(prompt) = optional_string(payload, "prompt")? {
        task.prompt = prompt;
    }
    if let Some(enabled) = optional_bool(payload, "enabled")? {
        task.enabled = enabled;
    }
    if let Some(description) = optional_string(payload, "description")? {
        task.name = description;
    }
    store
        .update(&task)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    sync_os_scheduler(&task, &resolve_maenifold_bin()?)?;
    Ok(render(
        context.output_format,
        json!({"action":"update","task":task_value(&task),"message":format!("Scheduled task '{}' updated.", id)}),
    ))
}

fn delete_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    uninstall_os_scheduler(id, &resolve_maenifold_bin()?)?;
    store
        .delete(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(render(
        context.output_format,
        json!({"action":"delete","id":id,"message":format!("Scheduled task '{}' deleted.", id)}),
    ))
}

fn list_tasks(context: &ToolContext) -> Result<String> {
    let tasks = schedule_store(context)
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .into_iter()
        .map(|task| task_value(&task))
        .collect::<Vec<_>>();
    Ok(render(context.output_format, json!(tasks)))
}

fn get_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let task = schedule_store(context)
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(render(context.output_format, task_value(&task)))
}

fn run_now(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let supervisor = Supervisor::new(schedule_store(context), resolve_maenifold_bin()?);
    let run = supervisor
        .run(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    Ok(render(
        context.output_format,
        json!({"action":"run_now","id":id,"run":run,"message":format!("Scheduled task '{}' executed.", id)}),
    ))
}

fn get_logs(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let limit = optional_u64(payload, "limit")?.unwrap_or(10) as usize;
    let store = schedule_store(context);
    store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let logs = read_recent_logs(&store.task_dir(id).join("logs"), limit)?;
    Ok(render(context.output_format, json!(logs)))
}

fn schedule_store(context: &ToolContext) -> ScheduleStore {
    ScheduleStore::new(context.config.maenifold_root.join("schedules"))
}

fn task_value(task: &ScheduledTask) -> Value {
    json!({
        "id": task.id,
        "description": task.name,
        "schedule": task.schedule,
        "schedule_human": task.schedule_human,
        "agent": task.agent,
        "prompt": task.prompt,
        "enabled": task.enabled,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
        "last_run": task.last_run,
    })
}

fn resolve_maenifold_bin() -> Result<PathBuf> {
    std::env::current_exe().map_err(|error| {
        MaenifoldError::Message(format!("failed to resolve current executable: {error}"))
    })
}

#[cfg(target_os = "macos")]
fn sync_os_scheduler(task: &ScheduledTask, bin: &Path) -> Result<()> {
    let manager = LaunchdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if task.enabled {
        manager
            .install(task)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    } else {
        manager
            .uninstall(&task.id)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    }
}

#[cfg(target_os = "linux")]
fn sync_os_scheduler(task: &ScheduledTask, bin: &Path) -> Result<()> {
    let manager = SystemdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if task.enabled {
        manager
            .install(task)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    } else {
        manager
            .uninstall(&task.id)
            .map_err(|error| MaenifoldError::Message(error.to_string()))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn sync_os_scheduler(_task: &ScheduledTask, _bin: &Path) -> Result<()> {
    Err(MaenifoldError::Message(
        "schedule tools are only supported on macOS and Linux".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn uninstall_os_scheduler(task_id: &str, bin: &Path) -> Result<()> {
    LaunchdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .uninstall(task_id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))
}

#[cfg(target_os = "linux")]
fn uninstall_os_scheduler(task_id: &str, bin: &Path) -> Result<()> {
    SystemdManager::new(bin.to_path_buf())
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
        .uninstall(task_id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn uninstall_os_scheduler(_task_id: &str, _bin: &Path) -> Result<()> {
    Err(MaenifoldError::Message(
        "schedule tools are only supported on macOS and Linux".to_string(),
    ))
}

fn read_recent_logs(log_dir: &Path, limit: usize) -> Result<Vec<Value>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(log_dir)?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("log"))
        .collect::<Vec<_>>();
    entries.sort();
    entries.reverse();
    entries
        .into_iter()
        .take(limit)
        .map(|path| {
            let content = fs::read(&path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())?;
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| MaenifoldError::NonUtf8Path(path.clone()))?;
            Ok(json!({"fileName":file_name,"content":content}))
        })
        .collect()
}

fn optional_string(payload: &Value, key: &'static str) -> Result<Option<String>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|text| Some(text.to_string()))
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a string")))
}

fn optional_bool(payload: &Value, key: &'static str) -> Result<Option<bool>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a boolean")))
}

fn optional_u64(payload: &Value, key: &'static str) -> Result<Option<u64>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_u64()
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an integer")))
}
