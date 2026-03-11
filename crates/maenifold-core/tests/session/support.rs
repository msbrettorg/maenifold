// T-12.7: RTM NFR-1
#![allow(dead_code, unused_imports)]

pub(crate) use chrono::{Datelike, TimeZone, Utc};
pub(crate) use maenifold_core::json::OutputFormat;
pub(crate) use maenifold_core::markdown::{Frontmatter, read_markdown, write_markdown};
pub(crate) use maenifold_core::sessions::{
    validate_assumption_append_input, validate_assumption_read_input, validate_assumption_status,
    validate_assumption_update_input,
};
pub(crate) use maenifold_core::{AssetManager, Config, ToolRegistry};
pub(crate) use once_cell::sync::Lazy;
pub(crate) use parking_lot::{Mutex, MutexGuard};
pub(crate) use serde_json::Value;
pub(crate) use serde_yaml::Value as YamlValue;
pub(crate) use tempfile::TempDir;

pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(crate) struct TestHarness {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    pub(crate) config: Config,
}

impl TestHarness {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn invoke_markdown(&self, tool: &str, payload: Value) -> String {
        ToolRegistry::global()
            .try_invoke(tool, &payload, OutputFormat::Markdown)
            .expect("tool invocation")
    }

    pub(crate) fn read(&self, uri: &str) -> (Frontmatter, String) {
        let (frontmatter, content, _) = read_markdown(uri, &self.config).expect("read markdown");
        (frontmatter.expect("frontmatter"), content)
    }

    pub(crate) fn write(&self, uri: &str, frontmatter: &Frontmatter, content: &str) {
        write_markdown(uri, Some(frontmatter), content, &self.config).expect("write markdown");
    }

    pub(crate) fn create_workflow_session(&self) -> String {
        let output = self.invoke_markdown(
            "Workflow",
            serde_json::json!({ "workflowId": "deductive-reasoning" }),
        );
        extract_session_id(&output)
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn extract_session_id(response_message: &str) -> String {
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

pub(crate) fn extract_memory_uri(response_message: &str) -> String {
    let start = response_message
        .find("memory://")
        .expect("memory uri start");
    let remainder = &response_message[start..];
    let end = remainder
        .find(char::is_whitespace)
        .unwrap_or(remainder.len());
    remainder[..end].to_string()
}

pub(crate) fn session_uri(kind: &str, session_id: &str) -> String {
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
        "memory://thinking/{kind}/{:04}/{:02}/{:02}/{session_id}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day()
    )
}

pub(crate) fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    match frontmatter.get(key) {
        Some(YamlValue::String(value)) => value.clone(),
        Some(YamlValue::Number(value)) => value.to_string(),
        Some(YamlValue::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn frontmatter_sequence(frontmatter: &Frontmatter, key: &str) -> Vec<String> {
    match frontmatter.get(key) {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}
