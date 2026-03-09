// T-0.1: RTM NFR-3 - Ports C# SyncPerformanceTests.cs coverage for sync/search regressions.

use chrono::{DateTime, Duration, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::leader::LeaderElection;
use maenifold_core::markdown::{
    Frontmatter, extract_wikilinks_with_sections, normalize_concept, read_markdown, write_markdown,
};
use maenifold_core::memory::compute_decay_weight;
use maenifold_core::search::{self, sync_single_file};
use maenifold_core::sessions;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, MutexGuard};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;
use uuid::Uuid;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _guard: MutexGuard<'static, ()>,
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn new() -> Self {
        let guard = TEST_MUTEX.lock();
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let cfg = Config::current();
        cfg.ensure_directories().expect("dirs");
        AssetManager::initialize_assets(&cfg).expect("assets");
        GraphDatabase::initialize_database(&cfg).expect("db init");
        Self {
            _guard: guard,
            _tmp: tmp,
            cfg,
        }
    }

    fn ctx_markdown(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn ctx_json(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Json)
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct AbandonmentEnvGuard {
    previous: Option<String>,
}

impl AbandonmentEnvGuard {
    fn set(minutes: &str) -> Self {
        let previous = std::env::var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES").ok();
        // SAFETY: tests in this file serialize access with TEST_MUTEX.
        unsafe {
            std::env::set_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES", minutes);
        }
        Self { previous }
    }
}

impl Drop for AbandonmentEnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // SAFETY: tests in this file serialize access with TEST_MUTEX.
            unsafe {
                std::env::set_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES", value);
            }
        } else {
            // SAFETY: tests in this file serialize access with TEST_MUTEX.
            unsafe {
                std::env::remove_var("MAENIFOLD_SESSION_ABANDONMENT_MINUTES");
            }
        }
    }
}

fn unique_suffix() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

fn open_db(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("open db")
}

fn memory_uri(relative_path: &str) -> String {
    format!(
        "memory://{}",
        relative_path.replace('\\', "/").trim_end_matches(".md")
    )
}

fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) -> String {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write fixture");
    memory_uri(relative_path)
}

fn write_memory_with_frontmatter(
    cfg: &Config,
    relative_path: &str,
    frontmatter: &Frontmatter,
    body: &str,
) -> String {
    let uri = memory_uri(relative_path);
    write_markdown(&uri, Some(frontmatter), body, cfg).expect("write markdown");
    uri
}

fn sync_one(env: &TestEnv, relative_path: &str) -> String {
    sync_single_file(
        &env.ctx_markdown(),
        &env.cfg.memory_path.join(relative_path),
    )
    .expect("sync single file")
}

fn sync_all(env: &TestEnv) -> String {
    search::sync(&env.ctx_markdown(), &json!({})).expect("sync")
}

