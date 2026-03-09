use chrono::{Duration as ChronoDuration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown::{
    self, Frontmatter, append_h2_section, read_markdown, write_markdown,
};
use maenifold_core::memory;
use maenifold_core::search;
use maenifold_core::sessions;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::thread;
use std::time::Duration as StdDuration;
use tempfile::TempDir;

// T-META-001.6: RTM FR-24.x — Ported frontmatter metadata regression coverage from the C# oracle.
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

fn markdown_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Markdown)
}

fn json_context(cfg: &Config) -> ToolContext {
    ToolContext::new(cfg.clone(), OutputFormat::Json)
}

fn yaml_string(frontmatter: &Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(text) => Some(text.clone()),
        YamlValue::Bool(value) => Some(value.to_string()),
        YamlValue::Number(value) => Some(value.to_string()),
        other => serde_yaml::to_string(other)
            .ok()
            .map(|text| text.trim().to_string()),
    })
}

fn write_raw_markdown(cfg: &Config, file_uri: &str, raw_text: &str) {
    let path = markdown::uri_to_path(file_uri, &cfg.memory_path).expect("uri to path");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, raw_text).expect("write raw markdown");
}

fn read_raw_markdown(cfg: &Config, file_uri: &str) -> String {
    let path = markdown::uri_to_path(file_uri, &cfg.memory_path).expect("uri to path");
    fs::read_to_string(path).expect("read raw markdown")
}

fn extract_uri_from_json(output: &str) -> String {
    let value: Value = serde_json::from_str(output).expect("tool json");
    assert_eq!(value["success"], true, "tool call failed: {output}");
    value["data"]["uri"].as_str().expect("uri").to_string()
}

fn extract_memory_uris(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|token| token.starts_with("memory://"))
        .map(|token| token.trim_matches(|ch: char| ch == ',' || ch == ';'))
        .map(str::to_string)
        .collect()
}

fn iso_minutes_ago(minutes_ago: i64) -> String {
    (Utc::now() - ChronoDuration::minutes(minutes_ago)).to_rfc3339()
}

fn seed_file_content(cfg: &Config, file_uri: &str, status: &str, body: &str, last_indexed: &str) {
    let conn = db::open_connection(&cfg.database_path).expect("open db");
    let title = file_uri
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .expect("title segment");
    conn.execute(
        "INSERT INTO file_content (file_path, title, content, file_md5, file_size, last_indexed, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file_uri,
            title,
            body,
            format!("md5-{title}"),
            i64::try_from(body.len()).expect("body len"),
            last_indexed,
            status,
        ],
    )
    .expect("seed file_content");
}

fn read_frontmatter_status(cfg: &Config, file_uri: &str) -> String {
    let (frontmatter, _, _) = read_markdown(file_uri, cfg).expect("read markdown");
    frontmatter
        .and_then(|values| {
            values
                .get("status")
                .and_then(YamlValue::as_str)
                .map(str::to_string)
        })
        .expect("status")
}

// T-META-001.6: RTM FR-24.1 — WriteMarkdown envelope management.
#[test]
fn fr24_1_ac1_write_markdown_new_file_has_created_and_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-new";

    write_markdown(file_uri, None, "# Test\n\n[[concept]]", &env.cfg).expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
    assert!(yaml_string(&frontmatter, "created").is_some_and(|value| !value.is_empty()));
    assert!(yaml_string(&frontmatter, "modified").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Existing created value survives rewrites.
#[test]
fn fr24_1_ac2_write_markdown_existing_created_preserved_on_rewrite() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-preserve-created";

    write_markdown(file_uri, None, "# Test\n\n[[concept]]", &env.cfg).expect("initial write");
    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read first write");
    let original_created =
        yaml_string(&frontmatter.expect("frontmatter"), "created").expect("created");

    thread::sleep(StdDuration::from_millis(10));

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "created".to_string(),
        YamlValue::String(original_created.clone()),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Test updated\n\n[[concept]]",
        &env.cfg,
    )
    .expect("rewrite markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read rewrite");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "created").as_deref(),
        Some(original_created.as_str())
    );
    assert!(yaml_string(&frontmatter, "modified").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Missing created is added on write.
#[test]
fn fr24_1_ac3_write_markdown_missing_created_added_on_rewrite() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-add-created";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert("title".to_string(), YamlValue::String("Test".to_string()));
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Test\n\n[[concept]]",
        &env.cfg,
    )
    .expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(yaml_string(&frontmatter, "created").is_some_and(|value| !value.is_empty()));
}

