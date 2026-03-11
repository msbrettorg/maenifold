use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{Frontmatter, normalize_concept};
use maenifold_core::search::sync_single_file;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use rusqlite::{Connection, params};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use tempfile::TempDir;

// T-12.7.1: RTM NFR-1 — Shared markdown reader/writer regression helpers split by theme.
pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(crate) struct TestEnv {
    _tmp: TempDir,
    pub(crate) cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub(crate) fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

pub(crate) fn memory_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

pub(crate) fn write_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, content).expect("write fixture");
}

fn open_db(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("open db")
}

pub(crate) fn file_concepts(cfg: &Config, file_uri: &str) -> BTreeSet<String> {
    let connection = open_db(cfg);
    let mut statement = connection
        .prepare(
            "SELECT concept_name FROM concept_mentions WHERE source_file = ?1 ORDER BY concept_name",
        )
        .expect("prepare concept_mentions");
    statement
        .query_map([file_uri], |row| row.get::<_, String>(0))
        .expect("query concept_mentions")
        .collect::<std::result::Result<BTreeSet<_>, _>>()
        .expect("collect concept_mentions")
}

pub(crate) fn mention_count(cfg: &Config, file_uri: &str) -> i64 {
    open_db(cfg)
        .query_row(
            "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE source_file = ?1",
            params![file_uri],
            |row| row.get(0),
        )
        .expect("mention count")
}

pub(crate) fn scalar(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        _ => None,
    })
}

pub(crate) fn count_concept_occurrences(content: &str, concept: &str) -> usize {
    let target = normalize_concept(concept);
    let mut count = 0usize;
    let mut visible = String::with_capacity(content.len());
    let mut code_block_depth = 0usize;

    for event in Parser::new_ext(content, Options::all()) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => {
                if code_block_depth == 0 && normalize_concept(dest_url.as_ref()) == target {
                    count += 1;
                }
            }
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

    count + count_regex_wikilinks(&visible, &target)
}

fn count_regex_wikilinks(content: &str, target: &str) -> usize {
    let bytes = content.as_bytes();
    let mut count = 0usize;
    let mut index = 0usize;

    while index + 3 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        if index > 0 && bytes[index - 1] == b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        let mut closing = None;
        let mut invalid = false;

        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    closing = Some(cursor);
                    break;
                }
                (b'[', _) | (b']', _) => {
                    invalid = true;
                    break;
                }
                _ => cursor += 1,
            }
        }

        let Some(end) = closing else {
            index += 2;
            continue;
        };

        if invalid || (end + 2 < bytes.len() && bytes[end + 2] == b']') {
            index = end + 2;
            continue;
        }

        if normalize_concept(&content[index + 2..end]) == target {
            count += 1;
        }
        index = end + 2;
    }

    count
}

pub(crate) fn count_all_concept_occurrences(
    content: &str,
    concepts: &[String],
) -> BTreeMap<String, usize> {
    concepts
        .iter()
        .map(|concept| {
            (
                concept.clone(),
                count_concept_occurrences(content, concept.as_str()),
            )
        })
        .collect()
}

pub(crate) fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}
