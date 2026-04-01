use maenifold_core::acp_native::{SessionOpts, spawn_agent};
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: HashMap<String, AgentConfig>,
}

#[derive(Debug, Deserialize)]
struct AgentConfig {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

#[tokio::test]
#[ignore = "requires a local OpenCode ACP binary configured in ~/maenifold/agents.json"]
async fn opencode_acp_smoke_test_end_to_end() {
    let cwd = repo_root();
    let AgentConfig { command, env } = load_opencode_agent();
    let handle = spawn_agent(command, cwd.clone(), env);

    let session = handle
        .new_session(SessionOpts {
            cwd: cwd.clone(),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: Some(1),
            timeout_seconds: Some(60),
            mcp_servers: vec![],
        })
        .await
        .expect("OpenCode session should start");

    assert!(
        !session.session_id.trim().is_empty(),
        "new_session should return a non-empty session id"
    );

    let response = handle
        .prompt(
            &session.session_id,
            "Reply with the single word READY and nothing else.",
        )
        .await
        .expect("OpenCode prompt should succeed");

    assert!(
        !response.text.trim().is_empty(),
        "prompt should return a non-empty response"
    );

    handle.close_session(&session.session_id).await;
    drop(handle);
}

fn load_opencode_agent() -> AgentConfig {
    let config_path = agents_config_path();
    let contents = fs::read_to_string(&config_path).unwrap_or_else(|error| {
        panic!(
            "failed to read OpenCode ACP config '{}': {error}",
            config_path.display()
        )
    });
    let mut config: AgentsFile = serde_json::from_str(&contents).unwrap_or_else(|error| {
        panic!(
            "failed to parse OpenCode ACP config '{}': {error}",
            config_path.display()
        )
    });

    config.agents.remove("OpenCode").unwrap_or_else(|| {
        panic!(
            "OpenCode agent is not configured in '{}'",
            config_path.display()
        )
    })
}

fn agents_config_path() -> PathBuf {
    PathBuf::from(
        env::var_os("HOME").expect("HOME must be set to locate ~/maenifold/agents.json"),
    )
    .join("maenifold")
    .join("agents.json")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate manifest should live under the repository root")
        .to_path_buf()
}
