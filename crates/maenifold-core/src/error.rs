use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MaenifoldError {
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
    // PARITY-ERR-PREFIX: RTM FR-1 — pass through message without prefix (C# parity).
    #[error("{0}")]
    InvalidArgument(String),
    #[error("Invalid resource URI format: {0}")]
    InvalidResourceUri(String),
    #[error("Asset not found: {0}/{1}")]
    AssetNotFound(String, String),
    #[error("Directory traversal rejected: {0}")]
    PathTraversal(String),
    // T-0.3h: RTM FR-1 / FR-2 / FR-4 / FR-5 / FR-6 keeps required-property diagnostics aligned with the C# oracle.
    #[error("Required property '{0}' is missing from payload.")]
    MissingProperty(&'static str),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Current executable has no parent directory")]
    MissingExecutableParent,
    #[error("Path is not valid UTF-8: {0:?}")]
    NonUtf8Path(PathBuf),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, MaenifoldError>;
