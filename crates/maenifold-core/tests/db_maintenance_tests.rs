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

/// Returns the expected journal mode for the current environment.
/// Mirrors `connection.rs::journal_mode()`: CI uses DELETE, production uses WAL.
fn expected_journal_mode() -> &'static str {
    if std::env::var("CI").ok().as_deref() == Some("true")
        || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
    {
        "delete"
    } else {
        "wal"
    }
}

/// SQLite may release a few pages during DELETE or light maintenance, but without
/// VACUUM the file should stay within this ratio of the peak data size. 90% is the
/// tighter bound — asserting that non-VACUUM operations leave the file nearly intact.
const NO_VACUUM_RETENTION_RATIO: f64 = 0.9;

// T-DB-MAINT-001: RTM NFR-2 — ac1 MERGED into ac2 (subsumed; ac2 already proves
// successful execution via .expect() plus a behavioral postcondition).

// T-DB-MAINT-001: RTM NFR-2
#[test]
fn ac2_run_full_maintenance_restores_journal_mode() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    run_full_maintenance(&mut conn).expect("run full maintenance");

    // P1: run_full_maintenance closes and reopens the connection via open_connection(),
    // which sets journal_mode based on CI detection. Assert the environment-appropriate
    // mode rather than unconditionally expecting WAL.
    let journal_mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("read journal mode");
    let expected = expected_journal_mode();
    assert_eq!(
        journal_mode.to_ascii_lowercase(),
        expected,
        "run_full_maintenance should restore journal mode to '{expected}'"
    );
}

// T-DB-MAINT-001: RTM NFR-2
#[test]
fn ac3_run_full_maintenance_reclaims_space() {
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
        size_after_delete >= ((size_with_data as f64) * NO_VACUUM_RETENTION_RATIO) as u64,
        "DELETE alone should not significantly shrink the DB file: \
         with_data={size_with_data}, after_delete={size_after_delete}"
    );

    run_full_maintenance(&mut conn).expect("run full maintenance");

    // P5: Compare against size_after_delete (where rows are gone but pages are still
    // occupied) rather than size_with_data. This proves VACUUM actually reclaimed the
    // freed pages, not just that the DB is smaller than it was at peak.
    let size_after_full = get_db_file_size(&env.cfg);
    assert!(
        size_after_full < size_after_delete,
        "RunFull should reclaim freed pages via VACUUM; post-VACUUM size should be \
         less than post-DELETE size: after_delete={size_after_delete}, after_full={size_after_full}"
    );
}

// T-DB-MAINT-001: RTM NFR-2 — ac4 MERGED into ac5 (subsumed; ac5 already proves
// successful execution via .expect() plus a behavioral postcondition).

// T-DB-MAINT-001: RTM NFR-2
#[test]
fn ac5_run_light_maintenance_does_not_vacuum() {
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
        size_after_light >= ((size_with_data as f64) * NO_VACUUM_RETENTION_RATIO) as u64,
        "RunLight should not reclaim space without VACUUM: \
         with_data={size_with_data}, after_light={size_after_light}"
    );
}

// T-DB-MAINT-001: RTM NFR-2
/// Idempotency: f(f(x)) = f(x). After the first VACUUM, a second VACUUM on the
/// same database should produce identical page layout — no pages freed or added.
#[test]
fn ac6a_run_full_maintenance_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let mut conn = open_wal_connection(&env.cfg);

    // Create meaningful work for VACUUM: insert bulk data, delete it, leave free pages.
    insert_bulk_data(&conn, 200);
    checkpoint_and_sync(&conn);
    delete_bulk_data(&conn);
    checkpoint_and_sync(&conn);

    run_full_maintenance(&mut conn).expect("first run full");

    let page_count_1: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .expect("page_count after first run");
    let freelist_1: i64 = conn
        .query_row("PRAGMA freelist_count", [], |r| r.get(0))
        .expect("freelist_count after first run");

    run_full_maintenance(&mut conn).expect("second run full");

    let page_count_2: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .expect("page_count after second run");
    let freelist_2: i64 = conn
        .query_row("PRAGMA freelist_count", [], |r| r.get(0))
        .expect("freelist_count after second run");

    assert_eq!(
        page_count_1, page_count_2,
        "idempotency: second VACUUM should not change page count"
    );
    assert_eq!(
        freelist_1, freelist_2,
        "idempotency: second VACUUM should not change freelist count"
    );
}

// T-DB-MAINT-001: RTM NFR-2
/// Idempotency: light maintenance (FTS5 optimize + PASSIVE checkpoint) is idempotent
/// by construction. The second call should leave the database in the same state — same
/// page count, and no additional WAL frames to checkpoint.
#[test]
fn ac6b_run_light_maintenance_is_idempotent() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_wal_connection(&env.cfg);

    // Create some FTS5 content so optimize has real work on the first call.
    insert_bulk_data(&conn, 200);
    checkpoint_and_sync(&conn);

    run_light_maintenance(&conn).expect("first run light");
    checkpoint_and_sync(&conn);

    let page_count_1: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .expect("page_count after first run");

    run_light_maintenance(&conn).expect("second run light");
    checkpoint_and_sync(&conn);

    let page_count_2: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .expect("page_count after second run");

    assert_eq!(
        page_count_1, page_count_2,
        "idempotency: second light maintenance should not change page count"
    );
}
