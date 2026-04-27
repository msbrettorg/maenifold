use std::process::Stdio;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use tokio::process::Command;

use crate::teams::token_provider::TeamsTokenError;

const GRAPH_PREFERRED_DATA_LOCATION_URL: &str =
    "https://graph.microsoft.com/v1.0/me?$select=preferredDataLocation";
const GRAPH_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamsJwtClaims {
    pub oid: String,
    pub tid: Option<String>,
    pub upn: Option<String>,
    pub preferred_username: Option<String>,
    pub name: Option<String>,
    pub exp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamsIdentity {
    pub mri: String,
    pub object_id: String,
    pub tenant_id: Option<String>,
    pub upn: Option<String>,
    pub display_name: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub chatsvc_region: Option<String>,
    pub identity_source: TeamsIdentitySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamsIdentitySource {
    Jwt,
    JwtWithGraphRegion,
}

#[derive(Debug, Deserialize)]
struct JwtPayload {
    oid: Option<String>,
    tid: Option<String>,
    upn: Option<String>,
    preferred_username: Option<String>,
    name: Option<String>,
    exp: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphMeResponse {
    preferred_data_location: Option<String>,
}

pub fn decode_teams_jwt(token: &str) -> Result<TeamsJwtClaims, TeamsTokenError> {
    let payload = jwt_payload(token)?;
    let claims: JwtPayload = serde_json::from_slice(&payload)
        .map_err(|error| TeamsTokenError::MalformedToken(error.to_string()))?;
    let oid = claims
        .oid
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| TeamsTokenError::MalformedToken("JWT is missing oid claim".to_string()))?;
    let exp = claims
        .exp
        .ok_or_else(|| TeamsTokenError::MalformedToken("JWT is missing exp claim".to_string()))?;
    let exp = Utc
        .timestamp_opt(exp, 0)
        .single()
        .ok_or_else(|| TeamsTokenError::MalformedToken("JWT exp claim is invalid".to_string()))?;
    Ok(TeamsJwtClaims {
        oid,
        tid: non_empty(claims.tid),
        upn: non_empty(claims.upn),
        preferred_username: non_empty(claims.preferred_username),
        name: non_empty(claims.name),
        exp,
    })
}

pub fn identity_from_jwt(token: &str) -> Result<TeamsIdentity, TeamsTokenError> {
    let claims = decode_teams_jwt(token)?;
    let upn = claims
        .upn
        .clone()
        .or_else(|| claims.preferred_username.clone());
    Ok(TeamsIdentity {
        mri: format!("8:orgid:{}", claims.oid),
        object_id: claims.oid,
        tenant_id: claims.tid,
        upn,
        display_name: claims.name,
        expires_at: claims.exp,
        chatsvc_region: None,
        identity_source: TeamsIdentitySource::Jwt,
    })
}

pub async fn detect_identity(token: &str) -> Result<TeamsIdentity, TeamsTokenError> {
    let mut identity = identity_from_jwt(token)?;
    if let Ok(Some(region)) = detect_chatsvc_region_via_graph().await {
        identity.chatsvc_region = Some(region);
        identity.identity_source = TeamsIdentitySource::JwtWithGraphRegion;
    }
    Ok(identity)
}

pub async fn detect_chatsvc_region_via_graph() -> Result<Option<String>, TeamsTokenError> {
    let output = tokio::time::timeout(
        GRAPH_TIMEOUT,
        Command::new(azure_cli_binary())
            .args([
                "rest",
                "--method",
                "GET",
                "--url",
                GRAPH_PREFERRED_DATA_LOCATION_URL,
                "-o",
                "json",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| {
        TeamsTokenError::GraphIdentityUnavailable("Azure CLI Graph request timed out".to_string())
    })?
    .map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            TeamsTokenError::AzureCliUnavailable
        } else {
            TeamsTokenError::GraphIdentityUnavailable(format!(
                "Azure CLI Graph request failed: {error}"
            ))
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(classify_graph_error(&stderr));
    }

    let response: GraphMeResponse = serde_json::from_slice(&output.stdout)
        .map_err(|error| TeamsTokenError::GraphIdentityUnavailable(error.to_string()))?;
    Ok(response
        .preferred_data_location
        .as_deref()
        .and_then(map_preferred_data_location))
}

pub fn map_preferred_data_location(location: &str) -> Option<String> {
    match location.trim().to_ascii_uppercase().as_str() {
        "NAM" | "CAN" | "BRA" => Some("amer".to_string()),
        "EUR" | "FRA" | "CHE" | "DEU" | "GBR" | "NOR" | "ZAF" | "ARE" | "SWE" => {
            Some("emea".to_string())
        }
        "APC" | "AUS" | "JPN" | "IND" | "KOR" | "SGP" => Some("apac".to_string()),
        _ => None,
    }
}

fn classify_graph_error(stderr: &str) -> TeamsTokenError {
    if stderr.contains("AADSTS530084") {
        TeamsTokenError::TokenProtectionRequiresBrowserSession
    } else if stderr.contains("Please run 'az login'")
        || stderr.contains("az login")
        || stderr.contains("not logged in")
    {
        TeamsTokenError::AzureCliNotLoggedIn
    } else {
        TeamsTokenError::GraphIdentityUnavailable(safe_cli_error(stderr))
    }
}

fn jwt_payload(token: &str) -> Result<Vec<u8>, TeamsTokenError> {
    let compact = token.trim().strip_prefix("Bearer ").unwrap_or(token.trim());
    let mut parts = compact.split('.');
    let _header = parts
        .next()
        .ok_or_else(|| TeamsTokenError::MalformedToken("JWT is missing header".to_string()))?;
    let payload = parts
        .next()
        .ok_or_else(|| TeamsTokenError::MalformedToken("JWT is missing payload".to_string()))?;
    if payload.is_empty() || parts.next().is_none() {
        return Err(TeamsTokenError::MalformedToken(
            "JWT must have three compact parts".to_string(),
        ));
    }
    URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| TeamsTokenError::MalformedToken(error.to_string()))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn safe_cli_error(stderr: &str) -> String {
    let compact = stderr.replace(['\r', '\n'], " ");
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        "Azure CLI Graph request failed".to_string()
    } else {
        trimmed.chars().take(500).collect()
    }
}

fn azure_cli_binary() -> &'static str {
    if cfg!(windows) { "az.cmd" } else { "az" }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
