use std::sync::{Arc, atomic::AtomicBool};

use rmcp::model::{CreateMessageRequestParams, CreateMessageResult};

use super::PendingRequestRegistry;
use super::loop_correlator;
use super::pending_requests::SamplingAction;
use super::sampling_client_helpers::{
    incoming_tool_result_ids, sampling_messages_to_prompt, text_result, tool_use_ids,
    tool_use_result,
};
use super::sampling_session_registry::{
    SamplingDecisionRecord, SamplingDispatch, SamplingSessionRegistry, SamplingSessionStatus,
};

pub(crate) async fn create_message(
    registry: Arc<PendingRequestRegistry>,
    sampling_agent: String,
    sampling_registry: Arc<SamplingSessionRegistry>,
    server_name: String,
    request: CreateMessageRequestParams,
) -> Result<CreateMessageResult, rmcp::ErrorData> {
    let incoming_tool_ids = incoming_tool_result_ids(&request.messages);
    let continuation = if incoming_tool_ids.is_empty() {
        None
    } else if let Some(id) = loop_correlator::lookup(&server_name, &incoming_tool_ids) {
        Some(id)
    } else {
        return Err(rmcp::ErrorData::invalid_request(
            "Sampling loop expired or unknown",
            None,
        ));
    };

    let (dispatch_id, app_session_id, model, role_name, resume_session_id) =
        if let Some(existing_id) = continuation {
            existing_dispatch(&sampling_registry, existing_id, incoming_tool_ids.len()).await?
        } else {
            new_dispatch(
                registry,
                &sampling_registry,
                &server_name,
                sampling_agent,
                &request,
            )
            .await?
        };

    sampling_registry
        .increment_turn(dispatch_id)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err, None))?;

    let prompt = sampling_messages_to_prompt(&request.messages)?;
    let (session_id_tx, session_id_rx) = tokio::sync::oneshot::channel::<String>();
    let event_tx = crate::mcp::sampling_events::sender_for(app_session_id);
    let streaming_registry = Arc::clone(&sampling_registry);
    tokio::spawn(async move {
        match session_id_rx.await {
            Ok(acp_session_id) => {
                let _ = streaming_registry
                    .update_acp_session(dispatch_id, acp_session_id)
                    .await;
                let _ = streaming_registry
                    .update_status(dispatch_id, SamplingSessionStatus::Streaming)
                    .await;
            }
            Err(_) => {
                let _ = streaming_registry
                    .update_status(dispatch_id, SamplingSessionStatus::Failed)
                    .await;
            }
        }
    });

    let result = jockey::acp::execute_runtime_sampling(
        &role_name,
        &role_name,
        app_session_id,
        prompt,
        Some(model.clone()),
        session_id_tx,
        event_tx,
        resume_session_id,
        request.tools.clone(),
    )
    .await;
    crate::mcp::sampling_events::unregister(app_session_id);

    match result {
        Ok(result) => {
            response_from_sampling(&sampling_registry, dispatch_id, server_name, model, result)
                .await
        }
        Err(err) => {
            update_failed(&sampling_registry, dispatch_id).await;
            Err(rmcp::ErrorData::new(
                rmcp::model::ErrorCode(-1),
                format!("Sampling failed: {err}"),
                None,
            ))
        }
    }
}

async fn existing_dispatch(
    registry: &SamplingSessionRegistry,
    existing_id: uuid::Uuid,
    incoming_count: usize,
) -> Result<(uuid::Uuid, uuid::Uuid, String, String, Option<String>), rmcp::ErrorData> {
    let Some(dispatch) = registry.get(existing_id).await else {
        return Err(rmcp::ErrorData::invalid_request(
            "Sampling loop expired or unknown",
            None,
        ));
    };
    let count = registry
        .increment_tool_calls(existing_id, incoming_count as u32)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err, None))?;
    if count > dispatch.tool_loop_budget {
        let _ = registry
            .update_status(
                existing_id,
                SamplingSessionStatus::BudgetExceeded {
                    allowed_so_far: count,
                    budget: dispatch.tool_loop_budget,
                },
            )
            .await;
        return Err(rmcp::ErrorData::invalid_request(
            "Sampling loop budget exceeded — user re-approval required",
            None,
        ));
    }
    let resume = registry.acp_session_id(existing_id).await;
    Ok((
        existing_id,
        dispatch.app_session_id,
        dispatch.model,
        dispatch.role_name,
        resume,
    ))
}