fn file_content_row_count(cfg: &Config) -> i64 {
    open_db(cfg)
        .query_row("SELECT COUNT(*) FROM file_content", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("row count")
}

fn fetch_string(cfg: &Config, sql: &str, file_uri: &str) -> Option<String> {
    open_db(cfg)
        .query_row(sql, [file_uri], |row| row.get::<_, String>(0))
        .optional()
        .expect("optional string")
}

fn fetch_created(cfg: &Config, file_uri: &str) -> Option<String> {
    fetch_string(
        cfg,
        "SELECT created FROM file_content WHERE file_path = ?1",
        file_uri,
    )
}

fn fetch_last_indexed(cfg: &Config, file_uri: &str) -> Option<String> {
    fetch_string(
        cfg,
        "SELECT last_indexed FROM file_content WHERE file_path = ?1",
        file_uri,
    )
}

fn fetch_file_md5(cfg: &Config, file_uri: &str) -> Option<String> {
    fetch_string(
        cfg,
        "SELECT file_md5 FROM file_content WHERE file_path = ?1",
        file_uri,
    )
}

fn set_last_indexed(cfg: &Config, file_uri: &str, value: &str) {
    open_db(cfg)
        .execute(
            "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
            params![file_uri, value],
        )
        .expect("set last_indexed");
}

fn relation_weight_for_uri(cfg: &Config, concept_a: &str, concept_b: &str, uri: &str) -> f64 {
    let (left, right) = ordered_pair(concept_a, concept_b);
    let raw = open_db(cfg)
        .query_row(
            "SELECT source_files FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, String>(0),
        )
        .expect("source_files raw");
    let parsed = serde_json::from_str::<BTreeMap<String, f64>>(&raw).expect("source_files json");
    parsed.get(uri).copied().unwrap_or(f64::NAN)
}

fn query_edge(
    cfg: &Config,
    concept_a: &str,
    concept_b: &str,
) -> Option<(f64, BTreeMap<String, f64>)> {
    let (left, right) = ordered_pair(concept_a, concept_b);
    open_db(cfg)
        .query_row(
            "SELECT COALESCE(co_occurrence_count, 0.0), COALESCE(source_files, '') FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| {
                let count = row.get::<_, f64>(0)?;
                let raw_files = row.get::<_, String>(1)?;
                let source_files = if raw_files.trim().is_empty() {
                    BTreeMap::new()
                } else {
                    serde_json::from_str::<BTreeMap<String, f64>>(&raw_files)
                        .expect("parse source_files")
                };
                Ok((count, source_files))
            },
        )
        .optional()
        .expect("query edge")
}

fn query_edge_row_count(cfg: &Config, concept_a: &str, concept_b: &str) -> i64 {
    let (left, right) = ordered_pair(concept_a, concept_b);
    open_db(cfg)
        .query_row(
            "SELECT COUNT(*) FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, i64>(0),
        )
        .expect("edge row count")
}

fn query_edge_rowid(cfg: &Config, concept_a: &str, concept_b: &str) -> Option<i64> {
    let (left, right) = ordered_pair(concept_a, concept_b);
    open_db(cfg)
        .query_row(
            "SELECT rowid FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2",
            params![left, right],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .expect("edge rowid")
}

fn ordered_pair<'a>(left: &'a str, right: &'a str) -> (&'a str, &'a str) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn visible_text_without_code(content: &str) -> String {
    let mut visible = String::new();
    let mut code_block_depth = 0usize;
    for event in Parser::new(content) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Code(_) => {}
            Event::Text(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                if code_block_depth == 0 {
                    visible.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if code_block_depth == 0 {
                    visible.push('\n');
                }
            }
            _ => {}
        }
    }
    visible
}

fn count_visible_wikilink_occurrences(content: &str, concept: &str) -> usize {
    let visible = visible_text_without_code(content);
    let mut count = 0usize;
    let bytes = visible.as_bytes();
    let target = normalize_concept(concept);
    let mut index = 0usize;

    while index + 3 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }
        let mut cursor = index + 2;
        while cursor + 1 < bytes.len() && !(bytes[cursor] == b']' && bytes[cursor + 1] == b']') {
            cursor += 1;
        }
        if cursor + 1 >= bytes.len() {
            break;
        }
        let concept_text = &visible[index + 2..cursor];
        if normalize_concept(concept_text) == target {
            count += 1;
        }
        index = cursor + 2;
    }

    count
}

fn count_all_visible_wikilink_occurrences(
    content: &str,
    concepts: &[&str],
) -> BTreeMap<String, usize> {
    let mut counts = concepts
        .iter()
        .map(|concept| ((*concept).to_string(), 0usize))
        .collect::<BTreeMap<_, _>>();
    for concept in concepts {
        counts.insert(
            (*concept).to_string(),
            count_visible_wikilink_occurrences(content, concept),
        );
    }
    counts
}

fn rfc3339_days_ago(days_ago: i64) -> String {
    (Utc::now() - Duration::days(days_ago)).to_rfc3339()
}

