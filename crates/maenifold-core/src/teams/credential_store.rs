use chrono::{DateTime, Utc};
use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};

use crate::error::MaenifoldError;

use super::TokenInfo;

const SERVICE_NAME: &str = "maenifold-teams";

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

/// Stores a Teams OAuth token in the OS credential store.
pub fn store_token(
    client_id: &str,
    tenant_id: &str,
    token: &TokenInfo,
) -> Result<(), MaenifoldError> {
    let token_json = serde_json::to_string(&StoredToken::from(token))?;
    credential_entry(client_id, tenant_id)?
        .set_password(&token_json)
        .map_err(map_keyring_error)
}

/// Retrieves a Teams OAuth token from the OS credential store.
///
/// Returns `Ok(None)` when no credential exists for the client and tenant.
pub fn retrieve_token(
    client_id: &str,
    tenant_id: &str,
) -> Result<Option<TokenInfo>, MaenifoldError> {
    let token_json = match credential_entry(client_id, tenant_id)?.get_password() {
        Ok(token_json) => token_json,
        Err(KeyringError::NoEntry) => return Ok(None),
        Err(err) => return Err(map_keyring_error(err)),
    };

    let token = serde_json::from_str::<StoredToken>(&token_json)?;
    Ok(Some(token.into()))
}

/// Deletes a Teams OAuth token from the OS credential store.
///
/// Deleting a non-existent credential is treated as success.
pub fn delete_token(client_id: &str, tenant_id: &str) -> Result<(), MaenifoldError> {
    match credential_entry(client_id, tenant_id)?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(map_keyring_error(err)),
    }
}

fn credential_entry(client_id: &str, tenant_id: &str) -> Result<Entry, MaenifoldError> {
    let account_key = format!("{client_id}:{tenant_id}");
    Entry::new(SERVICE_NAME, &account_key).map_err(map_keyring_error)
}

fn map_keyring_error(err: KeyringError) -> MaenifoldError {
    MaenifoldError::Message(format!("Teams credential store error: {err}"))
}
