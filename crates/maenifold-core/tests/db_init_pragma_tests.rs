// T-15.11 + T-15.12: RTM FR-11 — DB initialization fixes
//
// T-15.11: Verify PRAGMA foreign_keys=ON is set on every connection
// T-15.12: Verify co_occurrence_count column type is INTEGER (not REAL)

use maenifold_core::db::open_connection;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
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
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
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

    // Open a second connection to the same DB — should also have foreign_keys=ON
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

// T-15.11: RTM FR-11 — CASCADE delete works with foreign_keys=ON
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
    .expect("insert graph edge");

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
    assert_eq!(edge_count, 1, "precondition: edge should exist");

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
        "CASCADE should delete concept_graph edges when parent concept is deleted"
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

    // Verify the other concept still exists (not cascaded)
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
}

// ---------------------------------------------------------------------------
// T-15.22: RTM FR-11 — journal_mode=WAL set on every new connection
// ---------------------------------------------------------------------------

#[test]
fn t15_22_journal_mode_wal_on_new_connection() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let conn = open_connection(&env.cfg.database_path).expect("open db");
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .expect("read journal_mode pragma");

    // Outside CI, journal_mode should be "wal" (SQLite returns lowercase)
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

// T-15.22: When a database is in DELETE mode (e.g. set by another tool or an
// older SQLite version's VACUUM), open_connection must restore WAL mode.
// Modern SQLite (3.22+) preserves journal_mode across VACUUM, but the database
// file could still end up in DELETE mode via external tools or manual pragma.
#[test]
fn t15_22_journal_mode_wal_after_vacuum() {
    let _guard = TEST_MUTEX.lock();
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
