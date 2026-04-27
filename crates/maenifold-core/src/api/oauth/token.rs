use super::{OAUTH_STATE, oauth_error, purge_expired_codes};
use crate::api::{auth, routes::ApiState};
use axum::{
    Form,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
pub(super) struct TokenRequest {
    grant_type: Option<String>,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    resource: Option<String>,
}

pub(super) async fn token(
    State(state): State<ApiState>,
    Form(request): Form<TokenRequest>,
) -> Response {
    if request.grant_type.as_deref() != Some("authorization_code") {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "grant_type must be authorization_code",
        );
    }
    let Some(code) = request.code.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code is required",
        );
    };
    let Some(code_verifier) = request.code_verifier.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_verifier is required",
        );
    };
    let Some(redirect_uri) = request.redirect_uri.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "redirect_uri is required",
        );
    };
    let Some(client_id) = request.client_id.as_deref() else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "client_id is required",
        );
    };

    let pending = {
        let mut oauth_state = OAUTH_STATE.lock();
        purge_expired_codes(&mut oauth_state);
        oauth_state.codes.remove(code)
    };
    let Some(pending) = pending else {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "authorization code is invalid or expired",
        );
    };

    if pending.client_id != client_id {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "authorization code does not match this client_id",
        );
    }
    if pending.redirect_uri != redirect_uri {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "authorization code does not match this redirect_uri",
        );
    }
    if request.resource.as_deref() != Some(pending.resource.as_str()) {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_target",
            "resource does not match authorization request",
        );
    }
    if pkce_challenge(code_verifier) != pending.code_challenge {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "PKCE code_verifier does not match authorization request",
        );
    }

    let token_path = auth::token_path(&state.config().maenifold_root);
    let Ok(access_token) = std::fs::read_to_string(token_path) else {
        return oauth_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "server_error",
            "maenifold bearer token is not available; restart the maenifold HTTP daemon",
        );
    };

    axum::Json(json!({
        "access_token": access_token.trim(),
        "token_type": "Bearer"
    }))
    .into_response()
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}
