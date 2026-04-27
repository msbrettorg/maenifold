use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::schedule::{ScheduleStore, ScheduledTask};
use crate::test_util::ConfigResetGuard;
use crate::tools::{ToolContext, schedule as schedule_tool};
use crate::util::json::OutputFormat;
use rmcp::model::ResourceContents;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;

const MAX_LOG_BYTES_PER_FILE: usize = 64 * 1024;
const MAX_TOTAL_LOG_BYTES: usize = 256 * 1024;
const TRUNCATED: &str = "... truncated";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SemanticTask {
    pub(super) id: String,
    pub(super) description: String,
    pub(super) cron_expression: String,
    pub(super) agent: String,
    pub(super) prompt: String,
    pub(super) enabled: bool,
}

pub(super) fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
    let temp = TempDir::new().expect("create temp dir");
    let root = temp.path().join("maenifold");
    Config::override_root(&root);
    Config::set_database_path(root.join("memory.db"));
    let config = Config {
        maenifold_root: root.clone(),
        memory_path: root.join("memory"),
        assets_path: root.join("assets"),
        thinking_path: root.join("memory/thinking"),
        sequential_path: root.join("memory/thinking/sequential"),
        database_path: root.join("memory.db"),
        settings: MaenifoldSettings::default(),
    };
    config
        .ensure_directories()
        .expect("ensure test directories");
    (
        temp,
        ToolContext::new(config, OutputFormat::Json),
        ConfigResetGuard,
    )
}

pub(super) fn call_schedule(context: &ToolContext, payload: Value) -> Value {
    let text = schedule_tool(context, &payload).expect("schedule tool call succeeds");
    serde_json::from_str(&text).expect("schedule tool returns json")
}

pub(super) fn resource_json(uri: &str) -> Value {
    let result = handle_read_resource(uri).expect("schedule resource read succeeds");
    let first = result.contents.first().expect("resource has content");
    let ResourceContents::TextResourceContents { text, .. } = first else {
        panic!("schedule resources must return text content");
    };
    serde_json::from_str(text).expect("resource content is json")
}

fn task_from_tool(value: &Value) -> SemanticTask {
    SemanticTask {
        id: string_field(value, "id"),
        description: string_field(value, "description"),
        cron_expression: string_field(value, "schedule"),
        agent: string_field(value, "agent"),
        prompt: string_field(value, "prompt"),
        enabled: bool_field(value, "enabled"),
    }
}

fn task_from_resource(value: &Value) -> SemanticTask {
    SemanticTask {
        id: string_field(value, "id"),
        description: string_field(value, "name"),
        cron_expression: string_field(value, "schedule"),
        agent: string_field(value, "agent"),
        prompt: string_field(value, "prompt"),
        enabled: bool_field(value, "enabled"),
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("'{field}' must be a string in {value}"))
        .to_string()
}

fn bool_field(value: &Value, field: &str) -> bool {
    value[field]
        .as_bool()
        .unwrap_or_else(|| panic!("'{field}' must be a bool in {value}"))
}

fn assert_schema_difference_documented(tool_task: &Value, resource_task: &Value) {
    assert_eq!(tool_task["description"], resource_task["name"]);
    assert!(tool_task.get("name").is_none());
    assert!(resource_task.get("description").is_none());
    for raw_only in ["workdir", "timeout_seconds"] {
        assert!(
            tool_task.get(raw_only).is_none(),
            "tool task_value intentionally omits raw ScheduledTask field '{raw_only}'"
        );
        assert!(
            resource_task.get(raw_only).is_some(),
            "resource serializes raw ScheduledTask including '{raw_only}'"
        );
    }
}

pub(super) fn assert_all_surfaces_agree(context: &ToolContext, id: &str) -> SemanticTask {
    let tool_get = call_schedule(context, json!({"action": "get", "id": id}));
    let tool_list = call_schedule(context, json!({"action": "list"}));
    let tool_list_task = tool_list
        .as_array()
        .expect("tool list is array")
        .iter()
        .find(|task| task["id"] == id)
        .expect("tool list contains task");
    let resource_get = resource_json(&format!("schedule://job/{id}"));
    let resource_list = resource_json("schedule://jobs");
    let resource_list_task = resource_list
        .as_array()
        .expect("resource list is array")
        .iter()
        .find(|task| task["id"] == id)
        .expect("resource list contains task");

    assert_schema_difference_documented(&tool_get, &resource_get);
    let expected = task_from_tool(&tool_get);
    assert_eq!(task_from_tool(tool_list_task), expected);
    assert_eq!(task_from_resource(&resource_get), expected);
    assert_eq!(task_from_resource(resource_list_task), expected);
    expected
}

pub(super) fn store(context: &ToolContext) -> ScheduleStore {
    ScheduleStore::new(context.config.maenifold_root.join("schedules"))
}

pub(super) fn set_enabled_with_store(context: &ToolContext, id: &str, enabled: bool) {
    let schedule_store = store(context);
    let mut task = schedule_store.read(id).expect("read task before update");
    task.enabled = enabled;
    schedule_store
        .update(&task)
        .expect("store enabled mutation");
}

pub(super) fn update_fields_with_store(context: &ToolContext, id: &str) {
    let schedule_store = store(context);
    let mut task = schedule_store.read(id).expect("read task before update");
    task.name = "Updated matrix task".to_string();
    task.schedule = "30 10 * * 1-5".to_string();
    task.schedule_human = Some("At 10:30 AM, Monday through Friday".to_string());
    task.agent = "blue-team".to_string();
    task.prompt = "Verify schedule consistency".to_string();
    schedule_store.update(&task).expect("store field mutation");
}

pub(super) fn write_large_logs(task: &ScheduledTask) {
    let log_dir = Config::current()
        .maenifold_root
        .join("schedules")
        .join(&task.id)
        .join("logs");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let content = "x".repeat(MAX_LOG_BYTES_PER_FILE + 8 * 1024);
    for index in 0..6 {
        fs::write(
            log_dir.join(format!("20260422T120{index}00Z.log")),
            &content,
        )
        .expect("write log file");
    }
}

pub(super) fn assert_logs_within_bounds(logs: &Value) {
    let entries = logs.as_array().expect("logs are an array");
    let mut total = 0usize;
    let mut largest_retained_log = 0usize;
    for entry in entries {
        let content = entry["content"].as_str().expect("log content is string");
        if content == TRUNCATED {
            continue;
        }
        assert!(content.len() <= MAX_LOG_BYTES_PER_FILE);
        largest_retained_log = largest_retained_log.max(content.len());
        total += content.len();
    }
    assert!(total <= MAX_TOTAL_LOG_BYTES);
    assert!(largest_retained_log >= MAX_LOG_BYTES_PER_FILE - 1024);
    assert!(
        entries
            .iter()
            .any(|entry| entry["content"].as_str() == Some(TRUNCATED))
    );
}
