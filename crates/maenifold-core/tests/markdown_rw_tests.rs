use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{
    Frontmatter, append_h2_section, extract_wikilinks, extract_wikilinks_with_sections,
    normalize_concept, read_markdown, write_markdown,
};
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
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// T-W1-3: RTM NFR-2 — Ported Markdown reader/writer/sync coverage from the C# oracle.
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

fn memory_uri(relative_path: &str) -> String {
    format!("memory://{}", relative_path.trim_end_matches(".md"))
}

fn write_fixture(cfg: &Config, relative_path: &str, content: &str) {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, content).expect("write fixture");
}

fn open_db(cfg: &Config) -> Connection {
    db::open_connection(&cfg.database_path).expect("open db")
}

fn file_concepts(cfg: &Config, file_uri: &str) -> BTreeSet<String> {
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

fn mention_count(cfg: &Config, file_uri: &str) -> i64 {
    open_db(cfg)
        .query_row(
            "SELECT COALESCE(SUM(mention_count), 0) FROM concept_mentions WHERE source_file = ?1",
            params![file_uri],
            |row| row.get(0),
        )
        .expect("mention count")
}

fn scalar(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        _ => None,
    })
}

fn count_concept_occurrences(content: &str, concept: &str) -> usize {
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

fn count_all_concept_occurrences(content: &str, concepts: &[String]) -> BTreeMap<String, usize> {
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

fn sync_one(cfg: &Config, relative_path: &str) -> String {
    let context = ToolContext::new(cfg.clone(), OutputFormat::Markdown);
    sync_single_file(&context, &cfg.memory_path.join(relative_path)).expect("sync single file")
}

// T-W1-3: RTM NFR-2 — MarkdownReaderTests (10)

#[test]
fn extract_wikilinks_skips_code_blocks() {
    let content = r#"Real [[authentication]] here.

```bash
# This [[fake-credential]] and $variable should NOT be extracted
echo "test"
```

Another [[database-connection]] outside."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"authentication".to_string()));
    assert!(concepts.contains(&"database-connection".to_string()));
    assert!(!concepts.contains(&"fake-credential".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_skips_inline_code() {
    let content = "This is a [[machine-learning]] and `[[inline-code]]` in inline code.";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"machine-learning".to_string()));
    assert!(!concepts.contains(&"inline-code".to_string()));
    assert_eq!(concepts.len(), 1);
}

#[test]
fn extract_wikilinks_extracts_from_headings() {
    let content = "# Heading with [[neural-network]]\n\nBody with [[deep-learning]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"neural-network".to_string()));
    assert!(concepts.contains(&"deep-learning".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_extracts_from_lists() {
    let content = "- Item with [[api-endpoint]]\n- Another item with [[database-schema]]\n\nText with [[cache-strategy]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"api-endpoint".to_string()));
    assert!(concepts.contains(&"database-schema".to_string()));
    assert!(concepts.contains(&"cache-strategy".to_string()));
    assert_eq!(concepts.len(), 3);
}

#[test]
fn extract_wikilinks_normalizes_concepts() {
    let content = "[[Machine Learning]] and [[machine-learning]] are the same.";

    let concepts = extract_wikilinks(content);

    assert_eq!(concepts.len(), 1);
    assert!(concepts.contains(&"machine-learning".to_string()));
}

#[test]
fn extract_wikilinks_skips_fenced_code_block_with_language() {
    let content = r#"Valid [[dependency-injection]] here.

```csharp
var x = "[[code-snippet]]";
```

Another [[unit-testing]]."#;

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"dependency-injection".to_string()));
    assert!(concepts.contains(&"unit-testing".to_string()));
    assert!(!concepts.contains(&"code-snippet".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn extract_wikilinks_skips_indented_code_blocks() {
    let content = "Normal [[refactoring]].\n\n    This is indented code block\n    [[indented-code]]\n\nBack to normal [[code-review]].";

    let concepts = extract_wikilinks(content);

    assert!(concepts.contains(&"refactoring".to_string()));
    assert!(concepts.contains(&"code-review".to_string()));
    assert!(!concepts.contains(&"indented-code".to_string()));
    assert_eq!(concepts.len(), 2);
}

#[test]
fn count_concept_occurrences_skips_code_blocks() {
    let content = "First [[logging]].\n\n```bash\n[[logging]] in code\n```\n\nSecond [[logging]].";

    let count = count_concept_occurrences(content, "logging");

    assert_eq!(count, 2);
}

#[test]
fn count_concept_occurrences_skips_inline_code() {
    let content =
        "First [[microservices]] and `[[microservices]]` in code and third [[microservices]].";

    let count = count_concept_occurrences(content, "microservices");

    assert_eq!(count, 2);
}

#[test]
fn count_concept_occurrences_handles_normalization() {
    let content = "[[API Design]] and [[api-design]] are the same.";

    let count = count_concept_occurrences(content, "api-design");

    assert_eq!(count, 2);
}

// T-W1-3: RTM NFR-2 — MarkdownWriterTests (19)

#[test]
fn normalize_concept_double_space_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("API  Gateway"), "api-gateway");
}

#[test]
fn normalize_concept_double_underscore_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("user__profile"), "user-profile");
}

#[test]
fn normalize_concept_multiple_slashes_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("auth / / token"), "auth-token");
}

#[test]
fn normalize_concept_mixed_separators_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("API  __  //  Gateway"), "api-gateway");
}

