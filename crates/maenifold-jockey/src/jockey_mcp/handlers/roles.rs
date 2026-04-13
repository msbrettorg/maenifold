use serde_json::{json, Value};

use crate::db::global_mcp::{delete_global_mcp_server, list_global_mcp_servers, upsert_global_mcp_server};
use crate::db::role::{delete_role_internal, list_all_roles, load_role, upsert_role};
use crate::types::AppState;

pub(crate) fn list_roles(state: &AppState) -> Result<Value, String> {
    let roles = list_all_roles(state)?;
    let out: Vec<Value> = roles
        .iter()
        .map(|r| {
            json!({
                "roleName": r.role_name,
                "runtimeKind": r.runtime_kind,
                "model": r.model,
                "mode": r.mode,
                "mcpServers": serde_json::from_str::<Value>(&r.mcp_servers_json).unwrap_or(json!([])),
                "configOptions": serde_json::from_str::<Value>(&r.config_options_json).unwrap_or(json!({})),
                "configOptionDefs": serde_json::from_str::<Value>(&r.config_option_defs_json).unwrap_or(json!([])),
                "autoApprove": r.auto_approve,
            })
        })
        .collect();
    Ok(json!(out))
}

pub(crate) fn get_role(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params
        .get("roleName")
        .and_then(|v| v.as_str())
        .ok_or("roleName is required")?;
    let role = load_role(state, role_name)?
        .ok_or_else(|| format!("role not found: {role_name}"))?;
    Ok(json!({
        "roleName": role.role_name,
        "runtimeKind": role.runtime_kind,
        "systemPrompt": role.system_prompt,
        "model": role.model,
        "mode": role.mode,
        "mcpServers": serde_json::from_str::<Value>(&role.mcp_servers_json).unwrap_or(json!([])),
        "configOptions": serde_json::from_str::<Value>(&role.config_options_json).unwrap_or(json!({})),
        "configOptionDefs": serde_json::from_str::<Value>(&role.config_option_defs_json).unwrap_or(json!([])),
        "autoApprove": role.auto_approve,
    }))
}

