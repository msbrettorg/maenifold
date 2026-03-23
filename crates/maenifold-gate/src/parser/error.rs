use std::{error::Error, fmt};

use crate::{StageVerdict, TaskLocation};

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
    DuplicateVerdict {
        stage: String,
        line: usize,
    },
    UnknownVerdict {
        stage: String,
        line: usize,
        value: String,
    },
    InvalidVerdictForStage {
        stage: String,
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
        status: String,
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
            Self::DuplicateVerdict { stage, line } => write!(
                f,
                "duplicate verdict in {} section at line {}",
                stage, line
            ),
            Self::UnknownVerdict { stage, line, value } => write!(
                f,
                "unknown verdict '{}' for {} section at line {}",
                value, stage, line
            ),
            Self::InvalidVerdictForStage {
                stage,
                verdict,
                line,
            } => write!(
                f,
                "invalid verdict '{}' for {} section at line {}",
                verdict.as_str(),
                stage,
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
                status
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