#[test]
fn normalize_concept_leading_trailing_separators_trims_hyphens() {
    assert_eq!(normalize_concept("  concept  "), "concept");
}

#[test]
fn normalize_concept_only_separators_returns_empty() {
    assert_eq!(normalize_concept("   ___  ///  "), "");
}

#[test]
fn normalize_concept_single_separators_works_as_expected() {
    assert_eq!(normalize_concept("REST API Gateway"), "rest-api-gateway");
}

#[test]
fn normalize_concept_no_separators_works_as_expected() {
    assert_eq!(normalize_concept("Authentication"), "authentication");
}

#[test]
fn normalize_concept_triple_hyphens_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("test---concept"), "test-concept");
}

#[test]
fn normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen() {
    assert_eq!(normalize_concept("test----------concept"), "test-concept");
}

#[test]
fn write_markdown_null_frontmatter_creates_envelope_with_created_and_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/null-frontmatter.md");

    write_markdown(&uri, None, "# Test\n\nContent", &env.cfg).expect("write markdown");

    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
}

#[test]
fn write_markdown_existing_created_preserves_created_and_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/preserve-created.md");
    let original_created = "2020-01-01T00:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(original_created.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "created").as_deref(),
        Some(original_created)
    );
    assert!(read_frontmatter.contains_key("modified"));
    assert_ne!(
        scalar(&read_frontmatter, "modified").as_deref(),
        Some(original_created)
    );
}

#[test]
fn write_markdown_missing_created_adds_created_and_sets_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/add-created.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Note".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# My Note\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert!(read_frontmatter.contains_key("created"));
    assert!(read_frontmatter.contains_key("modified"));
}

#[test]
fn write_markdown_completed_and_cancelled_not_overwritten() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/lifecycle.md");
    let completed_time = "2025-06-01T12:00:00.0000000+00:00";
    let cancelled_time = "2025-06-02T08:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String(completed_time.to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String(cancelled_time.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Session\n\nContent", &env.cfg).expect("write");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "completed").as_deref(),
        Some(completed_time)
    );
    assert_eq!(
        scalar(&read_frontmatter, "cancelled").as_deref(),
        Some(cancelled_time)
    );
}

#[test]
fn write_markdown_does_not_mutate_caller_dict() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/no-mutate.md");
    let stale_modified = "2020-01-01T00:00:00.0000000+00:00";
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String(stale_modified.to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");

    assert_eq!(
        scalar(&frontmatter, "modified").as_deref(),
        Some(stale_modified)
    );
}

