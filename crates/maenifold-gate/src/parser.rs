use std::{fs, path::Path};

use crate::types::{StageState, StageVerdict, TaskLocation, TaskState};

mod error;
mod sections;

pub use error::ParseTaskError;
pub use sections::{ParsedSection, extract_sections};
use sections::{RawSection, scan_sections};

/// Normalize a markdown heading to a canonical stage name string.
///
/// Compatibility behavior:
/// - lowercased heading
/// - starts_with("swe") => "swe"
/// - starts_with("red") => "red"
/// - starts_with("blue") => "blue"
/// - otherwise first whitespace-delimited word lowercased, or whole heading lowercased.
pub fn normalize_stage_name(heading: &str) -> String {
    let lower = heading.to_lowercase();
    if lower.starts_with("swe") {
        return "swe".to_string();
    }
    if lower.starts_with("red") {
        return "red".to_string();
    }
    if lower.starts_with("blue") {
        return "blue".to_string();
    }
    lower
        .split_whitespace()
        .next()
        .unwrap_or(&lower)
        .to_string()
}

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

    let raw_sections = scan_sections(content)?;
    let sections: Vec<(String, RawSection)> = raw_sections
        .into_iter()
        .map(|section| {
            let stage = normalize_stage_name(&section.heading);
            (stage, section)
        })
        .collect();

    let mut history = Vec::with_capacity(sections.len());
    let mut attempts = 0u32;

    for (index, (stage, section)) in sections.iter().enumerate() {
        let has_later_same_stage = sections[index + 1..]
            .iter()
            .any(|other| other.0 == *stage);
        let verdict =
            parse_section_verdict(stage, section, location, has_later_same_stage, path)?;
        if verdict.is_some_and(StageVerdict::counts_as_attempt) {
            attempts += 1;
        }

        history.push(StageState {
            stage: stage.clone(),
            heading_line: section.heading_line,
            verdict,
        });
    }

    let (derived, stages) = derive_status(history.as_slice(), location)?;
    let status = reconcile_location(location, &derived, path)?;

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
    _location: TaskLocation,
) -> Result<(String, Vec<StageState>), ParseTaskError> {
    if history.is_empty() {
        return Ok(("pending".to_string(), vec![]));
    }

    let mut status = "pending".to_string();

    for (index, stage) in history.iter().enumerate() {
        let next_stage = history.get(index + 1).map(|next| next.stage.as_str());
        status = match stage.verdict {
            Some(StageVerdict::Pass) => next_stage.unwrap_or("done").to_string(),
            Some(StageVerdict::Reject | StageVerdict::Fail) => "pending".to_string(),
            None => next_stage.unwrap_or(stage.stage.as_str()).to_string(),
        };
    }

    Ok((status, history.to_vec()))
}

fn reconcile_location(
    location: TaskLocation,
    status: &str,
    path: &Path,
) -> Result<String, ParseTaskError> {
    match (location, status) {
        (TaskLocation::Todo, "done") => Err(ParseTaskError::LocationStatusMismatch {
            path: path.to_path_buf(),
            location,
            status: status.to_string(),
        }),
        (TaskLocation::Done, "done") => Ok("done".to_string()),
        (TaskLocation::Done, _) => Err(ParseTaskError::LocationStatusMismatch {
            path: path.to_path_buf(),
            location,
            status: status.to_string(),
        }),
        (TaskLocation::Todo, _) => Ok(status.to_string()),
    }
}

fn parse_section_verdict(
    stage: &str,
    section: &RawSection,
    location: TaskLocation,
    has_later_same_stage: bool,
    path: &Path,
) -> Result<Option<StageVerdict>, ParseTaskError> {
    let verdict = parse_detected_verdict(stage, section)?;

    if stage == "swe" {
        return Ok(None);
    }

    // For red/blue and other review stages
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
            stage,
            section.heading_line
        );
        Ok(None)
    }
}

fn parse_detected_verdict(
    stage: &str,
    section: &RawSection,
) -> Result<Option<StageVerdict>, ParseTaskError> {
    let mut verdict = None;

    for raw in &section.verdicts {
        let parsed = raw.parsed.ok_or_else(|| ParseTaskError::UnknownVerdict {
            stage: stage.to_string(),
            line: raw.line,
            value: raw.value.clone(),
        })?;
        validate_stage_verdict(stage, parsed, raw.line)?;
        if verdict.is_some() {
            return Err(ParseTaskError::DuplicateVerdict {
                stage: stage.to_string(),
                line: raw.line,
            });
        }
        verdict = Some(parsed);
    }

    Ok(verdict)
}

fn validate_stage_verdict(
    stage: &str,
    verdict: StageVerdict,
    line_number: usize,
) -> Result<(), ParseTaskError> {
    let allowed = match stage {
        "swe" => false,
        "red" => matches!(verdict, StageVerdict::Pass | StageVerdict::Reject),
        "blue" => matches!(verdict, StageVerdict::Pass | StageVerdict::Fail),
        // For unknown stages, allow all verdicts
        _ => true,
    };

    if !allowed {
        return Err(ParseTaskError::InvalidVerdictForStage {
            stage: stage.to_string(),
            verdict,
            line: line_number,
        });
    }

    Ok(())
}
