use chrono::{Duration, Utc};
use maenifold_core::db;
use maenifold_core::json::OutputFormat;
use maenifold_core::markdown;
use maenifold_core::memory;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }
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
    fn with_csharp_defaults() -> Self {
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

        // T-W1-1: RTM FR-memory-tools parity
        unsafe {
            std::env::set_var("MAENIFOLD_DECAY_FUNCTION", "power-law");
            std::env::set_var("MAENIFOLD_DECAY_HALF_LIFE_DAYS", "30");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", "7");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", "14");
            std::env::set_var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", "14");
        }

        Self { previous }
    }
}

impl Drop for DecayEnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            if let Some(value) = value {
                unsafe {
                    std::env::set_var(key, value);
                }
            } else {
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
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

fn extract_uri(write_result: &str) -> String {
    write_result
        .lines()
        .find_map(|line| {
            line.find("memory://")
                .map(|index| line[index..].trim().to_string())
        })
        .expect("memory:// URI in write result")
}

fn write_memory_file(env: &TestEnv, title: &str, content: &str, folder: Option<&str>) -> String {
    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory");
    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    extract_uri(&result)
}

fn read_memory_output(env: &TestEnv, identifier: &str) -> String {
    memory::read_memory(&env.ctx(), &json!({ "identifier": identifier })).expect("read memory")
}

fn edit_memory_output(env: &TestEnv, payload: serde_json::Value) -> String {
    memory::edit_memory(&env.ctx(), &payload).expect("edit memory")
}

fn move_memory_output(env: &TestEnv, source: &str, destination: &str) -> String {
    memory::move_memory(
        &env.ctx(),
        &json!({
            "source": source,
            "destination": destination,
        }),
    )
    .expect("move memory")
}

fn list_memories_output(env: &TestEnv, path: &str) -> String {
    memory::list_memories(&env.ctx(), &json!({ "path": path })).expect("list memories")
}

fn write_raw_memory(env: &TestEnv, relative_path: &str, content: &str) -> String {
    let file_path = env.cfg.memory_path.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(&file_path, content).expect("write raw memory");
    markdown::path_to_uri(&file_path, &env.cfg.memory_path).expect("path to uri")
}

fn raw_path_for_uri(env: &TestEnv, uri: &str) -> std::path::PathBuf {
    markdown::uri_to_path(uri, &env.cfg.memory_path).expect("uri to path")
}

fn block_for_file<'a>(output: &'a str, file_stem: &str) -> &'a str {
    let marker = format!("📄 {file_stem}.md");
    let start = output.find(&marker).expect("file block present");
    let rest = &output[start..];
    let end = rest
        .get(marker.len()..)
        .and_then(|tail| tail.find("\n📄 ").map(|index| index + marker.len()))
        .unwrap_or(rest.len());
    &rest[..end]
}

fn metadata_value(output: &str, file_stem: &str, key: &str) -> String {
    let block = block_for_file(output, file_stem);
    block
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&format!("{key}: ")).map(str::to_string))
        .unwrap_or_else(|| panic!("missing {key} in block:\n{block}"))
}

fn set_last_accessed(env: &TestEnv, uri: &str, last_accessed: &str) {
    let conn = db::open_connection(&env.cfg.database_path).expect("open db");
    conn.execute(
        "INSERT OR REPLACE INTO file_content (file_path, last_indexed, last_accessed) VALUES (?1, ?2, ?3)",
        params![uri, Utc::now().to_rfc3339(), last_accessed],
    )
    .expect("insert last_accessed");
}

// T-W1-1: RTM FR-memory-tools parity
#[test]
fn move_memory_with_path_based_destination_preserves_markdown_extension() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Move Extension Test",
        "# Move Extension Test\n\nTesting [[file-operations]] preserves .md extension.",
        None,
    );

    let move_result = move_memory_output(&env, &uri, "regression-test/Moved Extension Test");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://regression-test/moved-extension-test"));

    let read_result = read_memory_output(&env, "memory://regression-test/moved-extension-test");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[file-operations]] preserves .md extension."));
}