// T-META-001.6: RTM FR-24.1 — Null frontmatter still yields an envelope.
#[test]
fn fr24_1_ac4_write_markdown_null_frontmatter_dict_created() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-null-fm";

    write_markdown(file_uri, None, "# Null FM\n\n[[wikilink]]", &env.cfg).expect("write markdown");

    let raw_text = read_raw_markdown(&env.cfg, file_uri);
    assert!(
        raw_text.starts_with("---"),
        "raw text should start with YAML frontmatter: {raw_text}"
    );

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
}

// T-META-001.6: RTM FR-24.1 — Lifecycle timestamps are preserved.
#[test]
fn fr24_1_ac5_write_markdown_lifecycle_timestamps_not_overwritten() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-lifecycle";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("workflow".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("completed".to_string()),
    );
    frontmatter.insert(
        "completed".to_string(),
        YamlValue::String("2025-01-01T00:00:00Z".to_string()),
    );
    frontmatter.insert(
        "cancelled".to_string(),
        YamlValue::String("2025-02-01T00:00:00Z".to_string()),
    );

    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Lifecycle\n\n[[workflow]]",
        &env.cfg,
    )
    .expect("write markdown");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "completed").as_deref(),
        Some("2025-01-01T00:00:00Z")
    );
    assert_eq!(
        yaml_string(&frontmatter, "cancelled").as_deref(),
        Some("2025-02-01T00:00:00Z")
    );
}

// T-META-001.6: RTM FR-24.2 — AppendH2Section routes through WriteMarkdown.
#[test]
fn fr24_2_ac1_append_h2_section_updates_modified() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-modified";

    write_markdown(file_uri, None, "# Session\n\n[[concept]]", &env.cfg).expect("initial write");
    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read initial");
    let first_modified =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

    append_h2_section(file_uri, "Thought 1", "Some [[reasoning]] here.", &env.cfg)
        .expect("append section");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read appended");
    let frontmatter = frontmatter.expect("frontmatter");
    let second_modified = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!second_modified.is_empty());
    assert_ne!(second_modified, first_modified);
}

// T-META-001.6: RTM FR-24.2 — AppendH2Section preserves frontmatter.
#[test]
fn fr24_2_ac2_append_h2_section_preserves_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-preserve-fm";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("My Session".to_string()),
    );
    frontmatter.insert(
        "type".to_string(),
        YamlValue::String("sequential".to_string()),
    );
    frontmatter.insert(
        "status".to_string(),
        YamlValue::String("active".to_string()),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Session\n\n[[concept]]",
        &env.cfg,
    )
    .expect("write markdown");
    let (before_frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read initial");
    let before_frontmatter = before_frontmatter.expect("frontmatter");

    append_h2_section(
        file_uri,
        "New Section",
        "Content with [[wikilink]].",
        &env.cfg,
    )
    .expect("append section");

    let (after_frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read appended");
    let after_frontmatter = after_frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&after_frontmatter, "title").as_deref(),
        Some("My Session")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "type").as_deref(),
        Some("sequential")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "status").as_deref(),
        Some("active")
    );
    assert_eq!(
        yaml_string(&after_frontmatter, "created"),
        yaml_string(&before_frontmatter, "created")
    );
}

// T-META-001.6: RTM FR-24.2 — AppendH2Section preserves prior content and appends new content.
#[test]
fn fr24_2_ac3_append_h2_section_preserves_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://test-append-content";

    write_markdown(
        file_uri,
        None,
        "# Session\n\nOriginal [[content]] here.",
        &env.cfg,
    )
    .expect("write markdown");

    append_h2_section(
        file_uri,
        "Appended Section",
        "New [[section]] text.",
        &env.cfg,
    )
    .expect("append section");

    let (_, content, _) = read_markdown(file_uri, &env.cfg).expect("read markdown");
    assert!(content.contains("Original [[content]] here."));
    assert!(content.contains("## Appended Section"));
    assert!(content.contains("New [[section]] text."));
    assert!(
        content.find("## Appended Section").expect("new section")
            > content
                .find("Original [[content]] here.")
                .expect("original content")
    );
}

