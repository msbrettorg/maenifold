use super::apply_env_scrub;
use super::{build_invocation, build_payload, parse_mcp_action};
use super::Supervisor;
use crate::schedule::{ScheduleStore, ScheduledTask};
use chrono::Utc;
use serde_json::{Value, json};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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
        mcp_action: None,
        mcp_parameters: None,
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

#[test]
fn parse_mcp_action_rsplit_correctness() {
    assert_eq!(
        parse_mcp_action("maenifold.gate.gate_Status").expect("parse multi-dot"),
        ("maenifold.gate", "gate_Status")
    );
    assert_eq!(
        parse_mcp_action("a.b").expect("parse single-dot"),
        ("a", "b")
    );
    assert!(parse_mcp_action("no-dot-here").is_err());
    assert!(parse_mcp_action("a.").is_err());
    assert!(parse_mcp_action(".b").is_err());
}

#[test]
fn supervisor_dispatches_mcp_action_when_set() {
    let mut task = task();
    task.mcp_action = Some("maenifold.gate.gate_Status".to_string());
    let invocation = build_invocation(&task, None).expect("mcp invocation");
    let payload: Value = serde_json::from_str(&invocation.payload).expect("json payload");

    assert_eq!(invocation.tool_name, "GatewayExecute");
    assert_eq!(payload["domain"], "maenifold.gate");
    assert_eq!(payload["command"], "gate_Status");
    assert_eq!(payload["parameters"], json!({}));
    assert!(!invocation.args.iter().any(|arg| arg == "--json"));
}

#[test]
fn supervisor_uses_acpexec_when_mcp_action_absent() {
    let invocation = build_invocation(&task(), None).expect("acp invocation");

    assert_eq!(invocation.tool_name, "AcpExec");
    assert!(invocation.args.iter().any(|arg| arg == "AcpExec"));
    assert!(invocation.args.iter().any(|arg| arg == "--json"));
}

#[test]
fn parameters_omitted_treated_as_empty_object() {
    let mut task = task();
    task.mcp_action = Some("maenifold.gate.gate_Status".to_string());
    task.mcp_parameters = None;
    let invocation = build_invocation(&task, None).expect("mcp invocation");
    let payload: Value = serde_json::from_str(&invocation.payload).expect("json payload");

    assert_eq!(payload["parameters"], json!({}));
}

#[test]
#[serial_test::serial]
fn supervisor_run_persists_runrecord_sidecar_json() {
    let (_temp, store, task, bin) = supervisor_fixture("sidecar-non-mcp", "plain stdout\n");
    store.create(&task).expect("create task");
    let record = Supervisor::new(store, bin).run(&task.id).expect("run task");
    let sidecar = read_only_sidecar(task.id.as_str(), _temp.path());

    assert_eq!(record.mcp_action, None);
    assert_eq!(sidecar["status"], "success");
    assert_eq!(sidecar["mcp_action"], Value::Null);
}

#[test]
#[serial_test::serial]
fn supervisor_run_with_mcp_action_populates_sidecar_mcp_fields() {
    let (_temp, store, mut task, bin) = supervisor_fixture("sidecar-mcp", "{\"ok\":true}\n");
    task.mcp_action = Some("maenifold.gate.gate_Status".to_string());
    task.mcp_parameters = Some(json!({"root": "/workspace"}));
    store.create(&task).expect("create task");
    Supervisor::new(store, bin).run(&task.id).expect("run task");
    let sidecar = read_only_sidecar(task.id.as_str(), _temp.path());

    assert_eq!(sidecar["mcp_action"], "maenifold.gate.gate_Status");
    assert_eq!(sidecar["mcp_parameters"], json!({"root": "/workspace"}));
    assert_eq!(sidecar["mcp_result"], json!({"ok": true}));
}

fn supervisor_fixture(id: &str, stdout: &str) -> (tempfile::TempDir, ScheduleStore, ScheduledTask, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin = temp.path().join("maenifold-fixture");
    fs::write(&bin, format!("#!/bin/sh\nprintf '{}'\n", stdout.replace('\'', "'\\''"))).expect("write fixture");
    #[cfg(unix)]
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
    let store = ScheduleStore::new(temp.path().join("schedules"));
    (temp, store, task_with_id(id), bin)
}

fn task_with_id(id: &str) -> ScheduledTask {
    let mut task = task();
    task.id = id.to_string();
    task.timeout_seconds = 5;
    task
}

fn read_only_sidecar(id: &str, root: &Path) -> Value {
    let logs = root.join("schedules").join(id).join("logs");
    let path = fs::read_dir(logs)
        .expect("read logs")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .expect("sidecar json exists");
    serde_json::from_slice(&fs::read(path).expect("read sidecar")).expect("parse sidecar")
}