#[test]
fn move_memory_with_simple_destination_preserves_existing_behavior() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Simple Move Test",
        "# Simple Move Test\n\nTesting [[refactoring]] functionality.",
        None,
    );

    let move_result = move_memory_output(&env, &uri, "Simple Move Renamed");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://simple-move-renamed"));

    let read_result = read_memory_output(&env, "memory://simple-move-renamed");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[refactoring]] functionality."));
}

#[test]
fn read_memory_handles_missing_title_in_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_raw_memory(
        &env,
        "memory-tools-tests/no-title-test.md",
        "---\ntype: memory\nstatus: saved\n---\n\n# Test Content\n\nThis is a [[metadata-handling]] without a title in frontmatter.\n",
    );

    let read_result = read_memory_output(&env, &uri);

    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("# no-title-test"));
    assert!(read_result.contains("URI: memory://memory-tools-tests/no-title-test"));
    assert!(
        read_result.contains("This is a [[metadata-handling]] without a title in frontmatter.")
    );
}

#[test]
fn read_memory_handles_empty_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_raw_memory(
        &env,
        "memory-tools-tests/no-frontmatter-test.md",
        "# Test Without Frontmatter\n\nThis is a [[yaml-frontmatter]] without any frontmatter.\n",
    );

    let read_result = read_memory_output(&env, &uri);

    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("# no-frontmatter-test"));
    assert!(read_result.contains("This is a [[yaml-frontmatter]] without any frontmatter."));
}

#[test]
fn write_memory_does_not_write_embedding_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Embedding Removal Test",
        "Content with [[vector-embeddings]] concept to ensure write path works.",
        Some("memory-tools-tests"),
    );
    let raw = fs::read_to_string(raw_path_for_uri(&env, &uri)).expect("read raw file");

    assert!(!raw.contains("embedding_base64"));
    assert!(!raw.contains("embedding_model"));
    assert!(!raw.contains("embedding_date"));
}

#[test]
fn edit_memory_replace_section_replaces_existing_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Section Test",
        "This is the intro with a [[markdown-sections]].\n\n## First Section\n\nOld content in first section.\n\n## Second Section\n\nOld content in second section with [[document-structure]].\n\n## Third Section\n\nOld content in third section.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content in second section with [[content-editing]].\n\nMultiple lines work too.",
            "sectionName": "Second Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Second Section"));
    assert!(read_result.contains("New content in second section with [[content-editing]]"));
    assert!(read_result.contains("Multiple lines work too."));
    assert!(!read_result.contains("Old content in second section"));
    assert!(!read_result.contains("[[document-structure]]"));
    assert!(read_result.contains("## First Section"));
    assert!(read_result.contains("Old content in first section."));
    assert!(read_result.contains("## Third Section"));
    assert!(read_result.contains("Old content in third section."));
}

#[test]
fn edit_memory_replace_section_throws_when_section_not_found() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Section Not Found Test",
        "This is the intro with a [[error-handling]].\n\n## Existing Section\n\nSome content here.",
        Some("memory-tools-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content with [[validation]].",
            "sectionName": "Nonexistent Section"
        }),
    )
    .expect_err("missing section should error");

    assert!(err.to_string().contains("Section not found"));
    assert!(err.to_string().contains("Nonexistent Section"));
}

#[test]
fn edit_memory_replace_section_works_with_different_heading_levels() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Heading Levels Test",
        "Intro with [[markdown-hierarchy]].\n\n# Level 1 Heading\n\nContent under level 1.\n\n## Level 2 Heading\n\nContent under level 2.\n\n### Level 3 Heading\n\nContent under level 3.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New content for level 2 with [[section-replacement]].",
            "sectionName": "Level 2 Heading"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Level 2 Heading"));
    assert!(read_result.contains("New content for level 2 with [[section-replacement]]"));
    assert!(!read_result.contains("Content under level 2."));
    assert!(read_result.contains("# Level 1 Heading"));
    assert!(read_result.contains("Content under level 1."));
    assert!(read_result.contains("### Level 3 Heading"));
    assert!(read_result.contains("Content under level 3."));
}

