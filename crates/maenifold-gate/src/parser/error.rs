use std::{error::Error, fmt};

use crate::{StageVerdict, TaskLocation, TaskStage, TaskStatus};

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
