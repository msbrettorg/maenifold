use chrono::{Duration, Utc};
use maenifold_core::json::OutputFormat;
use maenifold_core::memory::compute_decay_weight;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::fs;
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

struct DecayEnvGuard {
    previous: Vec<(&'static str, Option<String>)>,
}

impl DecayEnvGuard {
    fn with_defaults() -> Self {
        let keys = [
            "MAENIFOLD_DECAY_FUNCTION",
            "MAENIFOLD_DECAY_HALF_LIFE_DAYS",
            "MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL",
            "MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",
            "MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",
        ];

        let previous = keys
            .into_iter()
            .map(|key| (key, std::env::var(key).ok()))
            .collect::<Vec<_>>();

        // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
        // so these process-wide environment mutations are serialized.
        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", "30");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", "7");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", "14");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", "28");
        }

        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so these process-wide environment mutations are serialized.
                unsafe {
                    std::env::remove_var(key);
                }
            }
        }
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

fn open_db(cfg: &Config) -> Connection {
    Connection::open(&cfg.database_path).expect("open db")
}

fn write_memory_file(cfg: &Config, relative_path: &str, content: &str) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    let parent = file_path.parent().expect("parent directory");
    fs::create_dir_all(parent).expect("create memory parent dirs");
    fs::write(&file_path, content).expect("write memory file");
    memory_uri_from_relative(relative_path)
}

fn memory_uri_from_relative(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

fn invoke_sync() -> String {
    ToolRegistry::global()
        .try_invoke("Sync", &json!({}), OutputFormat::Markdown)
        .expect("invoke sync")
}

fn invoke_search_memories_json(query: &str) -> Value {
    let raw = ToolRegistry::global()
        .try_invoke(
            "SearchMemories",
            &json!({
                "query": query,
                "mode": "Hybrid",
                "pageSize": 10,
                "page": 1
            }),
            OutputFormat::Json,
        )
        .expect("invoke search memories");
    serde_json::from_str(&raw).expect("search json")
}

fn relation_source_files(conn: &Connection, concept_a: &str, concept_b: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT source_files
             FROM concept_graph
             WHERE (concept_a = ?1 AND concept_b = ?2)
                OR (concept_a = ?2 AND concept_b = ?1)",
            params![concept_a, concept_b],
            |row| row.get(0),
        )
        .expect("relation source_files");
    serde_json::from_str(&raw).expect("source_files should be valid JSON")
}

fn relation_weight_for_uri(conn: &Connection, concept_a: &str, concept_b: &str, uri: &str) -> f64 {
    relation_source_files(conn, concept_a, concept_b)[uri]
        .as_f64()
        .expect("source_files entry should be numeric")
}

fn set_last_accessed(cfg: &Config, uri: &str, last_accessed: &str) {
    open_db(cfg)
        .execute(
            "UPDATE file_content SET last_accessed = ?1 WHERE file_path = ?2",
            params![last_accessed, uri],
        )
        .expect("update last_accessed");
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn assert_weight_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected weight {expected}, got {actual}"
    );
}

// T-4.5c: RTM FR-4.4
#[test]
fn sync_same_h2_section_relations_get_full_proximity_weight() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-same-section.md",
        "# Same Section\n\n## Identity\nPair [[alpha]] with [[beta]] in the same section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let weight = relation_weight_for_uri(&conn, "alpha", "beta", &uri);
    assert_weight_eq(weight, 1.0);
}

// T-4.5c: RTM FR-4.4
#[test]
fn sync_cross_h2_section_relations_get_half_proximity_weight() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-cross-section.md",
        "# Cross Section\n\n## Identity\nFirst [[alpha]] appears here.\n\n## Sessions\nThen [[beta]] appears in a different section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let weight = relation_weight_for_uri(&conn, "alpha", "beta", &uri);
    assert_weight_eq(weight, 0.5);
}

// T-4.5c: RTM FR-4.4
#[test]
fn sync_without_h2_headings_stores_cross_section_pair_weights() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();

    let uri = write_memory_file(
        &env.cfg,
        "projects/proximity-no-h2.md",
        "# No H2\n\nInline concepts [[alpha]], [[beta]], and [[gamma]] share the implicit root section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    // T-4.5c: RTM FR-4.4 - headingless docs still extract section zero, but sync must treat
    // the implicit root as cross-section when storing pair proximity weights.
    assert_weight_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 0.5);
    assert_weight_eq(relation_weight_for_uri(&conn, "alpha", "gamma", &uri), 0.5);
}

