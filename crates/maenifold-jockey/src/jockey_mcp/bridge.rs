use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::types::AppState;
use super::handlers::{context, roles, sessions, skills, workflows};

const MAX_BODY_SIZE: usize = 1024 * 1024;
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

static BRIDGE_PORT: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
static BRIDGE_ERROR: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static BRIDGE_TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn bridge_port() -> Option<u16> {
    BRIDGE_PORT.get().copied()
}

pub fn bridge_error() -> Option<&'static str> {
    BRIDGE_ERROR.get().map(|s| s.as_str())
}

pub fn bridge_token() -> Option<&'static str> {
    BRIDGE_TOKEN.get().map(|s| s.as_str())
}

pub(crate) async fn start_bridge(state: Arc<AppState>) -> Result<(u16, String), String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| {
        let msg = format!("bridge bind: {e}");
        let _ = BRIDGE_ERROR.set(msg.clone());
        msg
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| {
            let msg = format!("bridge addr: {e}");
            let _ = BRIDGE_ERROR.set(msg.clone());
            msg
        })?
        .port();
    let _ = BRIDGE_PORT.set(port);

    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let _ = BRIDGE_TOKEN.set(token.clone());
    let token_arc = std::sync::Arc::new(token.clone());

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => continue,
            };
            let state = state.clone();
            let tok = token_arc.clone();
            tokio::spawn(async move {
                let result =
                    tokio::time::timeout(REQUEST_TIMEOUT, handle_connection(stream, &state, &tok))
                        .await;
                if result.is_err() {
                    eprintln!("[jockey-mcp] request timed out");
                }
            });
        }
    });

    Ok((port, token))
}

async fn handle_connection(stream: tokio::net::TcpStream, state: &AppState, token: &str) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await.is_err() || request_line.is_empty() {
        return;
    }
    let is_post = request_line.starts_with("POST ");

    let mut content_length: usize = 0;
    let mut authorized = false;
    let expected_header = format!("Bearer {token}");
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            _ => {}
        }
        if line.trim().is_empty() {
            break;
        }
        if let Some(val) = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"))
        {
            content_length = val.trim().parse().unwrap_or(0);
        }
        if let Some(val) = line
            .strip_prefix("Authorization:")
            .or_else(|| line.strip_prefix("authorization:"))
        {
            if val.trim() == expected_header {
                authorized = true;
            }
        }
    }

    if !authorized {
        let resp = http_response(401, r#"{"error":"unauthorized"}"#);
        let _ = writer.write_all(resp.as_bytes()).await;
        return;
    }

    if !is_post {
        let resp = http_response(405, r#"{"error":"method not allowed"}"#);
        let _ = writer.write_all(resp.as_bytes()).await;
        return;
    }

    if content_length > MAX_BODY_SIZE {
        let resp = http_response(413, r#"{"error":"request too large"}"#);
        let _ = writer.write_all(resp.as_bytes()).await;
        return;
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        if reader.read_exact(&mut body).await.is_err() {
            let resp = http_response(400, r#"{"error":"incomplete body"}"#);
            let _ = writer.write_all(resp.as_bytes()).await;
            return;
        }
    }
    let body_str = match String::from_utf8(body) {
        Ok(s) => s,
        Err(_) => {
            let resp = http_response(400, r#"{"error":"invalid utf-8 body"}"#);
            let _ = writer.write_all(resp.as_bytes()).await;
            return;
        }
    };

    let response_body = handle_mcp_request(state, &body_str);
    let resp = http_response(200, &response_body);
    let _ = writer.write_all(resp.as_bytes()).await;
}

fn http_response(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn handle_mcp_request(state: &AppState, body: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Req {
        id: Option<Value>,
        method: String,
        #[serde(default)]
        params: Value,
    }

    let req: Req = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            }))
            .unwrap_or_default();
        }
    };

    let id = req.id.clone().unwrap_or(Value::Null);

    let resp = match req.method.as_str() {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "jockey", "version": "0.1.0" }
            }
        }),
        "notifications/initialized" => return String::new(),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tool_definitions() }
        }),
        "tools/call" => {
            let name = req.params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = req.params.get("arguments").cloned().unwrap_or(json!({}));
            match dispatch(state, name, arguments) {
                Ok(result) => {
                    let text = match result {
                        Value::String(s) => s,
                        other => serde_json::to_string_pretty(&other).unwrap_or_default(),
                    };
                    json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "content": [{ "type": "text", "text": text }] }
                    })
                }
                Err(e) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "content": [{ "type": "text", "text": format!("Error: {e}") }], "isError": true }
                }),
            }
        }
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("method not found: {}", req.method) }
        }),
    };

    serde_json::to_string(&resp).unwrap_or_default()
}

