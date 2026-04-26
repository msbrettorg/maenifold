#![allow(dead_code)]

use crate::acp_config::{AgentEntry, AgentsConfig, IntentResolverSettings};
use crate::mcp::domain_mapping::SafetyClass;
use crate::mcp::gateway_catalog::{CommandEntry, DomainEntry, GatewayCatalog};
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput, ResolverResult};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

const FALLBACK: &str =
    "Use GatewayIndex to inspect available domains and commands, then call GatewayExecute explicitly.";

pub(crate) async fn resolve_intent(catalog: &GatewayCatalog, settings: &IntentResolverSettings, user_intent: &str, root: &Path) -> ResolverResult<ResolverOutput> {
    if !settings.enabled {
        return Err(ResolverError::Disabled { fallback: fallback() });
    }
    let agent = settings
        .agent
        .as_deref()
        .filter(|agent| !agent.trim().is_empty())
        .ok_or_else(|| ResolverError::NoAgent { fallback: fallback() })?;
    let config = AgentsConfig::load(root).map_err(|err| ResolverError::NoAgent { fallback: format!("{FALLBACK} Resolver config load failed: {err}") })?;
    let entry = config.agents.get(agent).ok_or_else(|| ResolverError::NoAgent { fallback: fallback() })?;
    let runtime_kind = infer_runtime_kind(entry)?;
    let prompt = resolver_prompt(catalog, settings, user_intent);
    let context = vec![("component".into(), "gateway-intent-resolver".into()), ("schema".into(), "ResolverOutput/v1".into())];
    let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<jockey::JockeyEvent>(32);
    let mut role_config_options = vec![("model".to_string(), settings.model.clone())];
    if let Some(level) = settings.effective_thought_level() {
        role_config_options.push(("thoughtLevel".to_string(), level.to_string()));
    }
    let cwd = root.to_string_lossy().to_string();
    let mcp_servers = settings.mcp_servers.iter().map(|entry| entry.to_acp()).collect();
    let app_session_id = format!("gateway-resolver-{}", uuid::Uuid::new_v4());
    if settings.timeout_seconds == Some(0) {
        return Err(ResolverError::Timeout { seconds: 0, fallback: fallback() });
    }
    let timeout_seconds = settings.timeout_seconds.unwrap_or(30);
    let future = jockey::acp::execute_runtime(
        &runtime_kind,
        agent,
        &prompt,
        &context,
        &cwd,
        &event_tx,
        false,
        mcp_servers,
        None,
        role_config_options,
        None,
        &app_session_id,
        None,
    );
    let result = tokio::time::timeout(Duration::from_secs(timeout_seconds), future)
        .await
        .map_err(|_| ResolverError::Timeout { seconds: timeout_seconds, fallback: fallback() })?;
    if !result.ok {
        return Err(ResolverError::AcpError { message: result.output, fallback: fallback() });
    }
    parse_and_validate_output(catalog, &result.output, settings.max_candidates)
}

pub(crate) fn parse_and_validate_output(catalog: &GatewayCatalog, output: &str, max_candidates: Option<u32>) -> ResolverResult<ResolverOutput> {
    let mut parsed: ResolverOutput = serde_json::from_str(output.trim())
        .map_err(|err| ResolverError::Malformed { message: err.to_string(), fallback: fallback() })?;
    for candidate in &parsed.candidates {
        if !(0.0..=1.0).contains(&candidate.confidence) {
            return malformed("candidate confidence must be between 0.0 and 1.0");
        }
        let safety = candidate.safety_class.to_string();
        if !catalog
            .validate_candidate(&candidate.domain, &candidate.command, &safety)
            .map_err(|err| ResolverError::Malformed { message: err.to_string(), fallback: fallback() })?
        {
            return malformed("candidate safetyClass does not match the registry");
        }
        if candidate.safety_class != SafetyClass::Read && !candidate.requires_exact_execute {
            return malformed("non-read candidates must set requiresExactExecute=true");
        }
    }
    parsed.candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    if let Some(max) = max_candidates {
        parsed.candidates.truncate(max as usize);
    }
    Ok(parsed)
}

