//! T-01.17: frontmatter lineage extraction helpers split from
//! [`super::file_index`] to keep the parent under the per-file LOC budget.
//! No behavioral changes.

use crate::lineage::{gate_task_lineage_id, pipeline_run_lineage_id, thinking_session_lineage_id};
use crate::markdown::Frontmatter;

pub(super) fn frontmatter_lineage_gate_task_id(
    frontmatter: &Option<Frontmatter>,
) -> Option<String> {
    frontmatter
        .as_ref()
        .and_then(|fm| fm.get("lineage_gate_task_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            frontmatter
                .as_ref()
                .and_then(|fm| fm.get("gate_task_id"))
                .and_then(|value| value.as_str())
                .map(gate_task_lineage_id)
        })
}

pub(super) fn frontmatter_lineage_pipeline_run_id(
    frontmatter: &Option<Frontmatter>,
) -> Option<String> {
    frontmatter
        .as_ref()
        .and_then(|fm| fm.get("lineage_pipeline_run_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            frontmatter
                .as_ref()
                .and_then(|fm| fm.get("pipeline_run_id"))
                .and_then(|value| value.as_str())
                .map(pipeline_run_lineage_id)
        })
        .or_else(|| {
            frontmatter
                .as_ref()
                .and_then(|fm| fm.get("pipeline_session_id"))
                .and_then(|value| value.as_str())
                .map(pipeline_run_lineage_id)
        })
}

pub(super) fn frontmatter_lineage_thinking_session_id(
    frontmatter: &Option<Frontmatter>,
) -> Option<String> {
    frontmatter
        .as_ref()
        .and_then(|fm| fm.get("lineage_thinking_session_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            frontmatter
                .as_ref()
                .and_then(|fm| fm.get("thinking_session_id"))
                .and_then(|value| value.as_str())
                .map(thinking_session_lineage_id)
        })
}
