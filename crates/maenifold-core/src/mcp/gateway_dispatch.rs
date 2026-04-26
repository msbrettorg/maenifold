use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_consolidation::ConsolidatedServerTool;
use crate::mcp::gateway_schema_validation::validate_with_validator;
use rmcp::model::Tool;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum DispatchResult {
    CallTool {
        server_name: String,
        tool_name: String,
        parameters: Value,
    },
    LearnCatalog(String),
    LearnSchema(Value),
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
        let _ = parameters;
        return Err(MaenifoldError::Message(format!(
            "Gateway intent matching is handled by GatewayIntent; intent '{intent}' requires resolver-backed routing or an exact command."
        )));
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
    validate_dispatch_parameters(server, &tool.name.to_string(), &parameters)?;

    Ok(DispatchResult::CallTool {
        server_name: server.server_name.clone(),
        tool_name: tool.name.to_string(),
        parameters,
    })
}

pub(crate) fn validate_dispatch_parameters(
    server: &ConsolidatedServerTool,
    tool_name: &str,
    parameters: &Value,
) -> Result<()> {
    let Some(validator) = server
        .tool_validators
        .get(tool_name)
        .and_then(|cached| cached.validator.as_deref())
    else {
        return Ok(());
    };
    validate_with_validator(tool_name, parameters, validator)
        .map_err(|error| MaenifoldError::GatewayInvalidParams(error.to_string()))
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
