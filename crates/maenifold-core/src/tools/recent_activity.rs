use super::context::{ToolContext, load_help_doc};
use super::recent_activity_collect::{
    collect_recent_activity_records_with_filter, parse_recent_activity_filter,
};
use super::recent_activity_format::format_recent_activity_report;
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::util::json::{OutputFormat, success};
use chrono::TimeDelta;
use serde_json::Value;

pub fn recent_activity(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "RecentActivity")
            .unwrap_or_else(|| "ERROR: Help file not found for RecentActivity".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    // T-2.1: RTM FR-4 / RTM-026 default limit handling for RecentActivity.
    let limit = payload.get("limit").and_then(Value::as_i64).unwrap_or(10);
    if limit <= 0 {
        return Err(MaenifoldError::InvalidArgument(
            "limit must be a positive integer".to_string(),
        ));
    }
    // T-2.6: RTM FR-4 / RTM-026 clamp RecentActivity limit to prevent unbounded queries.
    let limit = limit.min(1_000);

    // T-2.1: RTM FR-4 / RTM-021-RTM-024 filter and timespan validation.
    let filter = parse_recent_activity_filter(payload.get("filter").and_then(Value::as_str))?;
    let include_content = payload
        .get("includeContent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let timespan = payload
        .get("timespan")
        .and_then(Value::as_str)
        .map(parse_recent_activity_timespan)
        .transpose()?;

    let rows =
        collect_recent_activity_records_with_filter(&context.config, timespan, filter, limit)?;
    let report = format_recent_activity_report(rows, include_content);

    Ok(match context.output_format {
        OutputFormat::Json => success(report),
        OutputFormat::Markdown => report,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecentActivityRecord {
    pub file_path: String,
    pub title: String,
    pub last_indexed: String,
    pub content: String,
    pub status: String,
    pub first: Option<String>,
    pub last: Option<String>,
}

pub fn parse_recent_activity_timespan(input: &str) -> Result<TimeDelta> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(MaenifoldError::InvalidArgument(
            "timespan must not be empty".to_string(),
        ));
    }
    if trimmed.starts_with('-') {
        return Err(MaenifoldError::InvalidArgument(
            "timespan must be positive".to_string(),
        ));
    }

    // T-2.1: RTM FR-4 / RTM-024 parse RecentActivity timespan in C#-style form.
    // Compatibility note: the shipped docs/examples use values like "24.00:00:00" for one day,
    // so values >= 24 before the dot are treated as total hours rather than calendar days.
    let (day_or_hour_prefix, time_part) = match trimmed.split_once('.') {
        Some((prefix, time_part)) => (prefix, time_part),
        None => ("0", trimmed),
    };
    let day_or_hour_prefix: i64 = day_or_hour_prefix
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let mut time_segments = time_part.split(':');
    let hours: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let minutes: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    let seconds: i64 = time_segments
        .next()
        .ok_or_else(|| invalid_timespan(trimmed))?
        .parse()
        .map_err(|_| invalid_timespan(trimmed))?;
    if time_segments.next().is_some() || hours < 0 || minutes < 0 || seconds < 0 {
        return Err(invalid_timespan(trimmed));
    }
    if minutes >= 60 || seconds >= 60 {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid timespan '{trimmed}'. Minutes and seconds must be less than 60"
        )));
    }

    let (days, total_hours) = if trimmed.contains('.') && day_or_hour_prefix >= 24 {
        (0, day_or_hour_prefix + hours)
    } else {
        (day_or_hour_prefix, hours)
    };

    TimeDelta::try_days(days)
        .and_then(|delta| delta.checked_add(&TimeDelta::try_hours(total_hours)?))
        .and_then(|delta| delta.checked_add(&TimeDelta::try_minutes(minutes)?))
        .and_then(|delta| delta.checked_add(&TimeDelta::try_seconds(seconds)?))
        .ok_or_else(|| {
            MaenifoldError::InvalidArgument(format!(
                "Invalid timespan '{trimmed}'. Value is out of range"
            ))
        })
}

fn invalid_timespan(trimmed: &str) -> MaenifoldError {
    MaenifoldError::InvalidArgument(format!(
        "Invalid timespan '{trimmed}'. Expected format days.HH:MM:SS"
    ))
}

pub fn collect_recent_activity_records(
    config: &Config,
    timespan: Option<TimeDelta>,
    filter: Option<&str>,
    limit: i64,
) -> Result<Vec<RecentActivityRecord>> {
    let filter = parse_recent_activity_filter(filter)?;
    collect_recent_activity_records_with_filter(config, timespan, filter, limit)
}
