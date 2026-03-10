use chrono::Utc;
use maenifold_core::community::{compute_edge_decay_weight, compute_nmi, detect_and_persist};
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::extract_wikilinks_with_sections;
use maenifold_core::{AssetManager, Config, GraphDatabase, ToolRegistry};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

// T-4.3: RTM FR-23.x
struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn isolated() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&cfg).expect("assets");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self { _tmp: tmp, cfg }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-4.3: RTM FR-23.7
struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
        // so process-wide environment mutation is serialized.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::set_var(self.key, value);
                }
            }
            None => {
                // SAFETY: all tests in this file hold TEST_MUTEX for their full duration,
                // so process-wide environment mutation is serialized.
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }
}

fn open_db(cfg: &Config) -> Connection {
    let connection = Connection::open(&cfg.database_path).expect("open db");
    connection
        .execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .expect("pragmas");
    connection
}

fn write_memory_file(cfg: &Config, relative_path: &str, content: &str) -> String {
    let file_path = cfg.memory_path.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
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

fn invoke_build_context_json(concept_name: &str) -> Value {
    let raw = ToolRegistry::global()
        .try_invoke(
            "BuildContext",
            &json!({ "conceptName": concept_name, "depth": 1, "maxEntities": 20 }),
            OutputFormat::Json,
        )
        .expect("invoke build context");
    serde_json::from_str(&raw).expect("build context json")
}

fn insert_concept(conn: &Connection, name: &str) {
    conn.execute(
        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?1, '2026-01-01')",
        params![name],
    )
    .expect("insert concept");
}

fn insert_edge_with_proximity(conn: &Connection, a: &str, b: &str, uri: &str, weight: f64) {
    let (left, right) = if a < b { (a, b) } else { (b, a) };
    let source_files = serde_json::to_string(&json!({ uri: weight })).expect("source_files json");
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
         VALUES (?1, ?2, 1, ?3)",
        params![left, right, source_files],
    )
    .expect("insert edge");
}

fn insert_edge_raw_source_files(
    conn: &Connection,
    a: &str,
    b: &str,
    co_occurrence_count: f64,
    source_files_json: &str,
) {
    let (left, right) = if a < b { (a, b) } else { (b, a) };
    conn.execute(
        "INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files) \
         VALUES (?1, ?2, ?3, ?4)",
        params![left, right, co_occurrence_count, source_files_json],
    )
    .expect("insert raw edge");
}

fn relation_source_files(conn: &Connection, concept_a: &str, concept_b: &str) -> Value {
    let raw: String = conn
        .query_row(
            "SELECT source_files FROM concept_graph WHERE (concept_a = ?1 AND concept_b = ?2) OR (concept_a = ?2 AND concept_b = ?1)",
            params![concept_a, concept_b],
            |row| row.get(0),
        )
        .expect("relation source_files");
    serde_json::from_str(&raw).expect("source_files should be valid json")
}

fn relation_weight_for_uri(conn: &Connection, concept_a: &str, concept_b: &str, uri: &str) -> f64 {
    relation_source_files(conn, concept_a, concept_b)[uri]
        .as_f64()
        .expect("relation weight should be numeric")
}

fn relation_count(conn: &Connection, concept_a: &str, concept_b: &str) -> Option<f64> {
    conn.query_row(
        "SELECT co_occurrence_count FROM concept_graph WHERE (concept_a = ?1 AND concept_b = ?2) OR (concept_a = ?2 AND concept_b = ?1)",
        params![concept_a, concept_b],
        |row| row.get(0),
    )
    .optional()
    .expect("read relation count")
}

fn relation_exists(conn: &Connection, concept_a: &str, concept_b: &str) -> bool {
    relation_count(conn, concept_a, concept_b).is_some()
}

fn direct_relation<'a>(payload: &'a Value, name: &str) -> &'a Value {
    payload["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations array")
        .iter()
        .find(|relation| relation["Name"].as_str() == Some(name))
        .expect("relation should exist")
}

fn community_id(conn: &Connection, concept_name: &str) -> i64 {
    conn.query_row(
        "SELECT community_id FROM concept_communities WHERE concept_name = ?1",
        params![concept_name],
        |row| row.get(0),
    )
    .expect("community id")
}

