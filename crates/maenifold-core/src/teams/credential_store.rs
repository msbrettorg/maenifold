use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::error::MaenifoldError;

use super::TokenInfo;

pub const AGENCY_COWORK_TOKEN_RESOURCE: &str = "https://ic3.teams.office.com";
const AGENCY_COWORK_TOKEN_CACHE_RELATIVE_PATH: &[&str] = &[".teams-agent", "token-cache.json"];
const MIN_VALIDITY_REUSE_SECONDS: i64 = 30;
const TOKEN_CACHE_HOME_ERROR: &str = "Teams token cache path unavailable: HOME/USERPROFILE is not set; cannot use ~/.teams-agent/token-cache.json";

#[derive(Debug, Deserialize, Serialize)]
struct StoredToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: DateTime<Utc>,
}

impl From<&TokenInfo> for StoredToken {
    fn from(token: &TokenInfo) -> Self {
        Self {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            expires_at: token.expires_at,
        }
    }
}

impl From<StoredToken> for TokenInfo {
    fn from(token: StoredToken) -> Self {
        Self {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: token.expires_at,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct AgencyCoworkTokenEntry {
    #[serde(alias = "access_token")]
    token: String,
    expires_at: serde_json::Value,
}

/// Reads a still-valid agency-cowork-compatible Teams token from
/// `~/.teams-agent/token-cache.json`.
///
/// Returns `Ok(None)` when the cache file, resource entry, token, or TTL is
/// absent/expired. Malformed JSON remains an error because callers should be
/// able to surface corrupt local state distinctly from a cache miss.
pub fn read_agency_cowork_token(resource: &str) -> Result<Option<TokenInfo>, MaenifoldError> {
    read_agency_cowork_token_from_path(resource, &agency_cowork_token_cache_path()?)
}

/// Writes the agency-cowork-compatible disk token cache entry for `resource`.
pub fn write_agency_cowork_token(resource: &str, token: &TokenInfo) -> Result<(), MaenifoldError> {
    write_agency_cowork_token_to_path(resource, token, &agency_cowork_token_cache_path()?)
}

pub fn delete_agency_cowork_token(resource: &str) -> Result<(), MaenifoldError> {
    delete_agency_cowork_token_from_path(resource, &agency_cowork_token_cache_path()?)
}

pub fn agency_cowork_token_cache_path() -> Result<PathBuf, MaenifoldError> {
    agency_cowork_token_cache_path_from_home_vars(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
    )
}

pub(crate) fn agency_cowork_token_cache_path_from_home_vars(
    home: Option<OsString>,
    userprofile: Option<OsString>,
) -> Result<PathBuf, MaenifoldError> {
    let mut path = home
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            userprofile
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
        .ok_or_else(|| MaenifoldError::Message(TOKEN_CACHE_HOME_ERROR.to_string()))?;
    for component in AGENCY_COWORK_TOKEN_CACHE_RELATIVE_PATH {
        path.push(component);
    }
    Ok(path)
}

pub fn read_agency_cowork_token_from_path(
    resource: &str,
    path: &Path,
) -> Result<Option<TokenInfo>, MaenifoldError> {
    if !path.exists() {
        return Ok(None);
    }
    let cache = std::fs::read_to_string(path)?;
    let cache: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&cache)?;
    let Some(entry) = cache.get(resource) else {
        return Ok(None);
    };
    let entry: AgencyCoworkTokenEntry = serde_json::from_value(entry.clone())?;
    if entry.token.trim().is_empty() {
        return Ok(None);
    }
    let Some(expires_at) = parse_agency_cowork_expiry(&entry.expires_at) else {
        return Ok(None);
    };
    if expires_at.signed_duration_since(Utc::now()).num_seconds() <= MIN_VALIDITY_REUSE_SECONDS {
        return Ok(None);
    }
    Ok(Some(TokenInfo {
        access_token: entry
            .token
            .trim()
            .strip_prefix("Bearer ")
            .unwrap_or(entry.token.trim())
            .to_string(),
        refresh_token: None,
        expires_at,
    }))
}

pub fn write_agency_cowork_token_to_path(
    resource: &str,
    token: &TokenInfo,
    path: &Path,
) -> Result<(), MaenifoldError> {
    let mut cache = if path.exists() {
        let cache = std::fs::read_to_string(path)?;
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&cache)?
    } else {
        serde_json::Map::new()
    };
    cache.insert(
        resource.to_string(),
        serde_json::json!({
            "token": token.access_token,
            "expires_at": token.expires_at.timestamp(),
        }),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

pub fn delete_agency_cowork_token_from_path(
    resource: &str,
    path: &Path,
) -> Result<(), MaenifoldError> {
    if !path.exists() {
        return Ok(());
    }
    let cache = std::fs::read_to_string(path)?;
    let mut cache = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&cache)?;
    cache.remove(resource);
    std::fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

fn parse_agency_cowork_expiry(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    match value {
        serde_json::Value::Number(number) => {
            let seconds = number
                .as_i64()
                .or_else(|| number.as_f64().map(|value| value as i64))?;
            DateTime::from_timestamp(seconds, 0)
        }
        serde_json::Value::String(text) => text
            .parse::<i64>()
            .ok()
            .and_then(|seconds| DateTime::from_timestamp(seconds, 0))
            .or_else(|| {
                DateTime::parse_from_rfc3339(text)
                    .ok()
                    .map(|value| value.with_timezone(&Utc))
            }),
        _ => None,
    }
}
