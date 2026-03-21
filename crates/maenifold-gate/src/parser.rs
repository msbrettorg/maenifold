use std::{error::Error, fmt, fs, path::Path};

use crate::types::{StageState, StageVerdict, TaskLocation, TaskStage, TaskState, TaskStatus};

const VERDICT_PREFIX: &str = "**Verdict**:";

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

    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if let Some(stage) = match_stage_heading(line)? {
            let heading_line = index + 1;
            sections.push(Section {
                stage,
                heading_line,
                start_index: index + 1,
                end_index: lines.len(),
            });

            if sections.len() > 1 {
                let previous = sections.len() - 2;
                sections[previous].end_index = index;
            }
        }
    }

    let mut history = Vec::with_capacity(sections.len());
    let mut attempts = 0u32;

    for (index, section) in sections.iter().enumerate() {
        let has_later_same_stage = sections[index + 1..]
            .iter()
            .any(|other| other.stage == section.stage);
        let verdict = parse_section_verdict(&lines, section, location, has_later_same_stage, path)?;
        if verdict.is_some_and(StageVerdict::counts_as_attempt) {
            attempts += 1;
        }

        history.push(StageState {
            stage: section.stage,
            heading_line: section.heading_line,
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
    lines: &[&str],
    section: &Section,
    location: TaskLocation,
    has_later_same_stage: bool,
    path: &Path,
) -> Result<Option<StageVerdict>, ParseTaskError> {
    let mut verdict = None;

    for (index, line) in lines[section.start_index..section.end_index]
        .iter()
        .enumerate()
    {
        let trimmed = line.trim();
        if !trimmed.starts_with(VERDICT_PREFIX) {
            continue;
        }

        let parsed = parse_verdict(trimmed, section.stage, section.start_index + index + 1)?;
        if verdict.is_some() {
            return Err(ParseTaskError::DuplicateVerdict {
                stage: section.stage,
                line: section.start_index + index + 1,
            });
        }

        verdict = Some(parsed);
    }

    match section.stage {
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
                    section.stage.as_str(),
                    section.heading_line
                );
                Ok(None)
            }
        }
    }
}

fn parse_verdict(
    line: &str,
    stage: TaskStage,
    line_number: usize,
) -> Result<StageVerdict, ParseTaskError> {
    let raw = line
        .split_once(':')
        .map(|(_, verdict)| verdict.trim())
        .unwrap_or_default();

    let verdict = match raw
        .split(|ch: char| !ch.is_alphanumeric())
        .find(|token| matches!(*token, "PASS" | "REJECT" | "FAIL"))
    {
        Some("PASS") => StageVerdict::Pass,
        Some("REJECT") => StageVerdict::Reject,
        Some("FAIL") => StageVerdict::Fail,
        _ => {
            return Err(ParseTaskError::UnknownVerdict {
                stage,
                line: line_number,
                value: raw.to_string(),
            });
        }
    };

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

    Ok(verdict)
}

fn match_stage_heading(line: &str) -> Result<Option<TaskStage>, ParseTaskError> {
    let trimmed = line.trim();
    for stage in TaskStage::ALL {
        if trimmed.starts_with(stage.canonical_heading()) {
            return Ok(Some(stage));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone)]
struct Section {
    stage: TaskStage,
    heading_line: usize,
    start_index: usize,
    end_index: usize,
}

/// T-53.2: RTM FR-41 — parser errors for file-authoritative task derivation.
#[derive(Debug)]
pub enum ParseTaskError {
    ReadFailed {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    UnknownTaskLocation {
        path: std::path::PathBuf,
    },
    OutOfOrderHeading {
        previous: TaskStage,
        current: TaskStage,
        line: usize,
    },
    DuplicateVerdict {
        stage: TaskStage,
        line: usize,
    },
    UnknownVerdict {
        stage: TaskStage,
        line: usize,
        value: String,
    },
    InvalidVerdictForStage {
        stage: TaskStage,
        verdict: StageVerdict,
        line: usize,
    },
    InvalidStatusFlow {
        message: String,
        line: usize,
    },
    LocationStatusMismatch {
        path: std::path::PathBuf,
        location: TaskLocation,
        status: TaskStatus,
    },
}

impl fmt::Display for ParseTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, source } => {
                write!(f, "failed to read task file {}: {source}", path.display())
            }
            Self::UnknownTaskLocation { path } => write!(
                f,
                "task file {} must live under todo/ or done/",
                path.display()
            ),
            Self::OutOfOrderHeading {
                previous,
                current,
                line,
            } => write!(
                f,
                "out-of-order heading '{}' at line {}; it cannot appear after '{}'",
                current.canonical_heading(),
                line,
                previous.canonical_heading()
            ),
            Self::DuplicateVerdict { stage, line } => write!(
                f,
                "duplicate verdict in {} section at line {}",
                stage.as_str(),
                line
            ),
            Self::UnknownVerdict { stage, line, value } => write!(
                f,
                "unknown verdict '{}' for {} section at line {}",
                value,
                stage.as_str(),
                line
            ),
            Self::InvalidVerdictForStage {
                stage,
                verdict,
                line,
            } => write!(
                f,
                "invalid verdict '{}' for {} section at line {}",
                verdict.as_str(),
                stage.as_str(),
                line
            ),
            Self::InvalidStatusFlow { message, line } => {
                write!(f, "invalid task flow at line {}: {message}", line)
            }
            Self::LocationStatusMismatch {
                path,
                location,
                status,
            } => write!(
                f,
                "task file {} in {}/ cannot represent '{}' status",
                path.display(),
                location.as_str(),
                status.as_str()
            ),
        }
    }
}

impl Error for ParseTaskError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}
