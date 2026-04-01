// acp-pool-real-test — real-agent coverage for AgentPool reuse.
use maenifold_core::acpx_driver::{AgentPool, SessionOpts};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::time::{Duration, timeout};

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: HashMap<String, AgentEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentEntry {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under repo/crates/maenifold-core")
        .to_path_buf()
}

fn agents_config_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME must be set"))
        .join("maenifold")
        .join("agents.json")
}

fn load_agent(name: &str) -> AgentEntry {
    let config_path = agents_config_path();
    let contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|error| panic!("failed to read '{}': {error}", config_path.display()));
    let config: AgentsFile = serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("failed to parse '{}': {error}", config_path.display()));
    config
        .agents
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("agent '{name}' missing from '{}'", config_path.display()))
}

fn session_opts(cwd: &Path) -> SessionOpts {
    SessionOpts {
        cwd: cwd.to_path_buf(),
        model: None,
        mode: None,
        thought_level: None,
        config_options: None,
        allowed_tools: None,
        max_turns: None,
        timeout_seconds: Some(30),
        mcp_servers: vec![],
    }
}

#[tokio::test]
#[ignore = "requires a real OpenCode ACP binary configured in ~/maenifold/agents.json"]
async fn agent_pool_reuses_live_opencode_handle_across_sessions() {
    let cwd = repo_root();
    let entry = load_agent("OpenCode");
    let pool = AgentPool::new();

    let handle = timeout(
        Duration::from_secs(10),
        pool.get_or_create("OpenCode", &entry.command, &cwd, entry.env.clone()),
    )
    .await
    .expect("get_or_create should not hang");
    assert!(handle.is_alive(), "spawned OpenCode handle should report alive");

    let first = timeout(Duration::from_secs(30), handle.new_session(session_opts(&cwd)))
        .await
        .expect("first new_session should not hang")
        .expect("first OpenCode session should initialize");
    let second = timeout(Duration::from_secs(30), handle.new_session(session_opts(&cwd)))
        .await
        .expect("second new_session should not hang")
        .expect("second OpenCode session should initialize");

    assert_ne!(
        first.session_id, second.session_id,
        "each new_session call should create a distinct remote session"
    );
    assert!(
        handle.is_alive(),
        "handle should remain alive after creating multiple sessions"
    );

    let reused = timeout(
        Duration::from_secs(10),
        pool.get_or_create("OpenCode", &entry.command, &cwd, entry.env.clone()),
    )
    .await
    .expect("reused get_or_create should not hang");
    assert!(
        Arc::ptr_eq(&handle, &reused),
        "AgentPool should reuse the existing OpenCode handle instead of spawning another process"
    );

    handle.close_session(&first.session_id).await;
    handle.close_session(&second.session_id).await;
    pool.close_all().await;
}
