use super::*;
use crate::acp_config::MaenifoldSettings;
use crate::config::Config;
use crate::util::json::OutputFormat;
use std::collections::BTreeMap;
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
fn action(context: &ToolContext, name: &str) -> Value {
    parse(&schedule(context, &json!({"action": name})).expect("schedule action"))
}

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

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn create_enabled_os_install_failure_leaves_no_persisted_schedule() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("install-fails", true);
    let invalid_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &invalid_bin);

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
    let fake_bin = PathBuf::from("/nonexistent/binary");

    create_task_with_bin(&context, &payload, &fake_bin).expect("create paused schedule");

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
    let fake_bin = PathBuf::from("/nonexistent/binary");

    create_task_with_bin(&context, &payload, &fake_bin).expect("create paused schedule");

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
    let fake_bin = PathBuf::from("/nonexistent/binary");

    create_task_with_bin(&context, &payload, &fake_bin).expect("create paused schedule");

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
    let fake_bin = PathBuf::from("/nonexistent/binary");
    create_task_with_bin(&context, &payload, &fake_bin).expect("create paused schedule");

    update_task(
        &context,
        &json!({
            "action": "update",
            "id": "update-execution",
            "workdir": "/var/tmp/updated-work",
            "timeout_seconds": 17
        }),
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

#[test] fn pause_all_snapshots_prior_enabled() { let (_t,c)=ctx(); for (id,en) in [("alpha",true),("beta",true),("gamma",false)] { create_direct(&c,id,en); } let p=action(&c,"pause_all"); assert_eq!(p["ok"],true); assert!(schedule_store(&c).list().expect("list").iter().all(|t|!t.enabled)); assert_eq!(pause_snapshot(&c),BTreeMap::from([("alpha".into(),true),("beta".into(),true),("gamma".into(),false)])); }
#[test] fn pause_all_idempotent() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); action(&c,"pause_all"); schedule_store(&c).update(&direct_task("alpha",false)).expect("manual"); let p=action(&c,"pause_all"); assert_eq!(p["alreadyPaused"],true); assert_eq!(pause_snapshot(&c).get("alpha"),Some(&true)); }
#[cfg(any(target_os = "macos", target_os = "linux"))] #[test] fn pause_all_uninstalls_disabled_with_discrepancy() { let (_t,c)=ctx(); let bin=resolve_maenifold_bin().expect("bin"); create_task_with_bin(&c,&create_payload("disabled-discrepancy",true),&bin).expect("install"); let mut task=direct_task("disabled-discrepancy",false); task.enabled=false; schedule_store(&c).update(&task).expect("disable json"); action(&c,"pause_all"); let p=parse(&schedule(&c,&json!({"action":"verify_installed","scheduleId":"disabled-discrepancy"})).expect("verify")); assert_eq!(p["installed"],false); }
#[test] fn pause_all_partial_os_failure() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); create_direct(&c,"beta",false); let p=parse(&schedule_batch::pause_all_with_bin(&c,&PathBuf::from("/nonexistent/binary")).expect("partial")); assert_eq!(p["ok"],false); assert_eq!(p["failed"][0]["id"],"alpha"); assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists()); assert!(!schedule_store(&c).read("beta").expect("beta").enabled); }
#[test] fn resume_all_restores_prior() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); create_direct(&c,"beta",false); action(&c,"pause_all"); let mut beta=schedule_store(&c).read("beta").expect("beta"); beta.enabled=true; schedule_store(&c).update(&beta).expect("flip"); let p=action(&c,"resume_all"); assert_eq!(p["ok"],true); assert!(schedule_store(&c).read("alpha").expect("alpha").enabled); assert!(!schedule_store(&c).read("beta").expect("beta").enabled); }
#[test] fn resume_all_when_not_paused_noop() { let (_t,c)=ctx(); let p=action(&c,"resume_all"); assert_eq!(p["ok"],true); assert_eq!(p["message"],"Schedules were not paused."); }
#[test] fn resume_all_skips_deleted() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); create_direct(&c,"beta",true); action(&c,"pause_all"); schedule_store(&c).delete("beta").expect("delete"); let p=action(&c,"resume_all"); assert_eq!(p["ok"],true); assert_eq!(p["missingSchedules"],json!(["beta"])); assert!(!crate::schedule::pause_state::path(&c.config.maenifold_root).exists()); }
#[test] fn resume_all_partial_os_failure_preserves_snapshot() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); action(&c,"pause_all"); let p=parse(&schedule_batch::resume_all_with_bin(&c,&PathBuf::from("/nonexistent/binary")).expect("partial")); assert_eq!(p["ok"],false); assert_eq!(p["failed"][0]["id"],"alpha"); assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists()); }
#[test] fn set_all_enabled_rejects_during_pause() { let (_t,c)=ctx(); create_direct(&c,"alpha",false); action(&c,"pause_all"); let r=schedule(&c,&json!({"action":"set_all_enabled","enabled":false})); assert!(matches!(r,Err(MaenifoldError::InvalidArgument(message)) if message=="Cannot bulk-toggle schedules while pause-all is active. Use Schedule action=resume_all to clear pause state first.")); }
#[test] fn set_all_enabled_changes_only_diff() { let (_t,c)=ctx(); for (id,en) in [("alpha",true),("beta",true),("gamma",false)] { create_direct(&c,id,en); } let p=parse(&schedule(&c,&json!({"action":"set_all_enabled","enabled":false})).expect("set")); assert_eq!(p["ok"],true); assert_eq!(p["changedSchedules"],json!(["alpha","beta"])); assert_eq!(p["unchangedSchedules"],json!(["gamma"])); }
#[test] fn pause_status_when_paused() { let (_t,c)=ctx(); create_direct(&c,"alpha",true); create_direct(&c,"beta",false); action(&c,"pause_all"); let p=action(&c,"pause_status"); assert_eq!(p["paused"],true); assert_eq!(p["schedulesPaused"],2); assert!(p["pausedAt"].as_str().is_some_and(|v|v.ends_with('Z'))); }
#[test] fn pause_status_when_not_paused() { let (_t,c)=ctx(); assert_eq!(action(&c,"pause_status"),json!({"action":"pause_status","paused":false,"ok":true})); }
#[test] fn create_rejects_enabled_during_pause() { let (_t,c)=ctx(); create_direct(&c,"seed",false); action(&c,"pause_all"); let implicit=schedule(&c,&json!({"action":"create","id":"implicit-enabled","schedule":"every weekday at 9am","agent":"swe","prompt":"Run scheduled maintenance"})); let explicit=schedule(&c,&create_payload("explicit-enabled",true)); let disabled=schedule(&c,&create_payload("disabled",false)); assert!(matches!(implicit,Err(MaenifoldError::InvalidArgument(_)))); assert!(matches!(explicit,Err(MaenifoldError::InvalidArgument(_)))); assert!(disabled.is_ok()); }
#[test] fn update_rejects_enabling_during_pause() { let (_t,c)=ctx(); create_direct(&c,"alpha",false); create_direct(&c,"beta",true); action(&c,"pause_all"); let enable=schedule(&c,&json!({"action":"update","id":"alpha","enabled":true})); schedule_store(&c).update(&direct_task("beta",true)).expect("force"); let inherit_true=update_task(&c,&json!({"action":"update","id":"beta","description":"still enabled"})); let disabled=schedule(&c,&json!({"action":"update","id":"alpha","enabled":false})); assert!(matches!(enable,Err(MaenifoldError::InvalidArgument(_)))); assert!(inherit_true.is_err()); assert!(disabled.is_ok()); }
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn create_enabled_unsupported_platform_is_blocked_before_persisting() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let payload = create_payload("unsupported-enabled", true);
    let fake_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &fake_bin);

    assert!(matches!(result, Err(MaenifoldError::InvalidArgument(_))));
    let persisted = schedule_store(&context).list().expect("list schedules");
    assert!(persisted.is_empty());
}
