use crate::api::routes::ApiState;
use crate::error::Result;
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const TOKEN_FILE_NAME: &str = "localhost.token";
const AUTH_DIR_NAME: &str = "auth";

pub fn token_path(root: &Path) -> PathBuf {
    root.join(AUTH_DIR_NAME).join(TOKEN_FILE_NAME)
}

pub fn generate_and_write_token(root: &Path) -> Result<()> {
    let token = format!(
        "{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let auth_dir = root.join(AUTH_DIR_NAME);
    fs::create_dir_all(&auth_dir)?;

    let token_path = token_path(root);
    let tmp_path = auth_dir.join(format!("{TOKEN_FILE_NAME}.tmp"));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);

    {
        let mut file = options.open(&tmp_path)?;
        file.write_all(token.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }

    #[cfg(unix)]
    fs::set_permissions(&tmp_path, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;

    fs::rename(&tmp_path, &token_path)?;
    Ok(())
}

pub async fn require_bearer_token(
    State(state): State<ApiState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let Some(provided_token) = bearer_token(&headers) else {
        return unauthorized();
    };

    let expected_path = token_path(&state.config().maenifold_root);
    let Ok(expected_token) = fs::read_to_string(expected_path) else {
        return unauthorized();
    };

    if !constant_time_eq(provided_token.as_bytes(), expected_token.trim().as_bytes()) {
        return unauthorized();
    }

    next.run(request).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "missing or invalid bearer token" })),
    )
        .into_response()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    let max_len = a.len().max(b.len());
    for index in 0..max_len {
        let a_byte = a.get(index).copied().unwrap_or(0);
        let b_byte = b.get(index).copied().unwrap_or(0);
        diff |= usize::from(a_byte ^ b_byte);
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generated_token_is_hex_64_chars() {
        let temp = TempDir::new().expect("temp dir");
        generate_and_write_token(temp.path()).expect("write token");
        let token = fs::read_to_string(token_path(temp.path())).expect("read token");
        let token = token.trim();

        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
