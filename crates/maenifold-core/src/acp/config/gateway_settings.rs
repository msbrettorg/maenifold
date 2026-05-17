use serde::{Deserialize, Serialize};

use super::types::McpServerEntry;

fn default_model() -> String {
    "gpt-5.4-mini".to_string()
}

fn default_threshold() -> f64 {
    0.9
}

fn default_catalog_chars() -> u32 {
    24000
}

fn default_api_version() -> String {
    "2024-10-21".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySettings {
    #[serde(default)]
    pub intent_resolver: IntentResolverSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentResolverSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_candidates: Option<u32>,
    #[serde(default)]
    pub auto_execute_read_safe: bool,
    #[serde(default = "default_threshold")]
    pub auto_execute_confidence_threshold: f64,
    #[serde(default = "default_catalog_chars")]
    pub max_catalog_characters: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
    /// Optional explicit direct-inference config. When absent the resolver
    /// uses env-var fallbacks (AZURE_OPENAI_ENDPOINT / AZURE_OPENAI_DEPLOYMENT_NAME
    /// / AZURE_OPENAI_API_KEY) so most setups need nothing here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direct_inference: Option<DirectInferenceSettings>,
}

impl IntentResolverSettings {
    pub fn effective_thought_level(&self) -> Option<&str> {
        self.thought_level
            .as_deref()
            .filter(|level| !level.eq_ignore_ascii_case("none"))
    }

    /// Resolve the effective direct-inference config from explicit settings
    /// plus env-var fallbacks. Returns Err with a setup message if anything
    /// required is still missing.
    pub fn resolved_direct_inference(&self) -> std::result::Result<ResolvedInference, String> {
        let explicit = self.direct_inference.clone().unwrap_or_default();
        let endpoint = explicit
            .endpoint
            .or_else(|| std::env::var("AZURE_OPENAI_ENDPOINT").ok())
            .or_else(|| std::env::var("OPENAI_BASE_URL").ok())
            .ok_or_else(|| {
                "set gateway.intentResolver.directInference.endpoint OR \
                 AZURE_OPENAI_ENDPOINT / OPENAI_BASE_URL env var"
                    .to_string()
            })?;
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let provider = explicit
            .provider
            .unwrap_or_else(|| infer_provider(&endpoint));
        let api_key_env = explicit.api_key_env.unwrap_or_else(|| match provider.as_str() {
            "azureOpenAI" => "AZURE_OPENAI_API_KEY".to_string(),
            _ => "OPENAI_API_KEY".to_string(),
        });
        let api_key = std::env::var(&api_key_env).map_err(|_| {
            format!("env var {api_key_env} is not set; required for direct inference")
        })?;
        if api_key.trim().is_empty() {
            return Err(format!("env var {api_key_env} is empty"));
        }
        let deployment = explicit
            .deployment
            .or_else(|| std::env::var("AZURE_OPENAI_CHAT_DEPLOYMENT_NAME").ok())
            .or_else(|| std::env::var("AZURE_OPENAI_DEPLOYMENT_NAME").ok())
            .unwrap_or_else(|| self.model.clone());
        let api_version = explicit.api_version.unwrap_or_else(default_api_version);
        Ok(ResolvedInference {
            provider,
            endpoint,
            deployment,
            api_version,
            api_key,
        })
    }
}

impl Default for IntentResolverSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            model: default_model(),
            thought_level: None,
            timeout_seconds: None,
            max_candidates: None,
            auto_execute_read_safe: false,
            auto_execute_confidence_threshold: default_threshold(),
            max_catalog_characters: default_catalog_chars(),
            mcp_servers: Vec::new(),
            direct_inference: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DirectInferenceSettings {
    /// "azureOpenAI" | "openai". Inferred from endpoint URL when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Base URL. Defaults to AZURE_OPENAI_ENDPOINT / OPENAI_BASE_URL env var.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// Azure deployment name (or OpenAI model). Defaults to
    /// AZURE_OPENAI_CHAT_DEPLOYMENT_NAME / AZURE_OPENAI_DEPLOYMENT_NAME env var,
    /// then IntentResolverSettings.model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<String>,
    /// Azure api-version. Defaults to 2024-10-21.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// Env var holding the API key. Defaults: AZURE_OPENAI_API_KEY (azure)
    /// or OPENAI_API_KEY (openai).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
}

/// Fully resolved direct-inference config ready to make an HTTP call.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedInference {
    pub provider: String,
    pub endpoint: String,
    pub deployment: String,
    pub api_version: String,
    pub api_key: String,
}

fn infer_provider(endpoint: &str) -> String {
    if endpoint.contains("azure.com") || endpoint.contains("azure-api.net") {
        "azureOpenAI".to_string()
    } else {
        "openai".to_string()
    }
}
