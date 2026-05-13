use rmcp::{
    ClientHandler,
    model::{
        ClientCapabilities, ClientInfo, CreateElicitationRequestParams, CreateElicitationResult,
        ElicitationAction, Implementation, ProtocolVersion, TasksCapability,
    },
    service::{RequestContext, RoleClient},
};

pub struct MaenifoldClient {
    info: ClientInfo,
}

impl MaenifoldClient {
    pub fn new() -> Self {
        let capabilities = ClientCapabilities::builder()
            .enable_sampling()
            .enable_elicitation()
            .enable_tasks_with(TasksCapability::client_default())
            .build();
        let info = ClientInfo::new(capabilities, Implementation::new("maenifold-core", "0.1.0"))
            .with_protocol_version(ProtocolVersion::V_2025_03_26);
        Self { info }
    }
}

impl ClientHandler for MaenifoldClient {
    async fn create_elicitation(
        &self,
        request: CreateElicitationRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateElicitationResult, rmcp::ErrorData> {
        tracing::info!(
            ?request,
            "received MCP elicitation request; returning cancel"
        );
        Ok(CreateElicitationResult::new(ElicitationAction::Cancel))
    }

    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}
