// T-0.1: RTM NFR-3 - Shared helpers for split sync performance coverage.

use chrono::{DateTime, Duration, Utc};
use filetime::{FileTime, set_file_mtime};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
// T-12.6: RTM NFR-1 - Keep regression-test imports aligned with the sync module split.
use maenifold_core::markdown::{
    Frontmatter, extract_wikilinks_with_sections, normalize_concept, read_markdown, write_markdown,
};
use maenifold_core::memory::compute_decay_weight;
use maenifold_core::search::{self, sync_single_file};
use maenifold_core::sessions;
use maenifold_core::sync::leader::LeaderElection;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use parking_lot::MutexGuard;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;
use uuid::Uuid;

pub(crate) use crate::GLOBAL_CONFIG_TEST_MUTEX as TEST_MUTEX;

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
        let previous = std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES").ok();
        // SAFETY: tests in this file serialize access with TEST_MUTEX.
        unsafe {
            std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", minutes);
        }
        Self { previous }
    }
}

impl Drop for AbandonmentEnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // SAFETY: tests in this file serialize access with TEST_MUTEX.
            unsafe {
                std::env::set_var("MAENIFOLD_SESSION_ABANDON_MINUTES", value);
            }
        } else {
            // SAFETY: tests in this file serialize access with TEST_MUTEX.
            unsafe {
                std::env::remove_var("MAENIFOLD_SESSION_ABANDON_MINUTES");
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
