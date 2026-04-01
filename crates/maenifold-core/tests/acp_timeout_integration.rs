/// ACP timeout integration tests — verifies prompt completion within generous
/// deadlines and correct timeout / cleanup behavior with near-zero deadlines.
///
/// Both tests are `#[ignore]` and require a local OpenCode ACP binary
/// configured in `~/maenifold/agents.json`.
use maenifold_core::acp_native::{AgentError, SessionOpts, spawn_agent};
use serde::Deserialize;
use serial_test::serial;
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

fn agents_config_path() -> PathBuf {
    PathBuf::from(env::var_os("HOME").expect("HOME must be set to locate ~/maenifold/agents.json"))
        .join("maenifold")
        .join("agents.json")
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

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate manifest should live under the repository root")
        .to_path_buf()
}

/// A prompt with a generous timeout should complete successfully and return
/// non-empty text.
#[tokio::test]
#[ignore = "requires a local OpenCode ACP binary configured in ~/maenifold/agents.json"]
#[serial]
async fn prompt_within_timeout() {
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
            timeout_seconds: Some(120),
            mcp_servers: vec![],
        })
        .await
        .expect("OpenCode session should start with generous timeout");

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
        .expect("prompt should complete within the generous timeout");

    assert!(
        !response.text.trim().is_empty(),
        "prompt response should contain non-empty text"
    );

    handle.close_session(&session.session_id).await;
    drop(handle);
}

/// A prompt with a zero-second timeout should fire immediately, return an
/// `AgentError::Timeout`, and clean up the session so that subsequent
/// prompts on the same session id fail.
#[tokio::test]
#[ignore = "requires a local OpenCode ACP binary configured in ~/maenifold/agents.json"]
#[serial]
async fn prompt_timeout_fires() {
    let cwd = repo_root();
    let AgentConfig { command, env } = load_opencode_agent();
    let handle = spawn_agent(command, cwd.clone(), env);

    // Use timeout_seconds: 0 — the minimum expressible value — so the
    // `tokio::time::timeout(Duration::from_secs(0), ..)` fires before the
    // prompt round-trip can complete.
    let session = handle
        .new_session(SessionOpts {
            cwd: cwd.clone(),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: Some(1),
            timeout_seconds: Some(0),
            mcp_servers: vec![],
        })
        .await
        .expect("OpenCode session should start even with zero timeout");

    let error = match handle
        .prompt(
            &session.session_id,
            "Reply with the single word READY and nothing else.",
        )
        .await
    {
        Ok(_) => panic!("prompt should time out with a zero-second timeout"),
        Err(error) => error,
    };

    assert!(
        matches!(&error, AgentError::Timeout { .. }),
        "expected AgentError::Timeout, got: {error}"
    );

    // --- cleanup verification ---
    // After a timeout, `cleanup_timed_out_prompt` runs: it removes the
    // session accumulator, sends a cancel notification, and closes the remote
    // session.  The agent handle itself stays alive (only the session is
    // torn down).
    assert!(
        handle.is_alive(),
        "agent handle should remain alive after a prompt timeout"
    );

    // A second prompt on the same (now-closed) session should fail because
    // cleanup already closed it on the remote side.
    let retry = handle.prompt(&session.session_id, "ping").await;
    assert!(
        retry.is_err(),
        "prompting a cleaned-up session should fail, confirming cleanup ran"
    );

    let replacement = handle
        .new_session(SessionOpts {
            cwd: cwd.clone(),
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: Some(1),
            timeout_seconds: Some(120),
            mcp_servers: vec![],
        })
        .await
        .expect("agent handle should still be able to create a fresh session after timeout cleanup");
    let replacement_response = handle
        .prompt(
            &replacement.session_id,
            "Reply with the single word READY and nothing else.",
        )
        .await
        .expect("agent handle should still be usable after timeout cleanup");
    assert!(
        !replacement_response.text.trim().is_empty(),
        "replacement session should still produce a non-empty response after timeout cleanup"
    );
    handle.close_session(&replacement.session_id).await;

    drop(handle);
}
