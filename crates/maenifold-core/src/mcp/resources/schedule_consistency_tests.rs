use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::mcp::resources::map_resource_error;
use crate::schedule::{ScheduleStore, ScheduledTask};
use crate::test_util::ConfigResetGuard;
use crate::tools::{ToolContext, schedule as schedule_tool};
use crate::util::json::OutputFormat;
use rmcp::model::{ErrorCode, ResourceContents};
use serde_json::{Value, json};
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

const MAX_LOG_BYTES_PER_FILE: usize = 64 * 1024;
const MAX_TOTAL_LOG_BYTES: usize = 256 * 1024;
const TRUNCATED: &str = "... truncated";

#[derive(Debug, PartialEq, Eq)]
struct SemanticTask {
    id: String,
    description: String,
    cron_expression: String,
    agent: String,
    prompt: String,
    enabled: bool,
}

fn test_context() -> (TempDir, ToolContext, ConfigResetGuard) {
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
    config.ensure_directories().expect("ensure test directories");
    (temp, ToolContext::new(config, OutputFormat::Json), ConfigResetGuard)
}

fn call_schedule(context: &ToolContext, payload: Value) -> Value {
    let text = schedule_tool(context, &payload).expect("schedule tool call succeeds");
    serde_json::from_str(&text).expect("schedule tool returns json")
}

fn resource_json(uri: &str) -> Value {
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
    for raw_only in ["workdir", "timeout_seconds", "skip_if_running"] {
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

fn assert_all_surfaces_agree(context: &ToolContext, id: &str) -> SemanticTask {
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

fn store(context: &ToolContext) -> ScheduleStore {
    ScheduleStore::new(context.config.maenifold_root.join("schedules"))
}

fn set_enabled_with_store(context: &ToolContext, id: &str, enabled: bool) {
    let schedule_store = store(context);
    let mut task = schedule_store.read(id).expect("read task before update");
    task.enabled = enabled;
    schedule_store.update(&task).expect("store enabled mutation");
}

fn update_fields_with_store(context: &ToolContext, id: &str) {
    let schedule_store = store(context);
    let mut task = schedule_store.read(id).expect("read task before update");
    task.name = "Updated matrix task".to_string();
    task.schedule = "30 10 * * 1-5".to_string();
    task.schedule_human = Some("At 10:30 AM, Monday through Friday".to_string());
    task.agent = "blue-team".to_string();
    task.prompt = "Verify schedule consistency".to_string();
    schedule_store.update(&task).expect("store field mutation");
}

#[test]
#[serial]
fn schedule_tool_and_resources_match_after_mutation_matrix() {
    let (_temp, context, _guard) = test_context();
    let id = "matrix-task";

    let create = call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Initial matrix task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Run consistency matrix",
            "enabled": false
        }),
    );
    assert_eq!(create["action"], "create");
    assert_eq!(
        assert_all_surfaces_agree(&context, id),
        SemanticTask {
            id: id.to_string(),
            description: "Initial matrix task".to_string(),
            cron_expression: "0 9 * * 1-5".to_string(),
            agent: "swe".to_string(),
            prompt: "Run consistency matrix".to_string(),
            enabled: false,
        }
    );

    // Tool update currently synchronizes the OS scheduler even for disabled tasks.
    // Mutating through ScheduleStore keeps this matrix side-effect free while the
    // assertions still compare the public tool get/list and resource surfaces.
    update_fields_with_store(&context, id);
    let updated = assert_all_surfaces_agree(&context, id);
    assert_eq!(updated.description, "Updated matrix task");
    assert_eq!(updated.cron_expression, "30 10 * * 1-5");
    assert_eq!(updated.agent, "blue-team");
    assert_eq!(updated.prompt, "Verify schedule consistency");

    set_enabled_with_store(&context, id, true);
    assert!(assert_all_surfaces_agree(&context, id).enabled);
    set_enabled_with_store(&context, id, false);
    assert!(!assert_all_surfaces_agree(&context, id).enabled);

    let run_now = call_schedule(&context, json!({"action": "run_now", "id": id}));
    assert_eq!(run_now["action"], "run_now");
    assert_eq!(run_now["id"], id);
    let tool_after_run = call_schedule(&context, json!({"action": "get", "id": id}));
    let resource_after_run = resource_json(&format!("schedule://job/{id}"));
    let (tool_last_run, resource_last_run) = (&tool_after_run["last_run"], &resource_after_run["last_run"]);
    assert!(!tool_last_run.is_null() && !tool_last_run.as_str().is_some_and(str::is_empty));
    assert!(!resource_last_run.is_null() && !resource_last_run.as_str().is_some_and(str::is_empty));
    assert!(!assert_all_surfaces_agree(&context, id).enabled);

    let schedule_store = store(&context);
    schedule_store.delete(id).expect("delete without OS scheduler");
    let tool_get_error = schedule_tool(&context, &json!({"action": "get", "id": id}))
        .expect_err("tool get after delete fails");
    assert!(tool_get_error.to_string().contains("scheduled task"));
    let resource_error = map_resource_error(
        handle_read_resource(&format!("schedule://job/{id}")).expect_err("resource is missing"),
    );
    assert_eq!(resource_error.code, ErrorCode::RESOURCE_NOT_FOUND);
}

#[test]
#[serial]
fn missing_schedule_resources_are_not_found_without_filesystem_paths() {
    let (temp, _context, _guard) = test_context();

    for uri in ["schedule://job/missing-id", "schedule://logs/missing-id"] {
        let error = handle_read_resource(uri).expect_err("missing schedule resource fails");
        let mapped = map_resource_error(error);
        assert_eq!(mapped.code, ErrorCode::RESOURCE_NOT_FOUND);
        let message = mapped.message.to_string();
        assert!(message.contains("scheduled-task/missing-id"));
        assert!(!message.contains(&temp.path().display().to_string()));
        assert!(!message.contains('\\'));
    }
}

#[test]
#[serial]
fn schedule_log_tool_and_resource_share_bounds() {
    let (_temp, context, _guard) = test_context();
    let id = "log-bounds";
    call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Log bounds task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Generate logs",
            "enabled": false
        }),
    );
    write_large_logs(&store(&context).read(id).expect("read scheduled task"));

    let tool_logs = call_schedule(&context, json!({"action": "logs", "id": id, "limit": 10}));
    let resource_logs = resource_json(&format!("schedule://logs/{id}"));
    assert_eq!(tool_logs, resource_logs);
    assert_logs_within_bounds(&tool_logs);
}

fn write_large_logs(task: &ScheduledTask) {
    let log_dir = Config::current()
        .maenifold_root
        .join("schedules")
        .join(&task.id)
        .join("logs");
    fs::create_dir_all(&log_dir).expect("create log dir");
    let content = "x".repeat(MAX_LOG_BYTES_PER_FILE + 8 * 1024);
    for index in 0..6 {
        fs::write(log_dir.join(format!("20260422T120{index}00Z.log")), &content)
            .expect("write log file");
    }
}

fn assert_logs_within_bounds(logs: &Value) {
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
    assert!(entries.iter().any(|entry| entry["content"].as_str() == Some(TRUNCATED)));
}
