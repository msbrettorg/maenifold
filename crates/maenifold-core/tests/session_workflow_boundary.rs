use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;
use chrono::Datelike;
use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, ToolRegistry};
use serde_json::{Value, json};
use tempfile::TempDir;

fn isolated_root() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    (tmp, cfg)
}

#[test]
fn registry_exposes_session_and_workflow_tools() {
    let registry = ToolRegistry::global();
    for name in [
        "SequentialThinking",
        "sequentialthinking",
        "Workflow",
        "workflow",
        "AssumptionLedger",
        "assumptionledger",
    ] {
        assert!(
            registry.contains(name),
            "missing tool registration for {name}"
        );
    }
}

#[test]
fn config_exposes_expected_session_roots_under_memory() {
    let config = Config::current();
    assert!(config.thinking_path.ends_with("memory/thinking"));
    assert!(
        config
            .sequential_path
            .ends_with("memory/thinking/sequential")
    );

    let workflow_path = config.memory_path.join("thinking").join("workflow");
    assert!(workflow_path.ends_with("memory/thinking/workflow"));
}

#[test]
fn sequential_thinking_session_ids_should_persist_under_dated_sequential_paths() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, config) = isolated_root();
    let output = ToolRegistry::global()
        .try_invoke(
            "SequentialThinking",
            &json!({
                "response": "Exploring [[session-paths]] and [[workflow-state]].",
                "thoughtNumber": 0,
                "totalThoughts": 2,
                "nextThoughtNeeded": true
            }),
            OutputFormat::Json,
        )
        .expect("sequential thinking");
    let value: Value = serde_json::from_str(&output).expect("json");
    let session_id = value["data"]["sessionId"].as_str().expect("sessionId");
    let now = chrono::Utc::now();
    let expected = config
        .memory_path
        .join("thinking")
        .join("sequential")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{:02}", now.day()))
        .join(format!("{session_id}.md"));

    assert!(
        expected.exists(),
        "expected sequential session file at {}",
        expected.display()
    );
    Config::reset_overrides();
}

#[test]
fn workflow_session_ids_should_persist_under_dated_workflow_paths() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, config) = isolated_root();
    let output = ToolRegistry::global()
        .try_invoke(
            "Workflow",
            &json!({
                "workflowId": "deductive-reasoning"
            }),
            OutputFormat::Markdown,
        )
        .expect("workflow");
    let session_id = output
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("Created session: "))
        .expect("session line");
    let now = chrono::Utc::now();
    let expected = config
        .memory_path
        .join("thinking")
        .join("workflow")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .join(format!("{:02}", now.day()))
        .join(format!("{session_id}.md"));

    assert!(
        expected.exists(),
        "expected workflow session file at {}",
        expected.display()
    );
    Config::reset_overrides();
}

#[test]
fn assumption_ledger_append_persists_under_assumptions_year_month_path() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, config) = isolated_root();
    let text = ToolRegistry::global()
        .try_invoke(
            "AssumptionLedger",
            &json!({
                "action": "append",
                "assumption": "Track [[session-contract]] parity",
                "concepts": ["session-contract"],
                "confidence": "medium"
            }),
            OutputFormat::Markdown,
        )
        .expect("assumption append");

    let uri = text
        .lines()
        .find_map(|line| {
            line.split("memory://")
                .nth(1)
                .map(|rest| format!("memory://{}", rest))
        })
        .expect("memory uri in output");
    let now = chrono::Utc::now();
    assert!(uri.starts_with(&format!(
        "memory://assumptions/{:04}/{:02}/assumption-",
        now.year(),
        now.month()
    )));

    let relative = uri.trim_start_matches("memory://");
    let expected = config.memory_path.join(format!("{relative}.md"));
    assert!(
        expected.exists(),
        "expected assumption file at {}",
        expected.display()
    );
    Config::reset_overrides();
}

#[test]
fn sequential_thinking_without_wikilinks_should_fail_validation_contract() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, _config) = isolated_root();
    let output = ToolRegistry::global()
        .try_invoke(
            "SequentialThinking",
            &json!({
                "response": "This thought has no concepts at all.",
                "thoughtNumber": 0,
                "totalThoughts": 1,
                "nextThoughtNeeded": true
            }),
            OutputFormat::Json,
        )
        .expect("sequential thinking");
    let value: Value = serde_json::from_str(&output).expect("json");
    assert_eq!(value["success"], false);
    assert_eq!(value["error"]["code"], "WIKILINK_REQUIRED");
    Config::reset_overrides();
}