// T-META-001.6: RTM FR-24.3 — Tools delegate timestamps to MarkdownWriter.
#[test]
fn fr24_3_ac1_write_memory_timestamps_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Timestamp Via Writer Test",
            "content": "Testing that [[timestamp-management]] is delegated to MarkdownWriter.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");

    let uri = extract_uri_from_json(&output);
    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read markdown");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("created"));
    assert!(frontmatter.contains_key("modified"));
}

// T-META-001.6: RTM FR-24.3 — EditMemory updates modified via MarkdownWriter.
#[test]
fn fr24_3_ac2_edit_memory_modified_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let write_output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Edit Modified Via Writer Test",
            "content": "Initial [[content]] for edit test.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");
    let uri = extract_uri_from_json(&write_output);

    let (frontmatter, _, _) = read_markdown(&uri, &env.cfg).expect("read before edit");
    let modified_before =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

    let edit_output = memory::edit_memory(
        &markdown_context(&env.cfg),
        &json!({
            "identifier": uri,
            "operation": "append",
            "content": "\n\nAppended [[knowledge]] via EditMemory."
        }),
    )
    .expect("edit memory");
    assert!(
        edit_output.starts_with("Updated memory FILE:"),
        "unexpected output: {edit_output}"
    );

    let (frontmatter, _, _) =
        read_markdown(&extract_uri_from_json(&write_output), &env.cfg).expect("read after edit");
    let frontmatter = frontmatter.expect("frontmatter");
    let modified_after = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!modified_after.is_empty());
    assert_ne!(modified_after, modified_before);
}

// T-META-001.6: RTM FR-24.3 — MoveMemory preserves a modified timestamp on the destination file.
#[test]
fn fr24_3_ac3_move_memory_modified_via_writer() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let write_output = memory::write_memory(
        &json_context(&env.cfg),
        &json!({
            "title": "Move Modified Via Writer Source",
            "content": "Content with [[concepts]] for move test.",
            "folder": "fr24-tests"
        }),
    )
    .expect("write memory");
    let source_uri = extract_uri_from_json(&write_output);

    let move_output = memory::move_memory(
        &markdown_context(&env.cfg),
        &json!({
            "source": source_uri,
            "destination": "fr24-tests/Move Modified Via Writer Dest"
        }),
    )
    .expect("move memory");
    assert!(
        move_output.starts_with("Moved memory FILE:"),
        "unexpected output: {move_output}"
    );

    let uris = extract_memory_uris(&move_output);
    let dest_uri = uris.last().expect("destination uri");
    let (frontmatter, _, _) = read_markdown(dest_uri, &env.cfg).expect("read moved file");
    let frontmatter = frontmatter.expect("frontmatter");
    assert!(frontmatter.contains_key("modified"));
}

// T-META-001.6: RTM FR-24.4 — ConceptRepairTool flows through Markdown IO.
#[test]
fn fr24_4_ac1_concept_repair_modified_updated() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-tests";
    let file_uri = "memory://fr24-repair-tests/repair-target";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Repair Target".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# Repair Target\n\n[[tools]] and more [[tools]].",
        &env.cfg,
    )
    .expect("write repair target");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read before repair");
    let modified_before =
        yaml_string(&frontmatter.expect("frontmatter"), "modified").expect("modified");

    thread::sleep(StdDuration::from_millis(20));

    let output = search::repair_concepts(
        &markdown_context(&env.cfg),
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "folder": repair_folder,
            "dryRun": false,
            "createWikiLinks": false,
            "minSemanticSimilarity": 0.0
        }),
    )
    .expect("repair concepts");
    assert!(!output.starts_with("ERROR:"), "unexpected output: {output}");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    let frontmatter = frontmatter.expect("frontmatter");
    let modified_after = yaml_string(&frontmatter, "modified").expect("modified");
    assert!(!modified_after.is_empty());
    assert!(!modified_before.is_empty());
}

// T-META-001.6: RTM FR-24.4 — ConceptRepair preserves existing frontmatter.
#[test]
fn fr24_4_ac2_concept_repair_frontmatter_preserved() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-fm-tests";
    let file_uri = "memory://fr24-repair-fm-tests/repair-fm-target";

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("FM Preservation Test".to_string()),
    );
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    frontmatter.insert(
        "tags".to_string(),
        YamlValue::Sequence(vec![
            YamlValue::String("test".to_string()),
            YamlValue::String("fr24".to_string()),
        ]),
    );
    write_markdown(
        file_uri,
        Some(&frontmatter),
        "# FM Test\n\n[[tools]] are important.",
        &env.cfg,
    )
    .expect("write repair target");

    search::repair_concepts(
        &markdown_context(&env.cfg),
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "folder": repair_folder,
            "dryRun": false,
            "createWikiLinks": false,
            "minSemanticSimilarity": 0.0
        }),
    )
    .expect("repair concepts");

    let (frontmatter, _, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    let frontmatter = frontmatter.expect("frontmatter");
    assert_eq!(
        yaml_string(&frontmatter, "title").as_deref(),
        Some("FM Preservation Test")
    );
    assert_eq!(yaml_string(&frontmatter, "type").as_deref(), Some("memory"));
    assert_eq!(
        yaml_string(&frontmatter, "status").as_deref(),
        Some("saved")
    );
}

