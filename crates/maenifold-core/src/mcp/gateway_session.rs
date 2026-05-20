use crate::acp_config::HttpAuthConfig;
use crate::error::{MaenifoldError, Result};
use crate::mcp::PendingRequestRegistry;
use crate::mcp::http_session::HttpSession;
use crate::mcp::stdio_pool::StdioServerConfig;
use crate::mcp::stdio_session::{StdioSession, StdioStartupError};
use agent_client_protocol_schema::{EnvVariable, McpServerStdio as McpServerStdioConfig};
use reqwest::header::{HeaderName, HeaderValue};
use rmcp::model::{CallToolResult, GetPromptResult, Prompt, ReadResourceResult, Tool};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct HttpServerConfig {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) auth: Option<HttpAuthConfig>,
    pub(crate) timeout: Duration,
}

#[derive(Debug, Clone)]
pub(crate) enum GatewayServerConfig {
    Stdio(StdioServerConfig),
    Http(HttpServerConfig),
}

impl GatewayServerConfig {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Stdio(config) => &config.name,
            Self::Http(config) => &config.name,
        }
    }
}

impl From<StdioServerConfig> for GatewayServerConfig {
    fn from(config: StdioServerConfig) -> Self {
        Self::Stdio(config)
    }
}

#[derive(Debug)]
pub(crate) struct GatewayStartupError {
    pub(crate) error: MaenifoldError,
    pub(crate) stderr: Option<String>,
}

impl GatewayStartupError {
    pub(crate) fn new(error: MaenifoldError) -> Self {
        Self {
            error,
            stderr: None,
        }
    }
}

pub(crate) enum GatewaySession {
    Stdio(StdioSession),
    Http(HttpSession),
}

impl GatewaySession {
    pub(crate) fn tools(&self) -> Arc<[Tool]> {
        match self {
            Self::Stdio(session) => session.tools(),
            Self::Http(session) => session.tools(),
        }
    }

    pub(crate) fn prompts(&self) -> Arc<[Prompt]> {
        match self {
            Self::Stdio(session) => session.prompts(),
            Self::Http(session) => session.prompts(),
        }
    }

    pub(crate) fn initialize_description(&self) -> Option<String> {
        match self {
            Self::Stdio(session) => session.initialize_description(),
            Self::Http(session) => session.initialize_description(),
        }
    }

    pub(crate) fn is_usable(&self) -> bool {
        match self {
            Self::Stdio(session) => session.is_usable(),
            Self::Http(session) => session.is_usable(),
        }
    }

    pub(crate) async fn shutdown(&self) {
        match self {
            Self::Stdio(session) => session.shutdown().await,
            Self::Http(session) => session.shutdown().await,
        }
    }

    pub(crate) async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        match self {
            Self::Stdio(session) => session.call_tool(name, arguments).await,
            Self::Http(session) => session.call_tool(name, arguments).await,
        }
    }

    pub(crate) async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        match self {
            Self::Stdio(session) => session.read_resource(uri).await,
            Self::Http(session) => session.read_resource(uri).await,
        }
    }

    pub(crate) async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<GetPromptResult> {
        match self {
            Self::Stdio(session) => session.get_prompt(name, arguments).await,
            Self::Http(session) => session.get_prompt(name, arguments).await,
        }
    }
}

pub(crate) async fn start_session_with_registry(
    config: &GatewayServerConfig,
    registry: Arc<PendingRequestRegistry>,
) -> std::result::Result<GatewaySession, GatewayStartupError> {
    match config {
        GatewayServerConfig::Stdio(config) => start_stdio_session_with_registry(config, registry)
            .await
            .map(GatewaySession::Stdio),
        GatewayServerConfig::Http(config) => HttpSession::start_with_registry(config, registry)
            .await
            .map(GatewaySession::Http),
    }
}

impl HttpServerConfig {
    pub(crate) fn custom_headers(&self) -> Result<HashMap<HeaderName, HeaderValue>> {
        self.headers
            .iter()
            .map(|(name, value)| {
                let name = HeaderName::try_from(name.as_str()).map_err(|error| {
                    MaenifoldError::Message(format!(
                        "invalid HTTP header name for gateway server '{}': {error}",
                        self.name
                    ))
                })?;
                let value = HeaderValue::from_str(value).map_err(|error| {
                    MaenifoldError::Message(format!(
                        "invalid HTTP header value for gateway server '{}': {error}",
                        self.name
                    ))
                })?;
                Ok((name, value))
            })
            .collect()
    }
}

async fn start_stdio_session_with_registry(
    config: &StdioServerConfig,
    registry: Arc<PendingRequestRegistry>,
) -> std::result::Result<StdioSession, GatewayStartupError> {
    let entry = McpServerStdioConfig::new(config.name.clone(), config.command.clone())
        .args(config.args.clone())
        .env(
            config
                .env
                .iter()
                .map(|(name, value)| EnvVariable::new(name.clone(), value.clone()))
                .collect(),
        );
    StdioSession::start_with_registry(
        &entry,
        config.timeout.unwrap_or(Duration::from_secs(60)),
        registry,
    )
    .await
    .map_err(map_stdio_startup_error)
}

fn map_stdio_startup_error(error: StdioStartupError) -> GatewayStartupError {
    GatewayStartupError {
        error: error.error,
        stderr: error.stderr,
    }
}
