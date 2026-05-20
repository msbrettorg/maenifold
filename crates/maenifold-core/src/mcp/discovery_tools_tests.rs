use super::*;
use crate::acp_config::IntentResolverSettings;
use crate::config::Config;
use crate::mcp::gateway_catalog::GatewayCatalog;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::StdioPool;
use crate::mcp::tool_routing::ToolRoutingRegistry;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static CONFIG_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[tokio::test]
async fn discover_mcp_tools_returns_native_maenifold_entry_with_prefixed_tools() {
    let _config_lock = lock_config();
    let root = write_test_config("native-entry", json!({}));
    let _guard = ConfigOverrideGuard::new(&root);
    let gateway = gateway_state().await;

    let results = discover_mcp_tools(Some(&gateway)).await;
    let entry = maenifold_entry(&results);

    assert_eq!(entry.server_type, "native");
    assert!(
        matches!(entry.status, mcp_discovery::McpDiscoveryStatus::Ok),
        "expected synthetic native entry to be ok"
    );
    assert!(entry.tools.iter().all(|tool| tool.name.starts_with("maenifold.")));
    assert_tool_names(
        entry,
        &[
            "maenifold.GatewayExecute",
            "maenifold.GatewayIndex",
            "maenifold.GatewayBatch",
        ],
    );
}

#[tokio::test]
async fn discover_mcp_tools_surfaces_synthetic_entry_once_with_self_loopback_configured() {
    let _config_lock = lock_config();
    let root = write_test_config(
        "self-loopback",
        json!({
            "maenifold": {
                "command": "/usr/bin/false",
                "args": [],
                "gateway": false
            }
        }),
    );
    let _guard = ConfigOverrideGuard::new(&root);
    let gateway = gateway_state().await;

    let results = discover_mcp_tools(Some(&gateway)).await;

    assert_eq!(
        results.iter().filter(|entry| entry.server == "maenifold").count(),
        1,
        "maenifold should appear exactly once"
    );
    let entry = maenifold_entry(&results);
    assert_eq!(entry.server_type, "native");
    assert!(
        matches!(entry.status, mcp_discovery::McpDiscoveryStatus::Ok),
        "expected synthetic native entry to win over self-loopback config"
    );
    assert_tool_names(entry, &["maenifold.GatewayExecute"]);
}

#[tokio::test]
async fn discover_mcp_tools_keeps_synthetic_entry_without_self_loopback_config() {
    let _config_lock = lock_config();
    let root = write_test_config("no-self-loopback", json!({}));
    let _guard = ConfigOverrideGuard::new(&root);
    let gateway = gateway_state().await;

    let results = discover_mcp_tools(Some(&gateway)).await;

    assert_eq!(
        results.iter().filter(|entry| entry.server == "maenifold").count(),
        1,
        "synthetic maenifold entry should exist even without config self-loopback"
    );
}

#[tokio::test]
async fn discover_mcp_prompts_returns_empty_when_no_pool() {
    assert!(discover_mcp_prompts(None).await.is_empty());
}

#[tokio::test]
async fn get_mcp_prompt_unknown_server_returns_error() {
    let error = get_mcp_prompt(
        None,
        GetMcpPromptParams {
            server: "missing".to_string(),
            name: "summarize".to_string(),
            arguments: None,
        },
    )
    .await
    .expect_err("missing gateway should fail");
    assert!(error.message.contains("unknown server 'missing'"));
}

fn maenifold_entry(
    results: &[mcp_discovery::McpServerDiscoveryResult],
) -> &mcp_discovery::McpServerDiscoveryResult {
    results
        .iter()
        .find(|entry| entry.server == "maenifold")
        .expect("maenifold entry")
}

fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

fn assert_tool_names(entry: &mcp_discovery::McpServerDiscoveryResult, expected: &[&str]) {
    let tool_names = entry
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>();
    for name in expected {
        assert!(tool_names.contains(name), "missing tool {name} in {tool_names:?}");
    }
}

async fn gateway_state() -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("empty pool"),
    );
    let routing = ToolRoutingRegistry::build(&[], pool.as_ref()).expect("routing");
    let catalog = GatewayCatalog::build(&[], &[], &routing).expect("catalog");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(
        pool,
        resources,
        catalog,
        Vec::new(),
        IntentResolverSettings::default(),
        PathBuf::from("."),
    )
}

fn write_test_config(test_name: &str, mcp_servers: serde_json::Value) -> PathBuf {
    let root = unique_test_root(test_name);
    fs::create_dir_all(&root).expect("create test root");
    fs::write(
        root.join("maenifold.json"),
        serde_json::to_string_pretty(&json!({
            "mcpServers": mcp_servers,
            "agents": {}
        }))
        .expect("serialize config"),
    )
    .expect("write config");
    root
}

fn unique_test_root(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    std::env::current_dir()
        .expect("current dir")
        .join("target")
        .join("test-scratch")
        .join(format!(
            "discovery-tools-{test_name}-{}-{nonce}",
            std::process::id()
        ))
}

struct ConfigOverrideGuard;

impl ConfigOverrideGuard {
    fn new(root: &Path) -> Self {
        Config::override_root(root);
        Self
    }
}

impl Drop for ConfigOverrideGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}