fn dispatch(state: &AppState, method: &str, params: Value) -> Result<Value, String> {
    match method {
        // Roles
        "list_roles" => roles::list_roles(state),
        "get_role" => roles::get_role(state, params),
        "upsert_role" => roles::upsert_role_handler(state, params),
        "delete_role" => roles::delete_role_handler(state, params),
        // MCP (role-level + global registry)
        "list_mcp_servers" => roles::list_mcp_servers(state, params),
        "add_mcp_to_role" => roles::add_mcp_to_role(state, params),
        "remove_mcp_from_role" => roles::remove_mcp_from_role(state, params),
        "upsert_global_mcp" => roles::upsert_global_mcp(state, params),
        "delete_global_mcp" => roles::delete_global_mcp(state, params),
        // Skills
        "list_skills" => skills::list_skills(state),
        "get_skill" => skills::get_skill(state, params),
        "upsert_skill" => skills::upsert_skill(state, params),
        "delete_skill" => skills::delete_skill(state, params),
        // Sessions
        "list_sessions" => sessions::list_sessions(state, params),
        "get_session" => sessions::get_session(state, params),
        "update_session" => sessions::update_session(state, params),
        "create_session" => sessions::create_session(state, params),
        "close_session" => sessions::close_session(state, params),
        "get_session_history" => sessions::get_session_history(state, params),
        // Workflows
        "get_workflow" => workflows::get_workflow(state, params),
        "update_workflow" => workflows::update_workflow(state, params),
        "list_workflows" => workflows::list_workflows(state),
        "create_workflow" => workflows::create_workflow(state, params),
        "delete_workflow" => workflows::delete_workflow(state, params),
        // Shared context
        "set_shared_context" => context::set_shared_context(state, params),
        "get_shared_context" => context::get_shared_context(state, params),
        "delete_shared_context" => context::delete_shared_context(state, params),
        _ => Err(format!("unknown method: {method}")),
    }
}

