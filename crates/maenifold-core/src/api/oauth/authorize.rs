use super::{
    CODE_TTL, OAUTH_STATE, PendingAuthorizationCode, expected_resource, is_loopback_redirect_uri,
    oauth_error, purge_expired_codes, redirect_with_params,
};
use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    resource: Option<String>,
}

pub(super) async fn authorize(headers: HeaderMap, Query(query): Query<AuthorizeQuery>) -> Response {
    if query.response_type.as_deref() != Some("code") {
        return oauth_error_redirect_or_json(
            &query,
            "unsupported_response_type",
            "response_type must be code",
        );
    }
    let Some(client_id) = query.client_id.as_deref().filter(|value| !value.is_empty()) else {
        return oauth_error_redirect_or_json(&query, "invalid_request", "client_id is required");
    };
    let Some(redirect_uri) = query.redirect_uri.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "redirect_uri is required",
        );
    };
    if !is_loopback_redirect_uri(redirect_uri) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uri must be http://127.0.0.1, http://localhost, or http://[::1]",
        );
    }

    let mut state = OAUTH_STATE.lock();
    purge_expired_codes(&mut state);
    // T-240.8: Accept any maenifold-local-* client_id. If not in memory
    // (e.g., after daemon restart), auto-register with the provided redirect_uri.
    if !client_id.starts_with("maenifold-local-") {
        return oauth_error_redirect_or_json(
            &query,
            "unauthorized_client",
            "client_id must start with 'maenifold-local-'",
        );
    }
    if !state.clients.contains_key(client_id) {
        // Auto-register: daemon restarted but client still has old client_id.
        state.clients.insert(
            client_id.to_owned(),
            super::RegisteredClient {
                redirect_uris: vec![redirect_uri.to_owned()],
            },
        );
    }
    let client = state.clients.get(client_id).unwrap();
    if !client.redirect_uris.iter().any(|uri| uri == redirect_uri) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uri was not registered for this client",
        );
    }
    let Some(code_challenge) = query
        .code_challenge
        .as_deref()
        .filter(|value| !value.is_empty())
    else {
        return oauth_error_redirect_or_json(
            &query,
            "invalid_request",
            "PKCE code_challenge is required",
        );
    };
    if query.code_challenge_method.as_deref() != Some("S256") {
        return oauth_error_redirect_or_json(
            &query,
            "invalid_request",
            "code_challenge_method must be S256",
        );
    }
    let expected_resource = expected_resource(&headers);
    if query.resource.as_deref() != Some(expected_resource.as_str()) {
        return oauth_error_redirect_or_json(
            &query,
            "invalid_target",
            "resource is required and must match the maenifold MCP endpoint",
        );
    }

    let code = format!("ma-code-{}", Uuid::new_v4());
    state.codes.insert(
        code.clone(),
        PendingAuthorizationCode {
            client_id: client_id.to_owned(),
            redirect_uri: redirect_uri.to_owned(),
            code_challenge: code_challenge.to_owned(),
            resource: expected_resource,
            expires_at: std::time::Instant::now() + CODE_TTL,
        },
    );
    drop(state);

    redirect_with_params(
        redirect_uri,
        &[
            ("code", code.as_str()),
            ("state", query.state.as_deref().unwrap_or("")),
        ],
    )
}

fn oauth_error_redirect_or_json(
    query: &AuthorizeQuery,
    error: &'static str,
    description: &'static str,
) -> Response {
    if let Some(redirect_uri) = query
        .redirect_uri
        .as_deref()
        .filter(|uri| is_loopback_redirect_uri(uri))
    {
        return redirect_with_params(
            redirect_uri,
            &[
                ("error", error),
                ("error_description", description),
                ("state", query.state.as_deref().unwrap_or("")),
            ],
        );
    }

    oauth_error(StatusCode::BAD_REQUEST, error, description)
}