#[test]
fn append_h2_section_existing_file_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-updates-modified.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String("2020-01-01T00:00:00.0000000+00:00".to_string()),
    );
    frontmatter.insert(
        "modified".to_string(),
        YamlValue::String("2020-01-01T00:00:00.0000000+00:00".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Test\n\nContent", &env.cfg).expect("write");
    let (before_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read before");
    let before_modified = scalar(&before_frontmatter.expect("frontmatter"), "modified")
        .expect("modified before append");

    thread::sleep(Duration::from_millis(10));
    append_h2_section(&uri, "New Section", "New content", &env.cfg).expect("append section");

    let (after_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read after");
    let after_frontmatter = after_frontmatter.expect("frontmatter");
    assert!(after_frontmatter.contains_key("modified"));
    assert_ne!(
        scalar(&after_frontmatter, "modified").expect("modified after append"),
        before_modified
    );
}

#[test]
fn append_h2_section_existing_file_preserves_all_frontmatter_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-preserves-frontmatter.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Session".to_string()),
    );

    write_markdown(&uri, Some(&frontmatter), "# Session\n\nContent", &env.cfg).expect("write");
    append_h2_section(&uri, "Thought 1", "First thought content", &env.cfg).expect("append");

    let (read_frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read");
    let read_frontmatter = read_frontmatter.expect("frontmatter");
    assert_eq!(
        scalar(&read_frontmatter, "type").as_deref(),
        Some("sequential")
    );
    assert_eq!(
        scalar(&read_frontmatter, "status").as_deref(),
        Some("active")
    );
    assert_eq!(
        scalar(&read_frontmatter, "title").as_deref(),
        Some("My Session")
    );
}

#[test]
fn append_h2_section_existing_file_preserves_existing_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/append-preserves-content.md");
    let mut frontmatter = Frontmatter::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );

    write_markdown(
        &uri,
        Some(&frontmatter),
        "# Session\n\nExisting content here.",
        &env.cfg,
    )
    .expect("write");
    append_h2_section(&uri, "Appended Section", "New content", &env.cfg).expect("append");

    let (_, content, _) = read_markdown(&uri, &env.cfg).expect("read");
    assert!(content.contains("Existing content here."));
    assert!(content.contains("## Appended Section"));
    assert!(content.contains("New content"));
}

#[test]
fn append_h2_section_non_existent_file_creates_file_with_envelope() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = memory_uri("writer/new-append-file.md");

    append_h2_section(&uri, "First Section", "First content", &env.cfg).expect("append");

    let (frontmatter, content, _) = read_markdown(&uri, &env.cfg).expect("read");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
    assert!(content.contains("## First Section"));
}

// T-W1-3: RTM NFR-2 — MarkdownReaderSyncTests (7)

#[test]
fn sync_basic_file_frontmatter_and_content_match_expected_processing() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/basic.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"---
title: Test Note
tags:
  - alpha
  - beta
---

This note mentions [[alpha]] and [[beta]].
Also references [[gamma]] for good measure.
"#,
    );

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let frontmatter = frontmatter.expect("frontmatter");
    let concepts_with_sections = extract_wikilinks_with_sections(&content);
    let concepts = concepts_with_sections
        .iter()
        .map(|(concept, _)| concept.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let concept_counts = count_all_concept_occurrences(&content, &concepts);
    let output = sync_one(&env.cfg, relative_path);

    assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("Test Note"));
    assert!(content.contains("[[alpha]]"));
    assert!(!checksum.is_empty());
    assert_eq!(
        concepts,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
    assert_eq!(
        concept_counts,
        BTreeMap::from([
            ("alpha".to_string(), 1usize),
            ("beta".to_string(), 1usize),
            ("gamma".to_string(), 1usize),
        ])
    );
    assert!(output.contains("Synced memory://sync/basic"));
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        concepts.iter().cloned().collect::<BTreeSet<_>>()
    );
    assert_eq!(mention_count(&env.cfg, &uri), 3);
}

