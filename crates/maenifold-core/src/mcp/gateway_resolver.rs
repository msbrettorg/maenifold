#![allow(dead_code)]

use crate::acp_config::IntentResolverSettings;
use crate::mcp::domain_mapping::SafetyClass;
use crate::mcp::gateway_catalog::{CommandEntry, DomainEntry, GatewayCatalog};
use crate::mcp::gateway_resolver_types::{ResolverError, ResolverOutput, ResolverResult};
use serde_json::{Value, json};

const FALLBACK: &str = "Use GatewayIndex to inspect available domains and commands, then call GatewayExecute explicitly.";

pub(crate) async fn resolve_intent(
    _catalog: &GatewayCatalog,
    settings: &IntentResolverSettings,
    _user_intent: &str,
    _root: &std::path::Path,
) -> ResolverResult<ResolverOutput> {
    if !settings.enabled {
        return Err(ResolverError::Disabled {
            fallback: fallback(),
        });
    }
    Err(ResolverError::DirectInferenceSetup {
        fallback: fallback(),
    })
}

pub(crate) fn parse_and_validate_output(
    catalog: &GatewayCatalog,
    output: &str,
    max_candidates: Option<u32>,
) -> ResolverResult<ResolverOutput> {
    let mut parsed: ResolverOutput =
        serde_json::from_str(output.trim()).map_err(|err| ResolverError::Malformed {
            message: err.to_string(),
            fallback: fallback(),
        })?;
    for candidate in &parsed.candidates {
        if !(0.0..=1.0).contains(&candidate.confidence) {
            return malformed("candidate confidence must be between 0.0 and 1.0");
        }
        let safety = candidate.safety_class.to_string();
        if !catalog
            .validate_candidate(&candidate.domain, &candidate.command, &safety)
            .map_err(|err| ResolverError::Malformed {
                message: err.to_string(),
                fallback: fallback(),
            })?
        {
            return malformed("candidate safetyClass does not match the registry");
        }
        if candidate.safety_class != SafetyClass::Read && !candidate.requires_exact_execute {
            return malformed("non-read candidates must set requiresExactExecute=true");
        }
    }
    parsed
        .candidates
        .sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
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
        .unwrap_or_else(|| {
            truncate_with_marker(
                &render_catalog(catalog_records(catalog, false, false, Some(80))),
                max_chars,
            )
        })
}

pub(crate) fn resolver_prompt(
    catalog: &GatewayCatalog,
    settings: &IntentResolverSettings,
    user_intent: &str,
) -> String {
    format!(
        "Return only JSON matching {{\"intentSummary\":string,\"candidates\":[{{\"domain\":string,\"command\":string,\"confidence\":number,\"reason\":string,\"safetyClass\":\"read|write|mutate|destructive|orchestration\",\"requiresExactExecute\":boolean,\"parametersDraft\":{{}}}}]}}.\n\
         Validate against this catalog; never execute commands. Max candidates: {}.\n\nCatalog:\n{}\n\nUser intent:\n{}",
        settings.max_candidates.unwrap_or(5),
        render_compact_catalog(catalog, settings.max_catalog_characters),
        user_intent
    )
}

fn catalog_records(
    catalog: &GatewayCatalog,
    include_examples: bool,
    include_schema: bool,
    description_limit: Option<usize>,
) -> Value {
    let domains: Vec<Value> = catalog
        .domains()
        .into_iter()
        .map(|d| {
            domain_record(
                catalog,
                d,
                include_examples,
                include_schema,
                description_limit,
            )
        })
        .collect();
    json!({ "domains": domains })
}

fn domain_record(
    catalog: &GatewayCatalog,
    domain: DomainEntry,
    include_examples: bool,
    include_schema: bool,
    description_limit: Option<usize>,
) -> Value {
    let commands: Vec<Value> = catalog
        .commands(&domain.name)
        .unwrap_or_default()
        .into_iter()
        .map(|c| command_record(c, include_examples, include_schema, description_limit))
        .collect();
    json!({"name": domain.name, "description": limit(&domain.description, description_limit), "commandCount": domain.command_count, "commands": commands})
}

fn command_record(
    command: CommandEntry,
    include_examples: bool,
    include_schema: bool,
    description_limit: Option<usize>,
) -> Value {
    let mut value = json!({"name": command.name, "description": limit(&command.description, description_limit), "safetyClass": command.safety_class});
    if include_schema {
        value["schema"] = command.schema;
    }
    if include_examples {
        value["examples"] = json!([]);
    }
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

fn malformed<T>(message: &str) -> ResolverResult<T> {
    Err(ResolverError::Malformed {
        message: message.to_string(),
        fallback: fallback(),
    })
}

pub(crate) fn fallback() -> String {
    FALLBACK.to_string()
}
