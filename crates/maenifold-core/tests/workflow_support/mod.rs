#![allow(dead_code)]

use chrono::{Datelike, TimeZone, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, read_markdown};
use maenifold_core::{AssetManager, Config, Result, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::fs;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestEnv {
    _tmp: TempDir,
    pub cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-12.0: RTM NFR-1 - share workflow integration helpers while splitting coverage by theme.
pub fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    TestEnv { _tmp: tmp, cfg }
}

// T-12.0: RTM NFR-1 - seed temporary workflow assets that mirror the C# fixture shape.
pub fn write_test_workflow(cfg: &Config, name: &str, step_count: usize, description_prefix: &str) {
    let steps = (1..=step_count)
        .map(|index| {
            json!({
                "id": format!("step-{index}"),
                "name": format!("Step {index}"),
                "description": format!("{description_prefix} {index} description for [[workflow]] coverage")
            })
        })
        .collect::<Vec<_>>();
    let workflow = json!({
        "id": name,
        "name": format!("Test Workflow {name}"),
        "description": format!("Test workflow for [[Workflow]] coverage: {name}"),
        "steps": steps
    });
    let path = cfg
        .assets_path
        .join("workflows")
        .join(format!("{name}.json"));
    fs::create_dir_all(path.parent().expect("workflow parent")).expect("workflow dirs");
    fs::write(
        path,
        serde_json::to_string_pretty(&workflow).expect("workflow json"),
    )
    .expect("write workflow");
}

pub fn workflow_tools_env() -> TestEnv {
    let env = isolated_env();
    write_test_workflow(&env.cfg, "wtt-alpha", 3, "Test step");
    write_test_workflow(&env.cfg, "wtt-beta", 2, "Test step");
    write_test_workflow(&env.cfg, "wtt-gamma", 1, "Test step");
    env
}

pub fn workflow_operations_env() -> TestEnv {
    let env = isolated_env();
    write_test_workflow(&env.cfg, "test-workflow-1", 3, "Operations step");
    write_test_workflow(&env.cfg, "test-workflow-2", 2, "Operations step");
    write_test_workflow(&env.cfg, "test-workflow-3", 2, "Operations step");
    env
}

pub fn invoke_workflow(payload: Value) -> Result<String> {
    ToolRegistry::global().try_invoke("Workflow", &payload, OutputFormat::Markdown)
}

pub fn extract_session_id(output: &str) -> String {
    output
        .lines()
        .find_map(|line| line.strip_prefix("Created session: "))
        .map(str::trim)
        .map(str::to_string)
        .expect("session id in output")
}

pub fn start_session(workflow_id: Value) -> (String, String) {
    let output = invoke_workflow(json!({ "workflowId": workflow_id })).expect("start workflow");
    let session_id = extract_session_id(&output);
    (output, session_id)
}

pub fn workflow_session_uri(session_id: &str) -> String {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|segment| segment.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    format!(
        "memory://thinking/workflow/{:04}/{:02}/{:02}/{}",
        timestamp.year(),
        timestamp.month(),
        timestamp.day(),
        session_id
    )
}

pub fn read_workflow_session(session_id: &str, cfg: &Config) -> (Option<Frontmatter>, String) {
    let uri = workflow_session_uri(session_id);
    let (frontmatter, content, _) = read_markdown(&uri, cfg).expect("read workflow session");
    (frontmatter, content)
}

pub fn queue_from_frontmatter(frontmatter: &Frontmatter) -> Vec<String> {
    match frontmatter.get("queue") {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::to_string)
            .collect(),
        Some(YamlValue::String(value)) if value.trim().starts_with('[') => {
            serde_json::from_str(value).expect("queue json")
        }
        Some(YamlValue::String(value)) if !value.trim().is_empty() => vec![value.clone()],
        _ => Vec::new(),
    }
}

pub fn frontmatter_usize(frontmatter: &Frontmatter, key: &str) -> usize {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            YamlValue::Number(number) => number.as_u64().map(|value| value as usize),
            YamlValue::String(text) => text.parse::<usize>().ok(),
            _ => None,
        })
        .expect("numeric frontmatter field")
}
