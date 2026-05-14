use super::super::start_workflow_session;
use crate::{
    config::Config,
    markdown::read_markdown,
    sessions::workflow_session_uri,
    test_util::ConfigResetGuard,
    tools::ToolContext,
    util::json::OutputFormat,
};
use serde_json::Value;
use serde_yml::Value as YamlValue;
use serial_test::serial;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
#[serial]
fn workflow_start_persists_inputs_to_session_frontmatter() {
    let temp = TempDir::new().expect("temp dir");
    Config::override_root(temp.path());
    Config::set_database_path(temp.path().join("memory.db"));
    let _guard = ConfigResetGuard;
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::create_dir_all(config.assets_path.join("workflows"))
        .expect("create workflows dir");
    std::fs::write(
        config.assets_path.join("workflows/inputs-test.json"),
        r#"{
            "id": "inputs-test",
            "steps": [{"name": "Step 1"}],
            "inputs": {"target": {"type": "string"}}
        }"#,
    )
    .expect("write workflow");

    let mut inputs = HashMap::new();
    inputs.insert(
        "target".to_string(),
        Value::String("my-fixture-target".to_string()),
    );
    let context = ToolContext::new(config, OutputFormat::Markdown);

    let response = start_workflow_session(&context, &["inputs-test".to_string()], Some(inputs))
        .expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("workflow start returns JSON");
    let session_id = value
        .get("sessionId")
        .and_then(Value::as_str)
        .expect("session id in response");
    let (frontmatter, _, _) = read_markdown(&workflow_session_uri(session_id), &context.config)
        .expect("read session markdown");
    let frontmatter = frontmatter.expect("session frontmatter");

    let inputs = frontmatter.get("inputs").expect("inputs frontmatter key");
    let mapping = match inputs {
        YamlValue::Mapping(mapping) => mapping,
        other => panic!("inputs should be a YAML mapping, got {other:?}"),
    };
    assert_eq!(
        mapping.get(YamlValue::String("target".to_string())),
        Some(&YamlValue::String("my-fixture-target".to_string()))
    );
}