fn expected_decay_weight(cfg: &Config, age_days: i64) -> f64 {
    let grace_days = cfg.decay_grace_days_default();
    let half_life_days = cfg.decay_half_life_days();
    if cfg.decay_function().eq_ignore_ascii_case("power-law") {
        if age_days <= grace_days {
            return 1.0;
        }
        let effective_age = (age_days - grace_days).max(1) as f64;
        return effective_age.powf(-0.5).clamp(0.0, 1.0);
    }
    if age_days <= grace_days {
        return 1.0;
    }
    let exponent = (age_days - grace_days) as f64 / half_life_days as f64;
    0.5f64.powf(exponent).clamp(0.0, 1.0)
}

fn decay_weight_for_uri(cfg: &Config, uri: &str) -> f64 {
    let (last_accessed, status) = open_db(cfg)
        .query_row(
            "SELECT last_accessed, status FROM file_content WHERE file_path = ?1",
            params![uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .expect("query decay metadata");
    compute_decay_weight(uri, last_accessed.as_deref(), cfg, status.as_deref())
}

fn create_aged_file(
    env: &TestEnv,
    relative_path: &str,
    title: &str,
    content: &str,
    days_ago: i64,
) -> String {
    let created = rfc3339_days_ago(days_ago);
    let full = format!(
        "---\ntitle: {title}\ntype: memory\nstatus: saved\ncreated: {created}\nmodified: {created}\n---\n\n# {title}\n\n{content}\n"
    );
    write_memory_fixture(&env.cfg, relative_path, &full)
}

fn seed_session(cfg: &Config, file_uri: &str, status: &str, last_indexed: &str) {
    let body = "# Session\n\n## Thought 1\nStill active [[sync-perf-tests]]\n";
    let mut frontmatter = Frontmatter::new();
    let title = file_uri
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .expect("title segment");
    let session_type = if file_uri.contains("/workflow/") {
        "workflow"
    } else {
        "sequential"
    };
    frontmatter.insert("title".to_string(), YamlValue::String(title.to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String(status.to_string()));
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String(session_type.to_string()),
    );
    write_markdown(file_uri, Some(&frontmatter), body, cfg).expect("write session markdown");

    open_db(cfg)
        .execute(
            "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![file_uri, title, body, format!("md5-{title}"), body.len() as i64, last_indexed, status],
        )
        .expect("seed file_content");
}

fn frontmatter_status(cfg: &Config, file_uri: &str) -> String {
    let (frontmatter, _, _) = read_markdown(file_uri, cfg).expect("read markdown");
    frontmatter
        .and_then(|frontmatter| {
            frontmatter
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_string)
        })
        .expect("status")
}

fn build_context_json(env: &TestEnv, concept_name: &str) -> Value {
    let raw = search::build_context(
        &env.ctx_json(),
        &json!({
            "conceptName": concept_name,
            "depth": 1,
            "maxEntities": 20,
            "includeContent": false,
        }),
    )
    .expect("build context");
    serde_json::from_str(&raw).expect("build context json")
}

fn direct_relation<'a>(payload: &'a Value, name: &str) -> &'a Value {
    payload["data"]["DirectRelations"]
        .as_array()
        .expect("direct relations")
        .iter()
        .find(|entry| entry["Name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing direct relation {name}: {payload}"))
}

// T-0.1: RTM NFR-3
#[test]
fn baseline_process_file_timing_per_file() {
    let env = TestEnv::new();
    let mut per_file_ms = Vec::new();

    for index in 0..10 {
        let unique = unique_suffix();
        let relative_path = format!("perf-baseline/note-{index:02}-{unique}.md");
        write_memory_fixture(
            &env.cfg,
            &relative_path,
            &format!(
                "# Note {index}\n\nReferences [[concept-a-{unique}]] and [[concept-b-{unique}]].\nAlso mentions [[concept-a-{unique}]] again for count=2.\n"
            ),
        );

        let started = Instant::now();
        let output = sync_one(&env, &relative_path);
        per_file_ms.push(started.elapsed().as_millis() as u64);
        assert!(
            output.contains("Synced memory://perf-baseline/"),
            "single-file sync should succeed: {output}"
        );
    }

    per_file_ms.sort_unstable();
    let total: u64 = per_file_ms.iter().sum();
    let median = per_file_ms[per_file_ms.len() / 2];
    let max = *per_file_ms.last().expect("max sample");
    eprintln!(
        "PROCESS-FILE BASELINE files=10 total_ms={total} median_ms={median} max_ms={max} samples={per_file_ms:?}"
    );

    assert_eq!(file_content_row_count(&env.cfg), 10);
}

