use super::context::{load_help_doc, render, required_string, ToolContext};
use super::schedule_logs::get_logs;
use crate::error::{MaenifoldError, Result};
use crate::schedule::{parse_schedule, ScheduleStore, ScheduledTask, Supervisor};
use crate::util::json::{success, OutputFormat};
use chrono::{SecondsFormat, Utc};
use croner::parser::CronParser;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tracing::warn;

#[path = "schedule_os.rs"]
mod schedule_os;
#[cfg(test)]
#[path = "schedule_tests.rs"]
mod schedule_tests;
use schedule_os::{is_not_installed_scheduler_error, sync_os_scheduler, uninstall_os_scheduler};

pub fn schedule(context: &ToolContext, payload: &Value) -> Result<String> {
    if payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let help = load_help_doc(&context.config.assets_path, "Schedule")
            .unwrap_or_else(|| "ERROR: Help file not found for Schedule".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let action = required_string(payload, "action")?;
    match action {
        "create" => create_task(context, payload),
        "update" => update_task(context, payload),
        "delete" => delete_task(context, payload),
        "list" => list_tasks(context),
        "get" => get_task(context, payload),
        "run_now" => run_now(context, payload),
        "logs" => get_logs(context, payload),
        "preview" => preview_schedule(context, payload),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown action '{}'. Available: create, update, delete, list, get, run_now, logs, preview",
            action
        ))),
    }
}

fn preview_schedule(context: &ToolContext, payload: &Value) -> Result<String> {
    let input = required_string(payload, "schedule")?;
    let parsed = parse_schedule(input)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let cron = CronParser::new()
        .parse(&parsed.cron)
        .map_err(|error| MaenifoldError::InvalidArgument(format!("invalid cron: {error}")))?;
    let now = Utc::now();
    let next_fires = cron
        .iter_after(now)
        .take(5)
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Secs, true))
        .collect::<Vec<_>>();

    Ok(render(
        context.output_format,
        json!({
            "action": "preview",
            "cron": parsed.cron,
            "description": parsed.description,
            "nextFires": next_fires,
            // Follow-up T-245.13: expose host IANA timezone when available without new deps.
            "timezone": "UTC",
        }),
    ))
}

fn create_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let bin = resolve_maenifold_bin()?;
    create_task_with_bin(context, payload, &bin)
}

fn create_task_with_bin(context: &ToolContext, payload: &Value, bin: &Path) -> Result<String> {
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
        workdir: optional_path_buf(payload, "workdir")?,
        auto_grant_permissions: optional_bool(payload, "auto_grant_permissions")?.unwrap_or(false),
        allowed_tools: optional_string_array(payload, "allowed_tools")?.unwrap_or_default(),
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
    if task.enabled {
        sync_os_scheduler(&task, bin)?;
    }
    if let Err(error) = store.create(&task) {
        if task.enabled {
            if let Err(rollback_error) = uninstall_os_scheduler(&task.id, bin) {
                return Err(MaenifoldError::Message(format!(
                    "failed to persist scheduled task '{}' after OS scheduler install; rollback uninstall also failed: {}; original store error: {}",
                    task.id, rollback_error, error
                )));
            }
        }
        return Err(MaenifoldError::Message(error.to_string()));
    }
    Ok(render(
        context.output_format,
        json!({"action":"create","task":task_value(&task),"message":format!("Scheduled task '{}' created.", id)}),
    ))
}

fn update_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    let original = store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let mut task = original.clone();
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
    if let Some(description) = optional_string(payload, "description")? {
        task.name = description;
    }
    store
        .update(&task)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let task = store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let bin = resolve_maenifold_bin()?;
    if let Err(error) = sync_os_scheduler(&task, &bin) {
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
        json!({"action":"update","task":task_value(&task),"message":format!("Scheduled task '{}' updated.", id)}),
    ))
}

fn delete_task(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let store = schedule_store(context);
    store
        .read(id)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    match resolve_maenifold_bin().and_then(|bin| uninstall_os_scheduler(id, &bin)) {
        Ok(()) => {}
        Err(error) if is_not_installed_scheduler_error(&error) => {}
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
        "auto_grant_permissions": task.auto_grant_permissions,
        "allowed_tools": task.allowed_tools,
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

fn optional_path_buf(payload: &Value, key: &'static str) -> Result<Option<PathBuf>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(PathBuf::from)
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a string")))
}

fn optional_u32(payload: &Value, key: &'static str) -> Result<Option<u32>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    let raw = value
        .as_u64()
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an integer")))?;
    u32::try_from(raw)
        .map(Some)
        .map_err(|_| MaenifoldError::InvalidArgument(format!("'{key}' must fit in u32")))
}

fn optional_string_array(payload: &Value, key: &'static str) -> Result<Option<Vec<String>>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be an array")))?;
    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_string).ok_or_else(|| {
                MaenifoldError::InvalidArgument(format!("'{key}' entries must be strings"))
            })
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}
