use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum DispatchResult {
    CallTool {
        server_name: String,
        tool_name: String,
        parameters: Value,
    },
    LearnCatalog(String),
    LearnSchema(Value),
    Candidates(Vec<(String, String)>),
}

pub fn dispatch_facade_call(
    server: &ConsolidatedServerTool,
    intent: Option<&str>,
    command: Option<&str>,
    learn: bool,
    parameters: Value,
) -> Result<DispatchResult> {
    if learn {
        return dispatch_learn(server, command);
    }

    if let Some(command) = command.filter(|value| !value.trim().is_empty()) {
        return dispatch_command(server, command, parameters);
    }

    if let Some(intent) = intent.filter(|value| !value.trim().is_empty()) {
        return dispatch_intent(server, intent, parameters);
    }

    Err(MaenifoldError::Message(
        "Gateway facade requires learn=true, an exact command, or an intent.".to_string(),
    ))
}

fn dispatch_learn(
    server: &ConsolidatedServerTool,
    command: Option<&str>,
) -> Result<DispatchResult> {
    if let Some(command) = command.filter(|value| !value.trim().is_empty()) {
        let tool = find_tool(server, command).ok_or_else(|| unknown_command(server, command))?;
        return Ok(DispatchResult::LearnSchema(Value::Object(
            (*tool.input_schema).clone(),
        )));
    }

    let catalog = server
        .tool_catalog
        .iter()
        .map(|tool| format!("- {}: {}", tool.name, tool_description(tool)))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(DispatchResult::LearnCatalog(format!(
        "Available commands for {}:\n{}",
        server.server_name, catalog
    )))
}

fn dispatch_command(
    server: &ConsolidatedServerTool,
    command: &str,
    parameters: Value,
) -> Result<DispatchResult> {
    let tool = find_tool(server, command).ok_or_else(|| unknown_command(server, command))?;

    Ok(DispatchResult::CallTool {
        server_name: server.server_name.clone(),
        tool_name: tool.name.to_string(),
        parameters,
    })
}

fn dispatch_intent(
    server: &ConsolidatedServerTool,
    intent: &str,
    parameters: Value,
) -> Result<DispatchResult> {
    let mut matches = server
        .tool_catalog
        .iter()
        .filter_map(|tool| {
            let score = intent_match_score(tool, intent);
            (score > 0).then(|| (score, tool))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.name.to_string().cmp(&right.1.name.to_string()))
    });

    match matches.len() {
        0 => Err(MaenifoldError::Message(format!(
            "No gateway commands matched intent '{intent}'. Use learn=true to inspect available commands."
        ))),
        1 => Ok(DispatchResult::CallTool {
            server_name: server.server_name.clone(),
            tool_name: matches[0].1.name.to_string(),
            parameters,
        }),
        2..=5 => Ok(DispatchResult::Candidates(
            matches
                .into_iter()
                .map(|(_, tool)| (tool.name.to_string(), tool_description(tool).to_string()))
                .collect(),
        )),
        _ => Err(MaenifoldError::Message(format!(
            "Intent '{intent}' matched too many gateway commands. Be more specific or use learn=true."
        ))),
    }
}

fn intent_match_score(tool: &Tool, intent: &str) -> usize {
    let normalized_intent = normalize(intent);
    if normalized_intent.is_empty() {
        return 0;
    }

    let name = normalize(&tool.name.to_string());
    let description = normalize(tool_description(tool));
    let mut score = 0;
    let mut matched_terms = 0;
    let full_name_match = name.contains(&normalized_intent);
    let full_description_match = description.contains(&normalized_intent);

    if full_name_match {
        score += 100;
    }
    if full_description_match {
        score += 25;
    }

    let mut seen_terms = HashSet::new();
    for term in tokenize(&normalized_intent) {
        if !seen_terms.insert(term.clone()) {
            continue;
        }
        let mut matched = false;
        if name.contains(&term) {
            score += 10;
            matched = true;
        }
        if description.contains(&term) {
            score += 3;
            matched = true;
        }
        if matched {
            matched_terms += 1;
        }
    }

    if full_name_match || full_description_match || matched_terms >= 2 {
        score
    } else {
        0
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect()
}

fn find_tool<'a>(server: &'a ConsolidatedServerTool, command: &str) -> Option<&'a Tool> {
    server
        .tool_catalog
        .iter()
        .find(|tool| tool.name.to_string() == command)
}

fn tool_description(tool: &Tool) -> &str {
    tool.description
        .as_deref()
        .unwrap_or("No description available.")
}

fn unknown_command(server: &ConsolidatedServerTool, command: &str) -> MaenifoldError {
    let available = server
        .tool_catalog
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    MaenifoldError::Message(format!(
        "Unknown gateway command '{command}' for '{}'. Available commands: {available}. Use learn=true to inspect available commands.",
        server.server_name
    ))
}

#[cfg(test)]
#[path = "gateway_dispatch_tests.rs"]
mod tests;
