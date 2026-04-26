use thiserror::Error;

#[derive(Debug, Error)]
pub enum GateError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("missing task metadata field: {0}")]
    MissingMetadataField(&'static str),

    #[error("failed to parse task metadata: {0}")]
    TaskMetadataParse(String),

    #[error("duplicate state id in pipeline definition: {0}")]
    DuplicateStageName(String),

    #[error("pipeline definition state '{stage}' points to unknown transition target '{target}'")]
    UnknownTransitionTarget { stage: String, target: String },

    #[error("unknown state '{0}' for active pipeline")]
    UnknownState(String),

    #[error("illegal transition: state '{state}' does not allow outcome '{outcome}'")]
    IllegalTransition { state: String, outcome: String },
}