#[test]
fn sync_wikilinks_with_sections_section_indices_match_expected_extraction() {
    let content = r#"Preamble mentions [[preamble-concept]].

## First Section

Content with [[alpha]] and [[beta]].

## Second Section

Content with [[gamma]].
Also [[alpha]] appears again in section two.
"#;

    let concepts_with_sections = extract_wikilinks_with_sections(content);

    assert!(concepts_with_sections.contains(&("preamble-concept".to_string(), 0)));
    assert!(concepts_with_sections.contains(&("alpha".to_string(), 1)));
    assert!(concepts_with_sections.contains(&("beta".to_string(), 1)));
    assert!(concepts_with_sections.contains(&("gamma".to_string(), 2)));
    assert!(concepts_with_sections.contains(&("alpha".to_string(), 2)));
}

#[test]
fn sync_concept_counts_match_expected_occurrences() {
    let content = r#"[[alpha]] appears first.

Then [[beta]] appears.

Later [[alpha]] appears again, and [[alpha]] a third time.

[[beta]] also appears a second time.
"#;
    let concepts = vec!["alpha".to_string(), "beta".to_string()];
    let concept_counts = count_all_concept_occurrences(content, &concepts);

    assert_eq!(concept_counts["alpha"], 3);
    assert_eq!(concept_counts["beta"], 2);
}

#[test]
fn sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/codeblocks.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"This paragraph mentions [[real-concept]].

```markdown
This code block contains [[fake-concept]] that must be ignored.
```

Another paragraph with [[another-real-concept]].
"#,
    );

    let output = sync_one(&env.cfg, relative_path);
    let concepts = file_concepts(&env.cfg, &uri);

    assert!(output.contains("Synced memory://sync/codeblocks"));
    assert!(concepts.contains("real-concept"));
    assert!(concepts.contains("another-real-concept"));
    assert!(!concepts.contains("fake-concept"));
}

#[test]
fn sync_no_frontmatter_content_only_file_is_processed() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/no-frontmatter.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        "Just a plain note with [[some-concept]] and [[another-concept]].\n\nNo frontmatter block at all.\n",
    );

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert!(content.contains("[[some-concept]]"));
    assert!(!checksum.is_empty());
    assert!(output.contains("2 concept mentions updated"));
    assert_eq!(
        file_concepts(&env.cfg, &uri),
        BTreeSet::from(["another-concept".to_string(), "some-concept".to_string()])
    );
}

#[test]
fn sync_empty_file_is_handled_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/empty.md";
    let uri = memory_uri(relative_path);
    write_fixture(&env.cfg, relative_path, "");

    let (frontmatter, content, checksum) = read_markdown(&uri, &env.cfg).expect("read");
    let output = sync_one(&env.cfg, relative_path);

    assert!(frontmatter.is_none());
    assert_eq!(content, "");
    assert!(!checksum.is_empty());
    assert!(output.contains("0 concept mentions updated"));
    assert!(file_concepts(&env.cfg, &uri).is_empty());
}

#[test]
fn sync_inline_code_exclusion_ignores_wikilinks_in_backticks() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let relative_path = "sync/inline-code.md";
    let uri = memory_uri(relative_path);
    write_fixture(
        &env.cfg,
        relative_path,
        r#"This mentions [[real-concept]] normally.

But `[[fake-inline-concept]]` is inside backticks and must be ignored.

And [[another-real]] is also valid.
"#,
    );

    let output = sync_one(&env.cfg, relative_path);
    let concepts = file_concepts(&env.cfg, &uri);

    assert!(output.contains("Synced memory://sync/inline-code"));
    assert!(concepts.contains("real-concept"));
    assert!(concepts.contains("another-real"));
    assert!(!concepts.contains("fake-inline-concept"));
}
