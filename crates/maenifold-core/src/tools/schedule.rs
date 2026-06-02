use super::context::{ToolContext, load_help_doc, render, required_string};
use super::schedule_logs::{clear_history, get_logs};
use crate::error::{MaenifoldError, Result};
#[cfg(test)]
use crate::schedule::ScheduledTask;
use crate::util::json::{OutputFormat, success};
#[cfg(test)]
use chrono::Utc;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(test)]
use std::path::PathBuf;

#[path = "schedule_backfill.rs"]
pub(super) mod schedule_backfill;
#[path = "schedule_batch.rs"]
pub(super) mod schedule_batch;
#[path = "schedule_crud.rs"]
pub(super) mod schedule_crud;
#[path = "schedule_helpers.rs"]
pub(super) mod schedule_helpers;
#[path = "schedule_os.rs"]
pub(super) mod schedule_os;
#[cfg(test)]
#[path = "schedule_tests.rs"]
mod schedule_tests;
#[cfg(test)]
#[path = "schedule_run_now_async_tests.rs"]
mod schedule_run_now_async_tests;
#[path = "schedule_validation.rs"]
pub(super) mod schedule_validation;

#[cfg(test)]
use schedule_crud::create_task_with_bin;
#[cfg(all(test, unix))]
use schedule_crud::create_task_with_bin_for_test;
#[cfg(test)]
#[cfg(target_os = "macos")]
use schedule_crud::update_task_for_test;
use schedule_crud::{create_task, delete_task, get_task, list_tasks, options_action, run_now, update_task};
use schedule_helpers::{
    env_allowlist_view, optional_bool, optional_path_buf, optional_string, optional_string_array,
    optional_u32, optional_value, reject_enabled_while_paused, resolve_maenifold_bin,
    schedule_store, task_value, validate_mcp_action,
};
use schedule_validation::{preview_schedule, validate_and_parse_schedule, verify_installed_action};

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
        "clear_history" => clear_history(context, payload),
        "backfill" => schedule_backfill::backfill_action(context, payload),
        "preview" => preview_schedule(context, payload),
        "verify_installed" => verify_installed_action(context, payload),
        "pause_all" => schedule_batch::pause_all(context, payload),
        "resume_all" => schedule_batch::resume_all(context, payload),
        "set_all_enabled" => schedule_batch::set_all_enabled(context, payload),
        "pause_status" => schedule_batch::pause_status(context),
        "options" => options_action(context),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown action '{}'. Available: create, update, delete, list, get, run_now, logs, clear_history, backfill, preview, verify_installed, pause_all, resume_all, set_all_enabled, pause_status, options",
            action
        ))),
    }
}
