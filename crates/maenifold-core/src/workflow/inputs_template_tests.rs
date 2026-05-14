use super::handler_session::continue_workflow_session;
use super::inputs_template::expand_inputs_template;
use super::start_workflow_session;
use crate::config::Config;
use crate::markdown::read_markdown;
use crate::sessions::workflow_session_uri;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::HashMap;
use tempfile::TempDir;

fn inputs(values: Vec<(&str, Value)>) -> HashMap<String, Value> {
    values
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn workflow_context(
    workflow_id: &str,
    step_description: &str,
) -> (TempDir, ToolContext, ConfigResetGuard) {
    workflow_context_steps(workflow_id, vec![step_description])
}

fn workflow_context_steps(
    workflow_id: &str,
    step_descriptions: Vec<&str>,
) -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    std::fs::create_dir_all(config.assets_path.join("workflows")).expect("workflow dir");
    let steps = step_descriptions
        .iter()
        .enumerate()
        .map(|(index, description)| {
            json!({
                "name": format!("Step {}", index + 1),
                "description": description,
            })
        })
        .collect::<Vec<_>>();
    let asset = json!({ "id": workflow_id, "steps": steps });
    std::fs::write(
        config
            .assets_path
            .join("workflows")
            .join(format!("{workflow_id}.json")),
        serde_json::to_string_pretty(&asset).expect("workflow asset json"),
    )
    .expect("write workflow asset");
    (
        tmp,
        ToolContext::new(config, OutputFormat::Markdown),
        ConfigResetGuard,
    )
}

#[test]
fn expand_inputs_template_substitutes_string() {
    let result = expand_inputs_template(
        "Process {{inputs.target}}",
        &inputs(vec![("target", json!("hello"))]),
    )
    .expect("substitution succeeds");
    assert_eq!(result, "Process hello");
}

#[test]
fn expand_inputs_template_supports_input_singular_alias() {
    let result = expand_inputs_template(
        "Process {{input.target}}",
        &inputs(vec![("target", json!("hello"))]),
    )
    .expect("substitution succeeds");
    assert_eq!(result, "Process hello");
}

#[test]
fn expand_inputs_template_substitutes_number() {
    let result = expand_inputs_template("{{inputs.count}}", &inputs(vec![("count", json!(3.14))]))
        .expect("substitution succeeds");
    assert_eq!(result, "3.14");
}

#[test]
fn expand_inputs_template_substitutes_boolean() {
    let result = expand_inputs_template("{{inputs.flag}}", &inputs(vec![("flag", json!(true))]))
        .expect("substitution succeeds");
    assert_eq!(result, "true");
}

#[test]
fn expand_inputs_template_errors_on_unknown_key() {
    let error = expand_inputs_template(
        "{{inputs.unknown}}",
        &inputs(vec![("target", json!("hello"))]),
    )
    .expect_err("unknown key should fail");
    assert!(error.to_string().contains("unknown"));
}

#[test]
fn expand_inputs_template_handles_multiple_substitutions() {
    let result = expand_inputs_template(
        "{{inputs.foo}} and {{inputs.bar}}",
        &inputs(vec![("foo", json!("one")), ("bar", json!("two"))]),
    )
    .expect("substitution succeeds");
    assert_eq!(result, "one and two");
}

#[test]
fn expand_inputs_template_preserves_text_with_no_substitutions() {
    let text = "No workflow input references here";
    let result = expand_inputs_template(text, &HashMap::new()).expect("no-op succeeds");
    assert_eq!(result, text);
}

#[test]
#[serial]
fn start_workflow_session_renders_first_step_inputs() {
    let (_tmp, context, _guard) = workflow_context("input-render", "Process [[{{inputs.target}}]]");
    let response = start_workflow_session(
        &context,
        &["input-render".to_string()],
        Some(inputs(vec![("target", json!("my-fixture-target"))])),
    )
    .expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("start response json");
    let session_id = value["sessionId"].as_str().expect("session id");
    let (_, content, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read session");

    assert!(content.contains("Process [[my-fixture-target]]"));
    assert!(!content.contains("Process [[{{inputs.target}}]]"));
}

#[test]
#[serial]
fn start_workflow_session_errors_on_unknown_input_reference() {
    let (_tmp, context, _guard) = workflow_context("input-missing", "{{inputs.unknown}}");
    let error = start_workflow_session(
        &context,
        &["input-missing".to_string()],
        Some(inputs(vec![("target", json!("x"))])),
    )
    .expect_err("unknown input reference should fail");

    assert!(error.to_string().contains("unknown"));
}

#[test]
#[serial]
fn continue_workflow_session_renders_next_step_inputs_from_frontmatter() {
    let (_tmp, context, _guard) = workflow_context_steps(
        "input-next-step",
        vec!["First [[step]]", "Continue [[{{inputs.target}}]]"],
    );
    let response = start_workflow_session(
        &context,
        &["input-next-step".to_string()],
        Some(inputs(vec![("target", json!("persisted-target"))])),
    )
    .expect("start workflow");
    let value: Value = serde_json::from_str(&response).expect("start response json");
    let session_id = value["sessionId"].as_str().expect("session id");

    let step_response = continue_workflow_session(
        &context,
        session_id,
        "Finished first [[step]]",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("continue workflow");
    let (_, content, _) =
        read_markdown(&workflow_session_uri(session_id), &context.config).expect("read session");

    assert!(step_response.contains("Continue [[persisted-target]]"));
    assert!(content.contains("Continue [[persisted-target]]"));
    assert!(!content.contains("Continue [[{{inputs.target}}]]"));
}
