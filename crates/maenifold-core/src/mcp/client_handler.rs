use rmcp::{
    ClientHandler,
    model::{
        ClientCapabilities, ClientInfo, CreateElicitationRequestParams, CreateElicitationResult,
        CreateMessageRequestParams, CreateMessageResult, ElicitationAction, Implementation,
        ProtocolVersion, Role, SamplingMessage, TasksCapability,
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

    async fn create_message(
        &self,
        request: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, rmcp::ErrorData> {
        tracing::info!(
            ?request,
            "received MCP sampling request; returning user_denied stub"
        );
        let denial = SamplingMessage::new(
            Role::Assistant,
            "User has not yet approved sampling requests. T-303.4.1 will add the approval modal.",
        );
        Ok(CreateMessageResult::new(denial, "maenifold-stub".to_string())
            .with_stop_reason("user_denied"))
    }

    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}
