use super::*;

#[test]
fn pause_all_snapshots_prior_enabled() {
    let (_t, c) = ctx();
    for (id, en) in [("alpha", true), ("beta", true), ("gamma", false)] {
        create_direct(&c, id, en);
    }
    let p = action(&c, "pause_all");
    assert_eq!(p["ok"], true);
    assert!(
        schedule_store(&c)
            .list()
            .expect("list")
            .iter()
            .all(|t| !t.enabled)
    );
    assert_eq!(
        pause_snapshot(&c),
        BTreeMap::from([
            ("alpha".into(), true),
            ("beta".into(), true),
            ("gamma".into(), false)
        ])
    );
}
#[test]
fn pause_all_idempotent() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    action(&c, "pause_all");
    schedule_store(&c)
        .update(&direct_task("alpha", false))
        .expect("manual");
    let p = action(&c, "pause_all");
    assert_eq!(p["alreadyPaused"], true);
    assert_eq!(pause_snapshot(&c).get("alpha"), Some(&true));
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn pause_all_uninstalls_disabled_with_discrepancy() {
    let (_t, c) = ctx();
    let bin = resolve_maenifold_bin().expect("bin");
    create_task_with_bin(&c, &create_payload("disabled-discrepancy", true), &bin).expect("install");
    let mut task = direct_task("disabled-discrepancy", false);
    task.enabled = false;
    schedule_store(&c).update(&task).expect("disable json");
    action(&c, "pause_all");
    let p = parse(
        &schedule(
            &c,
            &json!({"action":"verify_installed","scheduleId":"disabled-discrepancy"}),
        )
        .expect("verify"),
    );
    assert_eq!(p[0]["installed"], false);
}
#[test]
fn pause_all_partial_os_failure() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    create_direct(&c, "beta", false);
    let p = parse(
        &schedule_batch::pause_all_with_bin(&c, &PathBuf::from("/nonexistent/binary"))
            .expect("partial"),
    );
    assert_eq!(p["ok"], false);
    assert_eq!(p["failed"][0]["id"], "alpha");
    assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
    assert!(!schedule_store(&c).read("beta").expect("beta").enabled);
}
#[test]
fn resume_all_restores_prior() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    create_direct(&c, "beta", false);
    action(&c, "pause_all");
    let mut beta = schedule_store(&c).read("beta").expect("beta");
    beta.enabled = true;
    schedule_store(&c).update(&beta).expect("flip");
    let p = action(&c, "resume_all");
    assert_eq!(p["ok"], true);
    assert!(schedule_store(&c).read("alpha").expect("alpha").enabled);
    assert!(!schedule_store(&c).read("beta").expect("beta").enabled);
}
#[test]
fn resume_all_when_not_paused_noop() {
    let (_t, c) = ctx();
    let p = action(&c, "resume_all");
    assert_eq!(p["ok"], true);
    assert_eq!(p["message"], "Schedules were not paused.");
}
#[test]
fn resume_all_skips_deleted() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    create_direct(&c, "beta", true);
    action(&c, "pause_all");
    schedule_store(&c).delete("beta").expect("delete");
    let p = action(&c, "resume_all");
    assert_eq!(p["ok"], true);
    assert_eq!(p["missingSchedules"], json!(["beta"]));
    assert!(!crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
}
#[test]
fn resume_all_partial_os_failure_preserves_snapshot() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    action(&c, "pause_all");
    let p = parse(
        &schedule_batch::resume_all_with_bin(&c, &PathBuf::from("/nonexistent/binary"))
            .expect("partial"),
    );
    assert_eq!(p["ok"], false);
    assert_eq!(p["failed"][0]["id"], "alpha");
    assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
}
#[test]
fn set_all_enabled_rejects_during_pause() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    action(&c, "pause_all");
    let r = schedule(&c, &json!({"action":"set_all_enabled","enabled":false}));
    assert!(
        matches!(r,Err(MaenifoldError::InvalidArgument(message)) if message=="Cannot bulk-toggle schedules while pause-all is active. Use Schedule action=resume_all to clear pause state first.")
    );
}
#[test]
fn set_all_enabled_changes_only_diff() {
    let (_t, c) = ctx();
    for (id, en) in [("alpha", true), ("beta", true), ("gamma", false)] {
        create_direct(&c, id, en);
    }
    let p =
        parse(&schedule(&c, &json!({"action":"set_all_enabled","enabled":false})).expect("set"));
    assert_eq!(p["ok"], true);
    assert_eq!(p["changedSchedules"], json!(["alpha", "beta"]));
    assert_eq!(p["unchangedSchedules"], json!(["gamma"]));
}
#[test]
fn pause_status_when_paused() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", true);
    create_direct(&c, "beta", false);
    action(&c, "pause_all");
    let p = action(&c, "pause_status");
    assert_eq!(p["paused"], true);
    assert_eq!(p["schedulesPaused"], 2);
    assert!(p["pausedAt"].as_str().is_some_and(|v| v.ends_with('Z')));
}
#[test]
fn pause_status_when_not_paused() {
    let (_t, c) = ctx();
    assert_eq!(
        action(&c, "pause_status"),
        json!({"action":"pause_status","paused":false,"ok":true})
    );
}
#[test]
fn create_rejects_enabled_during_pause() {
    let (_t, c) = ctx();
    create_direct(&c, "seed", false);
    action(&c, "pause_all");
    let implicit = schedule(
        &c,
        &json!({"action":"create","id":"implicit-enabled","schedule":"every weekday at 9am","agent":"swe","prompt":"Run scheduled maintenance"}),
    );
    let explicit = schedule(&c, &create_payload("explicit-enabled", true));
    let disabled = schedule(&c, &create_payload("disabled", false));
    assert!(matches!(implicit, Err(MaenifoldError::InvalidArgument(_))));
    assert!(matches!(explicit, Err(MaenifoldError::InvalidArgument(_))));
    assert!(disabled.is_ok());
}
#[test]
fn update_rejects_enabling_during_pause() {
    let (_t, c) = ctx();
    create_direct(&c, "alpha", false);
    create_direct(&c, "beta", true);
    action(&c, "pause_all");
    let enable = schedule(&c, &json!({"action":"update","id":"alpha","enabled":true}));
    schedule_store(&c)
        .update(&direct_task("beta", true))
        .expect("force");
    let inherit_true = update_task(
        &c,
        &json!({"action":"update","id":"beta","description":"still enabled"}),
    );
    let disabled = schedule(&c, &json!({"action":"update","id":"alpha","enabled":false}));
    assert!(matches!(enable, Err(MaenifoldError::InvalidArgument(_))));
    assert!(inherit_true.is_err());
    assert!(disabled.is_ok());
}
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
