use agent_client_protocol::schema::{self as acp};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use super::with_db;
use crate::types::AppState;

static GLOBAL_MCP_CACHE: OnceLock<RwLock<HashMap<usize, Vec<GlobalMcpEntry>>>> = OnceLock::new();
static GLOBAL_MCP_ACP_CACHE: OnceLock<RwLock<HashMap<usize, Vec<acp::McpServer>>>> =
    OnceLock::new();

fn global_mcp_cache() -> &'static RwLock<HashMap<usize, Vec<GlobalMcpEntry>>> {
    GLOBAL_MCP_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn global_mcp_acp_cache() -> &'static RwLock<HashMap<usize, Vec<acp::McpServer>>> {
    GLOBAL_MCP_ACP_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn cache_key(state: &AppState) -> usize {
    state.db.cache_key()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GlobalMcpEntry {
    pub name: String,
    pub config_json: String,
    pub is_builtin: bool,
}

pub(crate) fn list_global_mcp_servers(state: &AppState) -> Result<Vec<GlobalMcpEntry>, String> {
    let key = cache_key(state);
    if let Ok(cache) = global_mcp_cache().read() {
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }
    let result = with_db(state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT name, config_json, is_builtin
                 FROM global_mcp_servers
                 ORDER BY is_builtin DESC, name ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(GlobalMcpEntry {
                    name: row.get(0)?,
                    config_json: row.get(1)?,
                    is_builtin: row.get::<_, i64>(2)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
        Ok(entries)
    })?;
    if let Ok(mut cache) = global_mcp_cache().write() {
        cache.insert(key, result.clone());
    }
    Ok(result)
}

pub(crate) fn load_all_global_mcp_as_acp(state: &AppState) -> Vec<acp::McpServer> {
    let key = cache_key(state);
    if let Ok(cache) = global_mcp_acp_cache().read() {
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
    }
    let servers = list_global_mcp_servers(state)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let json = inject_name_if_missing(&entry.config_json, &entry.name);
            parse_mcp_server_json_compat(&json)
        })
        .collect::<Vec<_>>();
    if let Ok(mut cache) = global_mcp_acp_cache().write() {
        cache.insert(key, servers.clone());
    }
    servers
}

fn inject_name_if_missing(config_json: &str, name: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(config_json) else {
        return config_json.to_string();
    };
    if let Some(object) = value.as_object_mut() {
        object
            .entry("name".to_string())
            .or_insert_with(|| serde_json::Value::String(name.to_string()));
    }
    value.to_string()
}

fn normalize_mcp_server_json(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        if object.contains_key("command") {
            object
                .entry("args".to_string())
                .or_insert_with(|| json!([]));
            object.entry("env".to_string()).or_insert_with(|| json!([]));
        }
        if object.contains_key("url") {
            object
                .entry("headers".to_string())
                .or_insert_with(|| json!([]));
        }
    }
    value
}

pub(crate) fn parse_mcp_server_json_compat(raw: &str) -> Option<acp::McpServer> {
    serde_json::from_str::<acp::McpServer>(raw)
        .ok()
        .or_else(|| {
            let value = serde_json::from_str::<Value>(raw).ok()?;
            serde_json::from_value::<acp::McpServer>(normalize_mcp_server_json(value)).ok()
        })
}

#[cfg(test)]
pub(crate) fn parse_mcp_server_list_json_compat(raw: &str) -> Vec<acp::McpServer> {
    serde_json::from_str::<Vec<acp::McpServer>>(raw)
        .ok()
        .or_else(|| {
            let values = serde_json::from_str::<Vec<Value>>(raw).ok()?;
            Some(
                values
                    .into_iter()
                    .filter_map(|value| {
                        serde_json::from_value::<acp::McpServer>(value.clone())
                            .ok()
                            .or_else(|| {
                                serde_json::from_value::<acp::McpServer>(normalize_mcp_server_json(
                                    value,
                                ))
                                .ok()
                            })
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_mcp_server_list_json_compat;

    #[test]
    fn parses_legacy_stdio_without_env() {
        let raw = r#"[{"name":"chrome-devtools","command":"npx","args":["-y","@chrome-devtools/chrome-devtools-mcp"]}]"#;
        let parsed = parse_mcp_server_list_json_compat(raw);
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn parses_legacy_http_without_headers() {
        let raw = r#"[{"type":"http","name":"demo","url":"http://127.0.0.1:9999"}]"#;
        let parsed = parse_mcp_server_list_json_compat(raw);
        assert_eq!(parsed.len(), 1);
    }
}
