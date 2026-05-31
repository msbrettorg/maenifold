use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::sync::Arc;

#[cfg(unix)]
use axum::{
    Router,
    routing::{get, post},
};
#[cfg(unix)]
use axum::{body::Bytes, extract::State, http::StatusCode, response::IntoResponse};
use rmcp::model::{CallToolResult, Content};
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use tempfile::TempDir;
#[cfg(unix)]
use tokio::sync::Mutex;

use super::super::external_mcp_transport::build_child_path_env;
use super::*;
#[cfg(unix)]
use crate::{
    acp_config::HttpAuthConfig, config::Config, tools::ToolContext, util::json::OutputFormat,
};

#[cfg(unix)]
#[path = "external_mcp_tests_http.rs"]
mod http;
#[path = "external_mcp_tests_path_env.rs"]
mod path_env;
#[path = "external_mcp_tests_resolution.rs"]
mod resolution;

fn config_with_global_and_agents(
    global_servers: HashMap<String, McpServerEntry>,
    agents: impl IntoIterator<Item = (&'static str, HashMap<String, McpServerEntry>)>,
) -> AgentsConfig {
    AgentsConfig {
        mcp_servers: global_servers,
        agents: agents
            .into_iter()
            .map(|(agent_name, mcp_servers)| {
                (
                    agent_name.to_string(),
                    crate::acp_config::AgentEntry {
                        command: "/usr/bin/env true".to_string(),
                        args: None,
                        env: HashMap::new(),
                        env_from_file: HashMap::new(),
                        env_from_command: HashMap::new(),
                        mcp_servers,
                    },
                )
            })
            .collect(),
        settings: None,
    }
}

#[cfg(unix)]
type CapturedRequests = Vec<(String, HashMap<String, String>)>;

#[cfg(unix)]
#[derive(Clone)]
struct HeaderCaptureState {
    requests: Arc<Mutex<CapturedRequests>>,
}

#[cfg(unix)]
struct EnvVarGuard {
    key: &'static str,
    original: Option<OsString>,
}

#[cfg(unix)]
impl EnvVarGuard {
    fn set(key: &'static str, value: OsString) -> Self {
        let original = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, &value);
        }
        Self { key, original }
    }
}

#[cfg(unix)]
impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.as_ref() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}

#[cfg(unix)]
fn prepend_path_guard(path: &Path) -> EnvVarGuard {
    let mut paths = vec![path.to_path_buf()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    let joined = std::env::join_paths(paths).expect("join PATH");
    EnvVarGuard::set("PATH", joined)
}

#[cfg(unix)]
fn install_fake_azure_cli(dir: &Path, token: &str) {
    use std::os::unix::fs::PermissionsExt;

    let script = dir.join("az");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\n' '{{\"accessToken\":\"{token}\",\"expires_on\":1893557045,\"tokenType\":\"Bearer\"}}'\n"
        ),
    )
    .expect("write fake az");
    let mut permissions = fs::metadata(&script)
        .expect("fake az metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).expect("chmod fake az");
}

#[cfg(unix)]
async fn oauth_metadata_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json!({
            "scopes_supported": ["api://maenifold-tests/.default"]
        })
        .to_string(),
    )
}

#[cfg(unix)]
async fn capture_mcp_headers_handler(
    State(state): State<HeaderCaptureState>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let json_body: serde_json::Value = serde_json::from_slice(&body).expect("MCP request JSON");
    let method = json_body
        .get("method")
        .and_then(|value| value.as_str())
        .expect("MCP request method")
        .to_string();
    let mut captured = HashMap::new();
    for header_name in ["authorization", "x-custom-header"] {
        if let Some(value) = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
        {
            captured.insert(header_name.to_string(), value.to_string());
        }
    }
    state.requests.lock().await.push((method.clone(), captured));

    if method == "initialize" {
        let response = json!({
            "jsonrpc": "2.0",
            "id": json_body.get("id").cloned().expect("initialize id"),
            "result": {
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "test-server",
                    "version": "1.0.0"
                }
            }
        });
        return (
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, "application/json"),
                (
                    axum::http::HeaderName::from_static("mcp-session-id"),
                    "test-session-123",
                ),
            ],
            response.to_string(),
        );
    }

    if method == "notifications/initialized" {
        return (
            StatusCode::ACCEPTED,
            [
                (axum::http::header::CONTENT_TYPE, "application/json"),
                (
                    axum::http::HeaderName::from_static("mcp-session-id"),
                    "test-session-123",
                ),
            ],
            String::new(),
        );
    }

    if method == "tools/call" {
        let response = json!({
            "jsonrpc": "2.0",
            "id": json_body.get("id").cloned().expect("tools/call id"),
            "result": serde_json::to_value(CallToolResult::success(vec![Content::text("ok")]))
                .expect("serialize call result")
        });
        return (
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, "application/json"),
                (
                    axum::http::HeaderName::from_static("mcp-session-id"),
                    "test-session-123",
                ),
            ],
            response.to_string(),
        );
    }

    panic!("unexpected MCP method: {method}");
}

#[cfg(unix)]
async fn delete_mcp_session_handler() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