fn assert_approx_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9,
        "expected {expected:.10}, got {actual:.10}"
    );
}

#[cfg(unix)]
fn capture_stderr<F, T>(operation: F) -> (T, String)
where
    F: FnOnce() -> T,
{
    use std::fs::File;
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd};

    unsafe extern "C" {
        fn pipe(fds: *mut i32) -> i32;
        fn dup(fd: i32) -> i32;
        fn dup2(src: i32, dest: i32) -> i32;
        fn close(fd: i32) -> i32;
    }

    let stderr = std::io::stderr();
    let stderr_fd = stderr.as_raw_fd();
    let mut pipe_fds = [0_i32; 2];

    // SAFETY: pipe_fds points to valid writable memory for two file descriptors.
    let pipe_result = unsafe { pipe(pipe_fds.as_mut_ptr()) };
    assert_eq!(pipe_result, 0, "pipe failed");

    // SAFETY: stderr_fd is a valid process file descriptor.
    let saved_stderr = unsafe { dup(stderr_fd) };
    assert!(saved_stderr >= 0, "dup failed");

    // SAFETY: both descriptors are valid and dup2 atomically redirects stderr.
    let redirect_result = unsafe { dup2(pipe_fds[1], stderr_fd) };
    assert!(redirect_result >= 0, "dup2 failed");

    let result = operation();
    std::io::stderr().flush().expect("flush stderr");

    // SAFETY: restore original stderr and close the write end once no more writes are expected.
    assert!(
        unsafe { dup2(saved_stderr, stderr_fd) } >= 0,
        "restore dup2 failed"
    );
    unsafe {
        close(pipe_fds[1]);
        close(saved_stderr);
    }

    let mut output = String::new();
    // SAFETY: pipe_fds[0] is the remaining owned read-end descriptor.
    let mut reader = unsafe { File::from_raw_fd(pipe_fds[0]) };
    reader
        .read_to_string(&mut output)
        .expect("read captured stderr");

    (result, output)
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_tags_multi_section_documents() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Document Title\n\n## Section One\n\nDiscusses [[alpha]] and [[beta]].\n\n## Section Two\n\nIntroduces [[gamma]] and [[delta]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(lookup.get("alpha"), Some(&1));
    assert_eq!(lookup.get("beta"), Some(&1));
    assert_eq!(lookup.get("gamma"), Some(&2));
    assert_eq!(lookup.get("delta"), Some(&2));
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_uses_section_zero_for_flat_docs() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Just a Title\n\nThis note references [[concepta]], [[conceptb]], and [[conceptc]].\nNo H2 headings anywhere.\n";
    let concepts = extract_wikilinks_with_sections(content);

    assert!(!concepts.is_empty(), "flat doc should extract concepts");
    assert!(
        concepts
            .iter()
            .all(|(_, section_index)| *section_index == 0),
        "every flat-doc concept must remain in section zero: {concepts:?}"
    );
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_keeps_preamble_in_section_zero() {
    let _guard = TEST_MUTEX.lock();

    let content = "# Document\n\nThis preamble mentions [[preamble-concept]] before any heading.\n\n## First Section\n\nThis section has [[section-concept]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let lookup = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(lookup.get("preamble-concept"), Some(&0));
    assert_eq!(lookup.get("section-concept"), Some(&1));
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_deduplicates_per_section_not_globally() {
    let _guard = TEST_MUTEX.lock();

    let content = "## Section One\n\nReferences [[shared-concept]] here.\n\n## Section Two\n\nAlso references [[shared-concept]] here and again [[shared-concept]].\n\n## Section Three\n\nOnly [[unique-concept]] lives here.\n";
    let concepts = extract_wikilinks_with_sections(content);
    let shared_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "shared-concept")
        .map(|(_, section_index)| *section_index)
        .collect::<Vec<_>>();
    let unique_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "unique-concept")
        .map(|(_, section_index)| *section_index)
        .collect::<Vec<_>>();

    assert_eq!(shared_sections, vec![1, 2]);
    assert_eq!(unique_sections, vec![3]);
}

