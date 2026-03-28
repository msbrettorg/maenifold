use std::path::Path;

use crate::{
    pipeline::PipelineDef,
    types::{StageState, StageVerdict, TaskLocation, TaskState},
};

use super::{
    ParseTaskError, SectionVerdictContext, normalize_stage_name, parse_section_verdict,
    reconcile_location, scan_sections, verdict_key,
};

pub fn parse_task_content_with_pipeline(
    path: impl AsRef<Path>,
    content: &str,
    pipeline: &PipelineDef,
) -> Result<TaskState, ParseTaskError> {
    let path = path.as_ref();
    let location =
        TaskLocation::from_path(path).ok_or_else(|| ParseTaskError::UnknownTaskLocation {
            path: path.to_path_buf(),
        })?;
    let raw_sections = scan_sections(content)?;
    let mapped = raw_sections
        .iter()
        .map(|section| match_pipeline_stage(&section.heading, pipeline))
        .collect::<Vec<_>>();
    let mut history = Vec::new();
    let mut attempts = 0u32;

    for (index, section) in raw_sections.iter().enumerate() {
        let Some(stage) = mapped[index].as_deref() else {
            continue;
        };
        let allowed = pipeline
            .stages
            .iter()
            .find(|candidate| candidate.name == stage)
            .map(|candidate| candidate.verdicts.as_slice());
        let has_later_same_stage = mapped[index + 1..]
            .iter()
            .any(|other| other.as_deref() == Some(stage));
        let verdict = parse_section_verdict(
            section,
            SectionVerdictContext {
                stage,
                allowed_verdicts: allowed,
                location,
                has_later_same_stage,
                path,
            },
        )?;
        if verdict.is_some_and(StageVerdict::counts_as_attempt) {
            attempts += 1;
        }
        history.push(StageState {
            stage: stage.to_string(),
            heading_line: section.heading_line,
            verdict,
        });
    }

    let (status, stages) = derive_status(history.as_slice(), pipeline)?;
    let status = reconcile_location(location, &status, path)?;
    Ok(TaskState::new(
        path.to_path_buf(),
        location,
        status,
        attempts,
        stages,
    ))
}

fn derive_status(
    history: &[StageState],
    pipeline: &PipelineDef,
) -> Result<(String, Vec<StageState>), ParseTaskError> {
    let mut status = "pending".to_string();
    for stage in history {
        let Some(def) = pipeline
            .stages
            .iter()
            .find(|candidate| candidate.name == stage.stage)
        else {
            continue;
        };
        status = match stage.verdict {
            Some(verdict) => def
                .transitions
                .get(verdict_key(verdict))
                .cloned()
                .ok_or_else(|| ParseTaskError::InvalidStatusFlow {
                    message: format!(
                        "stage '{}' has no '{}' transition",
                        def.name,
                        verdict_key(verdict)
                    ),
                    line: stage.heading_line,
                })?,
            None if def.verdicts.is_empty() => {
                def.transitions.get("pass").cloned().ok_or_else(|| {
                    ParseTaskError::InvalidStatusFlow {
                        message: format!("stage '{}' has no 'pass' transition", def.name),
                        line: stage.heading_line,
                    }
                })?
            }
            None => def.name.clone(),
        };
    }
    Ok((status, history.to_vec()))
}

fn match_pipeline_stage(heading: &str, pipeline: &PipelineDef) -> Option<String> {
    let lower = heading.trim().to_lowercase();
    pipeline
        .stages
        .iter()
        .find(|stage| stage.name.eq_ignore_ascii_case(&lower))
        .or_else(|| {
            let normalized = normalize_stage_name(heading);
            pipeline
                .stages
                .iter()
                .find(|stage| stage.name.eq_ignore_ascii_case(&normalized))
        })
        .map(|stage| stage.name.clone())
}
