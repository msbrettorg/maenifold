// T-DATE-001: RTM FR-005, FR-006 — Ported timestamp regression tests from the C# oracle.

use chrono::{Datelike, TimeZone, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, read_markdown};
use maenifold_core::{AssetManager, Config, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use regex::Regex;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestHarness {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    config: Config,
}

impl TestHarness {
    fn new() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&config).expect("assets");
        Self {
            _guard: guard,
            _tmp: tmp,
            config,
        }
    }

    fn invoke_markdown(&self, tool: &str, payload: Value) -> String {
        ToolRegistry::global()
            .try_invoke(tool, &payload, OutputFormat::Markdown)
            .expect("tool invocation")
    }

    fn read(&self, uri: &str) -> (Frontmatter, String) {
        let (frontmatter, content, _) = read_markdown(uri, &self.config).expect("read markdown");
        (frontmatter.expect("frontmatter"), content)
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn extract_session_id(response_message: &str) -> String {
    response_message
        .lines()
        .find_map(|line| {
            if line.contains("Created session:") || line.contains("to session:") {
                line.split(':')
                    .next_back()
                    .map(|value| value.trim().to_string())
            } else {
                None
            }
        })
        .expect("session id")
}

fn session_uri(session_id: &str) -> String {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|value| value.parse::<i64>().ok())
        .expect("timestamp millis");
    let timestamp = Utc
        .timestamp_millis_opt(millis)
        .single()
        .expect("timestamp");
    format!(
        "memory://thinking/sequential/{:04}/{:02}/{:02}/{session_id}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day()
    )
}

fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    match frontmatter.get(key) {
        Some(YamlValue::String(value)) => value.clone(),
        Some(YamlValue::Number(value)) => value.to_string(),
        Some(YamlValue::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

#[test]
fn sequential_thought_timestamp_includes_utc_suffix() {
    let harness = TestHarness::new();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Test thought with [[utc-timestamp-verification]]",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );

    assert!(
        result.contains("Created session:"),
        "session should be created successfully"
    );

    let session_id = extract_session_id(&result);
    let (_, content) = harness.read(&session_uri(&session_id));

    // T-DATE-001: RTM FR-005 — Human-readable sequential thought timestamps must include UTC.
    let utc_timestamp_pattern =
        Regex::new(r"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC\*").expect("regex");
    assert!(
        utc_timestamp_pattern.is_match(&content),
        "content should contain timestamp with UTC suffix. Content:\n{content}"
    );
}

#[test]
fn sequential_thought_timestamp_does_not_contain_bare_timestamp_without_utc() {
    let harness = TestHarness::new();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Test thought with [[bare-timestamp-regression]]",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 2
        }),
    );

    assert!(
        result.contains("Created session:"),
        "session should be created successfully"
    );

    let session_id = extract_session_id(&result);
    let (_, content) = harness.read(&session_uri(&session_id));

    // T-DATE-001: RTM FR-005 — No italicized timestamp may omit the UTC suffix.
    let bare_timestamp_pattern =
        Regex::new(r"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\*").expect("regex");
    assert!(
        !bare_timestamp_pattern.is_match(&content),
        "content should not contain timestamp without UTC suffix. Content:\n{content}"
    );
}

#[test]
fn completed_session_has_iso8601_completed_field_in_frontmatter() {
    let harness = TestHarness::new();

    let result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Thought with [[iso8601-completion-test]]",
            "nextThoughtNeeded": false,
            "thoughtNumber": 0,
            "totalThoughts": 1,
            "conclusion": "Final synthesis with [[conclusion-verification]]"
        }),
    );

    assert!(
        result.contains("Created session:"),
        "session should be created successfully"
    );
    assert!(
        result.contains("Thinking complete"),
        "session should complete"
    );

    let session_id = extract_session_id(&result);
    let (frontmatter, _) = harness.read(&session_uri(&session_id));

    // T-DATE-001: RTM FR-006 — Completed sessions must persist ISO 8601 completion timestamps.
    assert_eq!(frontmatter_string(&frontmatter, "status"), "completed");

    let completed_value = frontmatter_string(&frontmatter, "completed");
    assert!(
        !completed_value.is_empty(),
        "completed value should not be null or empty"
    );

    let iso8601_pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}").expect("regex");
    assert!(
        iso8601_pattern.is_match(&completed_value),
        "completed timestamp should be ISO 8601 format (contains 'T' separator). Got: {completed_value}"
    );
}

#[test]
fn cancelled_session_has_iso8601_cancelled_field_in_frontmatter() {
    let harness = TestHarness::new();

    let create_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "response": "Initial thought with [[cancel-iso8601-test]]",
            "nextThoughtNeeded": true,
            "thoughtNumber": 0,
            "totalThoughts": 3
        }),
    );

    assert!(
        create_result.contains("Created session:"),
        "session should be created successfully"
    );

    let session_id = extract_session_id(&create_result);

    let cancel_result = harness.invoke_markdown(
        "SequentialThinking",
        json!({
            "sessionId": session_id,
            "cancel": true
        }),
    );

    assert!(
        cancel_result.contains("Thinking cancelled"),
        "session should be cancelled"
    );

    let (frontmatter, _) = harness.read(&session_uri(&session_id));

    // T-DATE-001: RTM FR-006 — Cancelled sessions must persist ISO 8601 cancellation timestamps.
    assert_eq!(frontmatter_string(&frontmatter, "status"), "cancelled");

    let cancelled_value = frontmatter_string(&frontmatter, "cancelled");
    assert!(
        !cancelled_value.is_empty(),
        "cancelled value should not be null or empty"
    );

    let iso8601_pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}").expect("regex");
    assert!(
        iso8601_pattern.is_match(&cancelled_value),
        "cancelled timestamp should be ISO 8601 format (contains 'T' separator). Got: {cancelled_value}"
    );
}
