#![allow(dead_code, unused_imports)]

// T-12.7: RTM NFR-1 - share sync leader fixtures while splitting regression coverage into themed files.
use maenifold_core::sync::leader::LeaderElection;
use maenifold_core::{AssetManager, Config, GraphDatabase, db};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseRow {
    pub holder_id: String,
    pub epoch: i64,
    pub lease_expires_at_ms: i64,
    pub updated_at_ms: i64,
}

pub fn isolated_env() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

pub fn leader_election(cfg: &Config) -> LeaderElection {
    LeaderElection::new(&cfg.database_path)
}

pub fn open_db(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("open db")
}

pub fn current_lease(cfg: &Config) -> Option<LeaseRow> {
    let conn = open_db(cfg);
    conn.query_row(
        "SELECT holder_id, epoch, lease_expires_at_ms, updated_at_ms
         FROM sync_leader_lease
         WHERE lock_name = 'sync_leader'",
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

pub fn expire_lease(cfg: &Config) {
    open_db(cfg)
        .execute(
            "UPDATE sync_leader_lease
             SET lease_expires_at_ms = 1
             WHERE lock_name = 'sync_leader'",
            params![],
        )
        .expect("expire lease");
}

pub fn short_sleep() {
    thread::sleep(Duration::from_millis(10));
}
