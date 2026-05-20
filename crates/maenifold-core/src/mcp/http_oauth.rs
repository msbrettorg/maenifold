use crate::acp_config::HttpAuthConfig;
use crate::error::{MaenifoldError, Result};
use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::{ClientSecretCredential, DeveloperToolsCredential, ManagedIdentityCredential};
use serde::Deserialize;
use std::env;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::gateway_session::HttpServerConfig;
use super::http_oauth_client::{AzureHttpClient, GatewayHttpClient};

const NO_CACHED_SOURCE: usize = usize::MAX;

#[derive(Clone)]
pub(super) struct AzureDefaultCredential {
    sources: Arc<[AzureCredentialSource]>,
    cached_source: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct AzureCredentialSource {
    name: &'static str,
    credential: Arc<dyn TokenCredential>,
}

#[derive(Deserialize)]
struct ProtectedResourceMetadata {
    #[serde(default)]
    scopes_supported: Vec<String>,
}

impl fmt::Debug for AzureDefaultCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureDefaultCredential")
            .field(
                "sources",
                &self
                    .sources
                    .iter()
                    .map(|source| source.name)
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl AzureDefaultCredential {
    pub(super) fn new() -> Result<Self> {
        let mut sources = Vec::new();
        if let Some(env_credential) = env_client_secret_credential()? {
            sources.push(AzureCredentialSource {
                name: "environment",
                credential: env_credential,
            });
        }
        // Developer tools (az login / azd) BEFORE managed identity — on local
        // dev machines, ManagedIdentityCredential hangs for 30+ seconds trying
        // IMDS at 169.254.169.254 which doesn't exist outside Azure VMs.
        let developer_tools = DeveloperToolsCredential::new(None)
            .map_err(|error| azure_auth_error(error.to_string()))?;
        sources.push(AzureCredentialSource {
            name: "developer-tools",
            credential: developer_tools,
        });
        if let Ok(credential) = ManagedIdentityCredential::new(None) {
            sources.push(AzureCredentialSource {
                name: "managed-identity",
                credential,
            });
        }
        Ok(Self {
            sources: sources.into(),
            cached_source: Arc::new(AtomicUsize::new(NO_CACHED_SOURCE)),
        })
    }

    pub(super) async fn get_token(&self, scope: &str) -> Result<String> {
        if let Some(token) = self.token_from_cached_source(scope).await? {
            return Ok(token);
        }
        let mut errors = Vec::new();
        for (index, source) in self.sources.iter().enumerate() {
            match source.credential.get_token(&[scope], None).await {
                Ok(token) => {
                    self.cached_source.store(index, Ordering::Relaxed);
                    return Ok(token.token.secret().to_string());
                }
                Err(error) => errors.push(format!("{}: {error}", source.name)),
            }
        }
        Err(azure_auth_error(format!(
            "no Azure credential source produced a token for scope '{scope}': {}",
            errors.join(" | ")
        )))
    }

    async fn token_from_cached_source(&self, scope: &str) -> Result<Option<String>> {
        let cached = self.cached_source.load(Ordering::Relaxed);
        if cached == NO_CACHED_SOURCE {
            return Ok(None);
        }
        let Some(source) = self.sources.get(cached) else {
            self.cached_source
                .store(NO_CACHED_SOURCE, Ordering::Relaxed);
            return Ok(None);
        };
        match source.credential.get_token(&[scope], None).await {
            Ok(token) => Ok(Some(token.token.secret().to_string())),
            Err(_) => {
                self.cached_source
                    .store(NO_CACHED_SOURCE, Ordering::Relaxed);
                Ok(None)
            }
        }
    }
}

pub(super) struct PreparedHttpClient {
    pub(super) client: GatewayHttpClient,
    pub(super) auth_header: Option<String>,
}

pub(super) async fn prepare_http_client(
    config: &HttpServerConfig,
    http_client: reqwest::Client,
) -> Result<PreparedHttpClient> {
    let Some(auth) = config.auth.as_ref() else {
        return Ok(PreparedHttpClient {
            client: GatewayHttpClient::Plain(http_client),
            auth_header: None,
        });
    };
    match auth {
        HttpAuthConfig::AzureDefault => {
            let scope = discover_scope(&http_client, &config.url).await?;
            let credential = AzureDefaultCredential::new()?;
            let token = credential.get_token(&scope).await?;
            Ok(PreparedHttpClient {
                client: GatewayHttpClient::Azure(AzureHttpClient::new(
                    http_client,
                    credential,
                    scope,
                )),
                auth_header: Some(token),
            })
        }
    }
}

async fn discover_scope(http_client: &reqwest::Client, base_url: &str) -> Result<String> {
    let metadata_url = protected_resource_metadata_url(base_url)?;
    let metadata = http_client
        .get(metadata_url.clone())
        .send()
        .await
        .map_err(|error| {
            azure_auth_error(format!(
                "failed to fetch protected-resource metadata from '{metadata_url}': {error}"
            ))
        })?
        .error_for_status()
        .map_err(|error| {
            azure_auth_error(format!(
                "protected-resource metadata request failed for '{metadata_url}': {error}"
            ))
        })?
        .json::<ProtectedResourceMetadata>()
        .await
        .map_err(|error| {
            azure_auth_error(format!(
                "failed to parse protected-resource metadata from '{metadata_url}': {error}"
            ))
        })?;
    metadata.scopes_supported.into_iter().next().ok_or_else(|| {
        azure_auth_error(format!(
            "protected-resource metadata at '{metadata_url}' did not include scopes_supported"
        ))
    })
}

fn protected_resource_metadata_url(base_url: &str) -> Result<reqwest::Url> {
    reqwest::Url::parse(&format!(
        "{}/.well-known/oauth-protected-resource/",
        base_url.trim_end_matches('/')
    ))
    .map_err(|error| azure_auth_error(format!("invalid HTTP MCP URL '{base_url}': {error}")))
}

fn env_client_secret_credential() -> Result<Option<Arc<dyn TokenCredential>>> {
    let client_id = env::var("AZURE_CLIENT_ID").ok();
    let tenant_id = env::var("AZURE_TENANT_ID").ok();
    let client_secret = env::var("AZURE_CLIENT_SECRET").ok();
    match (client_id, tenant_id, client_secret) {
        (Some(client_id), Some(tenant_id), Some(client_secret)) => ClientSecretCredential::new(
            &tenant_id,
            client_id,
            Secret::new(client_secret),
            None,
        )
        .map(|credential| Some(credential as Arc<dyn TokenCredential>))
        .map_err(|error| azure_auth_error(error.to_string())),
        (None, None, None) => Ok(None),
        _ => Err(azure_auth_error(
            "AZURE_CLIENT_ID, AZURE_TENANT_ID, and AZURE_CLIENT_SECRET must all be set to enable environment-based Azure auth".to_string(),
        )),
    }
}

fn azure_auth_error(message: String) -> MaenifoldError {
    MaenifoldError::Message(format!("HTTP MCP Azure auth failed: {message}"))
}
