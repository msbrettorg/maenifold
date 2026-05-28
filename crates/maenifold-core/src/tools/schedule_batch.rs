use super::schedule_os::{is_not_installed_scheduler_error, sync_os_scheduler};
use super::{ToolContext, render, resolve_maenifold_bin, schedule_store};
use crate::error::{MaenifoldError, Result};
#[cfg(all(test, target_os = "macos"))]
use crate::schedule::LaunchctlBackend;
use crate::schedule::{ScheduledTask, pause_state};
use chrono::Utc;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
#[cfg(all(test, target_os = "macos"))]
use std::path::PathBuf;
#[cfg(all(test, target_os = "macos"))]
use std::sync::Arc;

#[derive(Debug)]
struct Failure {
    id: String,
    error: String,
    phase: &'static str,
}

impl Failure {
    fn json(&self) -> Value {
        json!({"id": self.id, "error": self.error, "phase": self.phase})
    }
}

pub(super) fn pause_all(context: &ToolContext, _payload: &Value) -> Result<String> {
    pause_all_with_bin(context, &resolve_maenifold_bin()?)
}

pub(super) fn resume_all(context: &ToolContext, _payload: &Value) -> Result<String> {
    resume_all_with_bin(context, &resolve_maenifold_bin()?)
}

pub(super) fn set_all_enabled(context: &ToolContext, payload: &Value) -> Result<String> {
    let bin = resolve_maenifold_bin()?;
    set_all_enabled_with_scheduler(context, payload, |task| sync_for_batch(task, &bin))
}

#[cfg(all(test, target_os = "macos"))]
pub(in crate::tools) fn set_all_enabled_for_test(
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<String> {
    set_all_enabled_with_scheduler(context, payload, |task| {
        sync_for_batch_for_test(task, bin, agents_dir.clone(), backend.clone())
    })
}

fn set_all_enabled_with_scheduler<SyncFn>(
    context: &ToolContext,
    payload: &Value,
    sync_scheduler: SyncFn,
) -> Result<String>
where
    SyncFn: Fn(&ScheduledTask) -> Result<()>,
{
    if pause_state::is_paused(&context.config.maenifold_root)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
    {
        return Err(MaenifoldError::InvalidArgument(
            "Cannot bulk-toggle schedules while pause-all is active. Use Schedule action=resume_all to clear pause state first.".to_string(),
        ));
    }
    let enabled = payload
        .get("enabled")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument("'enabled' must be a boolean".to_string())
        })?;
    let store = schedule_store(context);
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();
    let mut failed = Vec::new();
    for mut task in store
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
    {
        if task.enabled == enabled {
            unchanged.push(task.id);
            continue;
        }
        task.enabled = enabled;
        match store.update(&task) {
            Ok(()) => match sync_scheduler(&task) {
                Ok(()) => changed.push(task.id),
                Err(error) => failed.push(Failure {
                    id: task.id,
                    error: error.to_string(),
                    phase: "os_sync",
                }),
            },
            Err(error) => failed.push(Failure {
                id: task.id,
                error: error.to_string(),
                phase: "store_update",
            }),
        }
    }
    Ok(batch_response(
        context,
        "set_all_enabled",
        (changed, unchanged, Vec::new()),
        failed,
        format!("Schedules set to enabled={enabled}."),
    ))
}

pub(super) fn pause_status(context: &ToolContext) -> Result<String> {
    let state = pause_state::read(&context.config.maenifold_root)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let payload = match state {
        Some(state) => json!({
            "action": "pause_status",
            "paused": true,
            "pausedAt": state.paused_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "schedulesPaused": state.prior_enabled.len(),
            "ok": true,
        }),
        None => json!({"action": "pause_status", "paused": false, "ok": true}),
    };
    Ok(render(context.output_format, payload))
}

pub(in crate::tools) fn pause_all_with_bin(context: &ToolContext, bin: &Path) -> Result<String> {
    pause_all_with_scheduler(context, |task| sync_for_batch(task, bin))
}

