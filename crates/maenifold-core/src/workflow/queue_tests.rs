use super::queue::start_workflow_session;
use crate::{
    config::Config, test_util::ConfigResetGuard, tools::ToolContext, util::json::OutputFormat,
};
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

#[test]
#[serial]
fn workflow_start_returns_structured_session_id_json() {
    let temp = TempDir::new().expect("temp dir");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    std::fs::write(
        config.assets_path.join("workflows/test.json"),
        r#"{"steps":[{"name":"Step 1"}]}"#,
    )
    .expect("write workflow");
    let _guard = ConfigResetGuard;
    let response = start_workflow_session(
        &ToolContext::new(config, OutputFormat::Markdown),
        &["test".to_string()],
        None,
    )
    .expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("workflow start should return JSON");
    assert!(value.get("sessionId").and_then(Value::as_str).is_some());
}
