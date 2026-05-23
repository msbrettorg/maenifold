mod client;
mod config;
mod support;
mod token_store;

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;

pub use token_store::{CopilotToken, CopilotTokenStore, CopilotTokenStoreError};

use client::{CopilotEndpointResolver, CopilotModelsClient};
use support::{filter_models, map_client_error, map_token_error};

pub type CopilotModelBilling = serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CopilotModelPolicy {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CopilotModelsEnvelope {
    pub data: Vec<CopilotModel>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CopilotModel {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub billing: CopilotModelBilling,
    pub capabilities: CopilotModelCapabilities,
    pub policy: Option<CopilotModelPolicy>,
    pub is_chat_default: bool,
    pub is_chat_fallback: bool,
    pub model_picker_enabled: bool,
    #[serde(default)]
    pub supported_endpoints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CopilotModelCapabilities {
    pub family: String,
    #[serde(default)]
    pub limits: CopilotModelLimits,
    pub supports: CopilotModelSupportedFeatures,
    #[serde(rename = "type")]
    pub model_type: String,
    #[serde(default)]
    pub tokenizer: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CopilotModelLimits {
    #[serde(default)]
    pub max_context_window_tokens: u32,
    #[serde(default)]
    pub max_output_tokens: u32,
    #[serde(default)]
    pub max_prompt_tokens: u64,
    #[serde(default)]
    pub max_non_streaming_output_tokens: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CopilotModelSupportedFeatures {
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub tool_calls: bool,
    #[serde(default)]
    pub parallel_tool_calls: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub thinking: bool,
    #[serde(default)]
    pub adaptive_thinking: bool,
    #[serde(default)]
    pub max_thinking_budget: Option<u32>,
    #[serde(default)]
    pub min_thinking_budget: Option<u32>,
    #[serde(default)]
    pub reasoning_effort: Vec<String>,
    #[serde(default)]
    pub structured_outputs: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetCopilotModelsResult {
    pub models: Vec<CopilotModel>,
    pub cached_at: DateTime<Utc>,
    pub source: CopilotModelsSource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CopilotModelsSource {
    Live,
    Cache,
}

#[derive(Debug, thiserror::Error)]
pub enum GetCopilotModelsError {
    #[error("Copilot token missing")]
    TokenMissing,
    #[error("Copilot endpoint unreachable")]
    EndpointUnreachable,
    #[error("Copilot models payload parse failed")]
    ParseFailed,
    #[error("Copilot models request forbidden")]
    Forbidden,
    #[error("Copilot models request failed")]
    Other,
}

#[derive(Clone, Debug, Default)]
pub struct CopilotModelsCache {
    pub entries: HashMap<String, CopilotModelsCacheEntry>,
}

#[derive(Clone, Debug)]
pub struct CopilotModelsCacheEntry {
    pub endpoint: String,
    pub fingerprint: String,
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub models: Vec<CopilotModel>,
}

pub type SharedCopilotModelsCache = Arc<RwLock<CopilotModelsCache>>;

static COPILOT_MODELS_CACHE: Lazy<SharedCopilotModelsCache> =
    Lazy::new(|| Arc::new(RwLock::new(CopilotModelsCache::default())));

pub struct CopilotModelsServices {
    token_store: CopilotTokenStore,
    endpoint_resolver: CopilotEndpointResolver,
    models_client: CopilotModelsClient,
    cache: SharedCopilotModelsCache,
    enterprise_uri: Option<String>,
}

impl CopilotModelsServices {
    pub fn for_config_root(config_root: PathBuf) -> anyhow::Result<Self> {
        let config_dir = client::copilot_config_dir()?;
        let http = client::build_http_client()?;
        Ok(Self {
            token_store: CopilotTokenStore::new(config_dir),
            endpoint_resolver: CopilotEndpointResolver::new(http.clone()),
            models_client: CopilotModelsClient::new(http),
            cache: Arc::clone(&COPILOT_MODELS_CACHE),
            enterprise_uri: config::load_copilot_enterprise_uri(&config_root),
        })
    }
}

pub async fn get_copilot_models(
    services: &CopilotModelsServices,
    refresh: bool,
) -> Result<GetCopilotModelsResult, GetCopilotModelsError> {
    let oauth_domain =
        client::oauth_domain_for_enterprise_uri(services.enterprise_uri.as_deref());
    let token = services
        .token_store
        .load_token(&oauth_domain)
        .await
        .map_err(map_token_error)?;
    let endpoint = services
        .endpoint_resolver
        .resolve(&token.access_token, services.enterprise_uri.as_deref())
        .await
        .unwrap_or_else(|error| {
            warn!(error = %error, "Copilot endpoint discovery failed; using default endpoint");
            client::default_endpoint()
        });
    if let Some(result) = cached_result(services, &endpoint.api, &token.fingerprint, refresh).await {
        return Ok(result);
    }
    match services
        .models_client
        .fetch_models(&endpoint.api, &token.access_token)
        .await
    {
        Ok(fetch) => {
            let cached_at = Utc::now();
            let filtered = filter_models(fetch.models);
            write_cache(
                services,
                endpoint.api.clone(),
                token.fingerprint,
                cached_at,
                fetch.expires_at,
                filtered.clone(),
            )
            .await;
            Ok(GetCopilotModelsResult {
                models: filtered,
                cached_at,
                source: CopilotModelsSource::Live,
            })
        }
        Err(error) => {
            warn!(error = %error, "Copilot models refresh failed");
            stale_result(services, &endpoint.api)
                .await
                .ok_or_else(|| map_client_error(&error))
        }
    }
}

async fn cached_result(
    services: &CopilotModelsServices,
    endpoint: &str,
    fingerprint: &str,
    refresh: bool,
) -> Option<GetCopilotModelsResult> {
    let cache = services.cache.read().await;
    let entry = cache.entries.get(endpoint)?;
    if refresh || entry.fingerprint != fingerprint || entry.expires_at <= Utc::now() {
        return None;
    }
    Some(GetCopilotModelsResult {
        models: entry.models.clone(),
        cached_at: entry.cached_at,
        source: CopilotModelsSource::Cache,
    })
}

async fn stale_result(
    services: &CopilotModelsServices,
    endpoint: &str,
) -> Option<GetCopilotModelsResult> {
    let cache = services.cache.read().await;
    let entry = cache.entries.get(endpoint)?;
    Some(GetCopilotModelsResult {
        models: entry.models.clone(),
        cached_at: entry.cached_at,
        source: CopilotModelsSource::Cache,
    })
}

async fn write_cache(
    services: &CopilotModelsServices,
    endpoint: String,
    fingerprint: String,
    cached_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    models: Vec<CopilotModel>,
) {
    let mut cache = services.cache.write().await;
    cache.entries.insert(
        endpoint.clone(),
        CopilotModelsCacheEntry {
            endpoint,
            fingerprint,
            cached_at,
            expires_at: expires_at.max(cached_at + Duration::hours(6)),
            models,
        },
    );
}
