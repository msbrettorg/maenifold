// T-15.11 + T-15.12 + T-15.22 + T-15.23: RTM FR-11 — DB initialization fixes
// T-10.9: RTM FR-15 — busy_timeout pragma pinned
//
// T-15.11: Verify PRAGMA foreign_keys=ON is set on every connection
// T-15.12: Verify co_occurrence_count column type is INTEGER (not REAL)
// T-15.22: Verify journal_mode=WAL and synchronous=NORMAL on connections
// T-15.23: Verify no query_only on standard connections
// T-10.9:  Verify busy_timeout is set on every connection

use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;
use maenifold_core::db::open_connection;
use maenifold_core::{Config, GraphDatabase};
use rusqlite::params;
use tempfile::TempDir;

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
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

/// Returns true when running under CI (GitHub Actions, etc.) where WAL mode
/// is intentionally disabled in favour of DELETE journal mode due to tmpfs
/// limitations. Mirrors `journal_mode()` in `connection.rs`.
fn is_ci() -> bool {
    std::env::var("CI").ok().as_deref() == Some("true")
        || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
}

// T-15.11: RTM FR-11 — PRAGMA foreign_keys returns 1 after open_connection
#[test]
fn t15_11_foreign_keys_enabled_after_open_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_connection(&env.cfg.database_path).expect("open db");

    let fk_enabled: i64 = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .expect("read foreign_keys pragma");

    assert_eq!(
        fk_enabled, 1,
        "PRAGMA foreign_keys should be ON (1) after open_connection"
    );
}

// T-15.11: RTM FR-11 — every new connection gets foreign_keys=ON
#[test]
fn t15_11_foreign_keys_enabled_on_second_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Open another connection to the same DB. Note: isolated_env() already calls
    // GraphDatabase::initialize_database, which internally calls open_connection()
    // once. This "second" connection is therefore at least the second handle opened
    // against this database. The important property: every call to open_connection()
    // sets foreign_keys=ON, not just the first.
    let conn2 = open_connection(&env.cfg.database_path).expect("open db second time");

    let fk_enabled: i64 = conn2
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .expect("read foreign_keys pragma on second connection");

    assert_eq!(
        fk_enabled, 1,
        "PRAGMA foreign_keys should be ON for every connection opened via open_connection"
    );
}

// T-15.12: RTM FR-11 — co_occurrence_count column in concept_graph is INTEGER type
#[test]
fn t15_12_co_occurrence_count_column_is_integer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_connection(&env.cfg.database_path).expect("open db");

    // Use PRAGMA table_info to introspect the concept_graph schema
    let col_type: String = conn
        .query_row(
            "SELECT type FROM pragma_table_info('concept_graph') WHERE name = 'co_occurrence_count'",
            [],
            |row| row.get(0),
        )
        .expect("should find co_occurrence_count column in concept_graph");

    assert_eq!(
        col_type.to_uppercase(),
        "INTEGER",
        "co_occurrence_count column type should be INTEGER, not REAL"
    );
}

