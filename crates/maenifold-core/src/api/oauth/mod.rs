use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::{
    collections::HashMap,
    path::Path,
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

#[derive(Clone, Debug)]
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

/// T-240.8: OAuth client registration is stateless, so there is no on-disk
/// client registry to load at startup. Kept as a no-op for existing callers.
pub fn load_clients_from_disk(_root: &Path) {
    // No-op.
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
