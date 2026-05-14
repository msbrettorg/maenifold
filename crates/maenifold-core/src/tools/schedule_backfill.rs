use super::{ToolContext, optional_string, render, required_string, schedule_store};
use crate::error::{JSONRPC_INTERNAL_ERROR, MaenifoldError, Result};
use crate::schedule::parse_schedule;
use chrono::{DateTime, Duration, Utc};
use croner::parser::CronParser;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn backfill_action(context: &ToolContext, payload: &Value) -> Result<String> {
    let id = required_string(payload, "id")?;
    let since = parse_timestamp(payload, "since")?;
    let until = match optional_string(payload, "until")? {
        Some(value) => parse_rfc3339(&value, "until")?,
        None => Utc::now(),
    };
    if until < since {
        return Err(MaenifoldError::InvalidArgument(
            "'until' must be greater than or equal to 'since'".into(),
        ));
    }

    let store = schedule_store(context);
    let task = store
        .read(id)
        .map_err(|_| MaenifoldError::InvalidArgument(format!("unknown scheduleId '{id}'")))?;
    let parsed = parse_schedule(&task.schedule)
        .map_err(|error| MaenifoldError::InvalidArgument(error.to_string()))?;
    let cron = CronParser::new()
        .parse(&parsed.cron)
        .map_err(|error| MaenifoldError::InvalidArgument(format!("invalid cron: {error}")))?;

    let log_dir = store.task_dir(id).join("logs");
    let existing = log_prefixes(&log_dir)?;
    // croner's iter_after() is exclusive, so shift the cursor back one
    // nanosecond to keep the backfill contract inclusive of `since`.
    let cursor = since
        .checked_sub_signed(Duration::nanoseconds(1))
        .unwrap_or(since);
    let planned = cron
        .iter_after(cursor)
        .take_while(|time| *time <= until)
        .map(|time| run_file_name(&time))
        .collect::<Vec<_>>();
    let mut run_ids = Vec::new();

    for file_name in &planned {
        let prefix = file_name.trim_end_matches(".log");
        if existing.contains(prefix) {
            continue;
        }
        let content = format!(
            "started_at={} action=backfill placeholder=true\nNo agent invoked. Backfill marker.",
            timestamp_from_file_name(file_name)
        );
        fs::create_dir_all(&log_dir)?;
        fs::write(log_dir.join(file_name), content)?;
        run_ids.push(file_name.clone());
    }

    for file_name in &run_ids {
        if !log_dir.join(file_name).exists() {
            return Err(MaenifoldError::JsonRpc {
                code: JSONRPC_INTERNAL_ERROR,
                message: format!(
                    "backfill verification failed for schedule '{id}': missing {file_name}"
                ),
                data: json!(null),
            });
        }
    }

    Ok(render(
        context.output_format,
        json!({
            "action": "backfill",
            "scheduleId": id,
            "plannedCount": planned.len(),
            "executedCount": run_ids.len(),
            "skippedCount": planned.len() - run_ids.len(),
            "runIds": run_ids,
        }),
    ))
}

fn parse_timestamp(payload: &Value, key: &'static str) -> Result<DateTime<Utc>> {
    parse_rfc3339(required_string(payload, key)?, key)
}

fn parse_rfc3339(value: &str, key: &'static str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|error| {
            MaenifoldError::InvalidArgument(format!("'{key}' must be RFC3339: {error}"))
        })
}

fn run_file_name(time: &DateTime<Utc>) -> String {
    format!("{}.log", time.format("%Y%m%dT%H%M%SZ"))
}

fn timestamp_from_file_name(file_name: &str) -> String {
    let raw = file_name.trim_end_matches(".log");
    format!(
        "{}-{}-{}T{}:{}:{}Z",
        &raw[0..4],
        &raw[4..6],
        &raw[6..8],
        &raw[9..11],
        &raw[11..13],
        &raw[13..15]
    )
}

fn log_prefixes(log_dir: &Path) -> Result<BTreeSet<String>> {
    let mut prefixes = BTreeSet::new();
    for path in list_log_files(log_dir)? {
        if let Some(prefix) = run_prefix(&path)? {
            prefixes.insert(prefix);
        }
    }
    Ok(prefixes)
}

fn list_log_files(log_dir: &Path) -> Result<Vec<PathBuf>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

fn run_prefix(path: &Path) -> Result<Option<String>> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| MaenifoldError::NonUtf8Path(path.to_path_buf()))?;
    let bytes = file_name.as_bytes();
    let valid = bytes.len() >= 16
        && bytes[0..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'T'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'Z';
    Ok(valid.then(|| file_name[0..16].to_string()))
}