#[test]
fn edit_memory_replace_section_replaces_last_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Last Section Test",
        "Intro with [[boundary-conditions]].\n\n## First Section\n\nFirst content.\n\n## Last Section\n\nThis is the last section content.",
        Some("memory-tools-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New last section content with [[document-end]].",
            "sectionName": "Last Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Last Section"));
    assert!(read_result.contains("New last section content with [[document-end]]"));
    assert!(!read_result.contains("This is the last section content."));
    assert!(read_result.contains("## First Section"));
    assert!(read_result.contains("First content."));
}

#[test]
fn write_memory_with_existing_h1_does_not_create_duplicate_heading() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Duplicate H1 Test",
        "# Existing Heading\n\nContent with [[concept]]",
        Some("memory-tools-tests"),
    );

    let raw = fs::read_to_string(raw_path_for_uri(&env, &uri)).expect("read raw file");
    let h1_count = raw
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("# ") && !trimmed.starts_with("## ")
        })
        .count();

    assert_eq!(h1_count, 1, "unexpected raw content:\n{raw}");
    assert!(raw.contains("# Existing Heading"));

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Content with [[concept]]"));
}

#[test]
fn sanitize_user_input_blocks_url_encoded_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%2e%2e%2fmalicious",
            "content": "Safe content with [[security-validation]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
    assert!(result.contains("testmalicious"));

    let read_result = read_memory_output(&env, "memory://memory-tools-tests/testmalicious");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Safe content with [[security-validation]]"));
}

#[test]
fn sanitize_user_input_blocks_double_url_encoded_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%252e%252e%252fmalicious",
            "content": "Safe content with [[input-sanitization]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
}

#[test]
fn sanitize_user_input_blocks_url_encoded_dot_dot() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%2e%2e",
            "content": "Safe content with [[path-traversal-prevention]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
}

#[test]
fn sanitize_user_input_blocks_mixed_encoded_and_plain_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "%2e./test",
            "content": "Safe content with [[url-encoding]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));

    let read_result = read_memory_output(&env, "memory://memory-tools-tests/test");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
}

#[test]
fn sanitize_user_input_preserves_legitimate_percent_signs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "Task 50 Complete",
            "content": "Status update with [[task-tracking]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    let uri = extract_uri(&result);
    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Status update with [[task-tracking]]"));
}

#[test]
fn find_replace_with_correct_expected_count_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Find Replace Test",
        "This is a [[test]] with foo appearing twice: foo and foo.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[bar]]",
            "findText": "foo",
            "expectedCount": 3
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[bar]] appearing twice: [[bar]] and [[bar]]"));
    assert!(!read_result.contains("foo"));
}

#[test]
fn find_replace_with_incorrect_expected_count_throws_argument_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Find Replace Mismatch Test",
        "This is a [[test]] with foo appearing twice: foo and foo.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "foo",
            "expectedCount": 1
        }),
    )
    .expect_err("expected count mismatch should error");

    assert!(err.to_string().contains("Expected 1 matches but found 3"));
    assert!(err.to_string().contains("Find text: 'foo'"));
}

#[test]
fn find_replace_with_expected_count_zero_throws_when_matches_found() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Zero Expected Test",
        "This is a [[test]] with foo in it.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "foo",
            "expectedCount": 0
        }),
    )
    .expect_err("expected count zero should error");

    assert!(err.to_string().contains("Expected 0 matches but found 1"));
}

#[test]
fn find_replace_with_null_expected_count_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Count Validation Test",
        "This is a [[test]] with foo appearing multiple times: foo, foo, foo.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "bar",
            "findText": "foo",
            "expectedCount": null
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("bar appearing multiple times: bar, bar, bar."));
}

