use super::support::tool_definition;
use crate::mcp::McpToolDefinition;
use serde_json::{Value, json};

const NAME_PARAM: &str = "name";

pub(crate) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    match registry_name {
        "GetSettingsSchema" => Some(tool_definition(
            "GetSettingsSchema",
            "Returns the canonical JSON Schema for partial maenifold settings updates.",
            object_schema(json!({})),
        )),
        "ReloadConfig" => Some(tool_definition(
            "ReloadConfig",
            "Reloads maenifold.json and refreshes live daemon settings.",
            object_schema(json!({})),
        )),
        "UpdateSettings" => Some(settings_tool(
            "UpdateSettings",
            "Deep-merges partial Maenifold settings into maenifold.json and refreshes live settings.",
        )),
        "SetSettings" => Some(settings_tool(
            "SetSettings",
            "Deep-merges partial Maenifold settings into maenifold.json and refreshes live settings.",
        )),
        "AddAgent" => Some(agent_tool(
            "AddAgent",
            "Adds or replaces a configured ACP agent entry.",
        )),
        "UpdateAgent" => Some(agent_tool(
            "UpdateAgent",
            "Updates an existing configured ACP agent entry.",
        )),
        "DeleteAgent" => Some(name_tool(
            "DeleteAgent",
            "Deletes a configured ACP agent by name.",
        )),
        "AddMcpServer" => Some(mcp_server_tool(
            "AddMcpServer",
            "Adds a configured MCP server entry.",
        )),
        "UpdateMcpServer" => Some(mcp_server_tool(
            "UpdateMcpServer",
            "Updates an existing configured MCP server entry by name.",
        )),
        "DeleteMcpServer" => Some(name_tool(
            "DeleteMcpServer",
            "Deletes a configured MCP server by name.",
        )),
        _ => None,
    }
}

fn settings_tool(name: &str, description: &str) -> McpToolDefinition {
    tool_definition(
        name,
        description,
        object_schema(json!({ "settings": partial_settings_schema() })),
    )
}

fn agent_tool(name: &str, description: &str) -> McpToolDefinition {
    let mut properties = json!({ "config": agent_entry_schema() });
    properties[NAME_PARAM] = name_param_schema();
    tool_definition(name, description, object_schema(properties))
}

fn name_tool(name: &str, description: &str) -> McpToolDefinition {
    let mut properties = json!({});
    properties[NAME_PARAM] = name_param_schema();
    tool_definition(name, description, object_schema(properties))
}

fn name_param_schema() -> Value {
    json!({ "type": "string", "minLength": 1 })
}

fn mcp_server_tool(name: &str, description: &str) -> McpToolDefinition {
    let mut properties = json!({ "entry": mcp_server_schema() });
    properties[NAME_PARAM] = name_param_schema();
    tool_definition(name, description, object_schema(properties))
}

fn object_schema(properties: Value) -> Value {
    let required = properties
        .as_object()
        .map(|props| props.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn agent_entry_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string", "minLength": 1 },
            "args": { "type": "array", "items": { "type": "string" } },
            "env": { "type": "object", "additionalProperties": { "type": "string" } },
            "envFromFile": { "type": "object", "additionalProperties": { "type": "string" } },
            "envFromCommand": env_from_command_schema(),
            "mcpServers": { "type": "object", "additionalProperties": mcp_server_schema() }
        },
        "required": ["command"],
        "additionalProperties": false
    })
}

fn mcp_server_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "minLength": 1 },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "env": { "type": "object", "additionalProperties": { "type": "string" } },
                    "envFromFile": { "type": "object", "additionalProperties": { "type": "string" } },
                    "envFromCommand": env_from_command_schema(),
                    "gateway": { "type": "boolean" }
                },
                "required": ["command"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "minLength": 1 },
                    "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                    "auth": { "type": "string", "enum": ["azure-default"] },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "gateway": { "type": "boolean" }
                },
                "required": ["url"],
                "additionalProperties": false
            }
        ]
    })
}

fn env_from_command_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": {
            "type": "object",
            "properties": {
                "command": { "type": "string", "minLength": 1 },
                "args": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["command"],
            "additionalProperties": false
        }
    })
}

pub(crate) fn partial_settings_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "logRotation": { "type": "string" },
            "debounceMs": { "type": "integer", "minimum": 0 },
            "autoSync": { "type": "boolean" },
            "syncLogging": { "type": "boolean" },
            "sessionCleanup": { "type": "boolean" },
            "sqliteBusyTimeoutMs": { "type": "integer", "minimum": 0 },
            "acpConnectionTimeoutSeconds": { "type": "integer", "minimum": 1 },
            "vector": { "type": "object", "additionalProperties": true },
            "debug": { "type": "object", "additionalProperties": true },
            "community": { "type": "object", "additionalProperties": true },
            "decay": { "type": "object", "additionalProperties": true },
            "gateway": { "type": "object", "additionalProperties": true },
            "envPassthrough": { "type": "array", "items": { "type": "string" } }
        },
        "additionalProperties": true
    })
}
