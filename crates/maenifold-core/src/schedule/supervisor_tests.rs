use super::apply_env_scrub;
use super::build_payload;
use crate::schedule::ScheduledTask;
use chrono::Utc;
use serde_json::{json, Value};
use std::process::Command;

fn task() -> ScheduledTask {
    ScheduledTask {
        id: "permissions".into(),
        name: "Permissions".into(),
        schedule: "0 9 * * *".into(),
        schedule_human: None,
        agent: "swe".into(),
        prompt: "Use a sensitive tool".into(),
        workdir: None,
        auto_grant_permissions: false,
        allowed_tools: Vec::new(),
        timeout_seconds: 300,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
    }
}

#[test]
fn payload_uses_safe_permission_default() {
    let payload = build_payload(&task(), None).expect("payload");
    let payload: Value = serde_json::from_str(&payload).expect("json payload");

    assert_eq!(payload["autoGrantPermissions"], false);
    assert!(payload.get("allowedTools").is_none());
}

#[test]
fn denied_permission_scenario_threads_auto_grant_false() {
    let mut task = task();
    task.auto_grant_permissions = false;
    task.allowed_tools = Vec::new();

    let payload = build_payload(&task, None).expect("payload");
    let payload: Value = serde_json::from_str(&payload).expect("json payload");

    assert_eq!(payload["autoGrantPermissions"], false);
    assert!(payload.get("allowedTools").is_none());
}

#[test]
fn payload_threads_explicit_permission_fields() {
    let mut task = task();
    task.auto_grant_permissions = true;
    task.allowed_tools = vec!["ReadMemory".into(), "SearchMemories".into()];

    let payload = build_payload(&task, None).expect("payload");
    let payload: Value = serde_json::from_str(&payload).expect("json payload");

    assert_eq!(payload["autoGrantPermissions"], true);
    assert_eq!(
        payload["allowedTools"],
        json!(["ReadMemory", "SearchMemories"])
    );
}

#[test]
#[serial_test::serial]
fn supervisor_run_scrubs_environment() {
    let key = "FAKE_API_KEY_T3054";
    let original = std::env::var_os(key);
    unsafe { std::env::set_var(key, "secret123") };

    let mut command = Command::new("/usr/bin/env");
    // Testing the scrub helper directly keeps this focused on T-305.4. A full
    // Supervisor::run fixture would need to emulate the maenifold AcpExec CLI.
    apply_env_scrub(&mut command);
    let output = command.output().expect("env should run");

    match original {
        Some(value) => unsafe { std::env::set_var(key, value) },
        None => unsafe { std::env::remove_var(key) },
    }
    let stdout = String::from_utf8(output.stdout).expect("env stdout should be utf-8");
    assert!(!stdout.contains(key));
    assert!(!stdout.contains("secret123"));
}
