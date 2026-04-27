use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use tokio::process::Command;

use crate::teams::credential_store::{
    AGENCY_COWORK_TOKEN_RESOURCE, read_agency_cowork_token, write_agency_cowork_token,
};
use crate::teams::{TokenInfo, identity::decode_teams_jwt};

pub const TEAMS_TOKEN_RESOURCE: &str = AGENCY_COWORK_TOKEN_RESOURCE;
pub const TEAMS_CHATSVC_TOKEN_ENV: &str = "TEAMS_CHATSVC_TOKEN";

const AZURE_CLI_TIMEOUT: Duration = Duration::from_secs(15);
const MIN_VALIDITY_REUSE_SECONDS: i64 = 30;

// Refresh-buffer constants port the agency-cowork TokenManager defaults.
// Reference: skills/teams/scripts/api/auth.py L29-L32
pub(crate) const REFRESH_BUFFER_RATIO: f64 = 0.10;
pub(crate) const MIN_REFRESH_BUFFER_SECS: u64 = 60;
pub(crate) const MAX_REFRESH_BUFFER_SECS: u64 = 300;
pub const MAX_REFRESH_SLEEP: Duration = Duration::from_secs(MAX_REFRESH_BUFFER_SECS);

static CLI_BLOCKED_BY_TOKEN_PROTECTION: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamsTokenSource {
    Environment,
    DiskCache,
    AzureCli,
    BrowserSessionCache,
}

pub struct TeamsTokenAcquisition {
    pub token: TokenInfo,
    pub source: TeamsTokenSource,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TeamsTokenError {
    #[error("Teams token is malformed: {0}")]
    MalformedToken(String),
    #[error("Teams token is expired")]
    ExpiredToken,
    #[error(
        "no signed-in Teams token found. Checked TEAMS_CHATSVC_TOKEN and ~/.teams-agent/token-cache.json; Azure CLI fallback is not installed or not on PATH"
    )]
    AzureCliUnavailable,
    #[error("Azure CLI is not signed in; run az login")]
    AzureCliNotLoggedIn,
    #[error("Conditional Access Token Protection requires a signed-in Teams browser session")]
    TokenProtectionRequiresBrowserSession,
    #[error("Teams token cache is unavailable: {0}")]
    CacheUnavailable(String),
    #[error("Graph identity detection is unavailable: {0}")]
    GraphIdentityUnavailable(String),
    #[error("Teams token acquisition failed: {0}")]
    AcquisitionFailed(String),
}

impl TeamsTokenError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MalformedToken(_) => "MalformedToken",
            Self::ExpiredToken => "ExpiredToken",
            Self::AzureCliUnavailable => "AzureCliUnavailable",
            Self::AzureCliNotLoggedIn => "AzureCliNotLoggedIn",
            Self::TokenProtectionRequiresBrowserSession => "TokenProtectionRequiresBrowserSession",
            Self::CacheUnavailable(_) => "CacheUnavailable",
            Self::GraphIdentityUnavailable(_) => "GraphIdentityUnavailable",
            Self::AcquisitionFailed(_) => "AcquisitionFailed",
        }
    }
}

pub struct TeamsTokenProvider;

