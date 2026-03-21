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
    let mut current_stage: Option<TaskStage> = None;
    let mut seen_stages = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if let Some(stage) = match_stage_heading(line)? {
            let heading_line = index + 1;
            if seen_stages.contains(&stage) {
                return Err(ParseTaskError::DuplicateHeading {
                    stage,
                    line: heading_line,
                });
            }

            if let Some(previous) = current_stage {
                if stage < previous {
                    return Err(ParseTaskError::OutOfOrderHeading {
                        previous,
                        current: stage,
                        line: heading_line,
                    });
                }
            }

            current_stage = Some(stage);
            seen_stages.push(stage);
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

    let mut stages = Vec::with_capacity(sections.len());
    let mut attempts = 0u32;

    for section in sections {
        let verdict = parse_section_verdict(&lines, &section)?;
        if let Some(verdict) = verdict {
            if verdict.counts_as_attempt() {
                attempts += 1;
            }
        }

        stages.push(StageState {
            stage: section.stage,
            heading_line: section.heading_line,
            verdict,
        });
    }

    let derived = derive_status(stages.as_slice())?;
    let status = reconcile_location(location, derived, path)?;

    Ok(TaskState::new(
        path.to_path_buf(),
        location,
        status,
        attempts,
        stages,
    ))
}

fn derive_status(stages: &[StageState]) -> Result<TaskStatus, ParseTaskError> {
    if stages.is_empty() {
        return Ok(TaskStatus::Pending);
    }

    let has_swe = stages.iter().any(|stage| stage.stage == TaskStage::Swe);
    let red = stages.iter().find(|stage| stage.stage == TaskStage::Red);
    let blue = stages.iter().find(|stage| stage.stage == TaskStage::Blue);

    if blue.is_some() && red.is_none() {
        return Err(ParseTaskError::OutOfOrderHeading {
            previous: TaskStage::Swe,
            current: TaskStage::Blue,
            line: stages
                .iter()
                .find(|stage| stage.stage == TaskStage::Blue)
                .map(|stage| stage.heading_line)
                .unwrap_or(0),
        });
    }

    if !has_swe {
        let first = stages
            .first()
            .expect("non-empty stage list should have a first element");
        return Err(ParseTaskError::OutOfOrderHeading {
            previous: TaskStage::Swe,
            current: first.stage,
            line: first.heading_line,
        });
    }

    match (
        red.and_then(|stage| stage.verdict),
        blue.and_then(|stage| stage.verdict),
    ) {
        (None, None) => Ok(TaskStatus::Red),
        (Some(StageVerdict::Reject), None) => Ok(TaskStatus::Pending),
        (Some(StageVerdict::Pass), None) => Ok(TaskStatus::Blue),
        (Some(StageVerdict::Pass), Some(StageVerdict::Pass)) => Ok(TaskStatus::Done),
        (Some(StageVerdict::Pass), Some(StageVerdict::Fail)) => Ok(TaskStatus::Pending),
        (Some(StageVerdict::Pass), Some(StageVerdict::Reject)) => {
            Err(ParseTaskError::InvalidVerdictForStage {
                stage: TaskStage::Blue,
                verdict: StageVerdict::Reject,
                line: blue_heading_line(stages),
            })
        }
        (Some(StageVerdict::Reject), Some(_)) => Err(ParseTaskError::InvalidStatusFlow {
            message: format!(
                "blue stage cannot follow a red {} verdict",
                StageVerdict::Reject.as_str()
            ),
            line: line_for_stage(stages, TaskStage::Blue),
        }),
        (Some(StageVerdict::Fail), _) => Err(ParseTaskError::InvalidVerdictForStage {
            stage: TaskStage::Red,
            verdict: StageVerdict::Fail,
            line: line_for_stage(stages, TaskStage::Red),
        }),
        (None, Some(_)) => Err(ParseTaskError::InvalidStatusFlow {
            message: "blue stage cannot appear before a completed red verdict".to_string(),
            line: line_for_stage(stages, TaskStage::Blue),
        }),
    }
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
        TaskStage::Swe => Ok(verdict),
        TaskStage::Red | TaskStage::Blue => {
            verdict.map(Some).ok_or(ParseTaskError::MissingVerdict {
                stage: section.stage,
                line: section.heading_line,
            })
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

    let verdict = match raw {
        "PASS" => StageVerdict::Pass,
        "REJECT" => StageVerdict::Reject,
        "FAIL" => StageVerdict::Fail,
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
        let canonical = stage.canonical_heading();
        if trimmed == canonical {
            return Ok(Some(stage));
        }

        if trimmed.starts_with(canonical) {
            return Err(ParseTaskError::NonCanonicalHeading {
                stage,
                found: trimmed.to_string(),
            });
        }
    }

    Ok(None)
}

fn line_for_stage(stages: &[StageState], stage: TaskStage) -> usize {
    stages
        .iter()
        .find(|state| state.stage == stage)
        .map(|state| state.heading_line)
        .unwrap_or(0)
}

fn blue_heading_line(stages: &[StageState]) -> usize {
    line_for_stage(stages, TaskStage::Blue)
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
    NonCanonicalHeading {
        stage: TaskStage,
        found: String,
    },
    DuplicateHeading {
        stage: TaskStage,
        line: usize,
    },
    OutOfOrderHeading {
        previous: TaskStage,
        current: TaskStage,
        line: usize,
    },
    MissingVerdict {
        stage: TaskStage,
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
            Self::NonCanonicalHeading { stage, found } => write!(
                f,
                "non-canonical {} heading '{}' ; use '{}'",
                stage.as_str(),
                found,
                stage.canonical_heading()
            ),
            Self::DuplicateHeading { stage, line } => {
                write!(f, "duplicate {} heading at line {}", stage.as_str(), line)
            }
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
            Self::MissingVerdict { stage, line } => write!(
                f,
                "missing verdict for {} section starting at line {}",
                stage.as_str(),
                line
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
