use super::{MCP_PATH, expected_resource, public_base_url};
use axum::{Json, http::HeaderMap};
use serde_json::{Value, json};

pub(crate) fn protected_resource_metadata_url(headers: &HeaderMap) -> String {
    format!(
        "{}/.well-known/oauth-protected-resource{MCP_PATH}",
        public_base_url(headers)
    )
}

pub(super) async fn protected_resource_metadata(headers: HeaderMap) -> Json<Value> {
    let base_url = public_base_url(&headers);
    Json(json!({
        "resource": expected_resource(&headers),
        "authorization_servers": [base_url],
        "bearer_methods_supported": ["header"],
        "resource_documentation": "https://github.com/msbrettorg/maenifold-rs/blob/main/docs/copilot-cli-mcp-setup.md"
    }))
}

pub(super) async fn authorization_server_metadata(headers: HeaderMap) -> Json<Value> {
    let base_url = public_base_url(&headers);
    Json(json!({
        "issuer": base_url,
        "authorization_endpoint": format!("{base_url}/authorize"),
        "token_endpoint": format!("{base_url}/token"),
        "registration_endpoint": format!("{base_url}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"]
    }))
}