#[cfg(all(test, target_os = "macos"))]
pub(in crate::tools) fn pause_all_for_test(
    context: &ToolContext,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<String> {
    pause_all_with_scheduler(context, |task| {
        sync_for_batch_for_test(task, bin, agents_dir.clone(), backend.clone())
    })
}

fn pause_all_with_scheduler<SyncFn>(context: &ToolContext, sync_scheduler: SyncFn) -> Result<String>
where
    SyncFn: Fn(&ScheduledTask) -> Result<()>,
{
    let store = schedule_store(context);
    let tasks = store
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    let prior_enabled = tasks
        .iter()
        .map(|task| (task.id.clone(), task.enabled))
        .collect::<BTreeMap<_, _>>();
    let state = pause_state::PauseState {
        paused_at: Utc::now(),
        prior_enabled,
    };
    match pause_state::try_create(&context.config.maenifold_root, &state)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
    {
        pause_state::CreateOutcome::AlreadyExists(existing) => {
            return Ok(render(
                context.output_format,
                json!({
                    "action": "pause_all",
                    "ok": true,
                    "alreadyPaused": true,
                    "changedSchedules": [],
                    "unchangedSchedules": existing.prior_enabled.keys().cloned().collect::<Vec<_>>(),
                    "missingSchedules": [],
                    "failed": [],
                    "message": "Schedules already paused.",
                }),
            ));
        }
        pause_state::CreateOutcome::Created => {}
    }
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();
    let mut failed = Vec::new();
    for task in tasks {
        if task.enabled {
            let mut paused = task.clone();
            paused.enabled = false;
            match store.update(&paused) {
                Ok(()) => match sync_scheduler(&paused) {
                    Ok(()) => changed.push(task.id),
                    Err(error) => {
                        let _ = store.restore_snapshot_if_current(&task, &paused);
                        failed.push(Failure {
                            id: task.id,
                            error: error.to_string(),
                            phase: "os_sync",
                        });
                    }
                },
                Err(error) => failed.push(Failure {
                    id: task.id,
                    error: error.to_string(),
                    phase: "store_update",
                }),
            }
        } else {
            match sync_scheduler(&task) {
                Ok(()) => unchanged.push(task.id),
                Err(error) => failed.push(Failure {
                    id: task.id,
                    error: error.to_string(),
                    phase: "os_sync",
                }),
            }
        }
    }
    Ok(batch_response(
        context,
        "pause_all",
        (changed, unchanged, Vec::new()),
        failed,
        "Schedules paused.".to_string(),
    ))
}

pub(in crate::tools) fn resume_all_with_bin(context: &ToolContext, bin: &Path) -> Result<String> {
    resume_all_with_scheduler(context, |task| sync_for_batch(task, bin))
}

#[cfg(all(test, target_os = "macos"))]
pub(in crate::tools) fn resume_all_for_test(
    context: &ToolContext,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<String> {
    resume_all_with_scheduler(context, |task| {
        sync_for_batch_for_test(task, bin, agents_dir.clone(), backend.clone())
    })
}

fn resume_all_with_scheduler<SyncFn>(
    context: &ToolContext,
    sync_scheduler: SyncFn,
) -> Result<String>
where
    SyncFn: Fn(&ScheduledTask) -> Result<()>,
{
    let Some(state) = pause_state::read(&context.config.maenifold_root)
        .map_err(|error| MaenifoldError::Message(error.to_string()))?
    else {
        return Ok(render(
            context.output_format,
            json!({"action":"resume_all","ok":true,"changedSchedules":[],"unchangedSchedules":[],"missingSchedules":[],"failed":[],"message":"Schedules were not paused."}),
        ));
    };
    let store = schedule_store(context);
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();
    let mut missing = Vec::new();
    let mut failed = Vec::new();
    for (id, prior_enabled) in state.prior_enabled {
        let mut task = match store.read(&id) {
            Ok(task) => task,
            Err(_) => {
                missing.push(id);
                continue;
            }
        };
        task.enabled = prior_enabled;
        match store.update(&task) {
            Ok(()) => match sync_scheduler(&task) {
                Ok(()) if prior_enabled => changed.push(id),
                Ok(()) => unchanged.push(id),
                Err(error) => failed.push(Failure {
                    id,
                    error: error.to_string(),
                    phase: "os_sync",
                }),
            },
            Err(error) => failed.push(Failure {
                id,
                error: error.to_string(),
                phase: "store_update",
            }),
        }
    }
    if failed.is_empty() {
        pause_state::clear(&context.config.maenifold_root)
            .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    }
    Ok(batch_response(
        context,
        "resume_all",
        (changed, unchanged, missing),
        failed,
        "Schedules resumed.".to_string(),
    ))
}

fn sync_for_batch(task: &ScheduledTask, bin: &Path) -> Result<()> {
    match sync_os_scheduler(task, bin) {
        Ok(()) => Ok(()),
        Err(error) if !task.enabled && is_absent_uninstall_error(&error) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(all(test, target_os = "macos"))]
fn sync_for_batch_for_test(
    task: &ScheduledTask,
    bin: &Path,
    agents_dir: PathBuf,
    backend: Arc<dyn LaunchctlBackend>,
) -> Result<()> {
    match super::schedule_os::sync_os_scheduler_for_test(task, bin, agents_dir, backend) {
        Ok(()) => Ok(()),
        Err(error) if !task.enabled && is_absent_uninstall_error(&error) => Ok(()),
        Err(error) => Err(error),
    }
}

fn is_absent_uninstall_error(error: &MaenifoldError) -> bool {
    is_not_installed_scheduler_error(error)
        || error
            .to_string()
            .to_ascii_lowercase()
            .contains("not loaded")
}

fn batch_response(
    context: &ToolContext,
    action: &'static str,
    schedules: (Vec<String>, Vec<String>, Vec<String>),
    failed: Vec<Failure>,
    message: String,
) -> String {
    let (changed, unchanged, missing) = schedules;
    render(
        context.output_format,
        json!({
            "action": action,
            "ok": failed.is_empty(),
            "changedSchedules": changed,
            "unchangedSchedules": unchanged,
            "missingSchedules": missing,
            "failed": failed.iter().map(Failure::json).collect::<Vec<_>>(),
            "message": message,
        }),
    )
}
