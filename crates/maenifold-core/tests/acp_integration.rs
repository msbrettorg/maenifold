use maenifold_core::acp_native::{AgentError, AgentPool, SessionOpts, spawn_agent};
use serde::Deserialize;
use serial_test::serial;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

const REAL_AGENT_NAMES: &[&str] = &["OpenCode", "Copilot", "Codex"];

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: HashMap<String, AgentConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentConfig {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct SelectedAgent {
    name: String,
    command: String,
    env: HashMap<String, String>,
}

fn maenifold_root() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME must be set")).join("maenifold")
}

fn load_selected_agent() -> Option<SelectedAgent> {
    let root = maenifold_root();
    let config_path = root.join("agents.json");
    let contents = fs::read_to_string(&config_path).ok()?;
    let config: AgentsFile = serde_json::from_str(&contents).ok()?;

    REAL_AGENT_NAMES.iter().find_map(|name| {
        let entry = config.agents.get(*name)?;
        binary_path(&entry.command)
            .filter(|path| path.exists())
            .map(|_| SelectedAgent {
                name: (*name).to_string(),
                command: entry.command.clone(),
                env: entry.env.clone(),
            })
    })
}

fn binary_path(command: &str) -> Option<&Path> {
    command
        .split_whitespace()
        .next()
        .filter(|part| !part.is_empty())
        .map(Path::new)
}

fn session_opts(timeout_seconds: Option<u64>) -> SessionOpts {
    SessionOpts {
        cwd: maenifold_root(),
        model: None,
        mode: None,
        thought_level: None,
        config_options: None,
        allowed_tools: None,
        max_turns: None,
        timeout_seconds,
        mcp_servers: vec![],
    }
}

fn require_agent() -> Option<SelectedAgent> {
    let agent = load_selected_agent();
    if agent.is_none() {
        eprintln!(
            "skipping ignored ACP integration test: need one of {:?} configured in ~/maenifold/agents.json with an existing binary",
            REAL_AGENT_NAMES
        );
    }
    agent
}

#[tokio::test]
#[ignore = "requires real ACP agent from ~/maenifold/agents.json"]
#[serial]
async fn spawn_agent_new_session_close_session_lifecycle() {
    let Some(agent) = require_agent() else {
        return;
    };
    let handle = spawn_agent(agent.command, maenifold_root(), agent.env);

    let session = handle
        .new_session(session_opts(Some(30)))
        .await
        .unwrap_or_else(|error| panic!("failed to create session with {}: {error}", agent.name));

    assert!(
        !session.session_id.trim().is_empty(),
        "new_session should return a non-empty session id"
    );
    assert!(handle.is_alive(), "agent handle should stay alive after session creation");

    handle.close_session(&session.session_id).await;
}

#[tokio::test]
#[ignore = "requires real ACP agent from ~/maenifold/agents.json"]
#[serial]
async fn spawn_agent_prompt_say_hello_returns_non_empty_response() {
    let Some(agent) = require_agent() else {
        return;
    };
    let handle = spawn_agent(agent.command, maenifold_root(), agent.env);
    let session = handle
        .new_session(session_opts(Some(60)))
        .await
        .unwrap_or_else(|error| panic!("failed to create session with {}: {error}", agent.name));

    let response = handle
        .prompt(&session.session_id, "say hello")
        .await
        .unwrap_or_else(|error| panic!("failed to prompt {}: {error}", agent.name));

    let rendered = if response.text.trim().is_empty() {
        response.thought.unwrap_or_default()
    } else {
        response.text
    };
    assert!(
        !rendered.trim().is_empty(),
        "prompt should produce non-empty text or thought content"
    );

    handle.close_session(&session.session_id).await;
}

#[tokio::test]
#[ignore = "requires real ACP agent from ~/maenifold/agents.json"]
#[serial]
async fn agent_pool_get_or_create_reuses_handle() {
    let Some(agent) = require_agent() else {
        return;
    };
    let pool = AgentPool::new();
    let cwd = maenifold_root();

    let first = pool
        .get_or_create(&agent.name, &agent.command, &cwd, agent.env.clone())
        .await;
    let second = pool
        .get_or_create(&agent.name, &agent.command, &cwd, agent.env)
        .await;

    assert!(
        Arc::ptr_eq(&first, &second),
        "get_or_create should reuse the existing live agent handle"
    );

    pool.close_all().await;
}

#[tokio::test]
#[ignore = "requires real ACP agent from ~/maenifold/agents.json"]
#[serial]
async fn prompt_timeout_fires_correctly() {
    let Some(agent) = require_agent() else {
        return;
    };
    let handle = spawn_agent(agent.command, maenifold_root(), agent.env);
    let session = handle
        .new_session(session_opts(Some(0)))
        .await
        .unwrap_or_else(|error| panic!("failed to create session with {}: {error}", agent.name));

    let error = match handle.prompt(&session.session_id, "say hello").await {
        Ok(_) => panic!("prompt should time out with a zero-second timeout"),
        Err(error) => error,
    };

    assert!(
        matches!(&error, AgentError::Timeout { .. }),
        "expected AgentError::Timeout, got: {error}"
    );
}