// T-4.3: RTM FR-23.1
#[test]
fn extract_wikilinks_with_sections_reads_wikilinks_inside_h2_headings() {
    let _guard = TEST_MUTEX.lock();

    let content = "## Section about [[heading-concept]]\n\nBody text with [[body-concept]].\n\n## Another [[second-heading]] section\n\nMore body with [[second-body]].\n";
    let concepts = extract_wikilinks_with_sections(content);
    let heading_sections = concepts.into_iter().collect::<BTreeMap<_, _>>();

    assert_eq!(heading_sections.get("heading-concept"), Some(&1));
    assert_eq!(heading_sections.get("body-concept"), Some(&1));
    assert_eq!(heading_sections.get("second-heading"), Some(&2));
    assert_eq!(heading_sections.get("second-body"), Some(&2));
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_same_section_pairs_store_weight_one() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/same-section.md",
        "# Same Section\n\n## Identity\nPair [[alpha]] with [[beta]] in the same section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 1.0);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_cross_section_pairs_store_weight_half() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/cross-section.md",
        "# Cross Section\n\n## Identity\nFirst [[alpha]] appears here.\n\n## Sessions\nThen [[beta]] appears in a different section.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "alpha", "beta", &uri), 0.5);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_flat_docs_store_half_weight_for_all_pairs() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/flat-doc.md",
        "# No H2\n\nInline concepts [[x]], [[y]], and [[z]] all live in the preamble.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "x", "y", &uri), 0.5);
    assert_approx_eq(relation_weight_for_uri(&conn, "x", "z", &uri), 0.5);
    assert_approx_eq(relation_weight_for_uri(&conn, "y", "z", &uri), 0.5);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_prefers_named_section_overlap_over_preamble_overlap() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/preamble-and-section.md",
        "# Mixed\n\nPreamble mention [[multi]].\n\n## First Section\n\nSection mentions [[multi]] and [[peer]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert_approx_eq(relation_weight_for_uri(&conn, "multi", "peer", &uri), 1.0);
}

// T-4.3: RTM FR-23.2
#[test]
fn sync_serializes_source_files_as_weight_dictionary() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let uri = write_memory_file(
        &env.cfg,
        "projects/dict-format.md",
        "# Dict\n\n## Identity\nThis file links [[alpha]] and [[beta]].\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");

    assert!(
        source_files.is_object(),
        "source_files must be a json object: {source_files}"
    );
    assert_approx_eq(source_files[&uri].as_f64().expect("dictionary weight"), 1.0);
}

// T-4.3: RTM FR-23.4
#[test]
fn compute_edge_decay_weight_returns_proximity_for_fresh_file() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let relative_path = "projects/prox-decay-test.md";
    let file_path = env.cfg.memory_path.join(relative_path);
    let created_utc = Utc::now().to_rfc3339();
    fs::create_dir_all(file_path.parent().expect("parent")).expect("create parent dirs");
    fs::write(
        &file_path,
        format!(
            "---\ntitle: Proximity Decay Test\ncreated: {created_utc}\n---\n\nContent with [[test-concept]].\n"
        ),
    )
    .expect("write proximity decay file");

    let uri = memory_uri_from_relative(relative_path);
    let effective_weight = compute_edge_decay_weight(&json!({ uri: 0.5 }).to_string())
        .expect("compute edge decay weight");

    assert!(effective_weight > 0.0, "weight should remain positive");
    assert!(
        effective_weight <= 0.5 + 1e-6,
        "weight should not exceed proximity"
    );
    assert!(
        effective_weight > 0.4,
        "fresh file weight should remain close to proximity"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn compute_edge_decay_weight_rejects_corrupt_source_files_json() {
    let _guard = TEST_MUTEX.lock();

    let result = compute_edge_decay_weight("this is not valid json {{{{");
    assert!(result.is_err(), "corrupt source_files must fail");
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_uses_proximity_weight_in_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "prox-query");
    insert_concept(&conn, "prox-strong");
    insert_concept(&conn, "prox-weak");

    let uri = "memory://test/prox-buildcontext";
    insert_edge_with_proximity(&conn, "prox-query", "prox-strong", uri, 1.0);
    insert_edge_with_proximity(&conn, "prox-query", "prox-weak", uri, 0.5);
    drop(conn);

    let payload = invoke_build_context_json("prox-query");
    let strong = direct_relation(&payload, "prox-strong");
    let weak = direct_relation(&payload, "prox-weak");

    assert!(
        strong["DecayWeight"].as_f64().expect("strong decay")
            > weak["DecayWeight"].as_f64().expect("weak decay"),
        "strong proximity relation must have higher DecayWeight: {payload}"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_ranks_stronger_proximity_above_weaker_proximity() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "rank-query");
    insert_concept(&conn, "rank-strong");
    insert_concept(&conn, "rank-weak");

    let uri = "memory://test/prox-ranking";
    insert_edge_with_proximity(&conn, "rank-query", "rank-strong", uri, 1.0);
    insert_edge_with_proximity(&conn, "rank-query", "rank-weak", uri, 0.5);
    drop(conn);

    let payload = invoke_build_context_json("rank-query");
    let strong = direct_relation(&payload, "rank-strong");
    let weak = direct_relation(&payload, "rank-weak");

    assert!(
        strong["WeightedScore"].as_f64().expect("strong score")
            > weak["WeightedScore"].as_f64().expect("weak score"),
        "strong proximity relation must have higher WeightedScore: {payload}"
    );
}

