use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
#[cfg(target_os = "macos")]
use crate::schedule::{FailingLaunchctlBackend, NoopLaunchctlBackend};
use crate::util::json::OutputFormat;
use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::path::Path;
#[cfg(target_os = "macos")]
use std::sync::Arc;
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

fn create_payload(id: &str, enabled: bool) -> Value {
    json!({
        "action": "create",
        "id": id,
        "schedule": "every weekday at 9am",
        "agent": "swe",
        "prompt": "Run scheduled maintenance",
        "enabled": enabled
    })
}

fn direct_task(id: &str, enabled: bool) -> ScheduledTask {
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
        mcp_action: None,
        mcp_parameters: None,
        timeout_seconds: 300,
        enabled,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    }
}

fn parse(rendered: &str) -> Value {
    serde_json::from_str(rendered).expect("tool output should be json")
}

fn create_direct(context: &ToolContext, id: &str, enabled: bool) {
    schedule_store(context)
        .create(&direct_task(id, enabled))
        .expect("create direct schedule");
}

fn pause_snapshot(context: &ToolContext) -> BTreeMap<String, bool> {
    crate::schedule::pause_state::read(&context.config.maenifold_root)
        .expect("read pause state")
        .expect("pause state exists")
        .prior_enabled
}

fn ctx() -> (TempDir, ToolContext) {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    (temp, context)
}

fn fake_bin_in(temp: &TempDir) -> PathBuf {
    let bin = temp.path().join("fake-maenifold");
    std::fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write fake bin");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(&bin)
            .expect("fake bin metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&bin, permissions).expect("set fake bin permissions");
    }
    bin
}

/// Test isolation contract: create_task, update_task, and delete_task must route
/// through test-only launchd helpers so schedule tests never touch host launchctl.
#[cfg(target_os = "macos")]
fn create_task_with_test_launchd(
    temp: &TempDir,
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
) -> Result<String> {
    create_task_with_bin_for_test(
        context,
        payload,
        bin,
        temp.path().join("Library/LaunchAgents"),
        Arc::new(NoopLaunchctlBackend),
    )
}

#[cfg(not(target_os = "macos"))]
fn create_task_with_test_launchd(
    _temp: &TempDir,
    context: &ToolContext,
    payload: &Value,
    bin: &std::path::Path,
) -> Result<String> {
    create_task_with_bin(context, payload, bin)
}

#[cfg(target_os = "macos")]
fn create_task_with_failing_launchd(
    temp: &TempDir,
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
) -> Result<String> {
    create_task_with_bin_for_test(
        context,
        payload,
        bin,
        temp.path().join("Library/LaunchAgents"),
        Arc::new(FailingLaunchctlBackend),
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
fn create_task_with_failing_launchd(
    _temp: &TempDir,
    _context: &ToolContext,
    _payload: &Value,
    _bin: &std::path::Path,
) -> Result<String> {
    Err(MaenifoldError::Message(
        "simulated launchctl failure for test".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn update_task_with_test_launchd(
    temp: &TempDir,
    context: &ToolContext,
    payload: &Value,
    bin: &Path,
) -> Result<String> {
    update_task_for_test(
        context,
        payload,
        bin,
        temp.path().join("Library/LaunchAgents"),
        Arc::new(NoopLaunchctlBackend),
    )
}

#[cfg(not(target_os = "macos"))]
fn update_task_with_test_launchd(
    _temp: &TempDir,
    context: &ToolContext,
    payload: &Value,
    _bin: &std::path::Path,
) -> Result<String> {
    update_task(context, payload)
}

#[cfg(target_os = "macos")]
fn action_with_test_launchd(temp: &TempDir, context: &ToolContext, name: &str) -> Value {
    let bin = fake_bin_in(temp);
    match name {
        "pause_all" => parse(
            &schedule_batch::pause_all_for_test(
                context,
                &bin,
                temp.path().join("Library/LaunchAgents"),
                Arc::new(NoopLaunchctlBackend),
            )
            .expect("pause_all action"),
        ),
        "resume_all" => parse(
            &schedule_batch::resume_all_for_test(
                context,
                &bin,
                temp.path().join("Library/LaunchAgents"),
                Arc::new(NoopLaunchctlBackend),
            )
            .expect("resume_all action"),
        ),
        "set_all_enabled" => parse(
            &schedule_batch::set_all_enabled_for_test(
                context,
                &json!({"enabled": false}),
                &bin,
                temp.path().join("Library/LaunchAgents"),
                Arc::new(NoopLaunchctlBackend),
            )
            .expect("set_all_enabled action"),
        ),
        "pause_status" => action(context, name),
        _ => panic!("unsupported isolated action: {name}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn action_with_test_launchd(_temp: &TempDir, context: &ToolContext, name: &str) -> Value {
    action(context, name)
}

fn action(context: &ToolContext, name: &str) -> Value {
    parse(&schedule(context, &json!({"action": name})).expect("schedule action"))
}

#[path = "schedule_backfill_tests.rs"]
mod schedule_backfill_tests;
#[path = "schedule_clear_history_tests.rs"]
mod schedule_clear_history_tests;
#[path = "schedule_mcp_tests.rs"]
mod schedule_mcp_tests;
#[path = "schedule_pause_tests.rs"]
mod schedule_pause_tests;

#[test]
fn preview_returns_validated_cron_next_fires_and_timezone() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);

    let rendered = preview_schedule(
        &context,
        &json!({"action": "preview", "schedule": "every weekday at 9am"}),
    )
    .expect("preview should parse weekday schedule");
    let payload: Value = serde_json::from_str(&rendered).expect("preview should be json");

    assert_eq!(payload["action"], "preview");
    assert_eq!(payload["cron"], "0 9 * * 1-5");
    assert!(
        payload["description"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(
        payload["nextFires"]
            .as_array()
            .expect("nextFires array")
            .len(),
        5
    );
    assert!(
        payload["timezone"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
}

#[test]
fn preview_rejects_overly_long_schedule_expression() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let schedule = format!("every{}weekday at 9am", " ".repeat(1981));
    assert_eq!(schedule.len(), 2000);
    assert!(matches!(
        preview_schedule(&context, &json!({"action": "preview", "schedule": schedule})),
        Err(MaenifoldError::InvalidArgument(message)) if message.contains("too long")
    ));
}

#[test]
fn preview_rejects_impossible_cron_with_no_upcoming_fire_times() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    assert!(matches!(
        preview_schedule(&context, &json!({"action": "preview", "schedule": "0 0 30 2 *"})),
        Err(MaenifoldError::InvalidArgument(message))
            if message.contains("no upcoming fire times")
    ));
}

#[test]
fn verify_installed_empty_store_returns_empty_status_array() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);

    let rendered = schedule(&context, &json!({"action": "verify_installed"}))
        .expect("empty store verify should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("verify result should be json");

    assert_eq!(payload, json!([]));
}

#[test]
fn verify_installed_unknown_schedule_id_is_invalid_argument() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);

    let result = schedule(
        &context,
        &json!({"action": "verify_installed", "scheduleId": "missing-schedule"}),
    );

    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message))
            if message.contains("unknown scheduleId 'missing-schedule'")
    ));
}

