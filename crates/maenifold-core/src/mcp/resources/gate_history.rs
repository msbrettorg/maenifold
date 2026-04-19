use crate::error::{MaenifoldError, Result};
use crate::mcp::resources::gate::{find_task_path, slug_stage};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn gate_task_history(root: &Path, task_id: String) -> Result<Value> {
    let path = find_task_path(root, &task_id)
        .ok_or_else(|| MaenifoldError::InvalidArgument(format!("task '{task_id}' not found")))?;
    let markdown = fs::read_to_string(path)?;
    Ok(Value::Array(parse_history_records(&markdown)))
}

fn parse_history_records(markdown: &str) -> Vec<Value> {
    let mut records = Vec::new();
    let mut current_stage = String::new();
    let mut stage_id = String::new();
    let mut record_kind: Option<String> = None;
    let mut outcome: Option<String> = None;
    let mut agent: Option<String> = None;
    let mut model: Option<String> = None;
    let mut notes: Option<String> = None;
    let mut timestamp: Option<String> = None;

    let has_record_data = |record_kind: &Option<String>,
                           outcome: &Option<String>,
                           agent: &Option<String>,
                           model: &Option<String>,
                           notes: &Option<String>,
                           timestamp: &Option<String>| {
        record_kind.is_some()
            || outcome.is_some()
            || agent.is_some()
            || model.is_some()
            || notes.is_some()
            || timestamp.is_some()
    };

    let flush = |records: &mut Vec<Value>,
                 stage_id: &str,
                 current_stage: &str,
                 record_kind: &Option<String>,
                 outcome: &Option<String>,
                 agent: &Option<String>,
                 model: &Option<String>,
                 notes: &Option<String>,
                 timestamp: &Option<String>| {
        if !has_record_data(record_kind, outcome, agent, model, notes, timestamp) {
            return;
        }
        let stage = if !stage_id.is_empty() {
            stage_id.to_string()
        } else {
            slug_stage(current_stage)
        };
        let event = outcome
            .as_ref()
            .map(|value| value.to_uppercase())
            .or_else(|| record_kind.clone())
            .unwrap_or_else(|| "append".to_string());
        records.push(json!({
            "stage": stage,
            "event": event,
            "agent": agent,
            "model": model,
            "notes": notes,
            "timestamp": timestamp,
        }));
    };

    let reset_record = |stage_id: &mut String,
                        record_kind: &mut Option<String>,
                        outcome: &mut Option<String>,
                        agent: &mut Option<String>,
                        model: &mut Option<String>,
                        notes: &mut Option<String>,
                        timestamp: &mut Option<String>| {
        stage_id.clear();
        *record_kind = None;
        *outcome = None;
        *agent = None;
        *model = None;
        *notes = None;
        *timestamp = None;
    };

    for line in markdown.lines() {
        if let Some(stage) = line.strip_prefix("## ").map(str::trim) {
            flush(
                &mut records,
                &stage_id,
                &current_stage,
                &record_kind,
                &outcome,
                &agent,
                &model,
                &notes,
                &timestamp,
            );
            current_stage = stage.to_string();
            reset_record(
                &mut stage_id,
                &mut record_kind,
                &mut outcome,
                &mut agent,
                &mut model,
                &mut notes,
                &mut timestamp,
            );
            continue;
        }

        if let Some(heading) = line.strip_prefix("### ").map(str::trim) {
            if heading.to_ascii_lowercase().starts_with("recorded at") {
                flush(
                    &mut records,
                    &stage_id,
                    &current_stage,
                    &record_kind,
                    &outcome,
                    &agent,
                    &model,
                    &notes,
                    &timestamp,
                );
                reset_record(
                    &mut stage_id,
                    &mut record_kind,
                    &mut outcome,
                    &mut agent,
                    &mut model,
                    &mut notes,
                    &mut timestamp,
                );
                if let Some((_, value)) = heading.split_once(':') {
                    let parsed = value.trim();
                    if !parsed.is_empty() {
                        timestamp = Some(parsed.to_string());
                    }
                }
            }
            continue;
        }

        let Some(payload) = line.strip_prefix("- ").map(str::trim) else {
            continue;
        };
        let Some((raw_key, raw_value)) = payload.split_once(':') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        match key.as_str() {
            "stage id" => {
                if has_record_data(&record_kind, &outcome, &agent, &model, &notes, &timestamp) {
                    flush(
                        &mut records,
                        &stage_id,
                        &current_stage,
                        &record_kind,
                        &outcome,
                        &agent,
                        &model,
                        &notes,
                        &timestamp,
                    );
                    reset_record(
                        &mut stage_id,
                        &mut record_kind,
                        &mut outcome,
                        &mut agent,
                        &mut model,
                        &mut notes,
                        &mut timestamp,
                    );
                }
                stage_id = value.to_string();
            }
            "record kind" => record_kind = Some(value.to_string()),
            "outcome" => outcome = Some(value.to_string()),
            "agent" => agent = Some(value.to_string()),
            "model" => model = Some(value.to_string()),
            "notes" => notes = Some(value.to_string()),
            "recorded at" => {
                if timestamp.is_some() {
                    flush(
                        &mut records,
                        &stage_id,
                        &current_stage,
                        &record_kind,
                        &outcome,
                        &agent,
                        &model,
                        &notes,
                        &timestamp,
                    );
                    reset_record(
                        &mut stage_id,
                        &mut record_kind,
                        &mut outcome,
                        &mut agent,
                        &mut model,
                        &mut notes,
                        &mut timestamp,
                    );
                }
                timestamp = Some(value.to_string());
            }
            _ => {}
        }
    }

    flush(
        &mut records,
        &stage_id,
        &current_stage,
        &record_kind,
        &outcome,
        &agent,
        &model,
        &notes,
        &timestamp,
    );
    records
}
