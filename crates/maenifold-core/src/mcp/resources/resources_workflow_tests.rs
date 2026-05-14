use super::handle_read_resource;
use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::workflow;
use rmcp::model::ResourceContents;
use serde_json::{Value, json};
use serial_test::serial;
use tempfile::TempDir;

fn workflow_context() -> (TempDir, ToolContext, ConfigResetGuard) {
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
    (
        temp,
        ToolContext::new(config, OutputFormat::Json),
        ConfigResetGuard,
    )
}

fn resource_json(uri: &str) -> Value {
    let result = handle_read_resource(uri).expect("read resource");
    let first = result.contents.first().expect("resource has content");
    let ResourceContents::TextResourceContents { text, .. } = first else {
        panic!("workflow resources must return text content");
    };
    serde_json::from_str(text).expect("resource content is json")
}

#[test]
#[serial]
fn workflow_session_resource_includes_step_timings() {
    let (_temp, context, _guard) = workflow_context();
    let response = workflow(&context, &json!({ "workflowId": "test" })).expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("workflow response json");
    let session_id = value
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .expect("session id");

    let payload = resource_json(&format!("workflow://session/{session_id}"));

    assert!(
        payload
            .get("step_timings")
            .and_then(|value| value.get("1"))
            .and_then(|value| value.get("started_at"))
            .and_then(Value::as_str)
            .is_some(),
        "workflow session resource should expose step_timings.1.started_at"
    );
}
