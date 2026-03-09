use maenifold_core::db::{open_connection, run_full_maintenance, run_light_maintenance};
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    AssetManager::initialize_assets(&cfg).expect("assets");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn open_wal_connection(cfg: &Config) -> Connection {
    let conn = open_connection(&cfg.database_path).expect("open db");
    let journal_mode: String = conn
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .expect("set WAL journal mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
    conn.pragma_update(None, "synchronous", "NORMAL")
        .expect("set synchronous");
    conn
}

/// Inserts bulk rows into file_content (and by trigger, file_search FTS5) to create
/// database waste that VACUUM can reclaim. Returns the number of rows inserted.
fn insert_bulk_data(conn: &Connection, row_count: usize) -> usize {
    let large_content = "X".repeat(4096);
    let tx = conn.unchecked_transaction().expect("transaction");
    for i in 0..row_count {
        tx.execute(
            "INSERT OR REPLACE INTO file_content
             (file_path, title, content, last_indexed, status, file_md5, file_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("memory://test/bulk-{i:04}"),
                format!("Bulk row {i}"),
                format!("{large_content} Row {i}"),
                chrono::Utc::now().to_rfc3339(),
                "active",
                format!("md5-bulk-{i:04}"),
                4096_i64,
            ],
        )
        .unwrap_or_else(|error| panic!("insert bulk row {i}: {error}"));
    }
    tx.commit().expect("commit bulk data");
    row_count
}

/// Deletes all bulk-inserted rows from file_content, leaving free pages in the DB file.
fn delete_bulk_data(conn: &Connection) {
    conn.execute(
        "DELETE FROM file_content WHERE file_path LIKE 'memory://test/bulk-%'",
        [],
    )
    .expect("delete bulk data");
}

/// Forces WAL content into the main database file so file size reflects actual data.
fn checkpoint_and_sync(conn: &Connection) {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
}

/// Returns the current database file size in bytes.
fn get_db_file_size(cfg: &Config) -> u64 {
    std::fs::metadata(&cfg.database_path)
        .expect("db metadata")
        .len()
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac1_database_maintenance_run_full_executes_without_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    assert!(run_full_maintenance(&mut conn).is_ok());
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac2_database_maintenance_run_full_restores_wal_journal_mode() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    run_full_maintenance(&mut conn).expect("run full maintenance");

    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("read journal mode");
    assert_eq!(journal_mode.to_ascii_lowercase(), "wal");
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac3_database_maintenance_run_full_reclaims_space() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    insert_bulk_data(&conn, 200);
    checkpoint_and_sync(&conn);
    let size_with_data = get_db_file_size(&env.cfg);

    delete_bulk_data(&conn);
    checkpoint_and_sync(&conn);
    let size_after_delete = get_db_file_size(&env.cfg);

    assert!(
        size_after_delete >= ((size_with_data as f64) * 0.8) as u64,
        "DELETE alone should not significantly shrink the DB file: with_data={size_with_data}, after_delete={size_after_delete}"
    );

    run_full_maintenance(&mut conn).expect("run full maintenance");

    let size_after_full = get_db_file_size(&env.cfg);
    assert!(
        size_after_full < size_with_data,
        "RunFull should reclaim space from deleted rows: with_data={size_with_data}, after_full={size_after_full}"
    );
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac4_database_maintenance_run_light_executes_without_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_wal_connection(&env.cfg);

    assert!(run_light_maintenance(&conn).is_ok());
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac5_database_maintenance_run_light_does_not_vacuum() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_wal_connection(&env.cfg);

    insert_bulk_data(&conn, 200);
    checkpoint_and_sync(&conn);
    let size_with_data = get_db_file_size(&env.cfg);

    delete_bulk_data(&conn);
    checkpoint_and_sync(&conn);

    run_light_maintenance(&conn).expect("run light maintenance");

    let size_after_light = get_db_file_size(&env.cfg);
    assert!(
        size_after_light >= ((size_with_data as f64) * 0.9) as u64,
        "RunLight should not reclaim space without VACUUM: with_data={size_with_data}, after_light={size_after_light}"
    );
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac6a_database_maintenance_run_full_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    run_full_maintenance(&mut conn).expect("first run full");
    run_full_maintenance(&mut conn).expect("second run full");
}

// T-QUAL-SYNC-001.1: RTM FR-25.6
#[test]
fn fr25_6_ac6b_database_maintenance_run_light_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_wal_connection(&env.cfg);

    run_light_maintenance(&conn).expect("first run light");
    run_light_maintenance(&conn).expect("second run light");
}
