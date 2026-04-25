use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_schema_validation::{compile_validator, compute_schema_hash};
use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConsolidatedServerTool {
    pub server_name: String,
    pub tool_catalog: Vec<Tool>,
    pub tool_validators: HashMap<String, CachedToolValidator>,
    pub facade_tool: Tool,
}

#[derive(Clone)]
pub struct CachedToolValidator {
    pub schema_hash: String,
    pub validator: Option<Arc<jsonschema::Validator>>,
}

impl fmt::Debug for ConsolidatedServerTool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConsolidatedServerTool")
            .field("server_name", &self.server_name)
            .field("tool_catalog", &self.tool_catalog)
            .field("tool_validators", &self.tool_validators)
            .field("facade_tool", &self.facade_tool)
            .finish()
    }
}

impl fmt::Debug for CachedToolValidator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CachedToolValidator")
            .field("schema_hash", &self.schema_hash)
            .field("has_validator", &self.validator.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GatewayToolCounts {
    pub total: usize,
    pub facades: usize,
    pub passthrough: usize,
}

pub fn build_consolidated_tools(
    gateway_sources: Vec<(String, Vec<Tool>)>,
    min_tools_for_consolidation: usize,
) -> Result<(Vec<ConsolidatedServerTool>, Vec<Tool>)> {
    assert!(
        min_tools_for_consolidation >= 2,
        "min_tools_for_consolidation must be at least 2"
    );

    let mut consolidated = Vec::new();
    let mut passthrough = Vec::new();
    let mut facade_names = HashMap::<String, String>::new();

    for (server_name, mut tools) in gateway_sources {
        validate_server_name(&server_name)?;
        if tools.is_empty() {
            continue;
        }

        tools.sort_by(|left, right| tool_name(left).cmp(&tool_name(right)));

        if should_passthrough_server(&tools, min_tools_for_consolidation) {
            passthrough.extend(tools);
            continue;
        }

        let facade_name = facade_name(&server_name);
        if let Some(other_server) = facade_names.insert(facade_name.clone(), server_name.clone()) {
            return Err(MaenifoldError::Message(format!(
                "Gateway facade name collision: '{facade_name}' is produced by both '{other_server}' and '{server_name}'"
            )));
        }

        consolidated.push(ConsolidatedServerTool {
            server_name: server_name.clone(),
            tool_validators: cached_tool_validators(&tools),
            tool_catalog: tools.clone(),
            facade_tool: Tool::new(
                facade_name,
                facade_description(&server_name, &tools),
                Arc::new(facade_input_schema()),
            ),
        });
    }

    consolidated.sort_by(|left, right| left.server_name.cmp(&right.server_name));
    passthrough.sort_by(|left, right| tool_name(left).cmp(&tool_name(right)));
    Ok((consolidated, passthrough))
}

pub(super) fn gateway_tool_counts(
    consolidated: &[ConsolidatedServerTool],
    passthrough: &[Tool],
    native_tools: &[Tool],
) -> GatewayToolCounts {
    GatewayToolCounts {
        total: consolidated.len() + passthrough.len() + native_tools.len(),
        facades: consolidated.len(),
        passthrough: passthrough.len(),
    }
}

fn validate_server_name(server_name: &str) -> Result<()> {
    if server_name.trim().is_empty() {
        return Err(MaenifoldError::Message(
            "Gateway server name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn facade_name(server_name: &str) -> String {
    let sanitized: String = server_name
        .chars()
        .flat_map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec!['_']
            }
        })
        .take(60)
        .collect();
    format!("gateway_{sanitized}")
}

fn facade_description(server_name: &str, tools: &[Tool]) -> String {
    const LEARN_HINT: &str = " Use learn=true to discover available commands.";
    let summary = capability_summary(tools);
    let base = truncate(
        &format!("{server_name} — {summary}."),
        200usize.saturating_sub(LEARN_HINT.chars().count()),
    );
    format!("{base}{LEARN_HINT}")
}

fn capability_summary(tools: &[Tool]) -> String {
    let mut frequencies = HashMap::<String, usize>::new();
    for tool in tools {
        let mut seen = HashSet::new();
        for token in split_name(&tool_name(tool)) {
            if seen.insert(token.clone()) {
                *frequencies.entry(token).or_insert(0) += 1;
            }
        }
    }

    let mut tokens: Vec<_> = frequencies.into_iter().collect();
    tokens.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let top_tokens: Vec<_> = tokens.into_iter().map(|(token, _)| token).take(5).collect();

    if top_tokens.is_empty() {
        "gateway commands".to_string()
    } else {
        format!("commands covering {}", top_tokens.join(", "))
    }
}

fn split_name(name: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(name.len());
    let mut previous_was_lower_or_digit = false;

    for ch in name.chars() {
        if ch.is_uppercase() {
            if previous_was_lower_or_digit {
                normalized.push('_');
            }
            normalized.extend(ch.to_lowercase());
            previous_was_lower_or_digit = false;
        } else if ch.is_alphanumeric() {
            normalized.extend(ch.to_lowercase());
            previous_was_lower_or_digit = ch.is_lowercase() || ch.is_numeric();
        } else {
            normalized.push('_');
            previous_was_lower_or_digit = false;
        }
    }

    normalized
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn truncate(text: &str, max_chars: usize) -> String {
    text.chars()
        .take(max_chars)
        .collect::<String>()
        .trim_end_matches([' ', ',', ';', ':'])
        .to_string()
}

fn facade_input_schema() -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert(
        "intent".to_string(),
        schema_property(
            "string",
            "Natural language description of what you want to do",
        ),
    );
    properties.insert(
        "command".to_string(),
        schema_property("string", "Exact tool name to invoke"),
    );
    properties.insert(
        "learn".to_string(),
        schema_property("boolean", "Discovery mode - returns available commands"),
    );
    properties.insert(
        "parameters".to_string(),
        schema_property("object", "Arguments forwarded to the selected tool"),
    );

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(properties));
    schema.insert("additionalProperties".to_string(), Value::Bool(false));
    schema
}

fn schema_property(type_name: &str, description: &str) -> Value {
    let mut property = Map::new();
    property.insert("type".to_string(), Value::String(type_name.to_string()));
    property.insert(
        "description".to_string(),
        Value::String(description.to_string()),
    );
    Value::Object(property)
}

fn should_passthrough_server(tools: &[Tool], min_tools_for_consolidation: usize) -> bool {
    tools.len() < min_tools_for_consolidation || tools.iter().any(is_hierarchical_tool)
}

fn cached_tool_validators(tools: &[Tool]) -> HashMap<String, CachedToolValidator> {
    tools
        .iter()
        .map(|tool| {
            let schema = Value::Object((*tool.input_schema).clone());
            (
                tool_name(tool),
                CachedToolValidator {
                    schema_hash: compute_schema_hash(&schema),
                    validator: compile_validator(&schema),
                },
            )
        })
        .collect()
}

fn is_hierarchical_tool(tool: &Tool) -> bool {
    tool.input_schema
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| {
            ["command", "learn", "intent"]
                .iter()
                .any(|name| properties.contains_key(*name))
        })
}

fn tool_name(tool: &Tool) -> String {
    tool.name.to_string()
}

#[cfg(test)]
#[path = "gateway_consolidation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gateway_consolidation_counts_tests.rs"]
mod counts_tests;
