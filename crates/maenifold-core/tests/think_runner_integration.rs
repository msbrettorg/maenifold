//! Integration tests for [`maenifold_core::workflow::run_think`] against a real OpenCode ACP agent.
//!
//! All tests are `#[ignore]` — run explicitly with:
//!   cargo test -p maenifold-core --test think_runner_integration -- --ignored --nocapture
//!
//! Prerequisites:
//!   * `~/maenifold/agents.json` must contain an `"OpenCode"` entry with a valid ACP command.
//!   * `~/maenifold` must be a valid maenifold root (memory.db, assets/, etc.).

use maenifold_core::{
    Config,
    tools::ToolContext,
    util::json::OutputFormat,
    workflow::{ThinkOptions, ThinkResult, run_think},
};
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AgentsFile {
    agents: HashMap<String, AgentConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AgentConfig {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

fn maenifold_root() -> PathBuf {
    PathBuf::from(env::var_os("HOME").expect("HOME must be set")).join("maenifold")
}

fn load_opencode_config() -> AgentConfig {
    let config_path = maenifold_root().join("agents.json");
    let contents = fs::read_to_string(&config_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", config_path.display()));
    let mut file: AgentsFile = serde_json::from_str(&contents)
        .unwrap_or_else(|e| panic!("cannot parse agents.json: {e}"));
    file.agents
        .remove("OpenCode")
        .expect("OpenCode agent not found in ~/maenifold/agents.json")
}

/// Build a [`ToolContext`] that points at the real `~/maenifold` root.
fn live_context() -> ToolContext {
    let config = Config::current();
    ToolContext::new(config, OutputFormat::Markdown)
}

/// Locate the session file on disk given a session ID.
///
/// Session IDs look like `session-{timestamp_ms}-{entropy}`. The timestamp
/// determines the `YYYY/MM/DD` subdirectory under `~/maenifold/memory/thinking/sequential/`.
fn find_session_file(session_id: &str) -> Option<PathBuf> {
    let sequential_dir = maenifold_root()
        .join("memory")
        .join("thinking")
        .join("sequential");

    // Extract the timestamp millis from the session ID (`session-{millis}-{entropy}`).
    let millis: i64 = session_id
        .splitn(3, '-')
        .nth(1)
        .and_then(|s| s.parse().ok())?;

    let dt = chrono::DateTime::from_timestamp_millis(millis)?;
    let year = chrono::Datelike::year(&dt);
    let month = chrono::Datelike::month(&dt);
    let day = chrono::Datelike::day(&dt);

    let path = sequential_dir
        .join(format!("{year:04}"))
        .join(format!("{month:02}"))
        .join(format!("{day:02}"))
        .join(format!("{session_id}.md"));

    if path.exists() { Some(path) } else { None }
}

/// Read the session file and return a list of thought bodies (content under each
/// `## Thought N/M` heading).
fn extract_thoughts(session_path: &PathBuf) -> Vec<String> {
    let content = fs::read_to_string(session_path)
        .unwrap_or_else(|e| panic!("cannot read session file {}: {e}", session_path.display()));

    let mut thoughts: Vec<String> = Vec::new();
    let mut current: Option<String> = None;

    for line in content.lines() {
        if line.starts_with("## Thought ") {
            if let Some(body) = current.take() {
                thoughts.push(body);
            }
            current = Some(String::new());
        } else if let Some(ref mut body) = current {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(body) = current {
        thoughts.push(body);
    }

    // Trim whitespace from each thought body.
    thoughts
        .into_iter()
        .map(|t| t.trim().to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Smoke-test: `run_think` drives a 3-thought session to completion via OpenCode.
///
/// Verifies:
/// * A non-empty `session_id` is returned.
/// * At least 3 thoughts are completed.
/// * A conclusion is captured.
/// * The session file is written to disk at the expected path.
/// * Thoughts contain real content (not `[[unstructured-response]]` fallbacks).
#[tokio::test]
#[ignore = "requires a live OpenCode ACP agent and ~/maenifold setup"]
async fn run_think_produces_real_thoughts_with_opencode() {
    let _agent_cfg = load_opencode_config(); // confirms agent is configured

    let ctx = live_context();
    assert_eq!(
        ctx.config.maenifold_root,
        maenifold_root(),
        "run_think should use ~/maenifold as cwd via Config::current()"
    );
    let opts = ThinkOptions {
        total_thoughts: 3,
        parent_workflow_id: None,
    };

    let result: ThinkResult = run_think(
        &ctx,
        "OpenCode",
        "Analyze the concept of [[integration-testing]] in Rust \
         and explain how [[tokio]] async runtimes interact with test harnesses.",
        opts,
    )
    .await
    .expect("run_think should succeed with a live OpenCode agent");

    // ── Basic result assertions ──────────────────────────────────────────
    assert!(
        !result.session_id.is_empty(),
        "session_id must be non-empty; got: {:?}",
        result.session_id
    );
    assert!(
        result.thoughts_completed >= 3,
        "at least 3 thoughts must be completed; got {}",
        result.thoughts_completed
    );
    assert!(
        result.conclusion.is_some(),
        "a conclusion must be captured after the final thought"
    );

    // ── Session file on disk ─────────────────────────────────────────────
    let session_path = find_session_file(&result.session_id).unwrap_or_else(|| {
        panic!(
            "session file for '{}' not found under ~/maenifold/memory/thinking/sequential/",
            result.session_id
        )
    });
    let thoughts = extract_thoughts(&session_path);

    // Thought 0 is the seed; real agent thoughts start at index 1.
    // We expect >= 3 real agent thoughts (indices 1..=3+).
    let agent_thoughts: Vec<&String> = thoughts.iter().skip(1).collect();
    assert!(
        agent_thoughts.len() >= 3,
        "expected >= 3 agent thoughts in session file; found {} (total sections: {})",
        agent_thoughts.len(),
        thoughts.len()
    );

    // Verify thoughts contain real content, not the fallback tag.
    for (i, thought) in agent_thoughts.iter().enumerate() {
        assert!(
            !thought.is_empty(),
            "thought {} is empty in session file",
            i + 1
        );
        assert!(
            thought.contains("[["),
            "thought {} should contain at least one WikiLink: {}",
            i + 1,
            &thought[..thought.len().min(200)]
        );
        assert!(
            !thought.contains("[[step]]"),
            "thought {} fell back to [[step]] content: {}",
            i + 1,
            &thought[..thought.len().min(200)]
        );
        assert!(
            !thought.contains("[[unstructured-response]]"),
            "thought {} fell back to [[unstructured-response]]: {}",
            i + 1,
            &thought[..thought.len().min(200)]
        );
    }
}
