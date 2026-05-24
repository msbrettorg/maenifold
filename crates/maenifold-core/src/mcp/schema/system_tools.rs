use super::params::{
    AcpExecParams, AdoptParams, GetConfigParams, GetCopilotModelsParams, GetHelpParams,
    ListAssetsParams, ListSessionsParams, MemoryStatusParams, ReadMcpResourceParams,
    RunFullBenchmarkParams, UpdateAssetsParams,
};
use super::support::{derived_tool_definition, tool_definition};
use crate::mcp::McpToolDefinition;
use serde_json::{Value, json};

pub(super) fn tool(registry_name: &str) -> Option<McpToolDefinition> {
    let tool = match registry_name {
        "MemoryStatus" => derived_tool_definition::<MemoryStatusParams>(
            registry_name,
            "Provides high-level memory system statistics and health information.",
        ),
        "GetConfig" => derived_tool_definition::<GetConfigParams>(
            registry_name,
            "Displays maenifold configuration settings and resolved paths.",
        ),
        "GetCopilotModels" => derived_tool_definition::<GetCopilotModelsParams>(
            registry_name,
            "Fetches Copilot /models capabilities through the daemon cache and returns exact per-model reasoning-effort options.",
        ),
        "GetHelp" => derived_tool_definition::<GetHelpParams>(
            registry_name,
            "Loads canonical help for a named public MCP tool or gateway command. Use it after you know which tool or command needs deeper usage details. It complements GatewayIndex; it is not the first cold-start domain discovery call.",
        ),
        "UpdateAssets" => derived_tool_definition::<UpdateAssetsParams>(
            registry_name,
            "Refreshes runtime assets from packaged assets with optional dry-run mode.",
        ),
        // StartWatcher/StopWatcher removed — vestigial, caused empty schema errors
        "RunFullBenchmark" => derived_tool_definition::<RunFullBenchmarkParams>(
            registry_name,
            "Runs system performance benchmarks across search, sync, and graph traversal.",
        ),
        "Adopt" => derived_tool_definition::<AdoptParams>(
            registry_name,
            "Loads a role, color, or perspective asset as an adoption instruction.",
        ),
        "ListAssets" => derived_tool_definition::<ListAssetsParams>(
            registry_name,
            "Lists available asset types or metadata for a specific asset type.",
        ),
        "ReadMcpResource" => derived_tool_definition::<ReadMcpResourceParams>(
            registry_name,
            "Reads a concrete MCP resource URI such as asset://catalog or asset://.../id. Use it when you already have a resource URI and need packaged resource content. It does not discover gateway domains or read memory:// notes; use GatewayIndex for domains.",
        ),
        "ListSessions" => derived_tool_definition::<ListSessionsParams>(
            registry_name,
            "Lists resumable ACP sessions across configured agents.",
        ),
        "AcpExec" => derived_tool_definition::<AcpExecParams>(
            registry_name,
            "Unified ACP agent invocation. Supports prompt (single message), think (sequential thinking), and workflow (structured workflow) modes. All modes share a global agent pool for process reuse.",
        ),
        "Schedule" => schedule_tool(registry_name),
        _ => return None,
    };

    Some(tool)
}

fn schedule_tool(name: &str) -> McpToolDefinition {
    tool_definition(
        name,
        "Manages recurring agent tasks and automations.",
        schedule_schema(),
    )
}

