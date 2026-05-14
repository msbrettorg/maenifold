use crate::error::Result;
use crate::markdown::Frontmatter;
use crate::sessions::now_iso8601_roundtrip;
use serde::Serialize;
use serde_yml::Value as YamlValue;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct WorkflowStepTiming {
    pub(crate) started_at: Option<String>,
    pub(crate) ended_at: Option<String>,
}

pub(crate) type WorkflowStepTimings = BTreeMap<String, WorkflowStepTiming>;

pub(crate) fn read_step_timings(frontmatter: &Frontmatter) -> WorkflowStepTimings {
    let Some(YamlValue::Mapping(mapping)) = frontmatter.get("step_timings") else {
        return BTreeMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, value)| {
            let step_key = key.as_str()?.to_string();
            let timing = read_step_timing(value)?;
            Some((step_key, timing))
        })
        .collect()
}

pub(crate) fn record_step_started(frontmatter: &mut Frontmatter, step_number: usize) -> Result<()> {
    record_workflow_step_started(frontmatter, 0, step_number)
}

pub(crate) fn record_workflow_step_started(
    frontmatter: &mut Frontmatter,
    workflow_index: usize,
    step_number: usize,
) -> Result<()> {
    update_step_timing(
        frontmatter,
        workflow_index,
        step_number,
        |timing, timestamp| {
            if timing.started_at.is_none() {
                timing.started_at = Some(timestamp);
            }
        },
    )
}

pub(crate) fn record_workflow_step_ended(
    frontmatter: &mut Frontmatter,
    workflow_index: usize,
    step_number: usize,
) -> Result<()> {
    update_step_timing(
        frontmatter,
        workflow_index,
        step_number,
        |timing, timestamp| {
            timing.ended_at = Some(timestamp);
        },
    )
}

fn read_step_timing(value: &YamlValue) -> Option<WorkflowStepTiming> {
    let YamlValue::Mapping(mapping) = value else {
        return None;
    };
    Some(WorkflowStepTiming {
        started_at: timing_field(mapping, "started_at"),
        ended_at: timing_field(mapping, "ended_at"),
    })
}

fn timing_field(mapping: &serde_yml::Mapping, field: &str) -> Option<String> {
    mapping
        .get(YamlValue::String(field.to_string()))
        .and_then(YamlValue::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn update_step_timing(
    frontmatter: &mut Frontmatter,
    workflow_index: usize,
    step_number: usize,
    update: impl FnOnce(&mut WorkflowStepTiming, String),
) -> Result<()> {
    let mut timings = read_step_timings(frontmatter);
    let timing = timings
        .entry(timing_key(workflow_index, step_number))
        .or_default();
    update(timing, now_iso8601_roundtrip());
    frontmatter.insert("step_timings".to_string(), serde_yml::to_value(timings)?);
    Ok(())
}

fn timing_key(workflow_index: usize, step_number: usize) -> String {
    if workflow_index == 0 {
        step_number.to_string()
    } else {
        format!("{}:{step_number}", workflow_index + 1)
    }
}
