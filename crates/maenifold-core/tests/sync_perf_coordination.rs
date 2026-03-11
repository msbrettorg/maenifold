// T-0.1: RTM NFR-3
#![allow(dead_code, unused_imports)]

include!("sync_perf_support.rs");

// T-0.1: RTM NFR-3
#[test]
fn sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions() {
    let env = TestEnv::new();
    let _ttl = AbandonmentEnvGuard::set("1");
    let unique = unique_suffix();
    let file_uri = format!("memory://thinking/sequential/2026/03/09/session-mtime-{unique}");
    let old_last_indexed = (Utc::now() - Duration::hours(2)).to_rfc3339();

    seed_session(&env.cfg, &file_uri, "active", &old_last_indexed);
    let session_path = maenifold_core::markdown::uri_to_path(&file_uri, &env.cfg.memory_path)
        .expect("session path");

    let recent_mtime = FileTime::from_unix_time(Utc::now().timestamp(), 0);
    set_file_mtime(&session_path, recent_mtime).expect("set recent mtime");
    let abandoned_a = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup recent");
    assert_eq!(abandoned_a, 0, "recent mtime should protect session");
    assert_eq!(frontmatter_status(&env.cfg, &file_uri), "active");

    set_last_indexed(&env.cfg, &file_uri, &old_last_indexed);
    let stale_mtime = FileTime::from_unix_time((Utc::now() - Duration::hours(2)).timestamp(), 0);
    set_file_mtime(&session_path, stale_mtime).expect("set stale mtime");
    let abandoned_b = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup stale");
    assert_eq!(abandoned_b, 1, "stale mtime should abandon session");
    assert_eq!(frontmatter_status(&env.cfg, &file_uri), "abandoned");
}

// T-0.1: RTM NFR-3
#[test]
fn try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true() {
    let env = TestEnv::new();
    let election = LeaderElection::new(&env.cfg.database_path);

    if !election.try_acquire("holder-a", 10).expect("normalize").0 {
        panic!("lock was unexpectedly held at test start");
    }
    assert!(election.release("holder-a", 1).expect("normalize release"));

    let first = election.try_acquire("holder-a", 10).expect("first acquire");
    assert_eq!(first, (true, "acquired", 2));

    let second = election
        .try_acquire("holder-b", 10)
        .expect("second acquire");
    assert_eq!(second, (false, "held_by_other", 2));

    assert!(election.release("holder-a", 2).expect("release held lock"));
    let third = election.try_acquire("holder-c", 10).expect("third acquire");
    assert_eq!(third, (true, "acquired", 3));
    assert!(election.release("holder-c", 3).expect("cleanup release"));
}
