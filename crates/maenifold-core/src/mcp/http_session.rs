use crate::error::{GATEWAY_TIMEOUT_ERROR, JSONRPC_INTERNAL_ERROR, MaenifoldError, Result};
use crate::mcp::MaenifoldClient;
use crate::mcp::PendingRequestRegistry;
use crate::mcp::gateway_metadata;
use rmcp::{
    Peer, RoleClient, ServiceExt,
    model::{
        CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, GetPromptRequest,
        GetPromptRequestParams, GetPromptResult, Prompt, ProtocolVersion, ReadResourceRequest,
        ReadResourceRequestParams, ReadResourceResult, ServerResult, Tool,
    },
    service::{PeerRequestOptions, RunningService, ServiceError},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use super::gateway_session::{GatewayStartupError, HttpServerConfig};
use super::http_oauth::prepare_http_client;

const MCP_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2025_03_26;
const LEGACY_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2024_11_05;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

type RmcpClient = RunningService<RoleClient, MaenifoldClient>;

pub(super) struct HttpSession {
    client: StdMutex<Option<RmcpClient>>,
    peer: Peer<RoleClient>,
    tools: Arc<[Tool]>,
    prompts: Arc<[Prompt]>,
    initialize_description: Option<String>,
    call_timeout: Duration,
    server_name: String,
    dead: AtomicBool,
}

impl HttpSession {
    pub(super) async fn start_with_registry(
        config: &HttpServerConfig,
        registry: Arc<PendingRequestRegistry>,
    ) -> std::result::Result<Self, GatewayStartupError> {
        let startup = async move {
            let http_client = reqwest::Client::builder()
                .timeout(config.timeout)
                .build()
                .map_err(|error| MaenifoldError::Message(error.to_string()))?;
            let prepared_client = prepare_http_client(config, http_client).await?;
            let mut transport_config =
                StreamableHttpClientTransportConfig::with_uri(config.url.clone())
                    .custom_headers(config.custom_headers()?);
            if let Some(auth_header) = prepared_client.auth_header {
                transport_config = transport_config.auth_header(auth_header);
            }
            let transport = StreamableHttpClientTransport::with_client(
                prepared_client.client,
                transport_config,
            );
            let mut client = MaenifoldClient::new_with_server_name(registry, &config.name)
                .serve(transport)
                .await
                .map_err(|error| MaenifoldError::Message(error.to_string()))?;
            let server_info = client.peer().peer_info().cloned().ok_or_else(|| {
                MaenifoldError::Message("MCP server did not return initialize result".to_string())
            })?;
            if !is_supported_version(&server_info.protocol_version) {
                let _ = client.close_with_timeout(SHUTDOWN_WAIT).await;
                return Err(MaenifoldError::Message(format!(
                    "unsupported MCP protocol version '{}'",
                    server_info.protocol_version
                )));
            }
            let tools = client
                .peer()
                .list_all_tools()
                .await
                .map_err(map_service_error)?;
            let prompts = if server_info.capabilities.prompts.is_some() {
                client
                    .peer()
                    .list_all_prompts()
                    .await
                    .unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            };
            Ok((client, tools, prompts, server_info))
        };
        let (client, tools, prompts, server_info) = tokio::time::timeout(STARTUP_TIMEOUT, startup)
            .await
            .map_err(|_| {
                GatewayStartupError::new(MaenifoldError::Message(format!(
                    "MCP server startup timed out after {}s",
                    STARTUP_TIMEOUT.as_secs()
                )))
            })?
            .map_err(GatewayStartupError::new)?;
        let peer = client.peer().clone();
        Ok(Self {
            client: StdMutex::new(Some(client)),
            peer,
            tools: tools.into(),
            prompts: prompts.into(),
            initialize_description: gateway_metadata::upstream_initialize_description(&server_info),
            call_timeout: config.timeout,
            server_name: config.name.clone(),
            dead: AtomicBool::new(false),
        })
    }

    pub(super) fn tools(&self) -> Arc<[Tool]> {
        Arc::clone(&self.tools)
    }

    pub(super) fn prompts(&self) -> Arc<[Prompt]> {
        Arc::clone(&self.prompts)
    }

    pub(super) fn initialize_description(&self) -> Option<String> {
        self.initialize_description.clone()
    }

    pub(super) fn is_usable(&self) -> bool {
        if self.dead.load(Ordering::SeqCst) {
            return false;
        }
        let usable = self
            .client
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .is_some_and(|client| !client.is_closed() && !client.peer().is_transport_closed());
        if !usable {
            self.dead.store(true, Ordering::SeqCst);
        }
        usable
    }

    pub(super) async fn shutdown(&self) {
        let client = self
            .client
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(mut client) = client {
            let _ = client.close_with_timeout(SHUTDOWN_WAIT).await;
        }
    }

    pub(super) async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult> {
        let arguments = match arguments {
            Value::Object(arguments) => arguments,
            other => {
                return Err(MaenifoldError::Message(format!(
                    "MCP tool arguments for '{name}' must be an object, got {other}"
                )));
            }
        };
        let request = ClientRequest::CallToolRequest(CallToolRequest::new(
            CallToolRequestParams::new(name.to_string()).with_arguments(arguments),
        ));
        match self.send_request(request, format!("tool '{name}'")).await? {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP tools/call response".to_string(),
            )),
        }
    }

    pub(super) async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        let request = ClientRequest::ReadResourceRequest(ReadResourceRequest::new(
            ReadResourceRequestParams::new(uri.to_string()),
        ));
        match self
            .send_request(request, format!("resource '{uri}'"))
            .await?
        {
            ServerResult::ReadResourceResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP resources/read response".to_string(),
            )),
        }
    }

    pub(super) async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, Value>>,
    ) -> Result<GetPromptResult> {
        let mut params = GetPromptRequestParams::new(name.to_string());
        if let Some(arguments) = arguments {
            params = params.with_arguments(arguments);
        }
        let request = ClientRequest::GetPromptRequest(GetPromptRequest::new(params));
        match self
            .send_request(request, format!("prompt '{name}'"))
            .await?
        {
            ServerResult::GetPromptResult(result) => Ok(result),
            _ => Err(MaenifoldError::Message(
                "unexpected MCP prompts/get response".to_string(),
            )),
        }
    }

    async fn send_request(&self, request: ClientRequest, what: String) -> Result<ServerResult> {
        if !self.is_usable() {
            return Err(MaenifoldError::SessionUnusable);
        }
        let mut options = PeerRequestOptions::no_options();
        options.timeout = Some(self.call_timeout);
        let handle = self
            .peer
            .send_cancellable_request(request, options)
            .await
            .map_err(|error| self.map_request_error(error, &what))?;
        handle
            .await_response()
            .await
            .map_err(|error| self.map_request_error(error, &what))
    }

    fn map_request_error(&self, error: ServiceError, what: &str) -> MaenifoldError {
        match error {
            ServiceError::Timeout { .. } => {
                self.dead.store(true, Ordering::SeqCst);
                MaenifoldError::JsonRpc {
                    code: GATEWAY_TIMEOUT_ERROR,
                    message: format!(
                        "Gateway: {} {} timed out after {}s",
                        self.server_name,
                        what,
                        self.call_timeout.as_secs()
                    ),
                    data: Value::Null,
                }
            }
            ServiceError::TransportClosed | ServiceError::TransportSend(_) => {
                self.dead.store(true, Ordering::SeqCst);
                MaenifoldError::JsonRpc {
                    code: JSONRPC_INTERNAL_ERROR,
                    message: format!("Gateway: {} connection lost: {}", self.server_name, error),
                    data: Value::Null,
                }
            }
            ServiceError::McpError(error) => MaenifoldError::JsonRpc {
                code: i64::from(error.code.0),
                message: error.message.to_string(),
                data: error.data.unwrap_or(Value::Null),
            },
            other => MaenifoldError::Message(other.to_string()),
        }
    }
}

fn is_supported_version(version: &ProtocolVersion) -> bool {
    version == &MCP_PROTOCOL_VERSION || version == &LEGACY_PROTOCOL_VERSION
}

fn map_service_error(error: ServiceError) -> MaenifoldError {
    match error {
        ServiceError::McpError(error) => MaenifoldError::Message(error.message.to_string()),
        other => MaenifoldError::Message(other.to_string()),
    }
}
