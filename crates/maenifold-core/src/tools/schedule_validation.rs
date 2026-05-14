use super::{
    ToolContext, optional_string, render, required_string, resolve_maenifold_bin, schedule_store,
};
use crate::error::{MaenifoldError, Result};
use crate::schedule::{ParsedSchedule, parse_schedule};
use chrono::{SecondsFormat, Utc};
use croner::parser::CronParser;
use serde_json::{Value, json};

pub(super) fn verify_installed_action(context: &ToolContext, payload: &Value) -> Result<String> {
    let store = schedule_store(context);
    if let Some(id) = optional_string(payload, "scheduleId")? {
        let task = store
            .read(&id)
            .map_err(|_| MaenifoldError::InvalidArgument(format!("unknown scheduleId '{id}'")))?;
        let bin = resolve_maenifold_bin()?;
        return Ok(render(
            context.output_format,
            serde_json::to_value(super::schedule_os::verify_installed(&task, &bin)?)?,
        ));
    }
    let tasks = store
        .list()
        .map_err(|error| MaenifoldError::Message(error.to_string()))?;
    if tasks.is_empty() {
        return Ok(render(context.output_format, json!([])));
    }
    let bin = resolve_maenifold_bin()?;
    let statuses = tasks
        .into_iter()
        .map(|task| super::schedule_os::verify_installed(&task, &bin))
        .collect::<Result<Vec<_>>>()?;
    Ok(render(
        context.output_format,
        serde_json::to_value(statuses)?,
    ))
}

pub(super) fn preview_schedule(context: &ToolContext, payload: &Value) -> Result<String> {
    let input = required_string(payload, "schedule")?;
    let parsed = validate_and_parse_schedule(input)?;
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

pub(super) fn validate_and_parse_schedule(input: &str) -> Result<ParsedSchedule> {
    if input.len() > 1024 {
        return Err(MaenifoldError::InvalidArgument(
            "schedule expression too long: max 1024 bytes".into(),
        ));
    }
    let parsed = parse_schedule(input)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let cron = CronParser::new()
        .parse(&parsed.cron)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let now = Utc::now();
    let has_fires = cron.iter_after(now).take(1).next().is_some();
    if !has_fires {
        return Err(MaenifoldError::InvalidArgument(
            "schedule has no upcoming fire times in the next year".into(),
        ));
    }
    Ok(parsed)
}
