use crate::mcp::config_tools;
use crate::mcp::config_tools::ConfigMutationState;
use crate::mcp::domain_mapping;
use crate::mcp::gateway_registry::CommandEntry;
use crate::mcp::schema;
use rmcp::model::{CallToolResult, Content};
use serde_json::{Value, json};

pub(crate) fn commands() -> Vec<CommandEntry> {
    [
        "GetSettingsSchema",
        "SetSettings",
        "AddAgent",
        "UpdateAgent",
        "DeleteAgent",
        "AddMcpServer",
        "UpdateMcpServer",
        "DeleteMcpServer",
    ]
    .into_iter()
    .map(command_entry)
    .collect()
}

pub(crate) async fn invoke_config_command(
    state: &ConfigMutationState,
    command: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    match command.as_str() {
        "GetSettingsSchema" => Ok(CallToolResult::success(vec![Content::text(render_json(
            schema::config_tools::partial_settings_schema(),
        ))])),
        "SetSettings" => invoke_existing_tool(state, "UpdateSettings", parameters).await,
        "AddAgent" | "UpdateAgent" | "DeleteAgent" | "AddMcpServer" | "UpdateMcpServer"
        | "DeleteMcpServer" => invoke_existing_tool(state, &command, parameters).await,
        _ => Err(rmcp::ErrorData::invalid_params(
            format!("Unknown config gateway command: {command}"),
            None,
        )),
    }
}

fn command_entry(name: &str) -> CommandEntry {
    match schema::config_tools::tool(name) {
        Some(definition) => CommandEntry {
            name: definition.name,
            description: definition.description,
            schema: definition.input_schema,
            safety_class: domain_mapping::tool_safety(name).to_string(),
            meta: definition.meta,
        },
        None => {
            tracing::error!(command = name, "missing config gateway command schema");
            CommandEntry {
                name: name.to_string(),
                description: "Maenifold configuration gateway command.".to_string(),
                schema: json!({ "type": "object" }),
                safety_class: domain_mapping::tool_safety(name).to_string(),
                meta: None,
            }
        }
    }
}

async fn invoke_existing_tool(
    state: &ConfigMutationState,
    tool_name: &str,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    config_tools::handle_tool_call(state.clone(), tool_name, parameters).await
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}