#[test]
fn find_replace_with_no_matches_throws_when_expecting_matches() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Matches Test",
        "This is a [[test]] without the target text.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "nonexistent",
            "expectedCount": 2
        }),
    )
    .expect_err("no matches should error");

    assert!(err.to_string().contains("Expected 2 matches but found 0"));
    assert!(err.to_string().contains("Find text: 'nonexistent'"));
}

#[test]
fn find_replace_with_multiline_pattern_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiline Replace Test",
        "This is a [[test]] file.\n\nFirst paragraph with [[content]].\n\nSecond paragraph with [[more-content]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "section",
            "findText": "paragraph",
            "expectedCount": 2
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("First section with [[content]]"));
    assert!(read_result.contains("Second section with [[more-content]]"));
}

#[test]
fn append_with_concepts_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Append Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "append",
            "content": "Appended content with [[more-concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Original content with [[test]]."));
    assert!(read_result.contains("Appended content with [[more-concepts]]."));
}

#[test]
fn prepend_with_concepts_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Prepend Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "prepend",
            "content": "Prepended content with [[new-concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Prepended content with [[new-concepts]]."));
    assert!(read_result.contains("Original content with [[test]]."));
}

#[test]
fn edit_memory_without_concepts_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Concepts Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "append",
            "content": "Content without any concepts."
        }),
    );

    assert!(
        edit_result.starts_with("ERROR:"),
        "unexpected: {edit_result}"
    );
    assert!(edit_result.contains("must contain at least one [[concept]]"));
}

#[test]
fn replace_section_when_section_exists_replaces_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Section Test",
        "# Introduction\n\nThis is the introduction section with [[documentation-structure]].\n\n## Methodology\n\nThis is the OLD methodology content that will be replaced.\n\n## Results\n\nThese are the results section with [[data]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Methodology\n\nThis is the NEW methodology content with [[improved-approach]] and [[better-techniques]].",
            "sectionName": "Methodology"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("NEW methodology content"));
    assert!(read_result.contains("[[improved-approach]]"));
    assert!(read_result.contains("[[better-techniques]]"));
    assert!(!read_result.contains("OLD methodology content"));
    assert!(read_result.contains("introduction section with [[documentation-structure]]"));
    assert!(read_result.contains("results section with [[data]]"));
}

#[test]
fn replace_section_when_section_does_not_exist_throws_invalid_operation() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Missing Section Test",
        "# Introduction\n\nThis is the introduction with [[section-replacement]].\n\n## Results\n\nThese are the results with [[data]].",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Methodology\n\nNew content with [[test]].",
            "sectionName": "Methodology"
        }),
    )
    .expect_err("missing section should error");

    assert!(err.to_string().contains("Methodology"));
}

#[test]
fn replace_section_with_h1_heading_replaces_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "H1 Replace Test",
        "# First Section\n\nContent of first section with [[alpha]].\n\n# Second Section\n\nContent of second section with [[beta]].\n\n# Third Section\n\nContent of third section with [[gamma]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "# Second Section\n\nREPLACED content with [[delta]] and [[epsilon]].",
            "sectionName": "Second Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("REPLACED content"));
    assert!(read_result.contains("[[delta]]"));
    assert!(!read_result.contains("[[beta]]"));
    assert!(read_result.contains("[[alpha]]"));
    assert!(read_result.contains("[[gamma]]"));
}

#[test]
fn replace_section_with_h3_heading_replaces_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "H3 Replace Test",
        "## Main Section\n\nMain content with [[primary]].\n\n### Subsection A\n\nOld subsection A content with [[old-concept]].\n\n### Subsection B\n\nSubsection B content with [[secondary]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "### Subsection A\n\nNew subsection A content with [[new-concept]] and [[innovative-idea]].",
            "sectionName": "Subsection A"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("New subsection A content"));
    assert!(read_result.contains("[[new-concept]]"));
    assert!(!read_result.contains("[[old-concept]]"));
    assert!(read_result.contains("[[primary]]"));
    assert!(read_result.contains("[[secondary]]"));
}