impl TeamsTokenProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn acquire_token(
        &self,
        force_refresh: bool,
    ) -> Result<TeamsTokenAcquisition, TeamsTokenError> {
        if !force_refresh {
            if let Some(token) = Self::token_from_process_env()? {
                return Ok(TeamsTokenAcquisition {
                    token,
                    source: TeamsTokenSource::Environment,
                });
            }

            if let Some(token) = read_agency_cowork_token(TEAMS_TOKEN_RESOURCE)
                .map_err(|error| TeamsTokenError::CacheUnavailable(error.to_string()))?
            {
                return Ok(TeamsTokenAcquisition {
                    token,
                    source: TeamsTokenSource::DiskCache,
                });
            }
        }

        let cli_error = match acquire_via_azure_cli().await {
            Ok(token) => {
                write_agency_cowork_token(TEAMS_TOKEN_RESOURCE, &token)
                    .map_err(|error| TeamsTokenError::CacheUnavailable(error.to_string()))?;
                return Ok(TeamsTokenAcquisition {
                    token,
                    source: TeamsTokenSource::AzureCli,
                });
            }
            Err(error) => error,
        };

        if let Some(token) = read_agency_cowork_token(TEAMS_TOKEN_RESOURCE)
            .map_err(|error| TeamsTokenError::CacheUnavailable(error.to_string()))?
        {
            return Ok(TeamsTokenAcquisition {
                token,
                source: TeamsTokenSource::BrowserSessionCache,
            });
        }

        match cli_error {
            TeamsTokenError::TokenProtectionRequiresBrowserSession => {
                Err(TeamsTokenError::TokenProtectionRequiresBrowserSession)
            }
            error => Err(error),
        }
    }

    pub fn token_from_process_env() -> Result<Option<TokenInfo>, TeamsTokenError> {
        let Some(token) = std::env::var_os(TEAMS_CHATSVC_TOKEN_ENV) else {
            return Ok(None);
        };
        let Some(token) = token.to_str() else {
            return Ok(None);
        };
        Self::valid_token_from_str(token)
    }

    pub fn valid_token_from_str(token: &str) -> Result<Option<TokenInfo>, TeamsTokenError> {
        match token_info_from_jwt(token) {
            Ok(token) if has_reuse_ttl(&token) => Ok(Some(token)),
            Ok(_) | Err(TeamsTokenError::ExpiredToken | TeamsTokenError::MalformedToken(_)) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

impl Default for TeamsTokenProvider {
    fn default() -> Self {
        Self::new()
    }
}

pub fn token_info_from_jwt(token: &str) -> Result<TokenInfo, TeamsTokenError> {
    let claims = decode_teams_jwt(token)?;
    let now = Utc::now();
    if claims.exp <= now {
        return Err(TeamsTokenError::ExpiredToken);
    }
    Ok(TokenInfo {
        access_token: token
            .trim()
            .strip_prefix("Bearer ")
            .unwrap_or(token.trim())
            .to_string(),
        expires_at: claims.exp,
        refresh_token: None,
    })
}

pub fn reset_token_protection_block_for_tests() {
    CLI_BLOCKED_BY_TOKEN_PROTECTION.store(false, Ordering::SeqCst);
}

async fn acquire_via_azure_cli() -> Result<TokenInfo, TeamsTokenError> {
    if CLI_BLOCKED_BY_TOKEN_PROTECTION.load(Ordering::SeqCst) {
        return Err(TeamsTokenError::TokenProtectionRequiresBrowserSession);
    }

    let output = tokio::time::timeout(
        AZURE_CLI_TIMEOUT,
        Command::new(azure_cli_binary())
            .args([
                "account",
                "get-access-token",
                "--resource",
                TEAMS_TOKEN_RESOURCE,
                "--query",
                "accessToken",
                "-o",
                "tsv",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| {
        TeamsTokenError::AcquisitionFailed("Azure CLI token request timed out".to_string())
    })?
    .map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            TeamsTokenError::AzureCliUnavailable
        } else {
            TeamsTokenError::AcquisitionFailed(format!("Azure CLI token request failed: {error}"))
        }
    })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let token = stdout.trim();
        if token.is_empty() {
            return Err(TeamsTokenError::AcquisitionFailed(
                "Azure CLI returned an empty access token".to_string(),
            ));
        }
        return token_info_from_jwt(token);
    }

    Err(classify_azure_cli_error(&String::from_utf8_lossy(
        &output.stderr,
    )))
}

pub fn classify_azure_cli_error(stderr: &str) -> TeamsTokenError {
    if stderr.contains("AADSTS530084") {
        CLI_BLOCKED_BY_TOKEN_PROTECTION.store(true, Ordering::SeqCst);
        TeamsTokenError::TokenProtectionRequiresBrowserSession
    } else if stderr.contains("Please run 'az login'")
        || stderr.contains("az login")
        || stderr.contains("not logged in")
    {
        TeamsTokenError::AzureCliNotLoggedIn
    } else {
        TeamsTokenError::AcquisitionFailed(safe_cli_error(stderr))
    }
}

fn has_reuse_ttl(token: &TokenInfo) -> bool {
    token
        .expires_at
        .signed_duration_since(Utc::now())
        .num_seconds()
        > MIN_VALIDITY_REUSE_SECONDS
}

fn safe_cli_error(stderr: &str) -> String {
    let compact = stderr.replace(['\r', '\n'], " ");
    let trimmed = compact.trim();
    if trimmed.is_empty() {
        "Azure CLI token request failed".to_string()
    } else {
        trimmed.chars().take(500).collect()
    }
}

fn azure_cli_binary() -> &'static str {
    if cfg!(windows) {
        "az.cmd"
    } else {
        azure_cli_binary_from_candidates(|path| std::path::Path::new(path).exists())
    }
}

fn azure_cli_binary_from_candidates(exists: impl Fn(&str) -> bool) -> &'static str {
    if exists("/opt/homebrew/bin/az") {
        "/opt/homebrew/bin/az"
    } else if exists("/usr/local/bin/az") {
        "/usr/local/bin/az"
    } else {
        "az"
    }
}

#[cfg(test)]
#[path = "token_provider_tests.rs"]
mod tests;
