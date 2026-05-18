use rmcp::{
    ClientHandler,
    model::{
        ClientCapabilities, ClientInfo, CreateElicitationRequestParams, CreateElicitationResult,
        ElicitationAction, Implementation, ProtocolVersion, TasksCapability,
    },
    service::{RequestContext, RoleClient},
};
use std::sync::Arc;

use super::PendingRequestRegistry;

pub struct MaenifoldClient {
    info: ClientInfo,
    registry: Arc<PendingRequestRegistry>,
    #[allow(dead_code)]
    server_name: String,
}

impl MaenifoldClient {
    pub fn new(registry: Arc<PendingRequestRegistry>) -> Self {
        Self::new_with_server_name(registry, "maenifold")
    }

    pub(crate) fn new_with_server_name(
        registry: Arc<PendingRequestRegistry>,
        server_name: impl Into<String>,
    ) -> Self {
        let server_name = server_name.into();
        let capabilities = ClientCapabilities::builder()
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

    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}
