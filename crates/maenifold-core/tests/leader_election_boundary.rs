use maenifold_core::leader::LeaderElection;
use maenifold_core::{AssetManager, Config, GraphDatabase, db};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::Connection;
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

fn current_lease(cfg: &Config) -> Option<(String, i64, i64, i64)> {
    let conn = open_db(cfg);
    conn.query_row(
        "SELECT holder_id, epoch, lease_expires_at_ms, updated_at_ms
         FROM sync_leader_lease
         WHERE lock_name = 'sync'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .ok()
}

fn sleep_for_expiry() {
    thread::sleep(Duration::from_millis(1_100));
}

// T-4.5a: RTM FR-30.1
#[test]
fn first_acquire_succeeds_when_no_existing_leader() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let (acquired, reason) = election.try_acquire("holder-a", 5).expect("acquire");

    assert!(acquired);
    assert_eq!(reason, "acquired");
    assert!(election.is_leader("holder-a").expect("is leader"));

    let lease = current_lease(&cfg).expect("lease row");
    assert_eq!(lease.0, "holder-a");
    assert_eq!(lease.1, 1);
    assert!(
        lease.2 > lease.3,
        "lease should expire after it was updated"
    );
}

// T-4.5a: RTM FR-30.2
#[test]
fn same_holder_reacquire_renews_existing_lease() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    let first = election.try_acquire("holder-a", 1).expect("first acquire");
    assert_eq!(first, (true, "acquired"));
    let initial = current_lease(&cfg).expect("initial lease");

    thread::sleep(Duration::from_millis(25));

    let second = election.try_acquire("holder-a", 5).expect("second acquire");
    let renewed = election.try_renew("holder-a", 5).expect("renew");
    let current = current_lease(&cfg).expect("renewed lease");

    assert_eq!(second, (true, "renewed"));
    assert!(renewed, "active leader should renew successfully");
    assert_eq!(current.0, "holder-a");
    assert_eq!(current.1, initial.1, "re-acquire must not bump epoch");
    assert!(current.2 > initial.2, "lease expiry should move forward");
    assert!(current.3 >= initial.3, "update time should advance");
}

// T-4.5a: RTM FR-30.3
#[test]
fn different_holder_is_blocked_while_lease_is_active() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election.try_acquire("holder-a", 5).expect("acquire"),
        (true, "acquired")
    );

    let outcome = election
        .try_acquire("holder-b", 5)
        .expect("blocked acquire");

    assert_eq!(outcome, (false, "held_by_other"));
    assert!(election.is_leader("holder-a").expect("leader a"));
    assert!(!election.is_leader("holder-b").expect("leader b"));

    let lease = current_lease(&cfg).expect("lease row");
    assert_eq!(lease.0, "holder-a");
    assert_eq!(lease.1, 1);
}

// T-4.5a: RTM FR-30.4
#[test]
fn expired_lease_allows_new_holder_to_acquire() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert_eq!(
        election
            .try_acquire("holder-a", 1)
            .expect("initial acquire"),
        (true, "acquired")
    );
    sleep_for_expiry();

    let outcome = election
        .try_acquire("holder-b", 1)
        .expect("takeover acquire");

    assert_eq!(outcome, (true, "acquired"));
    assert!(!election.is_leader("holder-a").expect("old leader expired"));
    assert!(election.is_leader("holder-b").expect("new leader active"));

    let lease = current_lease(&cfg).expect("lease row");
    assert_eq!(lease.0, "holder-b");
}

// T-4.5a: RTM FR-30.7
#[test]
fn release_clears_leadership() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election.try_acquire("holder-a", 5).expect("acquire");
    election.release("holder-a").expect("release");

    assert!(!election.is_leader("holder-a").expect("released leader"));
    let remaining = current_lease(&cfg);
    assert!(
        remaining.is_none(),
        "release should delete lease row, found {remaining:?}"
    );
}

// T-4.5a: RTM FR-30.5 / FR-30.6 / FR-30.8
#[test]
fn is_leader_returns_correct_status() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    assert!(
        !election
            .is_leader("holder-a")
            .expect("no leader before acquire")
    );
    assert_eq!(
        election.try_acquire("holder-a", 1).expect("acquire"),
        (true, "acquired")
    );
    assert!(election.is_leader("holder-a").expect("leader active"));
    assert!(
        !election
            .is_leader("holder-b")
            .expect("other holder inactive")
    );

    assert!(
        election
            .try_renew("holder-a", 1)
            .expect("renew active lease")
    );
    sleep_for_expiry();

    assert!(!election.is_leader("holder-a").expect("lease expired"));
    assert!(
        !election
            .try_renew("holder-a", 1)
            .expect("cannot renew expired lease")
    );
}

// T-4.5a: RTM FR-30.4
#[test]
fn epoch_increments_when_new_holder_acquires_after_expiry() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_env();
    let election = leader_election(&cfg);

    election
        .try_acquire("holder-a", 1)
        .expect("initial acquire");
    let initial_epoch = current_lease(&cfg).expect("initial lease").1;

    sleep_for_expiry();
    election
        .try_acquire("holder-b", 1)
        .expect("successor acquire");

    let next_epoch = current_lease(&cfg).expect("successor lease").1;
    assert_eq!(next_epoch, initial_epoch + 1);
}
