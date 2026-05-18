use std::{collections::HashMap, ffi::OsStr, ffi::OsString};

use rmcp::model::{CallToolResult, Content};

use super::super::external_mcp_transport::build_child_path_env;
use super::*;

fn config_with_global_and_agents(
    global_servers: Vec<McpServerEntry>,
    agents: impl IntoIterator<Item = (&'static str, Vec<McpServerEntry>)>,
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

#[test]
fn split_namespaced_tool_name_requires_non_empty_segments() {
    assert_eq!(
        split_namespaced_tool_name("weather.forecast"),
        Some(("weather", "forecast"))
    );
    assert_eq!(split_namespaced_tool_name("weather."), None);
    assert_eq!(split_namespaced_tool_name(".forecast"), None);
    assert_eq!(split_namespaced_tool_name("forecast"), None);
}

#[test]
fn resolve_server_entry_finds_global_server_without_agent() {
    let server = McpServerEntry::Http {
        name: "gate".to_string(),
        url: "http://localhost:8081".to_string(),
    };
    let config = config_with_global_and_agents(vec![server.clone()], []);

    let resolved = resolve_server_entry(&config, "gate", None).expect("global server resolves");

    assert_eq!(resolved, server);
}

#[test]
fn resolve_server_entry_prefers_global_over_agent() {
    let global = McpServerEntry::Http {
        name: "gate".to_string(),
        url: "http://localhost:8081".to_string(),
    };
    let agent_local = McpServerEntry::Stdio {
        name: "gate".to_string(),
        command: "/usr/bin/gate".to_string(),
        args: vec![],
        timeout: None,
        env: HashMap::new(),
        env_from_file: HashMap::new(),
        env_from_command: HashMap::new(),
        gateway: false,
    };
    let config =
        config_with_global_and_agents(vec![global.clone()], [("agent-a", vec![agent_local])]);

    let resolved = resolve_server_entry(&config, "gate", Some("agent-a"))
        .expect("global server takes precedence");

    assert_eq!(resolved, global);
}

#[test]
fn resolve_server_entry_falls_back_to_agent_server() {
    let agent_server = McpServerEntry::Stdio {
        name: "weather".to_string(),
        command: "/usr/bin/weather".to_string(),
        args: vec![],
        timeout: None,
        env: HashMap::new(),
        env_from_file: HashMap::new(),
        env_from_command: HashMap::new(),
        gateway: false,
    };
    let config = config_with_global_and_agents(vec![], [("agent-a", vec![agent_server.clone()])]);

    let resolved = resolve_server_entry(&config, "weather", Some("agent-a"))
        .expect("agent server resolves as fallback");

    assert_eq!(resolved, agent_server);
}

#[test]
fn resolve_server_entry_errors_when_not_found() {
    let config = config_with_global_and_agents(vec![], []);
    let error =
        resolve_server_entry(&config, "missing", None).expect_err("unknown server should fail");
    assert!(error.to_string().contains("not found"));
}

#[test]
fn require_tool_allowed_defaults_to_all_when_none() {
    require_tool_allowed("gate.status", None).expect("None means all tools allowed");
}

#[test]
fn require_tool_allowed_permits_listed_tool() {
    let allowed = vec!["gate.status".to_string()];
    require_tool_allowed("gate.status", Some(&allowed)).expect("explicitly listed tool is allowed");
}

#[test]
fn require_tool_allowed_rejects_unlisted_tool() {
    let allowed = vec!["gate.sync".to_string()];
    let error = require_tool_allowed("gate.status", Some(&allowed))
        .expect_err("unlisted tool should be rejected");
    assert!(
        error
            .to_string()
            .contains("not in the explicit allowed_tools")
    );
}

#[test]
fn require_tool_allowed_permits_local_tool_when_listed() {
    let allowed = vec!["maenifold.WriteMemory".to_string()];
    require_tool_allowed("maenifold.WriteMemory", Some(&allowed))
        .expect("explicitly listed local tool is allowed");
}

#[test]
fn require_tool_allowed_rejects_local_tool_when_unlisted() {
    let allowed = vec!["gate.status".to_string()];
    let error = require_tool_allowed("maenifold.WriteMemory", Some(&allowed))
        .expect_err("unlisted local tool should be rejected");
    let message = error.to_string();
    assert!(
        message.contains("maenifold.WriteMemory"),
        "error should name blocked tool: {message}"
    );
    assert!(
        message.contains("gate.status"),
        "error should name allowed set: {message}"
    );
    assert!(message.contains("not in the explicit allowed_tools"));
}

#[test]
fn require_tool_allowed_permits_all_local_tools_when_none() {
    require_tool_allowed("maenifold.WriteMemory", None)
        .expect("None means all local tools are allowed");
}

#[test]
fn tool_arguments_rejects_non_object_values() {
    let error = tool_arguments(Some(&serde_json::json!("string")))
        .expect_err("non-object arguments should be rejected");

    assert_eq!(
        error.to_string(),
        "Invalid payload: External MCP tool arguments must be a JSON object"
    );
}

#[test]
fn require_tool_allowed_rejects_banned_tool_against_allow_list() {
    let allowed = vec!["good_tool".to_string()];
    let error = require_tool_allowed("banned_tool", Some(&allowed))
        .expect_err("disallowed tool should be rejected");

    assert_eq!(
        error.to_string(),
        "Agent setup failed: Tool 'banned_tool' is not in the explicit allowed_tools list: [\"good_tool\"]"
    );
}

#[test]
fn resolve_server_entry_reports_unresolved_server_name_clearly() {
    let config = config_with_global_and_agents(vec![], []);
    let error = resolve_server_entry(&config, "missing-server", Some("agent-a"))
        .expect_err("missing server should produce a clear error");

    assert_eq!(
        error.to_string(),
        "Agent setup failed: MCP server 'missing-server' not found in agent 'agent-a' or global config"
    );
}

#[test]
fn render_tool_result_prefers_structured_content() {
    let rendered = render_tool_result(CallToolResult::structured(serde_json::json!({
        "ok": true
    })))
    .expect("structured result renders");

    assert_eq!(rendered, r#"{"ok":true}"#);
}

#[test]
fn render_tool_result_uses_text_content_when_present() {
    let rendered = render_tool_result(CallToolResult::success(vec![Content::text("done")]))
        .expect("text result renders");

    assert_eq!(rendered, "done");
}

#[test]
fn render_tool_result_surfaces_external_errors() {
    let error = render_tool_result(CallToolResult::error(vec![Content::text("boom")]))
        .expect_err("error result should propagate");

    assert_eq!(error.to_string(), "boom");
}

#[test]
fn finalize_external_tool_result_preserves_success_when_close_fails() {
    let result = finalize_external_tool_result(
        "weather",
        Ok(CallToolResult::success(vec![Content::text("done")])),
        Err(MaenifoldError::Message("close failed".to_string())),
    )
    .expect("successful tool output should survive close failure");

    assert_eq!(result, "done");
}

#[test]
fn build_child_path_env_adds_common_user_tool_dirs_and_command_parent() {
    let path = build_child_path_env(
        Some(OsString::from("/usr/bin:/bin")),
        Some(OsString::from("/Users/tester")),
        "/Users/tester/.npm-global/bin/workiq",
    )
    .expect("path should be constructed");
    let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
        .map(|entry| entry.to_string_lossy().to_string())
        .collect();

    assert!(entries.iter().any(|entry| entry == "/usr/bin"));
    assert!(entries.iter().any(|entry| entry == "/bin"));
    assert!(
        entries
            .iter()
            .any(|entry| entry == "/Users/tester/.npm-global/bin")
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry == "/Users/tester/.dotnet/tools")
    );
    assert!(entries.iter().any(|entry| entry == "/opt/homebrew/bin"));
}

#[test]
fn build_child_path_env_deduplicates_existing_entries() {
    let path = build_child_path_env(
        Some(OsString::from("/usr/bin:/bin:/usr/bin")),
        Some(OsString::from("/Users/tester")),
        "/usr/bin/tool",
    )
    .expect("path should be constructed");
    let entries: Vec<_> = std::env::split_paths(OsStr::new(&path))
        .map(|entry| entry.to_string_lossy().to_string())
        .collect();

    assert_eq!(
        entries.iter().filter(|entry| *entry == "/usr/bin").count(),
        1
    );
}
