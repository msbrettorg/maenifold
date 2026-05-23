use std::{env, path::PathBuf, time::Duration};

use anyhow::Context;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;

use super::{CopilotModel, CopilotModelsEnvelope};

const DEFAULT_ENDPOINT: &str = "https://api.githubcopilot.com";
const DEFAULT_OAUTH_DOMAIN: &str = "github.com";
#[derive(Clone, Debug)]
pub struct CopilotEndpoint {
    pub api: String,
}

pub(super) struct CopilotModelsFetch {
    pub models: Vec<CopilotModel>,
    pub refresh_hint_at: Option<DateTime<Utc>>,
}

pub struct CopilotEndpointResolver {
    http: Client,
}

pub struct CopilotModelsClient {
    http: Client,
}

#[derive(Debug, thiserror::Error)]
pub enum CopilotEndpointResolverError {
    #[error("GraphQL endpoint network error: {0}")]
    Network(reqwest::Error),
    #[error("GraphQL endpoint timed out: {0}")]
    Timeout(reqwest::Error),
    #[error("GraphQL endpoint request failed: {0}")]
    Other(reqwest::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CopilotModelsClientError {
    #[error(transparent)]
    Endpoint(#[from] CopilotEndpointResolverError),
    #[error("Copilot endpoint returned 401")]
    Unauthorized,
    #[error("Copilot endpoint returned 403")]
    Forbidden,
    #[error("Copilot endpoint returned 404")]
    EndpointNotFound,
    #[error("Copilot endpoint returned 429")]
    RateLimited,
    #[error("Copilot endpoint returned server error {status}")]
    ServerError { status: StatusCode },
    #[error("Copilot endpoint returned unexpected status {status}")]
    UnexpectedStatus { status: StatusCode },
    #[error("Copilot models parse failed: {0}")]
    Parse(serde_json::Error),
    #[error("Copilot models network error: {0}")]
    Network(reqwest::Error),
    #[error("Copilot models timed out: {0}")]
    Timeout(reqwest::Error),
    #[error("Copilot models request failed: {0}")]
    Other(reqwest::Error),
}

impl CopilotEndpointResolver {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    pub async fn resolve(
        &self,
        access_token: &str,
        enterprise_uri: Option<&str>,
    ) -> std::result::Result<CopilotEndpoint, CopilotEndpointResolverError> {
        let graphql_url = graphql_url_for_enterprise_uri(enterprise_uri);
        let response = self
            .http
            .post(graphql_url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "query": "query { viewer { copilotEndpoints { api } } }" }))
            .send()
            .await
            .map_err(classify_endpoint_error)?;
        let body: GraphQlEnvelope = response.json().await.map_err(classify_endpoint_error)?;
        let api = body
            .data
            .and_then(|data| data.viewer.copilot_endpoints.api)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());
        Ok(CopilotEndpoint { api })
    }
}

impl CopilotModelsClient {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    pub async fn fetch_models(
        &self,
        endpoint: &str,
        access_token: &str,
    ) -> std::result::Result<CopilotModelsFetch, CopilotModelsClientError> {
        let response = self
            .http
            .get(format!("{}/models", endpoint.trim_end_matches('/')))
            .bearer_auth(access_token)
            .header(
                "Editor-Version",
                format!("maenifold/{}", env!("CARGO_PKG_VERSION")),
            )
            .header("X-Github-Api-Version", "2025-04-01")
            .send()
            .await
            .map_err(classify_models_error)?;
        let status = response.status();
        if status == StatusCode::FORBIDDEN {
            return Err(CopilotModelsClientError::Forbidden);
        }
        if !status.is_success() {
            return Err(match status {
                StatusCode::UNAUTHORIZED => CopilotModelsClientError::Unauthorized,
                StatusCode::NOT_FOUND => CopilotModelsClientError::EndpointNotFound,
                StatusCode::TOO_MANY_REQUESTS => CopilotModelsClientError::RateLimited,
                status if status.is_server_error() => CopilotModelsClientError::ServerError {
                    status,
                },
                status => CopilotModelsClientError::UnexpectedStatus { status },
            });
        }
        let refresh_hint_at = response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_max_age)
            .map(|seconds| Utc::now() + chrono::Duration::seconds(seconds));
        let body = response.text().await.map_err(classify_models_error)?;
        let envelope = serde_json::from_str::<CopilotModelsEnvelope>(&body)
            .map_err(CopilotModelsClientError::Parse)?;
        Ok(CopilotModelsFetch {
            models: envelope.data,
            refresh_hint_at,
        })
    }
}

pub fn build_http_client() -> anyhow::Result<Client> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent("maenifold-copilot-models")
        .build()
        .context("failed to build Copilot models HTTP client")
}

pub fn default_endpoint() -> CopilotEndpoint {
    CopilotEndpoint {
        api: DEFAULT_ENDPOINT.to_string(),
    }
}

pub fn oauth_domain_for_enterprise_uri(enterprise_uri: Option<&str>) -> String {
    enterprise_uri.map(parse_domain).unwrap_or_else(|| DEFAULT_OAUTH_DOMAIN.to_string())
}

fn graphql_url_for_enterprise_uri(enterprise_uri: Option<&str>) -> String {
    enterprise_uri
        .map(parse_domain)
        .map(|domain| format!("https://{domain}/api/graphql"))
        .unwrap_or_else(|| "https://api.github.com/graphql".to_string())
}

fn parse_domain(enterprise_uri: &str) -> String {
    enterprise_uri
        .trim_end_matches('/')
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(DEFAULT_OAUTH_DOMAIN)
        .to_string()
}

fn parse_max_age(cache_control: &str) -> Option<i64> {
    cache_control.split(',').find_map(|part| {
        let mut kv = part.trim().splitn(2, '=');
        let key = kv.next()?.trim();
        let value = kv.next()?.trim();
        (key == "max-age").then(|| value.parse::<i64>().ok()).flatten()
    })
}

fn classify_endpoint_error(error: reqwest::Error) -> CopilotEndpointResolverError {
    if error.is_timeout() {
        CopilotEndpointResolverError::Timeout(error)
    } else if error.is_connect() || error.is_request() {
        CopilotEndpointResolverError::Network(error)
    } else {
        CopilotEndpointResolverError::Other(error)
    }
}

fn classify_models_error(error: reqwest::Error) -> CopilotModelsClientError {
    if error.is_timeout() {
        CopilotModelsClientError::Timeout(error)
    } else if error.is_connect() || error.is_request() {
        CopilotModelsClientError::Network(error)
    } else {
        CopilotModelsClientError::Other(error)
    }
}

pub(crate) fn copilot_config_dir() -> anyhow::Result<PathBuf> {
    let home = env::var_os("HOME").context("HOME environment variable missing")?;
    Ok(PathBuf::from(home).join(".config/github-copilot"))
}

#[derive(Deserialize)]
struct GraphQlEnvelope {
    data: Option<GraphQlData>,
}

#[derive(Deserialize)]
struct GraphQlData {
    viewer: GraphQlViewer,
}

#[derive(Deserialize)]
struct GraphQlViewer {
    #[serde(rename = "copilotEndpoints")]
    copilot_endpoints: GraphQlCopilotEndpoints,
}

#[derive(Deserialize)]
struct GraphQlCopilotEndpoints {
    api: Option<String>,
}
