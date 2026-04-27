use super::{OAUTH_STATE, RegisteredClient, is_loopback_redirect_uri, issued_at, oauth_error};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub(super) struct RegisterRequest {
    redirect_uris: Option<Vec<String>>,
    client_name: Option<String>,
    token_endpoint_auth_method: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
}

#[derive(Serialize)]
pub(super) struct RegisterResponse {
    client_id: String,
    client_id_issued_at: u64,
    token_endpoint_auth_method: &'static str,
    redirect_uris: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_name: Option<String>,
    grant_types: Vec<&'static str>,
    response_types: Vec<&'static str>,
}

pub(super) async fn register_client(Json(request): Json<RegisterRequest>) -> Response {
    if request
        .token_endpoint_auth_method
        .as_deref()
        .is_some_and(|method| method != "none")
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "maenifold local OAuth bridge only supports public clients with token_endpoint_auth_method=none",
        );
    }

    let redirect_uris = match request.redirect_uris {
        Some(uris) if !uris.is_empty() && uris.iter().all(|uri| is_loopback_redirect_uri(uri)) => {
            uris
        }
        Some(_) => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_redirect_uri",
                "redirect_uris must be non-empty loopback http URIs",
            );
        }
        None => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "invalid_client_metadata",
                "redirect_uris is required",
            );
        }
    };

    if request
        .grant_types
        .as_ref()
        .is_some_and(|types| !types.iter().any(|value| value == "authorization_code"))
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "grant_types must include authorization_code",
        );
    }
    if request
        .response_types
        .as_ref()
        .is_some_and(|types| !types.iter().any(|value| value == "code"))
    {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "response_types must include code",
        );
    }

    let client_id = format!("maenifold-local-{}", Uuid::new_v4());
    OAUTH_STATE.lock().clients.insert(
        client_id.clone(),
        RegisteredClient {
            redirect_uris: redirect_uris.clone(),
        },
    );

    (
        StatusCode::CREATED,
        Json(RegisterResponse {
            client_id,
            client_id_issued_at: issued_at(),
            token_endpoint_auth_method: "none",
            redirect_uris,
            client_name: request.client_name,
            grant_types: vec!["authorization_code"],
            response_types: vec!["code"],
        }),
    )
        .into_response()
}