// T-0.1: RTM NFR-3
#[test]
fn baseline_full_sync_timing_over_corpus() {
    let env = TestEnv::new();

    for index in 0..10 {
        let unique = unique_suffix();
        let relative_path = format!("perf-sync-corpus/corpus-{index:02}-{unique}.md");
        write_memory_fixture(
            &env.cfg,
            &relative_path,
            &format!(
                "# Corpus Note {index}\n\nMentions [[sync-concept-{unique}]] and [[shared-concept]].\nSecond mention of [[sync-concept-{unique}]] for count testing.\n"
            ),
        );
    }

    let started = Instant::now();
    let output = sync_all(&env);
    let elapsed_ms = started.elapsed().as_millis();
    eprintln!("FULL-SYNC BASELINE files=10 elapsed_ms={elapsed_ms}\n{output}");

    assert!(
        output.contains("Sync complete:"),
        "unexpected sync output: {output}"
    );
    assert!(
        output.contains("files processed") || output.contains("FILES processed"),
        "sync output should report processed files: {output}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn count_concept_occurrences_golden_alpha_two_beta_one() {
    let content = "[[alpha]] and [[beta]] appear in [[alpha]] again";
    assert_eq!(count_visible_wikilink_occurrences(content, "alpha"), 2);
    assert_eq!(count_visible_wikilink_occurrences(content, "beta"), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn extract_wikilinks_with_sections_group_by_count_documents_relation_to_count_occurrences() {
    let content = "[[alpha]] and [[beta]] appear in [[alpha]] again";
    let concepts_with_sections = extract_wikilinks_with_sections(content);

    let section_counts = concepts_with_sections.into_iter().fold(
        BTreeMap::<String, usize>::new(),
        |mut counts, (concept, _section_index)| {
            *counts.entry(concept).or_default() += 1;
            counts
        },
    );

    assert_eq!(section_counts.get("alpha"), Some(&1));
    assert_eq!(section_counts.get("beta"), Some(&1));
    assert_eq!(count_visible_wikilink_occurrences(content, "alpha"), 2);
    assert_eq!(count_visible_wikilink_occurrences(content, "beta"), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn count_concept_occurrences_across_sections_counts_all_occurrences() {
    let content = "Preamble mentions [[gamma]].\n\n## Section One\n\n[[gamma]] and [[delta]] here.\n\n## Section Two\n\n[[gamma]] appears a third time.\n";
    assert_eq!(count_visible_wikilink_occurrences(content, "gamma"), 3);
    assert_eq!(count_visible_wikilink_occurrences(content, "delta"), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn extract_wikilinks_with_sections_assigns_section_indices_correctly() {
    let content = "Preamble mentions [[gamma]].\n\n## Section One\n\n[[gamma]] and [[delta]] here.\n\n## Section Two\n\n[[gamma]] appears a third time.\n";
    let concepts = extract_wikilinks_with_sections(content);

    let gamma_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "gamma")
        .map(|(_, section)| *section)
        .collect::<Vec<_>>();
    let delta_sections = concepts
        .iter()
        .filter(|(concept, _)| concept == "delta")
        .map(|(_, section)| *section)
        .collect::<Vec<_>>();

    assert_eq!(gamma_sections, vec![0, 1, 2]);
    assert_eq!(delta_sections, vec![1]);
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_creates_expected_edges_with_correct_source_files_json() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("edge-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("edge-beta-{unique}"));
    let concept_c = normalize_concept(&format!("edge-gamma-{unique}"));
    let relative_path = format!("edge-tests/edge-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Edge Test\n\n## Shared Section\n\nMentions [[{concept_a}]], [[{concept_b}]], and [[{concept_c}]].\n"
        ),
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    for (left, right) in [
        ordered_pair(concept_a.as_str(), concept_b.as_str()),
        ordered_pair(concept_a.as_str(), concept_c.as_str()),
        ordered_pair(concept_b.as_str(), concept_c.as_str()),
    ] {
        let (count, source_files) = query_edge(&env.cfg, left, right)
            .unwrap_or_else(|| panic!("missing edge ({left}, {right})"));
        assert!(
            count > 0.0,
            "edge weight should be positive for ({left}, {right})"
        );
        assert_eq!(source_files.get(&uri).copied(), Some(1.0));
    }
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_proximity_weight_same_section_vs_cross_section() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("prox-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("prox-beta-{unique}"));
    let concept_c = normalize_concept(&format!("prox-gamma-{unique}"));
    let relative_path = format!("prox-tests/prox-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Proximity Test\n\nPreamble: [[{concept_a}]] and [[{concept_c}]].\n\n## Section One\n\n[[{concept_b}]] and [[{concept_c}]] here.\n"
        ),
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_b, &uri),
        0.5
    );
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_c, &uri),
        0.5
    );
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_b, &concept_c, &uri),
        1.0
    );
}

// T-0.1: RTM NFR-3
#[test]
fn get_decay_weight_for_file_reads_from_db_matches_decay_calculator_directly() {
    let env = TestEnv::new();
    let relative_path = format!("decay-db-tests/decay-test-{}.md", unique_suffix());
    let uri = create_aged_file(
        &env,
        &relative_path,
        "Decay DB Test",
        "Content with [[decay-test]].",
        60,
    );

    let output = sync_one(&env, &relative_path);
    assert!(
        output.contains("Synced memory://decay-db-tests/"),
        "{output}"
    );

    let expected = expected_decay_weight(&env.cfg, 60);
    let actual = decay_weight_for_uri(&env.cfg, &uri);
    assert!(
        (actual - expected).abs() <= 0.005,
        "expected {expected}, got {actual}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_persists_created_from_frontmatter_to_database() {
    let env = TestEnv::new();
    let relative_path = format!("reg-created/created-reg-{}.md", unique_suffix());
    let expected_created = "2024-06-15T10:00:00Z";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Created Reg Test".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(expected_created.to_string()),
    );
    let uri = write_memory_with_frontmatter(
        &env.cfg,
        &relative_path,
        &frontmatter,
        "# Created Reg Test\n\n[[concept-alpha]] mentioned here.",
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://reg-created/"), "{output}");

    let stored = fetch_created(&env.cfg, &uri).expect("created timestamp");
    let parsed = DateTime::parse_from_rfc3339(&stored)
        .expect("stored created")
        .with_timezone(&Utc);
    let expected = DateTime::parse_from_rfc3339(expected_created)
        .expect("expected created")
        .with_timezone(&Utc);
    assert!(
        (parsed - expected).num_seconds().abs() < 2,
        "stored={stored} expected={expected_created}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_falls_back_to_filesystem_ctime_when_frontmatter_has_no_created_key() {
    let env = TestEnv::new();
    let relative_path = format!("reg-ctime-fallback/ctime-fallback-{}.md", unique_suffix());
    let before = Utc::now() - Duration::seconds(2);
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "---\ntitle: Ctime Fallback Test\ntype: memory\nstatus: saved\n---\n\n# Ctime Fallback\n\n[[concept-beta]] here.\n",
    );

    let output = sync_one(&env, &relative_path);
    let after = Utc::now() + Duration::seconds(2);
    assert!(
        output.contains("Synced memory://reg-ctime-fallback/"),
        "{output}"
    );

    let stored = fetch_created(&env.cfg, &uri).expect("created fallback");
    let parsed = DateTime::parse_from_rfc3339(&stored)
        .expect("parse created")
        .with_timezone(&Utc);
    assert!(
        parsed >= before && parsed <= after,
        "stored created out of range: {stored}"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn count_all_concept_occurrences_matches_per_concept_counts_for_all_concepts() {
    let content = "# Multi-Count Test\n\n[[alpha]] and [[beta]] start the file.\nThen [[alpha]] again, and [[gamma]].\n\n## Section One\n\n[[alpha]] and [[beta]] in section one.\nInline code `[[alpha]]` should NOT count.\n\n## Section Two\n\n[[gamma]] appears here too.\n[[delta]] only once.\n";
    let concepts = ["alpha", "beta", "gamma", "delta", "epsilon"];

    let expected = concepts
        .iter()
        .map(|concept| {
            (
                (*concept).to_string(),
                count_visible_wikilink_occurrences(content, concept),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let actual = count_all_visible_wikilink_occurrences(content, &concepts);

    assert_eq!(actual, expected);
    assert_eq!(actual.get("alpha"), Some(&3));
    assert_eq!(actual.get("beta"), Some(&2));
    assert_eq!(actual.get("gamma"), Some(&2));
    assert_eq!(actual.get("delta"), Some(&1));
    assert_eq!(actual.get("epsilon"), Some(&0));
}

// T-0.1: RTM NFR-3
#[test]
fn count_all_concept_occurrences_returns_zero_for_absent_concepts() {
    let content = "Only [[present-concept]] is here.";
    let counts = count_all_visible_wikilink_occurrences(
        content,
        &["present-concept", "absent-concept", "also-absent"],
    );

    assert_eq!(counts.get("present-concept"), Some(&1));
    assert_eq!(counts.get("absent-concept"), Some(&0));
    assert_eq!(counts.get("also-absent"), Some(&0));
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_skips_reindexing_when_mtime_unchanged() {
    let env = TestEnv::new();
    let relative_path = format!("reg-mtime/mtime-test-{}.md", unique_suffix());
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "# Mtime Test\n\n[[concept-mtime]] mentioned here.\n",
    );

    let first = sync_one(&env, &relative_path);
    assert!(first.contains("Synced memory://reg-mtime/"), "{first}");
    let last_indexed_before = fetch_last_indexed(&env.cfg, &uri).expect("last_indexed before");

    let second = sync_one(&env, &relative_path);
    assert!(second.contains("mtime guard"), "{second}");
    assert_eq!(
        fetch_last_indexed(&env.cfg, &uri),
        Some(last_indexed_before)
    );
}

// T-0.1: RTM NFR-3
#[test]
fn process_file_skips_reindexing_when_md5_matches_despite_mtime_change() {
    let env = TestEnv::new();
    let relative_path = format!("reg-md5/md5-test-{}.md", unique_suffix());
    let file_path = env.cfg.memory_path.join(&relative_path);
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "# MD5 Guard Test\n\n[[concept-md5]] appears here.\n",
    );

    let first = sync_one(&env, &relative_path);
    assert!(first.contains("Synced memory://reg-md5/"), "{first}");

    let old_timestamp = "2000-01-01T00:00:00+00:00";
    let original_hash = fetch_file_md5(&env.cfg, &uri).expect("original hash");
    set_last_indexed(&env.cfg, &uri, old_timestamp);
    let newer = FileTime::from_unix_time(Utc::now().timestamp() + 5, 0);
    set_file_mtime(&file_path, newer).expect("touch file mtime");

    let second = sync_one(&env, &relative_path);
    assert!(second.contains("hash guard"), "{second}");
    assert_ne!(
        fetch_last_indexed(&env.cfg, &uri).as_deref(),
        Some(old_timestamp)
    );
    assert_eq!(fetch_file_md5(&env.cfg, &uri), Some(original_hash));
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_resync_does_not_duplicate_edges_co_occurrence_count_correct() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let concept_a = normalize_concept(&format!("resync-alpha-{unique}"));
    let concept_b = normalize_concept(&format!("resync-beta-{unique}"));
    let relative_path = format!("reg-resync/resync-note-{unique}.md");
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!("# Resync Test\n\n## Shared\n\n[[{concept_a}]] and [[{concept_b}]] here.\n"),
    );

    let first = sync_one(&env, &relative_path);
    assert!(first.contains("affected relations rebuilt"), "{first}");
    let rowid_before = query_edge_rowid(&env.cfg, &concept_a, &concept_b).expect("edge rowid");

    set_last_indexed(&env.cfg, &uri, "2000-01-01T00:00:00+00:00");
    write_memory_fixture(
        &env.cfg,
        &relative_path,
        &format!(
            "# Resync Test Updated\n\n## Shared\n\n[[{concept_a}]] and [[{concept_b}]] still here.\n"
        ),
    );

    let second = sync_one(&env, &relative_path);
    assert!(second.contains("affected relations rebuilt"), "{second}");
    assert_eq!(query_edge_row_count(&env.cfg, &concept_a, &concept_b), 1);
    assert_eq!(
        query_edge_rowid(&env.cfg, &concept_a, &concept_b),
        Some(rowid_before)
    );
    assert_eq!(
        relation_weight_for_uri(&env.cfg, &concept_a, &concept_b, &uri),
        1.0
    );
}

// T-0.1: RTM NFR-3
#[test]
#[ignore = "requires ONNX/sqlite-vec runtime"]
fn generate_file_embedding_populates_vec_table_insert_or_replace_runs_to_completion() {
    let env = TestEnv::new();
    let relative_path = format!("perf-bt-001/embed-guard-{}.md", unique_suffix());
    let uri = write_memory_fixture(
        &env.cfg,
        &relative_path,
        "# Embed Guard Test\n\n[[concept-a]] is mentioned here.\n",
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://perf-bt-001/"), "{output}");

    let blob = open_db(&env.cfg)
        .query_row(
            "SELECT embedding FROM vec_memory_files WHERE file_path = ?1",
            [uri],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .expect("optional embedding")
        .expect("embedding row");

    assert!(!blob.is_empty(), "embedding blob should be populated");
    assert!(
        blob.iter().any(|byte| *byte != 0),
        "embedding blob should not be all zeros"
    );
}

// T-0.1: RTM NFR-3
#[test]
fn build_graph_edges_with_over_1000_concepts_creates_edges_without_error() {
    let env = TestEnv::new();
    let unique = unique_suffix();
    let relative_path = format!("perf-bt-002/large-concept-file-{unique}.md");
    let concept_names = (0..1100)
        .map(|index| format!("perf-bt-002-concept-{index:04}-{unique}"))
        .collect::<Vec<_>>();

    let mut content = String::from("# Large Concept File\n\n## Shared\n\n");
    for concept in &concept_names {
        content.push_str(&format!("[[{concept}]] "));
    }
    content.push('\n');
    write_memory_fixture(&env.cfg, &relative_path, &content);

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("affected relations rebuilt"), "{output}");

    let edge_count = open_db(&env.cfg)
        .query_row("SELECT COUNT(*) FROM concept_graph", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("edge count");
    assert!(edge_count > 0, "concept_graph should contain edges");

    let first = normalize_concept(&concept_names[0]);
    let second = normalize_concept(&concept_names[1]);
    let penultimate = normalize_concept(&concept_names[1098]);
    let last = normalize_concept(&concept_names[1099]);
    assert_eq!(query_edge_row_count(&env.cfg, &first, &second), 1);
    assert_eq!(query_edge_row_count(&env.cfg, &penultimate, &last), 1);
}

// T-0.1: RTM NFR-3
#[test]
fn sweep_abandoned_thinking_sessions_mtime_guard_protects_recently_edited_sessions() {
    let env = TestEnv::new();
    let _ttl = AbandonmentEnvGuard::set("1");
    let unique = unique_suffix();
    let file_uri = format!("memory://thinking/sequential/2026/03/09/session-mtime-{unique}");
    let old_last_indexed = (Utc::now() - Duration::hours(2)).to_rfc3339();

    seed_session(&env.cfg, &file_uri, "active", &old_last_indexed);
    let session_path = maenifold_core::markdown::uri_to_path(&file_uri, &env.cfg.memory_path)
        .expect("session path");

    let recent_mtime = FileTime::from_unix_time(Utc::now().timestamp(), 0);
    set_file_mtime(&session_path, recent_mtime).expect("set recent mtime");
    let abandoned_a = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup recent");
    assert_eq!(abandoned_a, 0, "recent mtime should protect session");
    assert_eq!(frontmatter_status(&env.cfg, &file_uri), "active");

    set_last_indexed(&env.cfg, &file_uri, &old_last_indexed);
    let stale_mtime = FileTime::from_unix_time((Utc::now() - Duration::hours(2)).timestamp(), 0);
    set_file_mtime(&session_path, stale_mtime).expect("set stale mtime");
    let abandoned_b = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup stale");
    assert_eq!(abandoned_b, 1, "stale mtime should abandon session");
    assert_eq!(frontmatter_status(&env.cfg, &file_uri), "abandoned");
}

// T-0.1: RTM NFR-3
#[test]
fn try_acquire_lock_atomic_behavior_held_returns_false_released_returns_true() {
    let env = TestEnv::new();
    let election = LeaderElection::new(&env.cfg.database_path);

    if !election.try_acquire("holder-a", 10).expect("normalize").0 {
        panic!("lock was unexpectedly held at test start");
    }
    election.release("holder-a").expect("normalize release");

    let first = election.try_acquire("holder-a", 10).expect("first acquire");
    assert_eq!(first, (true, "acquired"));

    let second = election
        .try_acquire("holder-b", 10)
        .expect("second acquire");
    assert_eq!(second, (false, "held_by_other"));

    election.release("holder-a").expect("release held lock");
    let third = election.try_acquire("holder-c", 10).expect("third acquire");
    assert_eq!(third, (true, "acquired"));
    election.release("holder-c").expect("cleanup release");
}

// T-0.1: RTM NFR-3
#[test]
fn get_decay_weight_for_file_uses_db_row_not_disk_io_matches_decay_calculator() {
    let env = TestEnv::new();
    let relative_path = format!("reg-decay/decay-reg-{}.md", unique_suffix());
    let uri = create_aged_file(
        &env,
        &relative_path,
        "Decay Reg Test",
        "[[api-design]] links to [[old-pattern]].",
        120,
    );

    let output = sync_one(&env, &relative_path);
    assert!(output.contains("Synced memory://reg-decay/"), "{output}");

    let expected = expected_decay_weight(&env.cfg, 120);
    let actual = decay_weight_for_uri(&env.cfg, &uri);
    assert!(expected < 1.0, "120-day-old file should decay below 1.0");
    assert!(
        (actual - expected).abs() <= 0.005,
        "expected {expected}, got {actual}"
    );

    let context_payload = build_context_json(&env, "api-design");
    let relation = direct_relation(&context_payload, "old-pattern");
    let decay_weight = relation["DecayWeight"].as_f64().expect("DecayWeight");
    let weighted_score = relation["WeightedScore"].as_f64().expect("WeightedScore");
    let co_occurrence = relation["CoOccurrenceCount"]
        .as_f64()
        .expect("CoOccurrenceCount");
    assert!(
        decay_weight > 0.0 && decay_weight < 1.0,
        "unexpected relation decay: {context_payload}"
    );
    assert!(
        (weighted_score - (co_occurrence * decay_weight)).abs() <= 1e-9,
        "weighted score should reflect co-occurrence-scaled decay: {context_payload}"
    );
}
