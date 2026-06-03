use super::schedule_crud::run_now_with_bin;
use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::schedule::{ScheduleStore, ScheduledTask};
use crate::util::json::OutputFormat;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn test_context(temp: &TempDir) -> ToolContext {
    let root = temp.path().join("maenifold");
    ToolContext::new(
        Config {
            maenifold_root: root.clone(),
            memory_path: root.join("memory"),
            assets_path: root.join("assets"),
            thinking_path: root.join("memory/thinking"),
            sequential_path: root.join("memory/thinking/sequential"),
            database_path: root.join("memory.db"),
            settings: MaenifoldSettings::default(),
        },
        OutputFormat::Json,
    )
}

fn direct_task(id: &str) -> ScheduledTask {
    ScheduledTask {
        id: id.to_string(),
        name: id.to_string(),
        schedule: "0 9 * * *".to_string(),
        schedule_human: Some("every day at 9am".to_string()),
        agent: "swe".to_string(),
        prompt: "Run scheduled maintenance".to_string(),
        workdir: None,
        auto_grant_permissions: false,
        allowed_tools: Vec::new(),
        config_options: serde_json::Map::new(),
        mcp_action: Some("maenifold.gate.gate_Status".to_string()),
        mcp_parameters: Some(json!({"root": "/tmp"})),
        timeout_seconds: 10,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    }
}

fn schedule_store_for(context: &ToolContext) -> ScheduleStore {
    ScheduleStore::new(context.config.maenifold_root.join("schedules"))
}

fn parse(rendered: &str) -> Value {
    serde_json::from_str(rendered).expect("tool output should be json")
}

fn write_fixture_bin(temp: &TempDir, name: &str, body: &str) -> PathBuf {
    let bin = temp.path().join(name);
    fs::write(&bin, body).expect("write fixture");
    let mut permissions = fs::metadata(&bin).expect("fixture metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&bin, permissions).expect("fixture chmod");
    bin
}

fn slow_fixture_bin(temp: &TempDir) -> PathBuf {
    write_fixture_bin(
        temp,
        "slow-maenifold-fixture",
        "#!/bin/sh\nsleep 3\nprintf '{\"ok\":true}\\n'\n",
    )
}

fn fast_fixture_bin(temp: &TempDir) -> PathBuf {
    write_fixture_bin(
        temp,
        "fast-maenifold-fixture",
        "#!/bin/sh\nprintf '{\"ok\":true}\\n'\n",
    )
}

fn create_direct(context: &ToolContext, task: &ScheduledTask) {
    schedule_store_for(context)
        .create(task)
        .expect("create direct schedule");
}

#[test]
fn schedule_run_now_async_run_now_returns_within_two_seconds_even_for_long_running_task() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let task = direct_task("slow-run-now");
    create_direct(&context, &task);
    let slow_bin = slow_fixture_bin(&temp);

    let started = Instant::now();
    let result = run_now_with_bin(&context, &json!({"id": task.id}), &slow_bin);
    let elapsed = started.elapsed();

    assert!(
        result.is_ok(),
        "run_now should still return a payload even for long-running tasks: {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "expected async run_now to return within 2s, but it took {elapsed:?}"
    );
}

#[test]
fn schedule_run_now_async_run_now_response_payload_includes_pid_and_started_at() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let task = direct_task("payload-shape");
    create_direct(&context, &task);
    let fast_bin = fast_fixture_bin(&temp);

    let payload = parse(
        &run_now_with_bin(&context, &json!({"id": &task.id}), &fast_bin).expect("run_now payload"),
    );

    assert_eq!(payload["action"], "run_now");
    assert_eq!(payload["id"], task.id);
    assert!(
        payload["pid"].as_u64().is_some(),
        "expected numeric pid field, got {payload}"
    );
    assert!(
        payload["started_at"]
            .as_str()
            .is_some_and(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok()),
        "expected RFC3339 started_at field, got {payload}"
    );
}

#[test]
fn schedule_run_now_async_run_now_does_not_block_subsequent_actions_on_same_store() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let slow_task = direct_task("blocking-task");
    let fast_task = direct_task("other-task");
    create_direct(&context, &slow_task);
    create_direct(&context, &fast_task);
    let slow_bin = slow_fixture_bin(&temp);

    let started = Instant::now();
    let _ = run_now_with_bin(&context, &json!({"id": &slow_task.id}), &slow_bin);
    let listed = list_tasks(&context).expect("list after run_now");
    let elapsed = started.elapsed();
    let payload = parse(&listed);

    assert!(
        payload
            .as_array()
            .is_some_and(|tasks| tasks.iter().any(|task| task["id"] == fast_task.id.as_str())),
        "list should still return sibling schedules"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "expected list/get path to remain responsive while run_now is in flight, but run_now + list took {elapsed:?}"
    );
}
