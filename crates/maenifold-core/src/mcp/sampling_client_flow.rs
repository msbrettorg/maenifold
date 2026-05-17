use std::sync::{Arc, atomic::AtomicBool};

use rmcp::model::{CreateMessageRequestParams, CreateMessageResult};
use serde_json::json;

use super::PendingRequestRegistry;
use super::loop_correlator::{self, LookupResult};
use super::pending_requests::{SamplingAction, SamplingDecision, SamplingPromptReason};
use super::sampling_client_helpers::{
    incoming_tool_result_ids, sampling_messages_to_prompt, text_result, tool_use_ids,
    tool_use_result,
};
use super::sampling_direct_inference::execute_direct_inference_sampling;
use super::sampling_session_registry::{
    SamplingDecisionRecord, SamplingDispatch, SamplingSessionRegistry, SamplingSessionStatus,
};
use crate::acp_config::AgentsConfig;
use crate::config::Config;

pub(crate) async fn create_message(
    registry: Arc<PendingRequestRegistry>,
    sampling_registry: Arc<SamplingSessionRegistry>,
    server_name: String,
    request: CreateMessageRequestParams,
) -> Result<CreateMessageResult, rmcp::ErrorData> {
    // Resolve direct-inference config up front. If it's not configured,
    // sampling fulfillment cannot proceed — we do NOT fall back to ACP / Jockey
    // (the old samplingAgent path is removed).
    let resolved = match resolve_direct_inference_config() {
        Ok(resolved) => resolved,
        Err(message) => {
            return Err(rmcp::ErrorData::internal_error(
                format!(
                    "sampling/createMessage cannot be fulfilled: direct inference not configured. {message}"
                ),
                None,
            ));
        }
    };
    let incoming_tool_ids = incoming_tool_result_ids(&request.messages);
    let continuation = if incoming_tool_ids.is_empty() {
        None
    } else {
        match loop_correlator::lookup(&server_name, &incoming_tool_ids) {
            Some(LookupResult::Active(id)) => Some(id),
            Some(LookupResult::Terminal(_)) => {
                let message =
                    "Sampling loop already terminal (completed, cancelled, denied, or failed)";
                return Err(rmcp::ErrorData::invalid_request(message, None));
            }
            None => {
                return Err(rmcp::ErrorData::invalid_request(
                    "Sampling loop expired or unknown",
                    None,
                ));
            }
        }
    };

    let (dispatch_id, _app_session_id, model, role_name, _resume_session_id, decision) =
        if let Some(existing_id) = continuation {
            existing_dispatch(
                Arc::clone(&registry),
                &sampling_registry,
                existing_id,
                incoming_tool_ids.len(),
                &server_name,
                &request,
            )
            .await?
        } else {
            new_dispatch(
                registry,
                &sampling_registry,
                &server_name,
                &resolved.deployment,
                &request,
            )
            .await?
        };

    sampling_registry
        .increment_turn(dispatch_id)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err, None))?;

    let prompt = sampling_messages_to_prompt(&request.messages)?;
    let _ = sampling_registry
        .update_status(dispatch_id, SamplingSessionStatus::Streaming)
        .await;

    let selected_model = decision
        .selected_model
        .clone()
        .filter(|model| !model.trim().is_empty())
        .or_else(|| Some(model.clone()));
    let _ = role_name; // kept for symmetry with prior contract; not needed for direct inference
    let max_tokens = u64::from(request.max_tokens.max(0) as u32);
    let result = execute_direct_inference_sampling(
        &resolved,
        prompt,
        selected_model,
        request.tools.clone(),
        if max_tokens == 0 { None } else { Some(max_tokens) },
    )
    .await;

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

fn resolve_direct_inference_config() -> Result<crate::acp_config::ResolvedInference, String> {
    let config = Config::current();
    let agents = AgentsConfig::load_or_default(&config.maenifold_root)
        .map_err(|err| format!("failed to load maenifold.json: {err}"))?;
    let intent_resolver = agents
        .settings
        .as_ref()
        .map(|s| s.gateway.intent_resolver.clone())
        .unwrap_or_default();
    intent_resolver.resolved_direct_inference()
}

async fn existing_dispatch(
    request_registry: Arc<PendingRequestRegistry>,
    sampling_registry: &SamplingSessionRegistry,
    existing_id: uuid::Uuid,
    incoming_count: usize,
    server_name: &str,
    request: &CreateMessageRequestParams,
) -> Result<
    (
        uuid::Uuid,
        uuid::Uuid,
        String,
        String,
        Option<String>,
        SamplingDecision,
    ),
    rmcp::ErrorData,