#[test]
fn verify_installed_with_known_schedule_id_returns_array_of_one() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    create_direct(&context, "known-schedule", false);

    let rendered = schedule(
        &context,
        &json!({"action": "verify_installed", "scheduleId": "known-schedule"}),
    )
    .expect("known schedule verify should succeed");
    let payload: Value = serde_json::from_str(&rendered).expect("verify result should be json");
    let statuses = payload
        .as_array()
        .expect("single verify should return array");

    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0]["id"], "known-schedule");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn create_enabled_os_install_failure_leaves_no_persisted_schedule() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("install-fails", true);
    let fake_bin = fake_bin_in(&temp);

    let result = create_task_with_failing_launchd(&temp, &context, &payload, &fake_bin);

    assert!(
        result.is_err(),
        "invalid scheduler binary should fail create"
    );
    let persisted = schedule_store(&context).list().expect("list schedules");
    assert!(
        persisted.iter().all(|task| task.id != "install-fails"),
        "failed OS install must not leave active-looking schedule config"
    );
}

#[test]
fn create_defaults_to_no_auto_grant_permissions() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("safe-default", false);
    let fake_bin = fake_bin_in(&temp);

    create_task_with_test_launchd(&temp, &context, &payload, &fake_bin)
        .expect("create paused schedule");

    let persisted = schedule_store(&context)
        .read("safe-default")
        .expect("read schedule");
    assert!(!persisted.auto_grant_permissions);
    assert!(persisted.allowed_tools.is_empty());
}

#[test]
fn create_accepts_explicit_permission_allowlist() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("explicit-permissions", false);
    payload["auto_grant_permissions"] = json!(true);
    payload["allowed_tools"] = json!(["ReadMemory", "SearchMemories"]);
    let fake_bin = fake_bin_in(&temp);

    create_task_with_test_launchd(&temp, &context, &payload, &fake_bin)
        .expect("create paused schedule");

    let persisted = schedule_store(&context)
        .read("explicit-permissions")
        .expect("read schedule");
    assert!(persisted.auto_grant_permissions);
    assert_eq!(
        persisted.allowed_tools,
        vec!["ReadMemory", "SearchMemories"]
    );
}

#[test]
fn create_persists_workdir_and_timeout_seconds_from_payload() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("custom-execution", false);
    payload["workdir"] = json!("/var/tmp/maenifold-work");
    payload["timeout_seconds"] = json!(42);
    let fake_bin = fake_bin_in(&temp);

    create_task_with_test_launchd(&temp, &context, &payload, &fake_bin)
        .expect("create paused schedule");

    let persisted = schedule_store(&context)
        .read("custom-execution")
        .expect("read schedule");
    assert_eq!(
        persisted.workdir,
        Some(PathBuf::from("/var/tmp/maenifold-work"))
    );
    assert_eq!(persisted.timeout_seconds, 42);
}

#[test]
fn update_persists_workdir_and_timeout_seconds_from_payload() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("update-execution", false);
    let fake_bin = fake_bin_in(&temp);
    create_task_with_test_launchd(&temp, &context, &payload, &fake_bin)
        .expect("create paused schedule");

    update_task_with_test_launchd(
        &temp,
        &context,
        &json!({
            "action": "update",
            "id": "update-execution",
            "workdir": "/var/tmp/updated-work",
            "timeout_seconds": 17
        }),
        &fake_bin,
    )
    .expect("update paused schedule");

    let persisted = schedule_store(&context)
        .read("update-execution")
        .expect("read schedule");
    assert_eq!(
        persisted.workdir,
        Some(PathBuf::from("/var/tmp/updated-work"))
    );
    assert_eq!(persisted.timeout_seconds, 17);
}