// T-4.3: RTM FR-23.4
#[test]
fn build_context_treats_empty_source_files_as_zero_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "empty-query");
    insert_concept(&conn, "empty-peer");
    insert_edge_raw_source_files(&conn, "empty-query", "empty-peer", 1.0, "{}");
    drop(conn);

    let payload = invoke_build_context_json("empty-query");
    let peer = direct_relation(&payload, "empty-peer");

    assert_approx_eq(peer["DecayWeight"].as_f64().expect("decay weight"), 0.0);
}

// T-4.3: RTM FR-23.5
#[test]
fn sync_removes_edge_when_last_source_file_is_deleted() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let relative = "tests/to-remove.md";
    write_memory_file(
        &env.cfg,
        relative,
        "# Remove\n\n## Identity\n[[alpha]] and [[beta]] live together.\n",
    );
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert!(
        relation_exists(&conn, "alpha", "beta"),
        "precondition: edge must exist"
    );
    drop(conn);

    fs::remove_file(env.cfg.memory_path.join(relative)).expect("remove memory file");
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    assert!(
        !relation_exists(&conn, "alpha", "beta"),
        "edge must disappear when its last backing file is removed"
    );
}

// T-4.3: RTM FR-23.5
#[test]
fn sync_preserves_remaining_source_file_weights_after_partial_delete() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();

    let remove_relative = "tests/remove-me.md";
    let keep_relative = "tests/keep-me.md";
    let remove_uri = write_memory_file(
        &env.cfg,
        remove_relative,
        "# Remove\n\n## Identity\n[[alpha]] and [[beta]] share a named section.\n",
    );
    let keep_uri = write_memory_file(
        &env.cfg,
        keep_relative,
        "# Keep\n\n## Identity\n[[alpha]] and [[beta]] stay together here.\n",
    );

    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    fs::remove_file(env.cfg.memory_path.join(remove_relative)).expect("remove one file");
    let sync = invoke_sync();
    assert!(
        sync.contains("Sync complete:"),
        "unexpected sync output: {sync}"
    );

    let conn = open_db(&env.cfg);
    let source_files = relation_source_files(&conn, "alpha", "beta");
    assert!(
        source_files.is_object(),
        "source_files must remain a json object"
    );
    assert!(
        source_files.get(&remove_uri).is_none(),
        "removed uri must disappear"
    );
    assert_approx_eq(source_files[&keep_uri].as_f64().expect("kept weight"), 1.0);
    assert_approx_eq(relation_count(&conn, "alpha", "beta").expect("count"), 1.0);
}

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_clusters_hubs_together_when_proximity_sum_is_strongest() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    let strong_edge = json!({ "memory://test/prox-h-heavy.md": 100.0 }).to_string();
    let satellite_edge = json!({ "memory://test/prox-h-sat.md": 1.0 }).to_string();

    insert_concept(&conn, "prox-h-a");
    insert_concept(&conn, "prox-h-b");
    insert_edge_raw_source_files(&conn, "prox-h-a", "prox-h-b", 1.0, &strong_edge);

    for name in ["prox-h-s1", "prox-h-s2", "prox-h-s3"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-a", name, 1.0, &satellite_edge);
    }
    for name in ["prox-h-s4", "prox-h-s5", "prox-h-s6"] {
        insert_concept(&conn, name);
        insert_edge_raw_source_files(&conn, "prox-h-b", name, 1.0, &satellite_edge);
    }
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 0.5).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_eq!(
        community_id(&conn, "prox-h-a"),
        community_id(&conn, "prox-h-b")
    );
}