pub(crate) fn render_compact_catalog(catalog: &GatewayCatalog, max_chars: u32) -> String {
    let domains = catalog_records(catalog, true, true, None);
    fit_catalog(domains, max_chars)
        .or_else(|| fit_catalog(catalog_records(catalog, false, true, None), max_chars))
        .or_else(|| fit_catalog(catalog_records(catalog, false, false, None), max_chars))
        .or_else(|| fit_catalog(catalog_records(catalog, false, false, Some(160)), max_chars))
        .unwrap_or_else(|| truncate_with_marker(&render_catalog(catalog_records(catalog, false, false, Some(80))), max_chars))
}

fn resolver_prompt(catalog: &GatewayCatalog, settings: &IntentResolverSettings, user_intent: &str) -> String {
    format!(
        "Return only JSON matching {{\"intentSummary\":string,\"candidates\":[{{\"domain\":string,\"command\":string,\"confidence\":number,\"reason\":string,\"safetyClass\":\"read|write|mutate|destructive|orchestration\",\"requiresExactExecute\":boolean,\"parametersDraft\":{{}}}}]}}.\n\
         Validate against this catalog; never execute commands. Max candidates: {}.\n\nCatalog:\n{}\n\nUser intent:\n{}",
        settings.max_candidates.unwrap_or(5),
        render_compact_catalog(catalog, settings.max_catalog_characters),
        user_intent
    )
}

fn catalog_records(catalog: &GatewayCatalog, include_examples: bool, include_schema: bool, description_limit: Option<usize>) -> Value {
    let domains: Vec<Value> = catalog
        .domains()
        .into_iter()
        .map(|d| domain_record(catalog, d, include_examples, include_schema, description_limit))
        .collect();
    json!({ "domains": domains })
}

fn domain_record(catalog: &GatewayCatalog, domain: DomainEntry, include_examples: bool, include_schema: bool, description_limit: Option<usize>) -> Value {
    let commands: Vec<Value> = catalog
        .commands(&domain.name)
        .unwrap_or_default()
        .into_iter()
        .map(|c| command_record(c, include_examples, include_schema, description_limit))
        .collect();
    json!({"name": domain.name, "description": limit(&domain.description, description_limit), "commandCount": domain.command_count, "commands": commands})
}

fn command_record(command: CommandEntry, include_examples: bool, include_schema: bool, description_limit: Option<usize>) -> Value {
    let mut value = json!({"name": command.name, "description": limit(&command.description, description_limit), "safetyClass": command.safety_class});
    if include_schema { value["schema"] = command.schema; }
    if include_examples { value["examples"] = json!([]); }
    value
}

fn fit_catalog(value: Value, max_chars: u32) -> Option<String> {
    let rendered = render_catalog(value);
    (rendered.chars().count() <= max_chars as usize).then_some(rendered)
}

fn render_catalog(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}

fn truncate_with_marker(value: &str, max_chars: u32) -> String {
    let omitted = value.chars().count().saturating_sub(max_chars as usize);
    let marker = format!("\n... omitted {omitted} catalog characters ...");
    if marker.chars().count() >= max_chars as usize {
        return marker.chars().take(max_chars as usize).collect();
    }
    let keep = (max_chars as usize).saturating_sub(marker.chars().count());
    format!("{}{}", value.chars().take(keep).collect::<String>(), marker)
}

fn limit(value: &str, max: Option<usize>) -> String {
    max.filter(|max| value.chars().count() > *max)
        .map(|max| format!("{}…", value.chars().take(max).collect::<String>()))
        .unwrap_or_else(|| value.to_string())
}

fn infer_runtime_kind(entry: &AgentEntry) -> ResolverResult<String> {
    let (binary, args) = entry.command_parts();
    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(binary)
        .to_ascii_lowercase();
    for candidate in std::iter::once(binary_name.as_str()).chain(args.into_iter()) {
        let normalized = candidate.to_ascii_lowercase();
        if normalized.contains("claude") { return Ok("claude-code".to_string()); }
        if normalized.contains("codex") { return Ok("codex-cli".to_string()); }
        if normalized.contains("gemini") { return Ok("gemini-cli".to_string()); }
    }
    Err(ResolverError::NoAgent { fallback: fallback() })
}

fn malformed<T>(message: &str) -> ResolverResult<T> {
    Err(ResolverError::Malformed { message: message.to_string(), fallback: fallback() })
}

fn fallback() -> String {
    FALLBACK.to_string()
}
