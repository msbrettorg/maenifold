use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

mod authorize;
mod metadata;
mod register;
mod token;

pub(crate) use metadata::protected_resource_metadata_url;

pub(super) const CODE_TTL: Duration = Duration::from_secs(300);
pub(super) const MCP_PATH: &str = "/mcp";

pub(super) static OAUTH_STATE: Lazy<Mutex<OAuthState>> =
    Lazy::new(|| Mutex::new(OAuthState::default()));

#[derive(Default)]
pub(super) struct OAuthState {
    pub clients: HashMap<String, RegisteredClient>,
    pub codes: HashMap<String, PendingAuthorizationCode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct RegisteredClient {
    pub redirect_uris: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct PendingAuthorizationCode {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub resource: String,
    pub expires_at: Instant,
}

const CLIENTS_FILE_NAME: &str = "clients.json";
const AUTH_DIR_NAME: &str = "auth";

pub(super) fn clients_path(root: &Path) -> PathBuf {
    root.join(AUTH_DIR_NAME).join(CLIENTS_FILE_NAME)
}

/// Load persisted dynamic-client registrations from disk on startup.
///
/// Per the MCP authorization spec (Dynamic Client Registration, RFC 7591),
/// clients persist their `client_id` between sessions and expect it to
/// remain valid across server restarts. Without this, OpenCode and other
/// MCP clients fail with `client_id is not registered` after every restart.
/// Spec: https://modelcontextprotocol.io/specification/draft/basic/authorization
pub fn load_clients_from_disk(root: &Path) {
    let path = clients_path(root);
    let Ok(data) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(loaded): std::result::Result<HashMap<String, RegisteredClient>, _> =
        serde_json::from_str(&data)
    else {
        tracing::warn!(
            path = %path.display(),
            "failed to deserialize OAuth clients registry; starting empty"
        );
        return;
    };
    let mut state = OAUTH_STATE.lock();
    state.clients = loaded;
}

/// Persist the in-memory client registry to disk via atomic write.
pub(super) fn save_clients_to_disk(root: &Path) {
    let auth_dir = root.join(AUTH_DIR_NAME);
    if let Err(error) = fs::create_dir_all(&auth_dir) {
        tracing::error!(?error, "failed to create auth dir for OAuth clients");
        return;
    }
    let snapshot: HashMap<String, RegisteredClient> = OAUTH_STATE.lock().clients.clone();
    let json = match serde_json::to_string_pretty(&snapshot) {
        Ok(value) => value,
        Err(error) => {
            tracing::error!(?error, "failed to serialize OAuth clients");
            return;
        }
    };
    let path = clients_path(root);
    let tmp_path = auth_dir.join(format!("{CLIENTS_FILE_NAME}.tmp"));
    if let Err(error) = fs::write(&tmp_path, json) {
        tracing::error!(?error, "failed to write tmp OAuth clients file");
        return;
    }
    #[cfg(unix)]
    {
        let _ = fs::set_permissions(
            &tmp_path,
            std::os::unix::fs::PermissionsExt::from_mode(0o600),
        );
    }
    if let Err(error) = fs::rename(&tmp_path, &path) {
        tracing::error!(?error, "failed to atomically rename OAuth clients file");
    }
}

pub(crate) fn routes(state: crate::api::routes::ApiState) -> Router {
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            axum::routing::get(metadata::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            axum::routing::get(metadata::protected_resource_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            axum::routing::get(metadata::authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-authorization-server/mcp",
            axum::routing::get(metadata::authorization_server_metadata),
        )
        .route("/register", axum::routing::post(register::register_client))
        .route("/authorize", axum::routing::get(authorize::authorize))
        .route("/token", axum::routing::post(token::token))
        .with_state(state)
}

pub(super) fn public_base_url(headers: &HeaderMap) -> String {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("127.0.0.1:8080");
    format!("http://{host}")
}

pub(super) fn expected_resource(headers: &HeaderMap) -> String {
    format!("{}{MCP_PATH}", public_base_url(headers))
}

pub(super) fn is_loopback_redirect_uri(uri: &str) -> bool {
    ["http://127.0.0.1", "http://localhost", "http://[::1]"]
        .iter()
        .any(|prefix| uri.strip_prefix(prefix).is_some_and(valid_loopback_suffix))
}

fn valid_loopback_suffix(rest: &str) -> bool {
    if rest.is_empty() || rest.starts_with('/') || rest.starts_with('?') {
        return true;
    }
    let Some(after_colon) = rest.strip_prefix(':') else {
        return false;
    };
    let port_len = after_colon
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    port_len > 0
        && after_colon[port_len..]
            .chars()
            .next()
            .is_none_or(|character| character == '/' || character == '?')
}

pub(super) fn purge_expired_codes(state: &mut OAuthState) {
    let now = Instant::now();
    state.codes.retain(|_, code| code.expires_at > now);
}

pub(super) fn oauth_error(
    status: StatusCode,
    error: &'static str,
    description: impl Into<String>,
) -> Response {
    (
        status,
        Json(json!({
            "error": error,
            "error_description": description.into()
        })),
    )
        .into_response()
}

pub(super) fn redirect_with_params(redirect_uri: &str, params: &[(&str, &str)]) -> Response {
    let separator = if redirect_uri.contains('?') { '&' } else { '?' };
    let query = params
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let location = if query.is_empty() {
        redirect_uri.to_owned()
    } else {
        format!("{redirect_uri}{separator}{query}")
    };

    (
        StatusCode::FOUND,
        [(header::LOCATION, location)],
        Json(json!({ "status": "redirecting" })),
    )
        .into_response()
}

pub(super) fn issued_at() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn insert_expired_code_for_test(
    code: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    resource: String,
) {
    OAUTH_STATE.lock().codes.insert(
        code,
        PendingAuthorizationCode {
            client_id,
            redirect_uri,
            code_challenge,
            resource,
            expires_at: Instant::now() - Duration::from_secs(1),
        },
    );
}
