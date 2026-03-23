use serde::Serialize;
use std::collections::BTreeMap;

use crate::{db::TaskRow, types::TaskState};

#[derive(Serialize)]
pub(super) struct StatusOutput {
    pub file: String,
    pub location: String,
    pub status: String,
    pub swe: bool,
    pub red: Option<String>,
    pub blue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub db_drift: Vec<String>,
}

pub(super) fn print_summary(counts: &BTreeMap<String, usize>) {
    println!("{:<8} {:>5}", "STATUS", "COUNT");
    for (key, count) in counts {
        if key != "error" {
            println!("{:<8} {:>5}", key, count);
        }
    }
    if let Some(errors) = counts.get("error") {
        println!("{:<8} {:>5}", "error", errors);
    }
    println!("---");
    println!("{:<8} {:>5}", "total", counts.values().sum::<usize>());
}

/// Discover the unique ordered list of stage names from a task's stages.
pub(super) fn discovered_stages(state: &TaskState) -> Vec<String> {
    let mut seen = Vec::new();
    for stage in &state.stages {
        if !seen.contains(&stage.stage) {
            seen.push(stage.stage.clone());
        }
    }
    seen
}

pub(super) fn stage_value(state: &TaskState, stage_name: &str) -> Option<Option<&'static str>> {
    state
        .stages
        .iter()
        .find(|item| item.stage == stage_name)
        .map(|item| item.verdict.map(|verdict| verdict.as_str()))
}

pub(super) fn stage_text(state: &TaskState, stage_name: &str) -> &'static str {
    match stage_value(state, stage_name) {
        Some(Some(verdict)) => verdict,
        Some(None) => "yes",
        None => "no",
    }
}

pub(super) fn maybe_drift(
    check_drift: bool,
    state: &TaskState,
    row: Option<&TaskRow>,
) -> Vec<String> {
    check_drift.then(|| drift(state, row)).unwrap_or_default()
}

fn drift(state: &TaskState, row: Option<&TaskRow>) -> Vec<String> {
    let mut out = Vec::new();
    match row {
        Some(row) => {
            if row.status != state.effective_status() {
                out.push(format!(
                    "db status={} but file status={}",
                    row.status,
                    state.effective_status()
                ));
            }
            if row.location != state.location.as_str() {
                out.push(format!(
                    "db location={} but file location={}",
                    row.location,
                    state.location.as_str()
                ));
            }
        }
        None => out.push("missing from db cache".into()),
    }
    out
}

pub(super) fn json_output(
    file: &str,
    location: &str,
    state: Option<&TaskState>,
    error: Option<String>,
    db_drift: Vec<String>,
) -> StatusOutput {
    StatusOutput {
        file: file.into(),
        location: location.into(),
        status: state
            .map_or_else(|| "error".to_string(), |item| item.effective_status().to_string()),
        swe: state.is_some_and(|item| stage_value(item, "swe").is_some()),
        red: state
            .and_then(|item| stage_value(item, "red"))
            .flatten()
            .map(|s| s.to_string()),
        blue: state
            .and_then(|item| stage_value(item, "blue"))
            .flatten()
            .map(|s| s.to_string()),
        error,
        db_drift,
    }
}