#[test]
fn replace_section_old_content_completely_removed_not_appended() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Complete Removal Test",
        "# Test\n\nInitial content with [[initial-setup]].\n\n## Target Section\n\nOld paragraph one with [[deprecated-approach]].\nOld paragraph two with [[legacy-implementation]].\nOld paragraph three with [[outdated-pattern]].\n\n## Next Section\n\nNext section content with [[future-work]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Target Section\n\nBrand new single paragraph with [[brand-new]].",
            "sectionName": "Target Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Brand new single paragraph"));
    assert!(read_result.contains("[[brand-new]]"));
    assert!(!read_result.contains("Old paragraph one"));
    assert!(!read_result.contains("Old paragraph two"));
    assert!(!read_result.contains("Old paragraph three"));
    assert!(!read_result.contains("[[deprecated-approach]]"));
    assert!(!read_result.contains("[[legacy-implementation]]"));
    assert!(!read_result.contains("[[outdated-pattern]]"));
    assert!(read_result.contains("[[initial-setup]]"));
    assert!(read_result.contains("[[future-work]]"));
}

#[test]
fn find_replace_replacing_text_inside_wikilink_creates_nested_brackets() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "WikiLink Nested Brackets Test",
        "This file has a [[Machine Learning]] concept.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Deep Learning]]",
            "findText": "Learning",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("ERROR:"),
        "unexpected: {edit_result}"
    );
    assert!(edit_result.contains("must contain at least one [[concept]]"));
}

#[test]
fn find_replace_replacing_entire_wikilink_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Entire WikiLink Test",
        "This file references [[Machine Learning]] and [[Deep Learning]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Artificial Intelligence]]",
            "findText": "[[Machine Learning]]",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[Artificial Intelligence]]"));
    assert!(!read_result.contains("[[Machine Learning]]"));
    assert!(read_result.contains("[[Deep Learning]]"));
}

#[test]
fn find_replace_multiple_wikilinks_with_same_inner_text_replaces_all() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiple WikiLinks Same Text Test",
        "We study [[Neural Networks]] and [[Convolutional Neural Networks]].\nAlso see [[Recurrent Neural Networks]] and [[Neural Networks Architecture]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "NN",
            "findText": "Neural Networks",
            "expectedCount": 4
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[NN]]"));
    assert!(read_result.contains("[[Convolutional NN]]"));
    assert!(read_result.contains("[[Recurrent NN]]"));
    assert!(read_result.contains("[[NN Architecture]]"));
    assert!(!read_result.contains("Neural Networks"));
}

#[test]
fn find_replace_replacing_with_multiple_wikilinks_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiple WikiLinks Replace Test",
        "This topic covers [[AI]] fundamentals.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Machine Learning]], [[Deep Learning]], and [[Neural Networks]]",
            "findText": "fundamentals",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[Machine Learning]]"));
    assert!(read_result.contains("[[Deep Learning]]"));
    assert!(read_result.contains("[[Neural Networks]]"));
}

#[test]
fn list_memories_output_includes_created_timestamp() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/created-test.md",
        "---\ntitle: Created Test\ntype: memory\nstatus: saved\ncreated: 2026-01-15T10:30:00Z\nmodified: 2026-01-15T10:30:00Z\n---\n\n# Created Test\n\nTest content for [[decay-metadata]] created timestamp verification.\n",
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");

    assert!(output.contains("created"));
    assert!(metadata_value(&output, "created-test", "created").contains("2026-01-15"));
}

