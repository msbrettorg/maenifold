use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationFailed,
    NotFound,
    AlreadyExists,
    DbError,
    PermissionDenied,
    InvalidInput,
    AdapterUnavailable,
    UnsupportedRuntime,
    IncompatibleVersion,
    RateLimited,
    Timeout,
    ProcessCrashed,
    AcpError,
    FilesystemError,
    InternalError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ValidationFailed,
            message: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: msg.into(),
        }
    }
    pub fn already_exists(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::AlreadyExists,
            message: msg.into(),
        }
    }
    pub fn db(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::DbError,
            message: msg.into(),
        }
    }
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidInput,
            message: msg.into(),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: msg.into(),
        }
    }
    pub fn fs(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::FilesystemError,
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).unwrap_or_else(|_| self.message.clone())
        )
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::db(e.to_string())
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}

#[allow(dead_code)]
pub fn classify_crud_error(entity: &str, raw: &str) -> AppError {
    let l = raw.to_ascii_lowercase();
    if l.contains("required")
        || l.contains("cannot contain")
        || l.contains("only allows")
        || l.contains("cannot be empty")
    {
        return AppError::validation(raw);
    }
    if l.contains("not found") {
        return AppError::not_found(raw);
    }
    if l.contains("already exists") {
        return AppError::already_exists(raw);
    }
    if l.contains("unique constraint") || l.contains("duplicate") {
        return AppError::already_exists(format!("{entity} already exists"));
    }
    if l.contains("no connection") || l.contains("pool") {
        return AppError::db(raw);
    }
    AppError::internal(raw)
}