// T-15.11 + T-15.12: RTM FR-11 — CASCADE delete works with foreign_keys=ON
// (Exercises FK enforcement on both concept_a and concept_b columns in concept_graph)
#[test]
fn t15_11_cascade_delete_works_with_foreign_keys_on() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let conn = open_connection(&env.cfg.database_path).expect("open db");

    // Verify foreign_keys is actually ON
    let fk_enabled: i64 = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .expect("read foreign_keys");
    assert_eq!(fk_enabled, 1, "precondition: foreign_keys should be ON");

    // Insert a concept
    conn.execute(
        "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)",
        params!["test-concept", "2024-01-01T00:00:00Z", 1],
    )
    .expect("insert concept");

    // Insert a mention referencing the concept (FK → concepts.concept_name)
    conn.execute(
        "INSERT INTO concept_mentions (concept_name, source_file, mention_count) VALUES (?1, ?2, ?3)",
        params!["test-concept", "memory://test/file.md", 3],
    )
    .expect("insert mention");

    // Insert a graph edge referencing the concept (FK → concepts.concept_name)
    conn.execute(
        "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)",
        params!["other-concept", "2024-01-01T00:00:00Z", 1],
    )
    .expect("insert other concept");
    conn.execute(
        "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
        params!["other-concept", "test-concept", 1, r#"{"memory://test/file.md":1.0}"#],
    )
    .expect("insert graph edge (concept_b = test-concept)");

    // Insert a symmetrical edge with test-concept in concept_a position.
    // This pins FK cascade behaviour on BOTH concept_a and concept_b columns:
    // the first edge above only exercises the concept_b FK; this edge exercises concept_a.
    conn.execute(
        "INSERT INTO concepts (concept_name, first_seen, occurrence_count) VALUES (?1, ?2, ?3)",
        params!["another-concept", "2024-01-01T00:00:00Z", 1],
    )
    .expect("insert another concept");
    conn.execute(
        "INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) VALUES (?1, ?2, ?3, ?4)",
        params!["test-concept", "another-concept", 1, r#"{"memory://test/file2.md":1.0}"#],
    )
    .expect("insert graph edge (concept_a = test-concept)");

    // Insert a community assignment referencing the concept
    conn.execute(
        "INSERT INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["test-concept", 1, 0.5, 1.0, "2024-01-01T00:00:00Z"],
    )
    .expect("insert community assignment");

    // Verify rows exist before cascade
    let mention_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_mentions WHERE concept_name = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count mentions before delete");
    assert_eq!(mention_count, 1, "precondition: mention should exist");

    let edge_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_graph WHERE concept_a = 'test-concept' OR concept_b = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count edges before delete");
    assert_eq!(edge_count, 2, "precondition: both graph edges should exist");

    let community_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_communities WHERE concept_name = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count communities before delete");
    assert_eq!(
        community_count, 1,
        "precondition: community assignment should exist"
    );

    // DELETE the parent concept — should CASCADE to mentions, edges, and communities
    conn.execute(
        "DELETE FROM concepts WHERE concept_name = 'test-concept'",
        [],
    )
    .expect("delete concept");

    // Verify CASCADE: mentions should be gone
    let mention_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_mentions WHERE concept_name = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count mentions after cascade");
    assert_eq!(
        mention_count_after, 0,
        "CASCADE should delete concept_mentions when parent concept is deleted"
    );

    // Verify CASCADE: graph edges should be gone
    let edge_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_graph WHERE concept_a = 'test-concept' OR concept_b = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count edges after cascade");
    assert_eq!(
        edge_count_after, 0,
        "CASCADE should delete concept_graph edges when parent concept is deleted \
         (covers both concept_a and concept_b FK columns)"
    );

    // Verify CASCADE: community assignments should be gone
    let community_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concept_communities WHERE concept_name = 'test-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count communities after cascade");
    assert_eq!(
        community_count_after, 0,
        "CASCADE should delete concept_communities when parent concept is deleted"
    );

    // Verify the other concepts still exist (not cascaded)
    let other_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concepts WHERE concept_name = 'other-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count other concept");
    assert_eq!(
        other_exists, 1,
        "other-concept should not be affected by cascade delete of test-concept"
    );

    let another_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM concepts WHERE concept_name = 'another-concept'",
            [],
            |row| row.get(0),
        )
        .expect("count another concept");
    assert_eq!(
        another_exists, 1,
        "another-concept should not be affected by cascade delete of test-concept"
    );
}

// ---------------------------------------------------------------------------
// T-15.22: RTM FR-11 — journal_mode=WAL set on every new connection
// ---------------------------------------------------------------------------

#[test]
fn t15_22_journal_mode_wal_on_new_connection() {
    let _guard = TEST_MUTEX.lock();

    // T-15.22: In CI (GitHub Actions), open_connection() intentionally sets
    // journal_mode=DELETE instead of WAL because CI tmpfs has WAL issues.
    // This test only validates the non-CI (production) code path.
    if is_ci() {
        return;
    }

    let env = isolated_env();

    let conn = open_connection(&env.cfg.database_path).expect("open db");
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("read journal_mode pragma");

    assert_eq!(
        mode.to_lowercase(),
        "wal",
        "PRAGMA journal_mode should be WAL on every connection opened via open_connection"
    );
}

