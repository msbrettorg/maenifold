use rmcp::{
    ClientHandler,
    model::{
        ClientCapabilities, ClientInfo, CreateElicitationRequestParams, CreateElicitationResult,
        CreateMessageRequestParams, CreateMessageResult, ElicitationAction, Implementation,
        ProtocolVersion, TasksCapability,
    },
    service::{RequestContext, RoleClient},
};
use std::sync::Arc;

use super::PendingRequestRegistry;
use super::sampling_client_flow;
use super::sampling_session_registry::SamplingSessionRegistry;

pub struct MaenifoldClient {
    info: ClientInfo,
    registry: Arc<PendingRequestRegistry>,
    #[allow(dead_code)]
    sampling_agent: Option<String>,
    sampling_registry: Arc<SamplingSessionRegistry>,
    server_name: String,
}

impl MaenifoldClient {
    pub fn new(registry: Arc<PendingRequestRegistry>) -> Self {
        Self::new_with_sampling_registry(
            registry,
            None,
            Arc::new(SamplingSessionRegistry::new()),
            "maenifold",
        )
    }

    pub fn new_with_sampling(
        registry: Arc<PendingRequestRegistry>,
        sampling_agent: Option<String>,
    ) -> Self {
        Self::new_with_sampling_registry(
            registry,
            sampling_agent,
            Arc::new(SamplingSessionRegistry::new()),
            "maenifold",
        )
    }

    /// Constructs a client with the already-selected sampling-session registry.
    /// The process-wide sampling resource pointer is owned by
    /// `build_gateway_with_registry`, not by per-server client construction.
    pub(crate) fn new_with_sampling_registry(
        registry: Arc<PendingRequestRegistry>,
        sampling_agent: Option<String>,
        sampling_registry: Arc<SamplingSessionRegistry>,
        server_name: impl Into<String>,
    ) -> Self {
        let server_name = server_name.into();
        let capabilities = ClientCapabilities::builder()
            .enable_sampling()
            .enable_sampling_tools()
            .enable_elicitation()
            .enable_tasks_with(TasksCapability::client_default())
            .build();
        let info = ClientInfo::new(capabilities, Implementation::new("maenifold-core", "0.1.0"))
            .with_protocol_version(ProtocolVersion::V_2025_03_26);
        tracing::debug!(
            server = server_name.as_str(),
            capabilities = %serde_json::to_string(&info.capabilities).unwrap_or_default(),
            "MCP client capabilities for initialize handshake"
        );
        Self {
            info,
            registry,
            sampling_agent,
            sampling_registry,
            server_name,
        }
    }
}

impl ClientHandler for MaenifoldClient {
    async fn create_elicitation(
        &self,
        request: CreateElicitationRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateElicitationResult, rmcp::ErrorData> {
        tracing::info!(?request, "received MCP elicitation request");
        let response = self.registry.register_elicitation(request).await.await;
        let Ok(response) = response else {
            tracing::warn!("MCP elicitation request expired before user response");
            return Ok(CreateElicitationResult::new(ElicitationAction::Cancel));
        };
        let mut result = CreateElicitationResult::new(response.to_mcp_action());
        if let Some(content) = response.content {
            result = result.with_content(content);
        }
        Ok(result)
    }

    async fn create_message(
        &self,
        request: CreateMessageRequestParams,
        context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, rmcp::ErrorData> {
        let server_name = if self.server_name.is_empty() {
            server_name_from_context(&context).unwrap_or_else(|| "maenifold".to_string())
        } else {
            self.server_name.clone()
        };
        sampling_client_flow::create_message(
            Arc::clone(&self.registry),
            Arc::clone(&self.sampling_registry),
            server_name,
            request,
        )
        .await
    }

    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}

fn server_name_from_context(context: &RequestContext<RoleClient>) -> Option<String> {
    context
        .peer
        .peer_info()
        .map(|info| info.server_info.name.clone())
        .filter(|name| !name.is_empty())
}