// T-4.3: RTM FR-23.6
#[test]
fn detect_and_persist_treats_empty_source_files_as_isolated_edges() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    insert_concept(&conn, "prox-anchor-a");
    insert_concept(&conn, "prox-anchor-b");
    insert_edge_with_proximity(
        &conn,
        "prox-anchor-a",
        "prox-anchor-b",
        "memory://test/anchor.md",
        1.0,
    );

    insert_concept(&conn, "prox-isolated-x");
    insert_concept(&conn, "prox-isolated-y");
    insert_edge_raw_source_files(&conn, "prox-isolated-x", "prox-isolated-y", 5.0, "{}");
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 1.0).expect("detect communities");
    let conn = open_db(&env.cfg);
    assert_ne!(
        community_id(&conn, "prox-isolated-x"),
        community_id(&conn, "prox-isolated-y"),
        "empty source_files must produce an isolated pair"
    );
}

// T-4.3: RTM FR-23.7
#[test]
fn compute_nmi_returns_one_for_identical_clusterings() {
    let _guard = TEST_MUTEX.lock();

    let clustering = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 0_usize),
        ("gamma".to_string(), 1_usize),
        ("delta".to_string(), 1_usize),
    ]);

    assert_approx_eq(compute_nmi(&clustering, &clustering), 1.0);
}

// T-4.3: RTM FR-23.7
#[test]
fn compute_nmi_returns_value_below_one_for_different_clusterings() {
    let _guard = TEST_MUTEX.lock();

    let clustering_a = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 0_usize),
        ("gamma".to_string(), 1_usize),
        ("delta".to_string(), 1_usize),
    ]);
    let clustering_b = BTreeMap::from([
        ("alpha".to_string(), 0_usize),
        ("beta".to_string(), 1_usize),
        ("gamma".to_string(), 0_usize),
        ("delta".to_string(), 1_usize),
    ]);

    let nmi = compute_nmi(&clustering_a, &clustering_b);
    assert!(
        nmi < 1.0,
        "different partitions must produce NMI < 1.0, got {nmi}"
    );
    assert!(
        (0.0..=1.0).contains(&nmi),
        "NMI must remain normalized, got {nmi}"
    );
}

// T-4.3: RTM FR-23.7
#[test]
#[cfg(unix)]
fn detect_and_persist_logs_warning_when_community_drift_exceeds_threshold() {
    let _guard = TEST_MUTEX.lock();
    let _threshold = EnvVarGuard::set("MAENIFOLD_NMI_THRESHOLD", "0.95");
    let env = TestEnv::isolated();
    let conn = open_db(&env.cfg);

    for name in ["drift-a0", "drift-a1", "drift-a2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a1",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-a2",
        "memory://test/drift-a.md",
        1.0,
    );

    for name in ["drift-b0", "drift-b1", "drift-b2"] {
        insert_concept(&conn, name);
    }
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b1",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b1",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-b0",
        "drift-b2",
        "memory://test/drift-b.md",
        1.0,
    );
    drop(conn);

    detect_and_persist(&env.cfg.database_path, 1.0).expect("first detect");

    let conn = open_db(&env.cfg);
    conn.execute("DELETE FROM concept_graph", [])
        .expect("clear graph");
    insert_edge_with_proximity(
        &conn,
        "drift-a0",
        "drift-b0",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a1",
        "drift-b1",
        "memory://test/drift-cross.md",
        1.0,
    );
    insert_edge_with_proximity(
        &conn,
        "drift-a2",
        "drift-b2",
        "memory://test/drift-cross.md",
        1.0,
    );
    drop(conn);

    let (_, stderr_output) = capture_stderr(|| {
        detect_and_persist(&env.cfg.database_path, 1.0).expect("second detect");
    });

    assert!(
        stderr_output.contains("drift")
            || stderr_output.contains("DRIFT")
            || stderr_output.contains("NMI"),
        "expected community drift warning in stderr, got: {stderr_output}"
    );
}