fn tool_definitions() -> Vec<Value> {
    vec![
        // Roles
        json!({ "name": "list_roles", "description": "List all configured roles. A role is a named persona that wraps a runtime (claude-code, codex-cli, gemini-cli) with a system prompt, model override, mode, and MCP servers. You (the agent) are running inside one of these roles right now — call get_context to find out which one.", "inputSchema": { "type": "object", "properties": {} } }),
        json!({
            "name": "get_role", "description": "Get full details of a role including system prompt, MCP servers, config option definitions, and saved option values.",
            "inputSchema": {
                "type": "object",
                "properties": { "roleName": { "type": "string", "description": "Role name. Call list_roles to get available names." } },
                "required": ["roleName"]
            }
        }),
        json!({
            "name": "upsert_role", "description": "Create or update a role. Omitted fields preserve existing values.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "roleName": { "type": "string", "description": "Role name (unique identifier)." },
                    "runtimeKind": { "type": "string", "description": "Runtime to use: claude-code, gemini-cli, or codex-cli.", "default": "claude-code" },
                    "systemPrompt": { "type": "string", "description": "System prompt injected at the start of each session." },
                    "model": { "type": "string", "description": "Model override (e.g. claude-opus-4-5). Leave empty to use the runtime default." },
                    "mode": { "type": "string", "description": "Execution mode (e.g. acceptEdits). Leave empty for the runtime default." },
                    "autoApprove": { "type": "boolean", "description": "If true, tool calls are auto-approved without user confirmation." },
                    "mcpServersJson": { "type": "string", "description": "JSON array of MCP server objects to set, replacing any existing servers. Omit to preserve current servers." },
                    "configOptionsJson": { "type": "string", "description": "JSON object of saved option values by option id, for example {\"model\":\"claude-sonnet-4\"}. Omit to preserve current values." },
                    "configOptionDefsJson": { "type": "string", "description": "JSON array of config option definitions discovered from runtime prewarm. Omit to preserve current definitions." }
                },
                "required": ["roleName"]
            }
        }),
        json!({
            "name": "delete_role", "description": "Delete a role by name.",
            "inputSchema": {
                "type": "object",
                "properties": { "roleName": { "type": "string", "description": "Role name to delete." } },
                "required": ["roleName"]
            }
        }),
        // MCP role-level
        json!({
            "name": "list_mcp_servers",
            "description": "List MCP servers configured in Jockey. Returns the global registry (servers available to all roles) and per-role assignments. The 'jockey' server is always injected automatically — it is this tool server itself. MCP servers attached to a role are passed into the agent's session at startup; after adding/removing a server the role must be reconnected for the change to take effect. Provide roleName to scope results to one role.",
            "inputSchema": {
                "type": "object",
                "properties": { "roleName": { "type": "string", "description": "Optional. Filter to a single role. Call list_roles for available names." } }
            }
        }),
        json!({
            "name": "add_mcp_to_role",
            "description": "Add an MCP server to a role. For stdio servers provide command+args+env. For HTTP/SSE servers provide type+url+headers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "roleName": { "type": "string", "description": "Role to add the server to." },
                    "server": {
                        "type": "object",
                        "description": "MCP server definition.",
                        "properties": {
                            "name": { "type": "string", "description": "Unique name for this server on the role." },
                            "command": { "type": "string", "description": "Executable to launch (stdio transport only)." },
                            "args": { "type": "array", "items": { "type": "string" }, "description": "CLI arguments passed to command (stdio transport)." },
                            "env": { "type": "array", "items": { "type": "object", "properties": { "name": { "type": "string" }, "value": { "type": "string" } } }, "description": "Environment variables for the subprocess (stdio transport)." },
                            "type": { "type": "string", "enum": ["http", "sse"], "description": "Transport type for remote servers. Omit for stdio." },
                            "url": { "type": "string", "description": "Server URL (http or sse transport)." },
                            "headers": { "type": "array", "items": { "type": "object", "properties": { "name": { "type": "string" }, "value": { "type": "string" } } }, "description": "HTTP headers sent with each request (http/sse transport)." }
                        },
                        "required": ["name"]
                    }
                },
                "required": ["roleName", "server"]
            }
        }),
        json!({
            "name": "remove_mcp_from_role",
            "description": "Remove an MCP server from a role by server name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "roleName": { "type": "string", "description": "Role to remove the server from." },
                    "serverName": { "type": "string", "description": "Name of the MCP server to remove." }
                },
                "required": ["roleName", "serverName"]
            }
        }),
        // MCP global registry
        json!({
            "name": "upsert_global_mcp",
            "description": "Register or update an MCP server in the global registry. Use add_mcp_to_role to attach it to specific roles.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Unique server name in the global registry." },
                    "config": {
                        "type": "object",
                        "description": "Server config. stdio: {command, args[], env[]}. Remote: {type:'http'|'sse', url, headers[]}."
                    }
                },
                "required": ["name", "config"]
            }
        }),
        json!({
            "name": "delete_global_mcp",
            "description": "Remove a custom MCP server from the global registry. Built-in servers cannot be deleted.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string", "description": "Server name to delete from the global registry." } },
                "required": ["name"]
            }
        }),
        // Skills
        json!({ "name": "list_skills", "description": "List all skills with id, name, and description.", "inputSchema": { "type": "object", "properties": {} } }),
        json!({
            "name": "get_skill", "description": "Get full content of a skill by name.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string", "description": "Skill name. Call list_skills for available names." } },
                "required": ["name"]
            }
        }),
        json!({
            "name": "upsert_skill", "description": "Create or update a skill. Provide id to update an existing skill; omit id to create a new one.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Skill id. Omit to create a new skill; provide to update an existing one." },
                    "name": { "type": "string", "description": "Skill name (unique identifier shown in /skill completions)." },
                    "description": { "type": "string", "description": "Short description shown to agents when selecting skills." },
                    "content": { "type": "string", "description": "Full skill prompt/instructions content." }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "delete_skill", "description": "Delete a skill by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Skill id to delete. Call list_skills to get available ids." } },
                "required": ["id"]
            }
        }),
        // Sessions
        json!({
            "name": "list_sessions", "description": "List active (non-closed) sessions ordered by most recently active.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum number of sessions to return. Default is 50.", "default": 50 }
                }
            }
        }),
        json!({
            "name": "get_session", "description": "Get details of a session by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Session id. Call list_sessions to get available ids." } },
                "required": ["id"]
            }
        }),
        json!({
            "name": "update_session", "description": "Update a session's title or active role.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Session id." },
                    "title": { "type": "string", "description": "New title (no spaces)." },
                    "activeRole": { "type": "string", "description": "Role name to set as the active role." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "create_session", "description": "Create a new chat session.",
            "inputSchema": {
                "type": "object",
                "properties": { "title": { "type": "string", "description": "Session title (no spaces). Defaults to Session_1." } }
            }
        }),
        json!({
            "name": "close_session", "description": "Close (soft-delete) a session by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Session id to close." } },
                "required": ["id"]
            }
        }),
        json!({
            "name": "get_session_history",
            "description": "Get recent chat messages for a session, ordered chronologically.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string", "description": "Session id. Call list_sessions to get available ids." },
                    "limit": { "type": "integer", "description": "Maximum number of messages to return. Default is 50.", "default": 50 }
                },
                "required": ["sessionId"]
            }
        }),
        // Workflows
        json!({ "name": "list_workflows", "description": "List all workflows.", "inputSchema": { "type": "object", "properties": {} } }),
        json!({
            "name": "get_workflow", "description": "Get full details of a workflow by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Workflow id. Call list_workflows to get available ids." } },
                "required": ["id"]
            }
        }),
        json!({
            "name": "create_workflow", "description": "Create a workflow with an ordered list of role steps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Workflow name (no spaces)." },
                    "steps": { "type": "array", "items": { "type": "string" }, "description": "Ordered list of role names to execute in sequence." }
                },
                "required": ["name", "steps"]
            }
        }),
        json!({
            "name": "update_workflow", "description": "Update a workflow's name or steps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Workflow id." },
                    "name": { "type": "string", "description": "New workflow name (no spaces)." },
                    "steps": { "type": "array", "items": { "type": "string" }, "description": "Replacement ordered list of role names." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "delete_workflow", "description": "Delete a workflow by id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string", "description": "Workflow id to delete." } },
                "required": ["id"]
            }
        }),
        // Shared context
        json!({
            "name": "set_shared_context", "description": "Set a key-value entry in shared context. Use scopes to namespace entries (e.g. a session id or 'global').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "description": "Namespace for the entry. Default is 'global'.", "default": "global" },
                    "key": { "type": "string", "description": "Context key." },
                    "value": { "type": "string", "description": "Context value." }
                },
                "required": ["key", "value"]
            }
        }),
        json!({
            "name": "get_shared_context", "description": "Get all shared context entries for a scope.",
            "inputSchema": {
                "type": "object",
                "properties": { "scope": { "type": "string", "description": "Scope to query. Default is 'global'.", "default": "global" } }
            }
        }),
        json!({
            "name": "delete_shared_context", "description": "Delete a shared context entry by scope and key.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "scope": { "type": "string", "description": "Scope of the entry. Default is 'global'.", "default": "global" },
                    "key": { "type": "string", "description": "Key to delete." }
                },
                "required": ["key"]
            }
        }),
    ]
}