pub(crate) fn upsert_role_handler(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params
        .get("roleName")
        .and_then(|v| v.as_str())
        .ok_or("roleName is required")?
        .to_string();
    let runtime_kind = params
        .get("runtimeKind")
        .and_then(|v| v.as_str())
        .unwrap_or("claude-code")
        .to_string();
    let system_prompt = params
        .get("systemPrompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let model = params.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
    let mode = params.get("mode").and_then(|v| v.as_str()).map(|s| s.to_string());
    let auto_approve = params.get("autoApprove").and_then(|v| v.as_bool());
    let mcp_servers_json = params.get("mcpServersJson").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config_options_json = params.get("configOptionsJson").and_then(|v| v.as_str()).map(|s| s.to_string());
    let config_option_defs_json = params.get("configOptionDefsJson").and_then(|v| v.as_str()).map(|s| s.to_string());

    upsert_role(
        state,
        role_name.clone(),
        runtime_kind.clone(),
        system_prompt,
        model,
        mode,
        mcp_servers_json,
        config_options_json,
        config_option_defs_json,
        auto_approve,
    )?;
    Ok(json!(format!("Role '{role_name}' saved (runtime: {runtime_kind})")))
}

pub(crate) fn delete_role_handler(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params
        .get("roleName")
        .and_then(|v| v.as_str())
        .ok_or("roleName is required")?;
    delete_role_internal(state, role_name)?;
    Ok(json!(format!("Role '{role_name}' deleted")))
}

pub(crate) fn list_mcp_servers(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params.get("roleName").and_then(|v| v.as_str());
    let global = list_global_mcp_servers(state)?;
    let global_out: Vec<Value> = global
        .iter()
        .map(|g| {
            json!({
                "name": g.name,
                "config": serde_json::from_str::<Value>(&g.config_json).unwrap_or(json!({})),
                "isBuiltin": g.is_builtin,
            })
        })
        .collect();

    let roles = list_all_roles(state)?;
    if let Some(rn) = role_name {
        let role = roles
            .iter()
            .find(|r| r.role_name == rn)
            .ok_or_else(|| format!("role not found: {rn}"))?;
        let role_servers: Value = serde_json::from_str(&role.mcp_servers_json).unwrap_or(json!([]));
        return Ok(json!({
            "globalRegistry": global_out,
            "roleServers": role_servers,
        }));
    }

    let roles_out: Vec<Value> = roles
        .iter()
        .map(|r| {
            json!({
                "roleName": r.role_name,
                "mcpServers": serde_json::from_str::<Value>(&r.mcp_servers_json).unwrap_or(json!([]))
            })
        })
        .collect();
    Ok(json!({
        "globalRegistry": global_out,
        "roles": roles_out,
    }))
}

pub(crate) fn add_mcp_to_role(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params
        .get("roleName")
        .and_then(|v| v.as_str())
        .ok_or("roleName is required")?;
    let server = params.get("server").ok_or("server object is required")?;
    let new_name = server.get("name").and_then(|v| v.as_str()).unwrap_or("");

    let role = load_role(state, role_name)?.ok_or_else(|| format!("role not found: {role_name}"))?;
    let mut servers: Vec<Value> = serde_json::from_str(&role.mcp_servers_json).map_err(|e| e.to_string())?;

    if servers.iter().any(|s| s.get("name").and_then(|v| v.as_str()) == Some(new_name)) {
        return Err(format!("MCP server '{new_name}' already exists on role '{role_name}'"));
    }

    servers.push(server.clone());
    let updated = serde_json::to_string(&servers).map_err(|e| e.to_string())?;

    upsert_role(
        state,
        role.role_name,
        role.runtime_kind,
        role.system_prompt,
        role.model,
        role.mode,
        Some(updated),
        Some(role.config_options_json),
        Some(role.config_option_defs_json),
        Some(role.auto_approve),
    )?;

    Ok(json!(format!(
        "Added MCP server '{new_name}' to role '{role_name}'. Total servers: {}",
        servers.len()
    )))
}

pub(crate) fn remove_mcp_from_role(state: &AppState, params: Value) -> Result<Value, String> {
    let role_name = params
        .get("roleName")
        .and_then(|v| v.as_str())
        .ok_or("roleName is required")?;
    let server_name = params
        .get("serverName")
        .and_then(|v| v.as_str())
        .ok_or("serverName is required")?;

    let role = load_role(state, role_name)?.ok_or_else(|| format!("role not found: {role_name}"))?;
    let mut servers: Vec<Value> = serde_json::from_str(&role.mcp_servers_json).map_err(|e| e.to_string())?;
    let before = servers.len();
    servers.retain(|s| s.get("name").and_then(|v| v.as_str()) != Some(server_name));
    if servers.len() == before {
        return Err(format!("MCP server '{server_name}' not found on role '{role_name}'"));
    }

    let updated = serde_json::to_string(&servers).map_err(|e| e.to_string())?;
    upsert_role(
        state,
        role.role_name,
        role.runtime_kind,
        role.system_prompt,
        role.model,
        role.mode,
        Some(updated),
        Some(role.config_options_json),
        Some(role.config_option_defs_json),
        Some(role.auto_approve),
    )?;

    Ok(json!(format!(
        "Removed MCP server '{server_name}' from role '{role_name}'. Remaining: {}",
        servers.len()
    )))
}

pub(crate) fn upsert_global_mcp(state: &AppState, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("name is required")?;
    let config = params.get("config").ok_or("config object is required")?;
    let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;
    upsert_global_mcp_server(state, name, &config_json, false)?;
    Ok(json!(format!("Global MCP server '{name}' saved")))
}

pub(crate) fn delete_global_mcp(state: &AppState, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("name is required")?;
    delete_global_mcp_server(state, name)?;
    Ok(json!(format!("Global MCP server '{name}' deleted")))
}