#[test]
fn t15_22_synchronous_normal_on_new_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let conn = open_connection(&env.cfg.database_path).expect("open db");
    let sync_val: i64 = conn
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .expect("read synchronous pragma");

    // synchronous=NORMAL is 1
    assert_eq!(
        sync_val, 1,
        "PRAGMA synchronous should be NORMAL (1) on every connection"
    );
}

// T-10.9: RTM FR-15 — busy_timeout is set on every connection to avoid SQLITE_BUSY
// errors under WAL concurrency. Default: 5000 ms (configurable via MAENIFOLD_SQLITE_BUSY_TIMEOUT).
#[test]
fn t10_9_busy_timeout_set_on_new_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let conn = open_connection(&env.cfg.database_path).expect("open db");
    let timeout: i64 = conn
        .pragma_query_value(None, "busy_timeout", |row| row.get(0))
        .expect("read busy_timeout pragma");

    // Default busy_timeout is 5000 ms. The Config default is used unless
    // MAENIFOLD_SQLITE_BUSY_TIMEOUT is set in the environment.
    assert!(
        timeout > 0,
        "busy_timeout should be set to a positive value on every connection, got {timeout}"
    );
    assert_eq!(
        timeout, 5000,
        "Default busy_timeout should be 5000 ms (configurable via MAENIFOLD_SQLITE_BUSY_TIMEOUT)"
    );
}

// T-15.22: When a database is in DELETE mode (e.g. set by another tool or an
// older SQLite version's VACUUM), open_connection must restore WAL mode.
// Modern SQLite (3.22+) preserves journal_mode across VACUUM, but the database
// file could still end up in DELETE mode via external tools or manual pragma.
#[test]
fn t15_22_journal_mode_wal_after_vacuum() {
    let _guard = TEST_MUTEX.lock();

    // T-15.22: In CI (GitHub Actions), open_connection() intentionally sets
    // journal_mode=DELETE. Restoring WAL from DELETE is the non-CI code path;
    // this test is skipped in CI for the same reason as the basic WAL test.
    if is_ci() {
        return;
    }

    let env = isolated_env();

    // Force the database into DELETE mode using a raw connection, simulating what
    // an older SQLite VACUUM or external tool might do.
    {
        let raw = rusqlite::Connection::open(&env.cfg.database_path).expect("raw open");
        raw.pragma_update(None, "journal_mode", "DELETE")
            .expect("set DELETE mode");
        let mode: String = raw
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("read journal_mode");
        assert_eq!(
            mode.to_lowercase(),
            "delete",
            "precondition: database should be in DELETE mode"
        );
    }

    // Verify a raw connection still sees DELETE mode
    {
        let raw2 = rusqlite::Connection::open(&env.cfg.database_path).expect("raw reopen");
        let mode: String = raw2
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("read journal_mode on raw reopen");
        assert_eq!(
            mode.to_lowercase(),
            "delete",
            "precondition: raw reopen should still see DELETE mode"
        );
    }

    // Now open via our API — it should restore WAL
    let conn = open_connection(&env.cfg.database_path).expect("open after DELETE");
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("read journal_mode after open_connection");

    assert_eq!(
        mode.to_lowercase(),
        "wal",
        "open_connection must restore journal_mode=WAL even when database was in DELETE mode"
    );
}

// ---------------------------------------------------------------------------
// T-15.23: RTM FR-11 — No read/write distinction (intentional simplification)
// ---------------------------------------------------------------------------

// This test documents that query_only is NOT set, confirming all connections are
// read-write. If a read-only path is added later, add a test for query_only=ON.
// Diagnostic anchor: if this is the first failure you see, open_connection is
// setting query_only=ON unexpectedly. Most write-path tests would also fail, but
// this test pinpoints the specific pragma.
#[test]
fn t15_23_no_query_only_on_standard_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let conn = open_connection(&env.cfg.database_path).expect("open db");
    let qo: i64 = conn
        .pragma_query_value(None, "query_only", |row| row.get(0))
        .expect("read query_only pragma");

    assert_eq!(
        qo, 0,
        "Standard connections should NOT have query_only=ON (intentional simplification — \
         Rust uses a single connection type; C# distinguishes OpenReadOnly vs OpenWithWAL)"
    );
}
