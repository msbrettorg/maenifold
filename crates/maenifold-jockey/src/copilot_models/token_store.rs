use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::fs;

#[derive(Clone, Debug)]
pub struct CopilotToken {
    pub oauth_domain: String,
    pub access_token: String,
    pub fingerprint: String,
}

pub struct CopilotTokenStore {
    config_dir: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum CopilotTokenStoreError {
    #[error("Copilot token missing for {oauth_domain}")]
    TokenMissing { oauth_domain: String },
    #[error("Copilot token expired for {oauth_domain}")]
    TokenExpired { oauth_domain: String },
    #[error("Copilot hosts.json missing at {path}")]
    HostsJsonMissing { path: PathBuf },
    #[error("Malformed Copilot JSON at {path}: {source}")]
    MalformedJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Copilot token I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl CopilotTokenStore {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub async fn load_token(
        &self,
        oauth_domain: &str,
    ) -> Result<CopilotToken, CopilotTokenStoreError> {
        let hosts_path = self.config_dir.join("hosts.json");
        let apps_path = self.config_dir.join("apps.json");
        let mut hosts_missing = false;
        for path in [&hosts_path, &apps_path] {
            match self.load_token_from_file(path, oauth_domain).await? {
                Some(token) => return Ok(token),
                None if path == &hosts_path && !path.exists() => {
                    hosts_missing = true;
                }
                None => {}
            }
        }
        if hosts_missing && !apps_path.exists() {
            return Err(CopilotTokenStoreError::HostsJsonMissing { path: hosts_path });
        }
        Err(CopilotTokenStoreError::TokenMissing {
            oauth_domain: oauth_domain.to_string(),
        })
    }

    async fn load_token_from_file(
        &self,
        path: &Path,
        oauth_domain: &str,
    ) -> Result<Option<CopilotToken>, CopilotTokenStoreError> {
        let contents = match fs::read_to_string(path).await {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(CopilotTokenStoreError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let entries: Map<String, Value> = serde_json::from_str(&contents).map_err(|source| {
            CopilotTokenStoreError::MalformedJson {
                path: path.to_path_buf(),
                source,
            }
        })?;
        for (key, value) in entries {
            if !key.starts_with(oauth_domain) {
                continue;
            }
            let Some(record) = value.as_object() else {
                continue;
            };
            let Some(access_token) = record
                .get("oauth_token")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if is_expired(record) {
                return Err(CopilotTokenStoreError::TokenExpired {
                    oauth_domain: oauth_domain.to_string(),
                });
            }
            return Ok(Some(CopilotToken {
                oauth_domain: oauth_domain.to_string(),
                access_token: access_token.to_string(),
                fingerprint: token_fingerprint(oauth_domain, access_token),
            }));
        }
        Ok(None)
    }
}

fn is_expired(record: &Map<String, Value>) -> bool {
    ["expires_at", "expiresAt", "expiry", "expires"]
        .into_iter()
        .find_map(|key| record.get(key))
        .and_then(expiry_to_datetime)
        .is_some_and(|expiry| expiry <= Utc::now())
}

fn expiry_to_datetime(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(raw) => DateTime::parse_from_rfc3339(raw)
            .ok()
            .map(|value| value.with_timezone(&Utc)),
        Value::Number(number) => {
            let raw = number.as_i64()?;
            let seconds = if raw > 1_000_000_000_000 { raw / 1000 } else { raw };
            DateTime::<Utc>::from_timestamp(seconds, 0)
        }
        _ => None,
    }
}

fn token_fingerprint(oauth_domain: &str, access_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(oauth_domain.as_bytes());
    hasher.update(b":");
    hasher.update(access_token.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
