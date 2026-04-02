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

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("missing task metadata field: {0}")]
    MissingMetadataField(&'static str),

    #[error("invalid pipeline digest: {0}")]
    InvalidPipelineDigest(String),

    #[error("task pipeline binding '{bound}' does not match requested pipeline '{requested}'")]
    PipelineOverrideMismatch { requested: String, bound: String },

    #[error("pipeline digest mismatch for '{pipeline}': task binds '{task_digest}' but authoritative asset is '{authoritative_digest}'")]
    PipelineDigestMismatch {
        pipeline: String,
        task_digest: String,
        authoritative_digest: String,
    },

    #[error("failed to parse task metadata: {0}")]
    TaskMetadataParse(String),

    #[error("unknown built-in gate definition: {0}")]
    UnknownBuiltinGate(String),

    #[error("duplicate stage name in pipeline definition: {0}")]
    DuplicateStageName(String),

    #[error("pipeline definition stage '{stage}' points to unknown transition target '{target}'")]
    UnknownTransitionTarget { stage: String, target: String },

    #[error("unknown state '{0}' for active pipeline")]
    UnknownState(String),

    #[error("illegal transition: state '{state}' does not allow verdict '{verdict}'")]
    IllegalTransition { state: String, verdict: String },

    #[error("task path does not exist: {0}")]
    MissingTaskPath(PathBuf),

    #[error("{0} is not implemented in this slice")]
    NotYetImplemented(&'static str),
}
