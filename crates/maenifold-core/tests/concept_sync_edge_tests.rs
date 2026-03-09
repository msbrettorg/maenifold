use maenifold_core::db;
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::from_str;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, content).expect("write fixture");
}

fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), maenifold_core::json::OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}

fn file_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

fn db_connection(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("db open")
}

fn set_last_indexed(cfg: &Config, file_uri: &str, value: &str) {
    db_connection(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, value],
        )
        .expect("set last_indexed");
}

fn query_edge_rowid(cfg: &Config, a: &str, b: &str) -> Option<i64> {
    let (a, b) = normalize_edge(a, b);
    db_connection(cfg)
        .query_row(
            "SELECT rowid FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![a, b],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("query rowid")
}

fn query_edge(cfg: &Config, a: &str, b: &str) -> Option<(f64, BTreeMap<String, f64>)> {
    let (a, b) = normalize_edge(a, b);
    db_connection(cfg)
        .query_row(
            "SELECT COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') \
             FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![a, b],
            |row| {
                let count = row.get::<_, f64>(0)?;
                let raw_files = row.get::<_, String>(1)?;
                let files = if raw_files.trim().is_empty() {
                    BTreeMap::new()
                } else {
                    from_str::<BTreeMap<String, f64>>(&raw_files).expect("parse source_files")
                };
                Ok((count, files))
            },
        )
        .optional()
        .expect("query edge")
}

fn normalize_edge<'a>(x: &'a str, y: &'a str) -> (&'a str, &'a str) {
    if x <= y { (x, y) } else { (y, x) }
}

fn initial_sectioned_content() -> &'static str {
    "# Test\n\n## Section One\n\n[[alpha]] [[beta]]\n\n## Section Two\n\n[[gamma]]\n"
}

// T-5.7: RTM FR-14.3
#[test]
fn identical_resync_preserves_existing_edge_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-a.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(&cfg, relative_path, initial_sectioned_content());

    let first_sync = sync_one(&cfg, relative_path);
    assert!(
        first_sync.contains("Synced memory://concept-sync/file-a"),
        "initial sync should succeed: {first_sync}"
    );

    let rowids_before = BTreeMap::from([
        (
            ("alpha".to_string(), "beta".to_string()),
            query_edge_rowid(&cfg, "alpha", "beta").expect("alpha-beta rowid"),
        ),
        (
            ("alpha".to_string(), "gamma".to_string()),
            query_edge_rowid(&cfg, "alpha", "gamma").expect("alpha-gamma rowid"),
        ),
        (
            ("beta".to_string(), "gamma".to_string()),
            query_edge_rowid(&cfg, "beta", "gamma").expect("beta-gamma rowid"),
        ),
    ]);

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]] [[beta]]\n\nExtra prose without new wikilinks.\n\n## Section Two\n\n[[gamma]]\n",
    );

    let resync = sync_one(&cfg, relative_path);
    assert!(
        resync.contains("concept mentions updated"),
        "resync should process file content changes: {resync}"
    );

    for ((left, right), rowid_before) in rowids_before {
        assert_eq!(
            query_edge_rowid(&cfg, &left, &right),
            Some(rowid_before),
            "re-syncing unchanged concept sections must not rewrite {left}-{right}"
        );
    }
}

// T-5.7: RTM FR-14.3
#[test]
fn section_change_rewrites_only_affected_edges() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-b.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(&cfg, relative_path, initial_sectioned_content());
    sync_one(&cfg, relative_path);

    let alpha_gamma_rowid_before =
        query_edge_rowid(&cfg, "alpha", "gamma").expect("alpha-gamma rowid");

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Section One\n\n[[alpha]]\n\n## Section Two\n\n[[beta]] [[gamma]]\n",
    );

    let output = sync_one(&cfg, relative_path);
    assert!(
        output.contains("affected relations rebuilt"),
        "section-change sync should complete: {output}"
    );

    assert_eq!(
        query_edge_rowid(&cfg, "alpha", "gamma"),
        Some(alpha_gamma_rowid_before),
        "alpha-gamma should keep its rowid because the edge weight stayed unchanged"
    );

    let alpha_beta = query_edge(&cfg, "alpha", "beta").expect("alpha-beta edge");
    assert_eq!(
        alpha_beta.1.get(&uri).copied(),
        Some(0.5),
        "alpha-beta weight must drop to 0.5 after beta moves to section two"
    );

    let beta_gamma = query_edge(&cfg, "beta", "gamma").expect("beta-gamma edge");
    assert_eq!(
        beta_gamma.1.get(&uri).copied(),
        Some(1.0),
        "beta-gamma weight must rise to 1.0 once both concepts share section two"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn concept_removed_cleans_stale_edge_contributions() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-c.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]] [[gamma]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge")
            .1
            .contains_key(&uri)
    );
    assert!(
        query_edge(&cfg, "alpha", "gamma")
            .expect("alpha-gamma edge")
            .1
            .contains_key(&uri)
    );
    assert!(
        query_edge(&cfg, "beta", "gamma")
            .expect("beta-gamma edge")
            .1
            .contains_key(&uri)
    );

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge after removal")
            .1
            .contains_key(&uri),
        "alpha-beta should keep the file contribution after gamma is removed"
    );
    assert!(
        !query_edge(&cfg, "alpha", "gamma")
            .map(|edge| edge.1.contains_key(&uri))
            .unwrap_or(false),
        "alpha-gamma must not retain a stale contribution after gamma is removed"
    );
    assert!(
        !query_edge(&cfg, "beta", "gamma")
            .map(|edge| edge.1.contains_key(&uri))
            .unwrap_or(false),
        "beta-gamma must not retain a stale contribution after gamma is removed"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn new_concept_added_creates_new_edge_rows() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-d.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(query_edge(&cfg, "alpha", "beta").is_some());
    assert!(query_edge(&cfg, "alpha", "gamma").is_none());
    assert!(query_edge(&cfg, "beta", "gamma").is_none());

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]] [[gamma]]\n",
    );
    sync_one(&cfg, relative_path);

    let alpha_gamma = query_edge(&cfg, "alpha", "gamma").expect("alpha-gamma edge");
    let beta_gamma = query_edge(&cfg, "beta", "gamma").expect("beta-gamma edge");
    assert!(
        alpha_gamma.1.contains_key(&uri),
        "alpha-gamma should reference the contributing file once gamma is added"
    );
    assert!(
        beta_gamma.1.contains_key(&uri),
        "beta-gamma should reference the contributing file once gamma is added"
    );
    assert!(
        query_edge(&cfg, "alpha", "beta")
            .expect("alpha-beta edge")
            .1
            .contains_key(&uri),
        "existing alpha-beta edge should be preserved"
    );
}

// T-5.7: RTM FR-14.3
#[test]
fn last_contributor_removed_deletes_edge_row() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let relative_path = "concept-sync/file-e.md";
    let uri = file_uri(relative_path);
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]] [[beta]]\n",
    );
    sync_one(&cfg, relative_path);
    assert!(query_edge(&cfg, "alpha", "beta").is_some());

    set_last_indexed(&cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &cfg,
        relative_path,
        "# Test\n\n## Shared Section\n\n[[alpha]]\n",
    );
    sync_one(&cfg, relative_path);

    assert!(
        query_edge(&cfg, "alpha", "beta").is_none(),
        "alpha-beta row should be deleted when no files contribute the pair anymore"
    );
}