// T-4.5c: RTM FR-4.4
#[test]
fn sync_stores_source_files_as_json_weight_dictionary() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();

    let same_uri = write_memory_file(
        &env.cfg,
        "projects/proximity-source-files-same.md",
        "# Same\n\n## Identity\nThis file links [[alpha]] and [[beta]].\n",
    );
    let cross_uri = write_memory_file(
        &env.cfg,
        "projects/proximity-source-files-cross.md",
        "# Cross\n\n## First\nThis file mentions [[alpha]].\n\n## Second\nThis file mentions [[beta]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");
    let source_files = source_files
        .as_object()
        .expect("source_files should deserialize to a JSON object");

    assert_eq!(source_files.len(), 2, "unexpected source_files payload");
    assert_weight_eq(
        source_files[&same_uri]
            .as_f64()
            .expect("same-section weight should be numeric"),
        1.0,
    );
    assert_weight_eq(
        source_files[&cross_uri]
            .as_f64()
            .expect("cross-section weight should be numeric"),
        0.5,
    );
}

// T-4.5c: RTM FR-4.4
#[test]
fn compute_decay_weight_is_one_within_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(3);

    let weight = compute_decay_weight(
        "memory://thinking/sequential/2026/03/within-grace",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert_weight_eq(weight, 1.0);
}

// T-4.5c: RTM FR-4.4
#[test]
fn compute_decay_weight_drops_below_one_after_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(12);

    let weight = compute_decay_weight(
        "memory://thinking/sequential/2026/03/beyond-grace",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert!(
        weight < 1.0,
        "weight should decay after grace period: {weight}"
    );
}

// T-4.5c: RTM FR-4.4
#[test]
fn compute_decay_weight_uses_path_specific_grace_windows() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();
    let last_accessed = rfc3339_days_ago(10);

    let sequential = compute_decay_weight(
        "memory://thinking/sequential/2026/03/sequential-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );
    let workflow = compute_decay_weight(
        "memory://thinking/workflow/2026/03/workflow-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );
    let standard = compute_decay_weight(
        "memory://projects/default-note",
        Some(&last_accessed),
        &env.cfg,
        None,
    );

    assert!(
        sequential < 1.0,
        "sequential notes should decay after the 7-day grace period: {sequential}"
    );
    assert_weight_eq(workflow, 1.0);
    assert_weight_eq(standard, 1.0);
}

// T-4.5c: RTM FR-4.4
#[test]
fn search_memories_orders_results_by_decay_weighted_score() {
    let _guard = TEST_MUTEX.lock();
    let _decay = DecayEnvGuard::with_defaults();
    let env = isolated_env();

    let sequential_uri = write_memory_file(
        &env.cfg,
        "thinking/sequential/2026/03/sequential-ranked.md",
        "# Ranked\n\nThis note repeats ranking-target for search ordering.\n",
    );
    let default_uri = write_memory_file(
        &env.cfg,
        "projects/default-ranked.md",
        "# Ranked\n\nThis note repeats ranking-target for search ordering.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let ten_days_ago = rfc3339_days_ago(10);
    set_last_accessed(&env.cfg, &sequential_uri, &ten_days_ago);
    set_last_accessed(&env.cfg, &default_uri, &ten_days_ago);

    let response = invoke_search_memories_json("ranking-target");
    assert_eq!(
        response["success"], true,
        "search should succeed: {response}"
    );
    let results = response["data"]["results"]
        .as_array()
        .expect("results array");

    assert!(
        results.len() >= 2,
        "expected at least two ranked results: {response}"
    );
    assert_eq!(
        results[0]["path"].as_str(),
        Some(default_uri.as_str()),
        "default memory should outrank sequential memory after decay: {response}"
    );
    assert_eq!(
        results[1]["path"].as_str(),
        Some(sequential_uri.as_str()),
        "sequential memory should rank below the non-decayed result: {response}"
    );
    assert!(
        results[0]["fusedScore"].as_f64().expect("top fused score")
            > results[1]["fusedScore"]
                .as_f64()
                .expect("second fused score"),
        "decay should reduce the sequential fused score: {response}"
    );
    assert_weight_eq(
        results[0]["decayWeight"]
            .as_f64()
            .expect("top decay weight"),
        1.0,
    );
    assert!(
        results[1]["decayWeight"]
            .as_f64()
            .expect("second decay weight")
            < 1.0,
        "sequential result should carry a decayed weight: {response}"
    );
}
