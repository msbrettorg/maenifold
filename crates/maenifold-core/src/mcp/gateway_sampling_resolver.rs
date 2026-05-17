use std::sync::Arc;
use std::time::Duration;

use rmcp::RoleServer;
use rmcp::model::{CreateMessageRequestParams, CreateMessageResult, SamplingMessage};
use rmcp::service::Peer;
use serde_json::json;

use crate::acp_config::IntentResolverSettings;
use crate::mcp::gateway_catalog::GatewayCatalog;
use crate::mcp::gateway_intent_observer::{pretty_payload, start};
use crate::mcp::gateway_resolver::{fallback, parse_and_validate_output, resolver_prompt};
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput, ResolverResult};
use crate::mcp::sampling_session_registry::{SamplingDecisionRecord, SamplingSessionRegistry};

pub(crate) fn client_supports_sampling(peer: &Peer<RoleServer>) -> bool {
    peer.peer_info().is_some_and(|info| {
        info.capabilities.sampling.is_some()
            || info
                .capabilities
                .tasks
                .as_ref()
                .is_some_and(|tasks| tasks.supports_sampling_create_message())
    })
}

pub(crate) async fn resolve_intent_with_client_sampling(
    catalog: &GatewayCatalog,
    settings: &IntentResolverSettings,
    user_intent: &str,
    peer: Peer<RoleServer>,
    registry: Arc<SamplingSessionRegistry>,
) -> ResolverResult<ResolverOutput> {
    if !settings.enabled {
        return Err(ResolverError::Disabled {
            fallback: fallback(),
        });
    }
    let prompt = resolver_prompt(catalog, settings, user_intent);
    let mut request =
        CreateMessageRequestParams::new(vec![SamplingMessage::user_text(prompt)], 2048);
    request.temperature = Some(0.0);
    request.metadata = Some(json!({
        "component": "gateway-intent-resolver",
        "schema": "ResolverOutput/v1",
        "backend": "clientSampling"
    }));
    let observation = start(
        registry,
        SamplingDecisionRecord::ClientSampling,
        "clientSampling",
        settings.model.as_str(),
        "calling-mcp-client",
        uuid::Uuid::new_v4(),
        pretty_payload(&request),
    )
    .await;
    let timeout_seconds = settings.timeout_seconds.unwrap_or(30);
    let result = match tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        peer.create_message(request),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            let error = ResolverError::DirectInferenceFailed {
                message: format!("client sampling failed: {err}"),
                fallback: fallback(),
            };
            observation
                .fail(&json!({ "error": error.to_string() }))
                .await;
            return Err(error);
        }
        Err(_) => {
            let error = ResolverError::Timeout {
                seconds: timeout_seconds,
                fallback: fallback(),
            };
            observation
                .fail(&json!({ "error": error.to_string() }))
                .await;
            return Err(error);
        }
    };
    let response_text = match response_text(&result) {
        Ok(text) => text,
        Err(error) => {
            observation
                .fail(&json!({ "error": error.to_string(), "rawResponse": result }))
                .await;
            return Err(error);
        }
    };
    let output = parse_and_validate_output(catalog, &response_text, settings.max_candidates);
    match &output {
        Ok(parsed) => {
            observation
                .complete(&json!({ "rawResponse": result, "parsed": parsed }))
                .await
        }
        Err(error) => {
            observation
                .fail(&json!({ "error": error.to_string(), "rawResponse": result }))
                .await
        }
    }
    output
}

fn response_text(result: &CreateMessageResult) -> ResolverResult<String> {
    let text = result
        .message
        .content
        .iter()
        .filter_map(|content| content.as_text().map(|text| text.text.clone()))
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        return Err(ResolverError::Malformed {
            message: "client sampling response did not include assistant text".to_string(),
            fallback: fallback(),
        });
    }
    Ok(text)
}
