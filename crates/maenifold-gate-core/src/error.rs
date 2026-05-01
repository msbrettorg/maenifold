use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GateError {
    #[error("{0}")]
    Cli(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("missing task metadata field: {0}")]
    MissingMetadataField(&'static str),

    #[error("failed to parse task metadata: {0}")]
    TaskMetadataParse(String),

    #[error(
        "malformed task id in '{}': expected format {}, actual id '{}'",
        path.display(),
        expected_format,
        actual_id
    )]
    MalformedTaskId {
        path: PathBuf,
        expected_format: &'static str,
        actual_id: String,
    },

    #[error("duplicate state id in pipeline definition: {0}")]
    DuplicateStageName(String),

    #[error("pipeline definition state '{stage}' points to unknown transition target '{target}'")]
    UnknownTransitionTarget { stage: String, target: String },

    #[error("unknown state '{0}' for active pipeline")]
    UnknownState(String),

    #[error("illegal transition: state '{state}' does not allow outcome '{outcome}'")]
    IllegalTransition { state: String, outcome: String },

    #[error("task '{task_id}' expected stage '{expected}' but current stage is '{actual}'")]
    StageMismatch {
        task_id: String,
        expected: String,
        actual: String,
    },

    #[error(
        "task '{task_id}' not found. searched directories: {searched_directories}. Task lookup matches frontmatter `id:` (falling back to the filename stem when no `id:` is present)."
    )]
    TaskNotFound {
        task_id: String,
        searched_directories: String,
    },

    #[error("task path does not exist: {0}")]
    MissingTaskPath(PathBuf),

    #[error(
        "duplicate canonical task id '{task_id}' found in both {} and {}",
        first_path.display(),
        second_path.display()
    )]
    DuplicateCanonicalTaskId {
        task_id: String,
        first_path: PathBuf,
        second_path: PathBuf,
    },
}
