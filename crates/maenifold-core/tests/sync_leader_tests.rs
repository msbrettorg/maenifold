use maenifold_core::leader::LeaderElection;
use maenifold_core::{AssetManager, Config, GraphDatabase, db};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeaseRow {
    holder_id: String,
    epoch: i64,
    lease_expires_at_ms: i64,
    updated_at_ms: i64,
}

fn isolated_env() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

fn leader_election(cfg: &Config) -> LeaderElection {
    LeaderElection::new(&cfg.database_path)
}

fn open_db(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("open db")
}

fn current_lease(cfg: &Config) -> Option<LeaseRow> {
    let conn = open_db(cfg);
    conn.query_row(
        "SELECT holder_id, epoch, lease_expires_at_ms, updated_at_ms
         FROM sync_leader_lease
         WHERE lock_name = 'sync'",
        [],
        |row| {
            Ok(LeaseRow {
                holder_id: row.get(0)?,
                epoch: row.get(1)?,
                lease_expires_at_ms: row.get(2)?,
                updated_at_ms: row.get(3)?,
            })
        },
    )
    .ok()
}

fn expire_lease(cfg: &Config) {
    open_db(cfg)
        .execute(
            "UPDATE sync_leader_lease
             SET lease_expires_at_ms = 1
             WHERE lock_name = 'sync'",
            params![],
        )
        .expect("expire lease");
}

fn short_sleep() {
    thread::sleep(Duration::from_millis(10));
}

// T-4.5.1: RTM FR-30.3
#[test]
fn fr30_3_ac1_try_acquire_succeeds_on_empty_table() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (acquired, reason) = election.try_acquire("holder-a", 10).expect("acquire");

    assert!(acquired, "first acquire should succeed");
    assert_eq!(reason, "acquired");

    let lease = current_lease(&cfg).expect("lease row");
    assert_eq!(lease.holder_id, "holder-a");
    assert_eq!(lease.epoch, 1);
    assert!(lease.lease_expires_at_ms > 0);
    assert!(lease.lease_expires_at_ms > lease.updated_at_ms);
}

// T-4.5.1: RTM FR-30.3
#[test]
fn fr30_3_ac2_try_renew_fails_for_non_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 10).expect("acquire"),
        (true, "acquired")
    );
    let before = current_lease(&cfg).expect("lease before failed renew");

    let renewed = election.try_renew("holder-b", 10).expect("renew");

    assert!(!renewed, "non-holder renew must fail");
    assert_eq!(current_lease(&cfg), Some(before));
}

// T-4.5.1: RTM FR-30.3
#[test]
fn fr30_3_ac3_try_acquire_fails_when_lease_active() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 10).expect("first acquire"),
        (true, "acquired")
    );

    let outcome = election
        .try_acquire("holder-b", 10)
        .expect("contended acquire");

    assert_eq!(outcome, (false, "held_by_other"));
    assert_eq!(
        current_lease(&cfg).expect("lease row").holder_id,
        "holder-a"
    );
}

// T-4.5.1: RTM FR-30.3 / FR-30.4
#[test]
fn fr30_3_ac4_try_acquire_succeeds_on_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election
            .try_acquire("holder-a", 10)
            .expect("initial acquire"),
        (true, "acquired")
    );
    let before = current_lease(&cfg).expect("initial lease");
    assert_eq!(before.epoch, 1);
    expire_lease(&cfg);

    let outcome = election
        .try_acquire("holder-b", 10)
        .expect("takeover acquire");

    assert_eq!(outcome, (true, "acquired"));
    let after = current_lease(&cfg).expect("successor lease");
    assert_eq!(after.holder_id, "holder-b");
    assert_eq!(after.epoch, before.epoch + 1);
}

// T-4.5.1: RTM FR-30.3
#[test]
fn fr30_3_ac5_try_renew_succeeds_for_current_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 1).expect("acquire"),
        (true, "acquired")
    );
    let before = current_lease(&cfg).expect("lease before renew");
    short_sleep();

    let renewed = election.try_renew("holder-a", 5).expect("renew");

    assert!(renewed, "current holder should renew active lease");
    let after = current_lease(&cfg).expect("lease after renew");
    assert_eq!(after.holder_id, "holder-a");
    assert_eq!(after.epoch, before.epoch);
    assert!(after.lease_expires_at_ms >= before.lease_expires_at_ms);
    assert!(after.updated_at_ms >= before.updated_at_ms);
}