> {
    let Some(dispatch) = sampling_registry.get(existing_id).await else {
        return Err(rmcp::ErrorData::invalid_request(
            "Sampling loop expired or unknown",
            None,
        ));
    };
    let count = sampling_registry
        .increment_tool_calls(existing_id, incoming_count as u32)
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err, None))?;
    if count >= dispatch.tool_loop_budget {
        let status = SamplingSessionStatus::BudgetExceeded {
            allowed_so_far: count,
            budget: dispatch.tool_loop_budget,
        };
        let _ = sampling_registry.update_status(existing_id, status).await;
        let reason = SamplingPromptReason::BudgetExceeded {
            used: count,
            budget: dispatch.tool_loop_budget,
        };
        let receiver = request_registry
            .register_sampling(request.clone(), server_name, reason)
            .await;
        match receiver.await {
            Ok(decision)
                if matches!(
                    decision.action,
                    SamplingAction::AllowOnce | SamplingAction::AllowAlways
                ) =>
            {
                sampling_registry
                    .reset_tool_calls(existing_id)
                    .await
                    .map_err(|err| rmcp::ErrorData::internal_error(err, None))?;
                sampling_registry
                    .update_status(existing_id, SamplingSessionStatus::Streaming)
                    .await
                    .map_err(|err| rmcp::ErrorData::internal_error(format!("{err:?}"), None))?;
            }
            Ok(_) => {
                let _ = sampling_registry
                    .update_status(existing_id, SamplingSessionStatus::Cancelled)
                    .await;
                return Err(sampling_denied_error());
            }
            Err(_) => {
                let _ = sampling_registry
                    .update_status(existing_id, SamplingSessionStatus::Cancelled)
                    .await;
                return Err(rmcp::ErrorData::invalid_request(
                    "Sampling session cancelled",
                    None,
                ));
            }
        }
    }
    let resume = sampling_registry.acp_session_id(existing_id).await;
    let decision = SamplingDecision {
        action: SamplingAction::AllowOnce,
        selected_model: Some(dispatch.model.clone()),
        selected_agent: Some(dispatch.role_name.clone()),
    };
    Ok((
        existing_id,
        dispatch.app_session_id,
        dispatch.model,
        dispatch.role_name,
        resume,
        decision,
    ))
}

async fn new_dispatch(
    registry: Arc<PendingRequestRegistry>,
    sampling_registry: &SamplingSessionRegistry,
    server_name: &str,
    default_model: &str,
    request: &CreateMessageRequestParams,
) -> Result<
    (
        uuid::Uuid,
        uuid::Uuid,
        String,
        String,
        Option<String>,
        SamplingDecision,
    ),
    rmcp::ErrorData,
> {
    let receiver = registry
        .register_sampling(
            request.clone(),
            server_name,
            SamplingPromptReason::NewRequest,
        )
        .await;
    let decision = receiver.await.map_err(|_| sampling_denied_error())?;
    let decision_record = sampling_decision_record(&decision.action);
    let dispatch_id = uuid::Uuid::new_v4();
    let app_session_id = uuid::Uuid::new_v4();
    let model = decision
        .selected_model
        .clone()
        .unwrap_or_else(|| default_model.to_string());
    let role_name = decision
        .selected_agent
        .clone()
        .unwrap_or_else(|| "direct-inference".to_string());
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
            request_payload: Some(pretty_payload(request)),
            response_payload: None,
            tool_call_count: 0,
            tool_loop_budget: sampling_registry.tool_loop_budget_for(server_name),
            turn_count: 0,
            status_handle: Arc::new(AtomicBool::new(false)),
        })
        .await;
    match decision.action {
        SamplingAction::Deny => Err(sampling_denied_error()),
        SamplingAction::AllowOnce | SamplingAction::AllowAlways => Ok((
            dispatch_id,
            app_session_id,
            model,
            role_name,
            None,
            decision,
        )),
    }
}

fn sampling_denied_error() -> rmcp::ErrorData {
    rmcp::ErrorData::new(
        rmcp::model::ErrorCode(-1),
        "Sampling session denied by user",
        None,
    )
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
    let response = match result.stop_reason {
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
    };
    let _ = registry
        .record_response(dispatch_id, pretty_payload(&response))
        .await;
    Ok(response)
}

fn pretty_payload<T: serde::Serialize>(payload: &T) -> String {
    serde_json::to_string_pretty(payload)
        .unwrap_or_else(|err| json!({ "serializationError": err.to_string() }).to_string())
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