#[test]
fn list_memories_output_includes_last_accessed_timestamp() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let uri = write_raw_memory(
        &env,
        "decay-metadata-tests/standard/accessed-test.md",
        "---\ntitle: Accessed Test\ntype: memory\nstatus: saved\ncreated: 2026-01-10T08:00:00Z\nmodified: 2026-01-10T08:00:00Z\n---\n\n# Accessed Test\n\nTest content for [[decay-metadata]] last_accessed timestamp verification.\n",
    );
    set_last_accessed(&env, &uri, "2026-01-20T14:45:00Z");

    let output = list_memories_output(&env, "decay-metadata-tests/standard");

    assert!(output.contains("last_accessed"));
    assert!(metadata_value(&output, "accessed-test", "last_accessed").contains("2026-01-20"));
}

#[test]
fn list_memories_output_includes_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(7)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/decay-weight-test.md",
        &format!(
            "---\ntitle: Decay Weight Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Decay Weight Test\n\nTest content for [[decay-metadata]] decay_weight verification.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "decay-weight-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert!(output.contains("decay_weight"));
    assert!((0.0..=1.0).contains(&weight), "unexpected weight: {weight}");
}

#[test]
fn list_memories_sequential_file_uses_7_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(6)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/sequential/decay-test/session-grace-test.md",
        &format!(
            "---\ntitle: Sequential Grace Test\ntype: sequential\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Sequential Grace Test\n\nTest content for [[decay-metadata]] sequential tier 7d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/sequential/decay-test");
    let weight = metadata_value(&output, "session-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_workflow_file_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/workflows/decay-test/workflow-grace-test.md",
        &format!(
            "---\ntitle: Workflow Grace Test\ntype: workflow\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Workflow Grace Test\n\nTest content for [[decay-metadata]] workflow tier 14d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/workflows/decay-test");
    let weight = metadata_value(&output, "workflow-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_standard_file_uses_14_day_grace_period() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(10)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/standard-grace-test.md",
        &format!(
            "---\ntitle: Standard Grace Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Standard Grace Test\n\nTest content for [[decay-metadata]] standard memory 14d grace verification.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "standard-grace-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_sequential_file_past_grace_period_has_decayed_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(14)).to_rfc3339();
    write_raw_memory(
        &env,
        "thinking/sequential/decay-test/session-decayed-test.md",
        &format!(
            "---\ntitle: Sequential Decayed Test\ntype: sequential\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# Sequential Decayed Test\n\nTest content for [[decay-metadata]] sequential tier decay after grace period.\n"
        ),
    );

    let output = list_memories_output(&env, "thinking/sequential/decay-test");
    let weight = metadata_value(&output, "session-decayed-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert!(weight < 1.0, "unexpected output:\n{output}");
}

#[test]
fn list_memories_file_without_last_accessed_uses_fallback() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    let created_date = (Utc::now() - Duration::days(5)).to_rfc3339();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/no-accessed-test.md",
        &format!(
            "---\ntitle: No Accessed Test\ntype: memory\nstatus: saved\ncreated: {created_date}\nmodified: {created_date}\n---\n\n# No Accessed Test\n\nTest content for [[decay-metadata]] fallback when last_accessed is missing.\n"
        ),
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");

    assert!(output.contains("last_accessed"));
    assert!(output.contains("decay_weight"));
    assert!(!metadata_value(&output, "no-accessed-test", "last_accessed").is_empty());
}

#[test]
fn list_memories_new_file_has_full_decay_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let _decay = DecayEnvGuard::with_csharp_defaults();
    write_raw_memory(
        &env,
        "decay-metadata-tests/standard/new-file-test.md",
        "---\ntitle: New File Test\ntype: memory\nstatus: saved\n---\n\n# New File Test\n\nTest content for [[decay-metadata]] new file full weight verification.\n",
    );

    let output = list_memories_output(&env, "decay-metadata-tests/standard");
    let weight = metadata_value(&output, "new-file-test", "decay_weight")
        .parse::<f64>()
        .expect("decay weight float");

    assert_eq!(weight, 1.0, "unexpected output:\n{output}");
}