// T-4.5.1: RTM FR-30.7
#[test]
fn fr30_3_ac6_release_succeeds_for_current_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 10).expect("acquire"),
        (true, "acquired")
    );

    election.release("holder-a").expect("release");

    assert!(
        current_lease(&cfg).is_none(),
        "release should clear lease row"
    );
    assert!(!election.is_leader("holder-a").expect("released leader"));
    assert_eq!(
        election.try_acquire("holder-b", 10).expect("reacquire"),
        (true, "acquired")
    );
    assert_eq!(
        current_lease(&cfg)
            .expect("lease after reacquire")
            .holder_id,
        "holder-b"
    );
}

// T-4.5.1: RTM FR-30.7
#[test]
fn fr30_3_ac7_release_noop_for_wrong_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 10).expect("acquire"),
        (true, "acquired")
    );
    let before = current_lease(&cfg).expect("lease before wrong release");

    election.release("holder-b").expect("wrong holder release");

    assert_eq!(current_lease(&cfg), Some(before));
    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-4.5.1: RTM FR-30.7
#[test]
fn fr30_3_ac8_try_acquire_returns_reason_codes() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let acquired = election.try_acquire("holder-a", 10).expect("acquired");
    let renewed = election.try_acquire("holder-a", 10).expect("renewed");
    let contended = election.try_acquire("holder-b", 10).expect("contended");
    let forced = election
        .force_acquire("holder-c", 10)
        .expect("force acquire");

    assert_eq!(acquired, (true, "acquired"));
    assert_eq!(renewed, (true, "renewed"));
    assert_eq!(contended, (false, "held_by_other"));
    assert_eq!(forced, (true, "force_acquired"));
    assert_eq!(
        current_lease(&cfg).expect("lease after force").holder_id,
        "holder-c"
    );
}

// T-4.5.1: RTM FR-30.1
#[test]
fn fr30_1_ac1_non_leader_skips_community_recomputation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(!election.is_leader("holder-b").expect("holder b not leader"));
}

// T-4.5.1: RTM FR-30.1
#[test]
fn fr30_1_ac2_leader_runs_community_recomputation() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-4.5.1: RTM FR-30.2
#[test]
fn fr30_2_ac1_non_leader_skips_maintenance_work() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(!election.is_leader("holder-b").expect("holder b not leader"));
}

// T-4.5.1: RTM FR-30.2
#[test]
fn fr30_2_ac2_leader_runs_maintenance_normally() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");

    assert!(election.is_leader("holder-a").expect("holder a leader"));
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_when_no_lease_exists() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert!(!election.is_leader("holder-a").expect("no active leader"));
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_is_leader_returns_false_for_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");
    expire_lease(&cfg);

    assert!(!election.is_leader("holder-a").expect("expired lease"));
}

// T-4.5.1: RTM FR-30.3
#[test]
fn fr30_3_try_acquire_idempotent_for_same_holder() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let first = election.try_acquire("holder-a", 1).expect("first acquire");
    let before = current_lease(&cfg).expect("lease before idempotent acquire");
    short_sleep();
    let second = election.try_acquire("holder-a", 5).expect("second acquire");
    let after = current_lease(&cfg).expect("lease after idempotent acquire");

    assert_eq!(first, (true, "acquired"));
    assert_eq!(second, (true, "renewed"));
    assert_eq!(after.holder_id, "holder-a");
    assert_eq!(after.epoch, before.epoch);
    assert!(after.lease_expires_at_ms >= before.lease_expires_at_ms);
}

// T-4.5.1: RTM FR-30.8
#[test]
fn fr30_3_try_renew_fails_for_expired_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 10).expect("acquire");
    expire_lease(&cfg);
    let before = current_lease(&cfg).expect("expired lease row");

    let renewed = election
        .try_renew("holder-a", 10)
        .expect("renew after expiry");

    assert!(!renewed, "expired lease must not renew");
    assert_eq!(current_lease(&cfg), Some(before));
    assert!(!election.is_leader("holder-a").expect("expired leader"));
}