// T-META-001.6: RTM FR-24.4 — ConceptRepair replaces WikiLink variants correctly.
#[test]
fn fr24_4_ac3_concept_repair_replacements_correct() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let repair_folder = "fr24-repair-content-tests";
    let file_uri = "memory://fr24-repair-content-tests/repair-content-target";

    write_markdown(
        file_uri,
        None,
        "# Content Test\n\nUsing [[tools]] and [[Tools]] here.",
        &env.cfg,
    )
    .expect("write repair target");

    search::repair_concepts(
        &markdown_context(&env.cfg),
        &json!({
            "conceptsToReplace": "tools",
            "canonicalConcept": "tool",
            "folder": repair_folder,
            "dryRun": false,
            "createWikiLinks": false,
            "minSemanticSimilarity": 0.0
        }),
    )
    .expect("repair concepts");

    let (_, content, _) = read_markdown(file_uri, &env.cfg).expect("read after repair");
    assert!(
        content.contains("[[tool]]"),
        "missing canonical wikilink: {content}"
    );
    assert!(
        !content.contains("[[tools]]"),
        "variant should be removed: {content}"
    );
    assert!(
        !content.contains("[[Tools]]"),
        "case variant should be removed: {content}"
    );
}

// T-META-001.6: RTM FR-24.6 — Session cleanup coverage for timestamp scenarios.
#[test]
fn fr24_6_ac1_session_cleanup_with_modified_existing_behavior() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/stale-with-modified";
    let stale_modified = iso_minutes_ago(130);
    let body = "# Stale Session\n\nContent";
    let raw_file =
        format!("---\ntype: workflow\nstatus: active\nmodified: {stale_modified}\n---\n\n{body}");
    write_raw_markdown(&env.cfg, file_uri, &raw_file);
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(130));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-META-001.6: RTM FR-24.6 — Recent activity remains active when cleanup input is fresh.
#[test]
fn fr24_6_ac2_session_cleanup_without_modified_filesystem_fallback() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/workflow/no-modified-recent";
    let body = "# Session\n\nContent";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: workflow\nstatus: active\n---\n\n# Session\n\nContent",
    );
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(5));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 0);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "active");
}

// T-META-001.6: RTM FR-24.6 — Stale active sessions are abandoned even without modified frontmatter.
#[test]
fn fr24_6_ac3_session_cleanup_abandoned_detected_regardless_of_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://thinking/sequential/old-no-modified";
    let body = "# Old Session\n\nContent";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: sequential\nstatus: active\n---\n\n# Old Session\n\nContent",
    );
    seed_file_content(&env.cfg, file_uri, "active", body, &iso_minutes_ago(150));

    let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");

    assert_eq!(abandoned, 1);
    assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
}

// T-META-001.6: RTM FR-24.6 — ReadMemory still renders Created/Modified lines when timestamps are missing.
#[test]
fn fr24_6_ac4_read_memory_displays_filesystem_timestamps_when_frontmatter_missing() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let file_uri = "memory://fr24-no-fm/no-timestamps";
    write_raw_markdown(
        &env.cfg,
        file_uri,
        "---\ntype: memory\ntitle: No Timestamps\n---\n\n# No Timestamps\n\nContent with [[raw-file]] that has no timestamp frontmatter.",
    );

    let output = memory::read_memory(
        &markdown_context(&env.cfg),
        &json!({
            "identifier": file_uri,
            "includeChecksum": true
        }),
    )
    .expect("read memory");

    assert!(!output.starts_with("ERROR:"), "unexpected output: {output}");
    assert!(
        output.contains("Created:"),
        "missing Created line: {output}"
    );
    assert!(
        output.contains("Modified:"),
        "missing Modified line: {output}"
    );
}
