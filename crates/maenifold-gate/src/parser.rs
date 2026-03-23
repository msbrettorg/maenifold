use std::{fs, path::Path};

use crate::types::{StageState, StageVerdict, TaskLocation, TaskStage, TaskState, TaskStatus};

mod error;
mod sections;

pub use error::ParseTaskError;
pub use sections::{ParsedSection, extract_sections};
use sections::{RawSection, scan_sections};

/// T-53.2: RTM FR-41 — parse an authoritative task file from disk.
pub fn parse_task_file(path: impl AsRef<Path>) -> Result<TaskState, ParseTaskError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| ParseTaskError::ReadFailed {
        path: path.to_path_buf(),
        source,
    })?;

    parse_task_content(path, &content)
}

/// T-53.2: RTM FR-41 — parse authoritative task state from path + markdown content.
pub fn parse_task_content(
    path: impl AsRef<Path>,
    content: &str,
) -> Result<TaskState, ParseTaskError> {
    let path = path.as_ref();
    let location =
        TaskLocation::from_path(path).ok_or_else(|| ParseTaskError::UnknownTaskLocation {
            path: path.to_path_buf(),
        })?;

    let sections = scan_sections(content)?;
    let sections = sections
        .into_iter()
        .filter_map(|section| stage_from_heading(&section.heading).map(|stage| (stage, section)))
        .collect::<Vec<_>>();

    let mut history = Vec::with_capacity(sections.len());
    let mut attempts = 0u32;

    for (index, section) in sections.iter().enumerate() {
        let has_later_same_stage = sections[index + 1..]
            .iter()
            .any(|other| other.0 == section.0);
        let verdict =
            parse_section_verdict(section.0, &section.1, location, has_later_same_stage, path)?;
        if verdict.is_some_and(StageVerdict::counts_as_attempt) {
            attempts += 1;
        }

        history.push(StageState {
            stage: section.0,
            heading_line: section.1.heading_line,
            verdict,
        });
    }

    let (derived, stages) = derive_status(history.as_slice(), location)?;
    let status = reconcile_location(location, derived, path)?;

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
    location: TaskLocation,
) -> Result<(TaskStatus, Vec<StageState>), ParseTaskError> {
    if history.is_empty() {
        return Ok((TaskStatus::Pending, vec![]));
    }

    let swe = history
        .iter()
        .rfind(|stage| stage.stage == TaskStage::Swe)
        .cloned();

    // T-54.1: done/ files tolerate missing SWE — treat as implicit completion.
    // todo/ files still require SWE before Red/Blue.
    if swe.is_none() && location != TaskLocation::Done {
        let first = history
            .first()
            .expect("non-empty stage list should have a first element");
        return Err(ParseTaskError::OutOfOrderHeading {
            previous: TaskStage::Swe,
            current: first.stage,
            line: first.heading_line,
        });
    }

    let red = if let Some(ref swe) = swe {
        history
            .iter()
            .rfind(|stage| stage.stage == TaskStage::Red && stage.heading_line > swe.heading_line)
            .cloned()
    } else {
        history
            .iter()
            .rfind(|stage| stage.stage == TaskStage::Red)
            .cloned()
    };

    let blue = red
        .as_ref()
        .filter(|stage| stage.verdict == Some(StageVerdict::Pass))
        .and_then(|red| {
            history
                .iter()
                .rfind(|stage| {
                    stage.stage == TaskStage::Blue && stage.heading_line > red.heading_line
                })
                .cloned()
        });

    let stages = [swe.clone(), red.clone(), blue.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let status = match (
        red.as_ref().and_then(|stage: &StageState| stage.verdict),
        blue.as_ref().and_then(|stage: &StageState| stage.verdict),
    ) {
        (None, None) => TaskStatus::Red,
        (Some(StageVerdict::Reject), None) => TaskStatus::Pending,
        (Some(StageVerdict::Reject), Some(_)) => TaskStatus::Pending,
        (Some(StageVerdict::Pass), None) => TaskStatus::Blue,
        (Some(StageVerdict::Pass), Some(StageVerdict::Pass)) => TaskStatus::Done,
        (Some(StageVerdict::Pass), Some(StageVerdict::Fail | StageVerdict::Reject)) => {
            TaskStatus::Pending
        }
        (Some(StageVerdict::Fail), _) => {
            unreachable!("red FAIL verdicts are rejected during parse")
        }
        (None, Some(_)) => unreachable!("blue is only selected after a PASS red section"),
    };

    Ok((status, stages))
}

fn reconcile_location(
    location: TaskLocation,
    status: TaskStatus,
    path: &Path,
) -> Result<TaskStatus, ParseTaskError> {
    match (location, status) {
        (TaskLocation::Todo, TaskStatus::Done) => Err(ParseTaskError::LocationStatusMismatch {
            path: path.to_path_buf(),
            location,
            status,
        }),
        (TaskLocation::Done, TaskStatus::Done) => Ok(TaskStatus::Done),
        (TaskLocation::Done, _) => Err(ParseTaskError::LocationStatusMismatch {
            path: path.to_path_buf(),
            location,
            status,
        }),
        (TaskLocation::Todo, status) => Ok(status),
    }
}

fn parse_section_verdict(
    stage: TaskStage,
    section: &RawSection,
    location: TaskLocation,
    has_later_same_stage: bool,
    path: &Path,
) -> Result<Option<StageVerdict>, ParseTaskError> {
    let verdict = parse_detected_verdict(stage, section)?;

    match stage {
        TaskStage::Swe => Ok(None),
        TaskStage::Red | TaskStage::Blue => {
            if let Some(verdict) = verdict {
                Ok(Some(verdict))
            } else if has_later_same_stage {
                Ok(Some(StageVerdict::Reject))
            } else if location == TaskLocation::Done {
                Ok(Some(StageVerdict::Pass))
            } else {
                eprintln!(
                    "warning: {}: missing verdict for {} section starting at line {}",
                    path.display(),
                    stage.as_str(),
                    section.heading_line
                );
                Ok(None)
            }
        }
    }
}

fn parse_detected_verdict(
    stage: TaskStage,
    section: &RawSection,
) -> Result<Option<StageVerdict>, ParseTaskError> {
    let mut verdict = None;

    for raw in &section.verdicts {
        let parsed = raw.parsed.ok_or_else(|| ParseTaskError::UnknownVerdict {
            stage,
            line: raw.line,
            value: raw.value.clone(),
        })?;
        validate_stage_verdict(stage, parsed, raw.line)?;
        if verdict.is_some() {
            return Err(ParseTaskError::DuplicateVerdict {
                stage,
                line: raw.line,
            });
        }
        verdict = Some(parsed);
    }

    Ok(verdict)
}

fn validate_stage_verdict(
    stage: TaskStage,
    verdict: StageVerdict,
    line_number: usize,
) -> Result<(), ParseTaskError> {
    let allowed = match stage {
        TaskStage::Swe => false,
        TaskStage::Red => matches!(verdict, StageVerdict::Pass | StageVerdict::Reject),
        TaskStage::Blue => matches!(verdict, StageVerdict::Pass | StageVerdict::Fail),
    };

    if !allowed {
        return Err(ParseTaskError::InvalidVerdictForStage {
            stage,
            verdict,
            line: line_number,
        });
    }

    Ok(())
}

fn stage_from_heading(heading: &str) -> Option<TaskStage> {
    TaskStage::ALL
        .into_iter()
        .find(|stage| heading == stage.canonical_heading().trim_start_matches('#').trim())
}
