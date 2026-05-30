use super::*;

fn scoped_id(temp: &TempDir, base: &str) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    temp.path().hash(&mut hasher);
    let scope = format!("{:016x}", hasher.finish());
    let prefix_len = 63usize.saturating_sub(scope.len()).min(base.len());
    let prefix = base
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .take(prefix_len)
        .collect::<String>();
    format!("{prefix}-{scope}")
}

#[test]
fn pause_all_snapshots_prior_enabled() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    let gamma = scoped_id(&temp, "gamma");
    for (id, en) in [(&alpha, true), (&beta, true), (&gamma, false)] {
        create_direct(&c, id, en);
    }
    let p = action_with_test_launchd(&temp, &c, "pause_all");
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
            (alpha.clone(), true),
            (beta.clone(), true),
            (gamma.clone(), false)
        ])
    );
}
#[test]
fn pause_all_idempotent() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    create_direct(&c, &alpha, true);
    action_with_test_launchd(&temp, &c, "pause_all");
    schedule_store(&c)
        .update(&direct_task(&alpha, false))
        .expect("manual");
    let p = action_with_test_launchd(&temp, &c, "pause_all");
    assert_eq!(p["alreadyPaused"], true);
    assert_eq!(pause_snapshot(&c).get(&alpha), Some(&true));
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn pause_all_uninstalls_disabled_with_discrepancy() {
    let (temp, c) = ctx();
    let disabled_discrepancy = scoped_id(&temp, "disabled-discrepancy");
    let bin = resolve_maenifold_bin().expect("bin");
    create_task_with_test_launchd(
        &temp,
        &c,
        &create_payload(&disabled_discrepancy, true),
        &bin,
    )
    .expect("install");
    let mut task = direct_task(&disabled_discrepancy, false);
    task.enabled = false;
    schedule_store(&c).update(&task).expect("disable json");
    action_with_test_launchd(&temp, &c, "pause_all");
    let p = parse(
        &schedule(
            &c,
            &json!({"action":"verify_installed","scheduleId": disabled_discrepancy}),
        )
        .expect("verify"),
    );
    assert_eq!(p[0]["installed"], false);
}
#[test]
fn pause_all_partial_os_failure() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    create_direct(&c, &alpha, true);
    create_direct(&c, &beta, false);
    let p = parse(
        &schedule_batch::pause_all_with_bin(&c, &temp.path().join("missing-maenifold"))
            .expect("partial"),
    );
    assert_eq!(p["ok"], false);
    assert_eq!(p["failed"][0]["id"], alpha);
    assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
    assert!(!schedule_store(&c).read(&beta).expect("beta").enabled);
}
#[test]
fn resume_all_restores_prior() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    create_direct(&c, &alpha, true);
    create_direct(&c, &beta, false);
    action_with_test_launchd(&temp, &c, "pause_all");
    let mut beta_task = schedule_store(&c).read(&beta).expect("beta");
    beta_task.enabled = true;
    schedule_store(&c).update(&beta_task).expect("flip");
    let p = action_with_test_launchd(&temp, &c, "resume_all");
    assert_eq!(p["ok"], true);
    assert!(schedule_store(&c).read(&alpha).expect("alpha").enabled);
    assert!(!schedule_store(&c).read(&beta).expect("beta").enabled);
}
#[test]
fn resume_all_when_not_paused_noop() {
    let (temp, c) = ctx();
    let p = action_with_test_launchd(&temp, &c, "resume_all");
    assert_eq!(p["ok"], true);
    assert_eq!(p["message"], "Schedules were not paused.");
}
#[test]
fn resume_all_skips_deleted() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    create_direct(&c, &alpha, true);
    create_direct(&c, &beta, true);
    action_with_test_launchd(&temp, &c, "pause_all");
    schedule_store(&c).delete(&beta).expect("delete");
    let p = action_with_test_launchd(&temp, &c, "resume_all");
    assert_eq!(p["ok"], true);
    assert_eq!(p["missingSchedules"], json!([beta]));
    assert!(!crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
}
#[test]
fn resume_all_partial_os_failure_preserves_snapshot() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    create_direct(&c, &alpha, true);
    action_with_test_launchd(&temp, &c, "pause_all");
    let p = parse(
        &schedule_batch::resume_all_with_bin(&c, &temp.path().join("missing-maenifold"))
            .expect("partial"),
    );
    assert_eq!(p["ok"], false);
    assert_eq!(p["failed"][0]["id"], alpha);
    assert!(crate::schedule::pause_state::path(&c.config.maenifold_root).exists());
}
#[test]
fn set_all_enabled_rejects_during_pause() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    create_direct(&c, &alpha, false);
    action_with_test_launchd(&temp, &c, "pause_all");
    let r = schedule(&c, &json!({"action":"set_all_enabled","enabled":false}));
    assert!(
        matches!(r,Err(MaenifoldError::InvalidArgument(message)) if message=="Cannot bulk-toggle schedules while pause-all is active. Use Schedule action=resume_all to clear pause state first.")
    );
}
#[test]
fn set_all_enabled_changes_only_diff() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    let gamma = scoped_id(&temp, "gamma");
    for (id, en) in [(&alpha, true), (&beta, true), (&gamma, false)] {
        create_direct(&c, id, en);
    }
    let p = action_with_test_launchd(&temp, &c, "set_all_enabled");
    assert_eq!(p["ok"], true);
    assert_eq!(p["changedSchedules"], json!([alpha, beta]));
    assert_eq!(p["unchangedSchedules"], json!([gamma]));
}
#[test]
fn pause_status_when_paused() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    create_direct(&c, &alpha, true);
    create_direct(&c, &beta, false);
    action_with_test_launchd(&temp, &c, "pause_all");
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
    let (temp, c) = ctx();
    let seed = scoped_id(&temp, "seed");
    let implicit_enabled = scoped_id(&temp, "implicit-enabled");
    let explicit_enabled = scoped_id(&temp, "explicit-enabled");
    let disabled = scoped_id(&temp, "disabled");
    create_direct(&c, &seed, false);
    action_with_test_launchd(&temp, &c, "pause_all");
    let implicit = schedule(
        &c,
        &json!({"action":"create","id": implicit_enabled,"schedule":"every weekday at 9am","agent":"swe","prompt":"Run scheduled maintenance"}),
    );
    let explicit = schedule(&c, &create_payload(&explicit_enabled, true));
    let disabled = schedule(&c, &create_payload(&disabled, false));
    assert!(
        matches!(implicit, Err(MaenifoldError::InvalidArgument(message)) if message.contains("Cannot enable schedule while pause-all is active"))
    );
    assert!(
        matches!(explicit, Err(MaenifoldError::InvalidArgument(message)) if message.contains("Cannot enable schedule while pause-all is active"))
    );
    assert!(disabled.is_ok());
}
#[test]
fn update_rejects_enabling_during_pause() {
    let (temp, c) = ctx();
    let alpha = scoped_id(&temp, "alpha");
    let beta = scoped_id(&temp, "beta");
    create_direct(&c, &alpha, false);
    create_direct(&c, &beta, true);
    action_with_test_launchd(&temp, &c, "pause_all");
    let enable = schedule(&c, &json!({"action":"update","id": &alpha,"enabled":true}));
    schedule_store(&c)
        .update(&direct_task(&beta, true))
        .expect("force");
    let inherit_true = update_task_with_test_launchd(
        &temp,
        &c,
        &json!({"action":"update","id": &beta,"description":"still enabled"}),
        &fake_bin_in(&temp),
    );
    let disabled = schedule(&c, &json!({"action":"update","id": &alpha,"enabled":false}));
    assert!(
        matches!(enable, Err(MaenifoldError::InvalidArgument(message)) if message.contains("Cannot enable schedule while pause-all is active"))
    );
    assert!(
        matches!(inherit_true, Err(MaenifoldError::InvalidArgument(message)) if message.contains("Cannot enable schedule while pause-all is active"))
    );
    assert!(disabled.is_ok());
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
#[test]
fn create_enabled_unsupported_platform_is_blocked_before_persisting() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let unsupported_enabled = scoped_id(&temp, "unsupported-enabled");
    let payload = create_payload(&unsupported_enabled, true);
    let fake_bin = fake_bin_in(&temp);

    let result = create_task_with_test_launchd(&temp, &context, &payload, &fake_bin);

    assert!(matches!(result, Err(MaenifoldError::InvalidArgument(_))));
    let persisted = schedule_store(&context).list().expect("list schedules");
    assert!(persisted.is_empty());
}
