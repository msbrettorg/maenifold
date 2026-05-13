use rmcp::{
    ClientHandler,
    model::{
        ClientCapabilities, ClientInfo, CreateElicitationRequestParams, CreateElicitationResult,
        CreateMessageRequestParams, CreateMessageResult, ElicitationAction, Implementation,
        ProtocolVersion, Role, SamplingContent, SamplingMessage, SamplingMessageContent,
        TasksCapability,
    },
    service::{RequestContext, RoleClient},
};
use std::sync::Arc;

use super::PendingRequestRegistry;
use super::pending_requests::SamplingAction;
use super::sampling_session_registry::{
    SamplingDecisionRecord, SamplingDispatch, SamplingSessionRegistry, SamplingSessionStatus,
};

pub struct MaenifoldClient {
    info: ClientInfo,
    registry: Arc<PendingRequestRegistry>,
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
        let capabilities = ClientCapabilities::builder()
            .enable_sampling()
            .enable_elicitation()
            .enable_tasks_with(TasksCapability::client_default())
            .build();
        let info = ClientInfo::new(capabilities, Implementation::new("maenifold-core", "0.1.0"))
            .with_protocol_version(ProtocolVersion::V_2025_03_26);
        Self {
            info,
            registry,
            sampling_agent,
            sampling_registry,
            server_name: server_name.into(),
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
        if request.tools.is_some() {
            return Err(rmcp::ErrorData::invalid_params(
                "Tool-enabled sampling is not supported. sampling.tools capability is not advertised.",
                None,
            ));
        }

        let Some(sampling_agent) = self.sampling_agent.clone() else {
            return Err(rmcp::ErrorData::internal_error(
                "No sampling agent configured. Set 'samplingAgent' in maenifold.json to enable sampling.",
                None,
            ));
        };

        let server_name = if self.server_name.is_empty() {
            server_name_from_context(&context).unwrap_or_else(|| "maenifold".to_string())
        } else {
            self.server_name.clone()
        };
        let receiver = self
            .registry
            .register_sampling(request.clone(), &server_name)
            .await;
        let decision = receiver.await.map_err(|_| {
            rmcp::ErrorData::new(rmcp::model::ErrorCode(-1), "Sampling request expired", None)
        })?;
        let decision_record = sampling_decision_record(&decision.action);
        let dispatch_id = uuid::Uuid::new_v4();
        let app_session_id = uuid::Uuid::new_v4();
        let model = decision
            .selected_model
            .clone()
            .unwrap_or_else(|| sampling_agent.clone());
        let role_name = decision.selected_agent.unwrap_or(sampling_agent);
        let status = if matches!(decision_record, SamplingDecisionRecord::Deny) {
            SamplingSessionStatus::Denied
        } else {
            SamplingSessionStatus::Dispatching
        };
        self.sampling_registry
            .insert(SamplingDispatch {
                sampling_request_id: dispatch_id,
                app_session_id,
                acp_session_id: None,
                server_name,
                model: model.clone(),
                runtime_kind: role_name.clone(),
                role_name: role_name.clone(),
                status,
                decision: decision_record,
                created_at: chrono::Utc::now().to_rfc3339(),
                token_usage: None,
            })
            .await;

        match decision.action {
            SamplingAction::Deny => Err(rmcp::ErrorData::new(
                rmcp::model::ErrorCode(-1),
                "User rejected sampling request",
                None,
            )),
            SamplingAction::AllowOnce | SamplingAction::AllowAlways => {
                let prompt = sampling_messages_to_prompt(&request.messages)?;
                let result = jockey::acp::execute_runtime_sampling(
                    &role_name,
                    &role_name,
                    app_session_id,
                    prompt,
                    decision.selected_model.clone(),
                )
                .await;

                match result {
                    Ok(text_result) => {
                        if let Err(err) = self
                            .sampling_registry
                            .update_status(dispatch_id, SamplingSessionStatus::Completed)
                            .await
                        {
                            tracing::info!(
                                ?err,
                                dispatch_id = %dispatch_id,
                                "sampling session terminal at completion"
                            );
                        }
                        Ok(CreateMessageResult::new(
                            SamplingMessage::new(Role::Assistant, text_result),
                            model,
                        )
                        .with_stop_reason("endTurn"))
                    }
                    Err(err) => {
                        if let Err(reg_err) = self
                            .sampling_registry
                            .update_status(dispatch_id, SamplingSessionStatus::Failed)
                            .await
                        {
                            tracing::info!(
                                ?reg_err,
                                dispatch_id = %dispatch_id,
                                "sampling session terminal at failure"
                            );
                        }
                        Err(rmcp::ErrorData::new(
                            rmcp::model::ErrorCode(-1),
                            format!("Sampling failed: {err}"),
                            None,
                        ))
                    }
                }
            }
        }
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

fn sampling_decision_record(action: &SamplingAction) -> SamplingDecisionRecord {
    match action {
        SamplingAction::AllowOnce => SamplingDecisionRecord::AllowOnce,
        SamplingAction::AllowAlways => SamplingDecisionRecord::AllowAlways,
        SamplingAction::Deny => SamplingDecisionRecord::Deny,
    }
}

fn sampling_messages_to_prompt(messages: &[SamplingMessage]) -> Result<String, rmcp::ErrorData> {
    let mut prompt = String::new();
    for message in messages {
        if !prompt.is_empty() {
            prompt.push('\n');
        }
        let role = match message.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        prompt.push_str(role);
        prompt.push_str(": ");
        match &message.content {
            SamplingContent::Single(content) => {
                prompt.push_str(sampling_content_text(content)?);
            }
            SamplingContent::Multiple(contents) => {
                let mut parts = Vec::new();
                for content in contents {
                    parts.push(sampling_content_text(content)?.to_string());
                }
                prompt.push_str(&parts.join("\n"));
            }
        }
    }
    Ok(prompt)
}

fn sampling_content_text(content: &SamplingMessageContent) -> Result<&str, rmcp::ErrorData> {
    match content {
        SamplingMessageContent::Text(text) => Ok(&text.text),
        _ => Err(rmcp::ErrorData::invalid_params(
            "Only text sampling message content is supported.",
            None,
        )),
    }
}
