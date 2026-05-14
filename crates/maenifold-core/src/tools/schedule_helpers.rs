use super::ToolContext;
use crate::error::{MaenifoldError, Result};
use crate::schedule::{
    ScheduleStore, ScheduledTask, env_allowlist::default_allowlist, pause_state,
};
use serde_json::{Value, json};
use std::path::PathBuf;

pub(super) fn schedule_store(context: &ToolContext) -> ScheduleStore {
    ScheduleStore::new(context.config.maenifold_root.join("schedules"))
}

pub(super) fn reject_enabled_while_paused(context: &ToolContext, enabled: bool) -> Result<()> {
    if enabled
        && pause_state::is_paused(&context.config.maenifold_root)
            .map_err(|error| MaenifoldError::Message(error.to_string()))?
    {
        return Err(MaenifoldError::InvalidArgument(
            "Cannot enable schedule while pause-all is active. Resume schedules first (Schedule action=resume_all) or save with enabled=false.".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn task_value(task: &ScheduledTask) -> Value {
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
        "mcp_action": task.mcp_action,
        "mcp_parameters": task.mcp_parameters,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
        "last_run": task.last_run,
    })
}

pub(super) fn env_allowlist_view() -> Value {
    let allowlist = default_allowlist();
    let mut exact = allowlist.exact.into_iter().collect::<Vec<_>>();
    exact.sort();
    let mut prefixes = allowlist.prefixes;
    prefixes.sort();
    json!({"enabled":true,"allowlistExact":exact,"allowlistPrefixes":prefixes})
}

pub(super) fn resolve_maenifold_bin() -> Result<PathBuf> {
    std::env::current_exe().map_err(|error| {
        MaenifoldError::Message(format!("failed to resolve current executable: {error}"))
    })
}

pub(super) fn optional_string(payload: &Value, key: &'static str) -> Result<Option<String>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_str()
        .map(|text| Some(text.to_string()))
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a string")))
}

pub(super) fn optional_value(payload: &Value, key: &'static str) -> Result<Option<Value>> {
    Ok(payload.get(key).cloned())
}

pub(super) fn validate_mcp_action(action: Option<&str>, parameters: Option<&Value>) -> Result<()> {
    let Some(action) = action else {
        return Ok(());
    };
    let Some((domain, command)) = action.rsplit_once('.') else {
        return Err(MaenifoldError::InvalidArgument(
            "mcp_action must be in domain.command form (<domain>.<command>, e.g. maenifold.gate.gate_Status)"
                .into(),
        ));
    };
    if domain.is_empty() || command.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "mcp_action domain and command must both be non-empty".into(),
        ));
    }
    if let Some(params) = parameters {
        if !params.is_object() {
            return Err(MaenifoldError::InvalidArgument(
                "mcp_parameters must be a JSON object (or omitted)".into(),
            ));
        }
    }
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
        )));
    }
    Ok(())
}

pub(super) fn optional_bool(payload: &Value, key: &'static str) -> Result<Option<bool>> {
    let Some(value) = payload.get(key) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("'{key}' must be a boolean")))
}

pub(super) fn optional_path_buf(payload: &Value, key: &'static str) -> Result<Option<PathBuf>> {
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

pub(super) fn optional_u32(payload: &Value, key: &'static str) -> Result<Option<u32>> {
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

pub(super) fn optional_string_array(
    payload: &Value,
    key: &'static str,
) -> Result<Option<Vec<String>>> {
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
