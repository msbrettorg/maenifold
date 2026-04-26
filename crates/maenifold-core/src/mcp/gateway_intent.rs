use crate::mcp::domain_mapping::SafetyClass;
use crate::mcp::gateway_errors::{invalid_params, map_gateway_error};
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::gateway_resolver::resolve_intent;
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput};
use crate::mcp::schema::gateway_tools::GatewayIntentParams;
use rmcp::model::{CallToolResult, Content};
use serde_json::{Value, json};

pub(crate) async fn handle_gateway_intent(
    state: &GatewayState,
    arguments: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let params: GatewayIntentParams = serde_json::from_value(arguments)
        .map_err(|err| map_gateway_error("GatewayIntent", invalid_params(err)))?;
    let intent = intent_with_hints(params.intent, params.domain_hints);
    match resolve_intent(
        state.catalog(),
        state.intent_settings(),
        &intent,
        state.workspace_root(),
    )
    .await
    {
        Ok(output) => handle_resolved_intent(state, output).await,
        Err(error) => Ok(render(fallback_response(error))),
    }
}

pub(crate) async fn handle_resolved_intent(
    state: &GatewayState,
    output: ResolverOutput,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let candidates = output.candidates;
    if can_auto_execute(state, &candidates) {
        let candidate = &candidates[0];
        return state
            .handle_gateway_execute(json!({
                "domain": candidate.domain,
                "command": candidate.command,
                "parameters": candidate.parameters_draft,
                "idempotencyKey": null
            }))
            .await;
    }

    let body = if candidates.len() == 1
        && (candidates[0].safety_class != SafetyClass::Read || candidates[0].requires_exact_execute)
    {
        json!({
            "status": "plan",
            "intentSummary": output.intent_summary,
            "plan": execute_plan(&candidates[0]),
            "candidates": candidates,
        })
    } else {
        json!({
            "status": "candidates",
            "intentSummary": output.intent_summary,
            "candidates": candidates,
            "instructions": "Review candidates, then call GatewayExecute with the exact domain, command, and parameters.",
        })
    };
    Ok(render(body))
}

fn can_auto_execute(
    state: &GatewayState,
    candidates: &[crate::mcp::gateway_resolver_types::ResolverCandidate],
) -> bool {
    let settings = state.intent_settings();
    settings.auto_execute_read_safe
        && candidates.len() == 1
        && candidates[0].safety_class == SafetyClass::Read
        && candidates[0].confidence >= settings.auto_execute_confidence_threshold
        && !candidates[0].requires_exact_execute
}

fn execute_plan(candidate: &crate::mcp::gateway_resolver_types::ResolverCandidate) -> Value {
    json!({
        "tool": "GatewayExecute",
        "domain": candidate.domain,
        "command": candidate.command,
        "parameters": candidate.parameters_draft,
        "reason": candidate.reason,
        "safetyClass": candidate.safety_class,
    })
}

fn fallback_response(error: ResolverError) -> Value {
    json!({
        "status": "fallback",
        "error": error.to_string(),
        "instructions": error.fallback(),
        "plan": {
            "inspect": { "tool": "GatewayIndex" },
            "execute": { "tool": "GatewayExecute" }
        }
    })
}

fn intent_with_hints(intent: String, hints: Option<Vec<String>>) -> String {
    let hints = hints
        .unwrap_or_default()
        .into_iter()
        .filter(|hint| !hint.trim().is_empty())
        .collect::<Vec<_>>();
    if hints.is_empty() {
        intent
    } else {
        format!("{intent}\nDomain hints: {}", hints.join(", "))
    }
}

fn render(value: Value) -> CallToolResult {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
    CallToolResult::success(vec![Content::text(text)])
}

#[cfg(test)]
pub(crate) fn resolved_text(result: &CallToolResult) -> String {
    result.content[0]
        .raw
        .as_text()
        .expect("text content")
        .text
        .clone()
}