async fn new_dispatch(
    registry: Arc<PendingRequestRegistry>,
    sampling_registry: &SamplingSessionRegistry,
    server_name: &str,
    sampling_agent: String,
    request: &CreateMessageRequestParams,
) -> Result<(uuid::Uuid, uuid::Uuid, String, String, Option<String>), rmcp::ErrorData> {
    let receiver = registry
        .register_sampling(request.clone(), server_name)
        .await;
    let decision = receiver
        .await
        .map_err(|_| rmcp::ErrorData::invalid_request("Sampling session cancelled", None))?;
    let decision_record = sampling_decision_record(&decision.action);
    let dispatch_id = uuid::Uuid::new_v4();
    let app_session_id = uuid::Uuid::new_v4();
    let model = decision
        .selected_model
        .clone()
        .unwrap_or_else(|| sampling_agent.clone());
    let role_name = decision.selected_agent.unwrap_or(sampling_agent);
    sampling_registry
        .insert(SamplingDispatch {
            sampling_request_id: dispatch_id,
            app_session_id,
            acp_session_id: None,
            server_name: server_name.to_string(),
            model: model.clone(),
            runtime_kind: role_name.clone(),
            role_name: role_name.clone(),
            status: if matches!(decision_record, SamplingDecisionRecord::Deny) {
                SamplingSessionStatus::Denied
            } else {
                SamplingSessionStatus::Dispatching
            },
            decision: decision_record,
            created_at: chrono::Utc::now().to_rfc3339(),
            token_usage: None,
            tool_call_count: 0,
            tool_loop_budget: sampling_registry.tool_loop_budget_for(server_name),
            turn_count: 0,
            status_handle: Arc::new(AtomicBool::new(false)),
        })
        .await;
    match decision.action {
        SamplingAction::Deny => Err(rmcp::ErrorData::new(
            rmcp::model::ErrorCode(-1),
            "User rejected sampling request",
            None,
        )),
        SamplingAction::AllowOnce | SamplingAction::AllowAlways => {
            Ok((dispatch_id, app_session_id, model, role_name, None))
        }
    }
}

async fn response_from_sampling(
    registry: &SamplingSessionRegistry,
    dispatch_id: uuid::Uuid,
    server_name: String,
    model: String,
    result: jockey::acp::SamplingResult,
) -> Result<CreateMessageResult, rmcp::ErrorData> {
    let _ = registry
        .update_acp_session(dispatch_id, result.acp_session_id.clone())
        .await;
    Ok(match result.stop_reason {
        jockey::acp::SamplingStopReason::ToolUse(blocks) => {
            if let Some(handle) = registry.status_handle(dispatch_id).await {
                loop_correlator::record(server_name, tool_use_ids(&blocks), dispatch_id, handle);
            }
            tool_use_result(blocks, model)
        }
        jockey::acp::SamplingStopReason::MaxTokens => {
            update_completed(registry, dispatch_id).await;
            text_result(
                result.text,
                model,
                CreateMessageResult::STOP_REASON_END_MAX_TOKEN,
            )
        }
        jockey::acp::SamplingStopReason::StopSequence(_) => {
            update_completed(registry, dispatch_id).await;
            text_result(
                result.text,
                model,
                CreateMessageResult::STOP_REASON_END_SEQUENCE,
            )
        }
        jockey::acp::SamplingStopReason::EndTurn | jockey::acp::SamplingStopReason::Unknown(_) => {
            update_completed(registry, dispatch_id).await;
            text_result(
                result.text,
                model,
                CreateMessageResult::STOP_REASON_END_TURN,
            )
        }
    })
}

async fn update_completed(registry: &SamplingSessionRegistry, dispatch_id: uuid::Uuid) {
    if let Err(err) = registry
        .update_status(dispatch_id, SamplingSessionStatus::Completed)
        .await
    {
        tracing::info!(?err, dispatch_id = %dispatch_id, "sampling session terminal at completion");
    }
}

async fn update_failed(registry: &SamplingSessionRegistry, dispatch_id: uuid::Uuid) {
    if let Err(err) = registry
        .update_status(dispatch_id, SamplingSessionStatus::Failed)
        .await
    {
        tracing::info!(?err, dispatch_id = %dispatch_id, "sampling session terminal at failure");
    }
}

fn sampling_decision_record(action: &SamplingAction) -> SamplingDecisionRecord {
    match action {
        SamplingAction::AllowOnce => SamplingDecisionRecord::AllowOnce,
        SamplingAction::AllowAlways => SamplingDecisionRecord::AllowAlways,
        SamplingAction::Deny => SamplingDecisionRecord::Deny,
    }
}
