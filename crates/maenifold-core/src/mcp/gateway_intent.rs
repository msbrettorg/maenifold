use crate::mcp::domain_mapping::SafetyClass;
use crate::mcp::gateway_direct_inference_resolver::resolve_intent_with_direct_inference;
use crate::mcp::gateway_errors::{invalid_params, map_gateway_error};
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput};
use crate::mcp::schema::gateway_tools::GatewayIntentParams;
use rmcp::RoleServer;
use rmcp::model::{CallToolResult, Content};
use rmcp::service::Peer;
use serde_json::{Value, json};

pub(crate) async fn handle_gateway_intent(
    state: &GatewayState,
    arguments: Value,
    _client_peer: Option<Peer<RoleServer>>,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let params: GatewayIntentParams = serde_json::from_value(arguments)
        .map_err(|err| map_gateway_error("GatewayIntent", invalid_params(err)))?;
    let intent = intent_with_hints(params.intent, params.domain_hints);
    // GatewayIntent resolver: direct inferencing only after T-331.1.
    let settings = state.intent_settings();
    let resolved = if settings.resolved_direct_inference().is_ok() {
        resolve_intent_with_direct_inference(state.catalog(), settings, &intent).await
    } else {
        let setup_message = settings
            .resolved_direct_inference()
            .err()
            .unwrap_or_else(|| "direct inference not configured".to_string());
        Err(ResolverError::DirectInferenceSetup {
            fallback: format!(
                "{} ({setup_message})",
                crate::mcp::gateway_resolver::fallback()
            ),
        })
    };
    match resolved {
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
                "parameters": candidate.parameters_draft
            }))
            .await;
    }

    let candidates_with_missing: Vec<Value> = candidates
        .iter()
        .map(|candidate| {
            let mut value = serde_json::to_value(candidate)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
            let missing = missing_required(state, candidate);
            if let Value::Object(ref mut map) = value {
                map.insert("missingRequiredParameters".to_string(), json!(missing));
            }
            value
        })
        .collect();
    let top_missing = candidates
        .first()
        .map(|c| missing_required(state, c))
        .unwrap_or_default();

    let body = if candidates.len() == 1
        && (candidates[0].safety_class != SafetyClass::Read
            || candidates[0].requires_exact_execute
            || !top_missing.is_empty())
    {
        json!({
            "status": "plan",
            "intentSummary": output.intent_summary,
            "plan": execute_plan(&candidates[0], &top_missing),
            "candidates": candidates_with_missing,
        })
    } else {
        let mut instructions = String::from(
            "Review candidates, then call GatewayExecute with the exact domain, command, and parameters.",
        );
        if !top_missing.is_empty() {
            instructions.push_str(
                " Each candidate's `missingRequiredParameters` lists fields you MUST supply in `parameters` before GatewayExecute will succeed.",
            );
        }
        json!({
            "status": "candidates",
            "intentSummary": output.intent_summary,
            "candidates": candidates_with_missing,
            "instructions": instructions,
        })
    };
    Ok(render(body))
}

fn can_auto_execute(
    state: &GatewayState,
    candidates: &[crate::mcp::gateway_resolver_types::ResolverCandidate],
) -> bool {
    // Auto-execute when the top candidate clearly wins. Requiring
    // `candidates.len() == 1` was wrong: the LLM is prompted to return up to
    // `max_candidates` (default 5), so a single-candidate response is rare even
    // when the route is obvious. Instead: look at the top candidate, and only
    // suppress auto-execute when the runner-up is close enough to suggest the
    // route is genuinely ambiguous.
    const DOMINANCE_GAP: f64 = 0.2;
    let settings = state.intent_settings();
    if !settings.auto_execute_read_safe || candidates.is_empty() {
        return false;
    }
    let top = &candidates[0];
    if top.safety_class != SafetyClass::Read
        || top.requires_exact_execute
        || top.confidence < settings.auto_execute_confidence_threshold
    {
        return false;
    }
    // BUG-1 guard: do not auto-execute when the LLM draft is missing any
    // schema-required parameter (e.g. `gate_Status` without `root`).
    // Without this guard we silently fire off `GatewayExecute` with an
    // incomplete payload and surface the registry's "missing required string
    // parameter" error to the caller, who has no way to recover because the
    // candidate was never shown.
    if !missing_required(state, top).is_empty() {
        return false;
    }
    match candidates.get(1) {
        Some(next) => top.confidence - next.confidence >= DOMINANCE_GAP,
        None => true,
    }
}

fn missing_required(
    state: &GatewayState,
    candidate: &crate::mcp::gateway_resolver_types::ResolverCandidate,
) -> Vec<String> {
    let Ok(required) = state
        .catalog()
        .required_parameters(&candidate.domain, &candidate.command)
    else {
        return Vec::new();
    };
    required
        .into_iter()
        .filter(|key| !candidate.parameters_draft.contains_key(key))
        .collect()
}

fn execute_plan(
    candidate: &crate::mcp::gateway_resolver_types::ResolverCandidate,
    missing_required: &[String],
) -> Value {
    json!({
        "tool": "GatewayExecute",
        "domain": candidate.domain,
        "command": candidate.command,
        "parameters": candidate.parameters_draft,
        "missingRequiredParameters": missing_required,
        "reason": candidate.reason,
        "safetyClass": candidate.safety_class,
    })
}

fn fallback_response(error: ResolverError) -> Value {
    let status = match error {
        ResolverError::DirectInferenceSetup { .. } => "directInferenceSetup",
        _ => "fallback",
    };
    json!({
        "status": status,
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