fn schedule_schema() -> Value {
    json!({
        "type": "object",
        "properties": schedule_properties(),
        "additionalProperties": false,
        "anyOf": [
            { "required": ["action"] },
            { "required": ["learn"] }
        ],
        "allOf": [
            schedule_required_fields("create", &["id", "schedule", "agent", "prompt"]),
            schedule_required_fields("update", &["id"]),
            schedule_required_fields("delete", &["id"]),
            schedule_required_fields("get", &["id"]),
            schedule_required_fields("run_now", &["id"]),
            schedule_required_fields("logs", &["id"]),
            schedule_required_fields("clear_history", &["id"]),
            schedule_required_fields("backfill", &["id", "since"]),
            schedule_required_fields("preview", &["schedule"]),
            schedule_required_fields("set_all_enabled", &["enabled"])
        ],
        "oneOf": [
            schedule_learn_branch(),
            schedule_branch_with_overrides("create", &["id", "schedule", "agent", "prompt"], &["description", "enabled", "workdir", "auto_grant_permissions", "allowed_tools", "mcp_action", "mcp_parameters", "timeout_seconds"], &[("mcp_action", json!({"type":"string","minLength":1})), ("mcp_parameters", json!({"type":"object"}))]),
            schedule_branch_with_overrides("update", &["id"], &["schedule", "agent", "prompt", "description", "enabled", "workdir", "auto_grant_permissions", "allowed_tools", "mcp_action", "mcp_parameters", "timeout_seconds"], &[("mcp_action", json!({"type":["string","null"],"minLength":1})), ("mcp_parameters", json!({"type":["object","null"]}))]),
            schedule_branch("delete", &["id"], &[]),
            schedule_branch("list", &[], &[]),
            schedule_branch("get", &["id"], &[]),
            schedule_branch("run_now", &["id"], &[]),
            schedule_branch("logs", &["id"], &["limit"]),
            schedule_branch("clear_history", &["id"], &[]),
            schedule_branch("backfill", &["id", "since"], &["until"]),
            schedule_branch("preview", &["schedule"], &[]),
            schedule_branch("verify_installed", &[], &["scheduleId"]),
            schedule_branch("pause_all", &[], &[]),
            schedule_branch("resume_all", &[], &[]),
            schedule_branch("set_all_enabled", &["enabled"], &[]),
            schedule_branch("pause_status", &[], &[])
        ]
    })
}

fn schedule_required_fields(action: &str, required: &[&str]) -> Value {
    json!({
        "if": {
            "properties": { "action": { "const": action } },
            "required": ["action"]
        },
        "then": { "required": required }
    })
}

fn schedule_properties() -> Value {
    json!({
        "action": {
            "type": "string",
            "enum": [
                "create",
                "update",
                "delete",
                "list",
                "get",
                "run_now",
                "logs",
                "clear_history",
                "backfill",
                "preview",
                "verify_installed",
                "pause_all",
                "resume_all",
                "set_all_enabled",
                "pause_status"
            ]
        },
        "id": { "type": "string", "minLength": 1 },
        "schedule": { "type": "string", "minLength": 1 },
        "agent": { "type": "string", "minLength": 1 },
        "prompt": { "type": "string", "minLength": 1 },
        "description": { "type": "string" },
        "enabled": { "type": "boolean" },
        "workdir": { "type": ["string", "null"] },
        "auto_grant_permissions": { "type": "boolean" },
        "allowed_tools": { "type": "array", "items": { "type": "string" } },
        "mcp_action": { "type": ["string", "null"], "minLength": 1 },
        "mcp_parameters": { "type": ["object", "null"] },
        "timeout_seconds": { "type": "integer", "minimum": 0 },
        "limit": { "type": "integer", "minimum": 0 },
        "since": { "type": "string", "minLength": 1 },
        "until": { "type": "string", "minLength": 1 },
        "scheduleId": { "type": "string", "minLength": 1 },
        "learn": { "type": "boolean" }
    })
}

fn schedule_learn_branch() -> Value {
    json!({
        "type": "object",
        "properties": { "learn": { "const": true } },
        "required": ["learn"],
        "additionalProperties": false
    })
}

fn schedule_branch(action: &str, required: &[&str], optional: &[&str]) -> Value {
    schedule_branch_with_overrides(action, required, optional, &[])
}

fn schedule_branch_with_overrides(
    action: &str,
    required: &[&str],
    optional: &[&str],
    overrides: &[(&str, Value)],
) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert("action".to_string(), json!({ "const": action }));

    let all_properties = schedule_properties();
    let available = all_properties
        .as_object()
        .expect("schedule properties schema must be an object");

    for name in required.iter().chain(optional.iter()) {
        let schema = overrides
            .iter()
            .find_map(|(override_name, override_value)| {
                (*override_name == *name).then(|| override_value.clone())
            })
            .unwrap_or_else(|| {
                available
                    .get(*name)
                    .cloned()
                    .expect("schedule branch property must exist in top-level schema")
            });
        properties.insert((*name).to_string(), schema);
    }

    let mut branch_required = vec!["action".to_string()];
    branch_required.extend(required.iter().map(|name| (*name).to_string()));

    Value::Object(
        [
            ("type".to_string(), json!("object")),
            ("properties".to_string(), Value::Object(properties)),
            ("required".to_string(), json!(branch_required)),
            ("additionalProperties".to_string(), json!(false)),
        ]
        .into_iter()
        .collect(),
    )
}
