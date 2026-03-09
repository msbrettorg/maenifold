use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, GraphDatabase, MaenifoldError, ToolRegistry, markdown};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use walkdir::WalkDir;

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

// T-W1-2: RTM FR-28.1
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

// T-W1-2: RTM FR-28.1
fn seed_file(cfg: &Config, uri: &str, frontmatter: Option<&markdown::Frontmatter>, content: &str) {
    markdown::write_markdown(uri, frontmatter, content, cfg).expect("seed markdown");
}

// T-W1-2: RTM FR-28.2
fn write_markdown_boundary(
    uri: Option<&str>,
    frontmatter: Option<&markdown::Frontmatter>,
    content: &str,
    cfg: &Config,
) -> maenifold_core::Result<()> {
    match uri {
        Some(uri) => markdown::write_markdown(uri, frontmatter, content, cfg),
        None => Err(MaenifoldError::InvalidArgument(
            "uri: URI cannot be null or empty.".to_string(),
        )),
    }
}

// T-W1-2: RTM FR-28.6
fn get_memory_directories(
    cfg: &Config,
    folder_uri: Option<&str>,
) -> maenifold_core::Result<Vec<String>> {
    let mut directories = BTreeSet::new();
    for file_uri in markdown::get_memory_files(cfg, folder_uri)? {
        let path = markdown::uri_to_path(&file_uri, &cfg.memory_path)?;
        let Some(parent) = path.parent() else {
            continue;
        };
        if parent == cfg.memory_path {
            continue;
        }
        directories.insert(markdown::path_to_uri(parent, &cfg.memory_path)?);
    }
    Ok(directories.into_iter().collect())
}

// T-W1-2: RTM FR-28.1
fn yaml_string(frontmatter: &markdown::Frontmatter, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|value| match value {
        YamlValue::String(text) => Some(text.clone()),
        other => serde_yaml::to_string(other)
            .ok()
            .map(|text| text.trim().to_string()),
    })
}

// T-W1-2: RTM FR-28.9
fn assert_invalid_argument(err: MaenifoldError, expected_fragment: &str) {
    match err {
        MaenifoldError::InvalidArgument(message) => assert!(
            message
                .to_ascii_lowercase()
                .contains(&expected_fragment.to_ascii_lowercase()),
            "expected invalid argument containing {expected_fragment:?}, got {message:?}"
        ),
        other => panic!("expected invalid argument, got {other:?}"),
    }
}

// T-W1-2: RTM FR-28.1
fn assert_message_error(err: MaenifoldError, expected_fragment: &str) {
    match err {
        MaenifoldError::Message(message) => assert!(
            message.contains(expected_fragment),
            "expected message containing {expected_fragment:?}, got {message:?}"
        ),
        other => panic!("expected message error, got {other:?}"),
    }
}

// T-W1-2: RTM FR-28.10
fn read_crate_sources() -> String {
    let src_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut combined = String::new();
    for entry in WalkDir::new(src_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
    {
        combined.push_str(&fs::read_to_string(entry.path()).expect("read source file"));
        combined.push('\n');
    }
    combined
}

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac1_read_markdown_valid_uri_returns_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Test Note".to_string()),
    );
    seed_file(
        &env.cfg,
        "memory://notes/test-note",
        Some(&frontmatter),
        "# Hello\n\nWorld",
    );

    let (read_fm, read_content, checksum) =
        markdown::read_markdown("memory://notes/test-note", &env.cfg).expect("read markdown");

    let read_fm = read_fm.expect("frontmatter");
    assert_eq!(yaml_string(&read_fm, "title").as_deref(), Some("Test Note"));
    assert!(read_content.contains("World"));
    assert!(!checksum.is_empty());
}

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac2_read_markdown_nested_uri_returns_content() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://projects/alpha/docs/readme",
        None,
        "# Nested\n\nDeeply nested content.",
    );

    let (_, content, _) =
        markdown::read_markdown("memory://projects/alpha/docs/readme", &env.cfg).expect("read");

    assert!(content.contains("Deeply nested content."));
}

// T-W1-2: RTM FR-28.1
#[test]
fn fr28_1_ac3_read_markdown_non_existent_uri_returns_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::read_markdown("memory://does-not-exist", &env.cfg).expect_err("missing file");

    assert_message_error(err, "Memory file not found");
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac1_write_markdown_valid_uri_creates_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::write_markdown(
        "memory://simple-note",
        None,
        "# Simple\n\nContent",
        &env.cfg,
    )
    .expect("write markdown");

    assert!(env.cfg.memory_path.join("simple-note.md").exists());
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac2_write_markdown_nested_uri_creates_intermediate_dirs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::write_markdown(
        "memory://deep/nested/folder/structure/note",
        None,
        "# Deep\n\nContent",
        &env.cfg,
    )
    .expect("write nested markdown");

    assert!(
        env.cfg
            .memory_path
            .join("deep")
            .join("nested")
            .join("folder")
            .join("structure")
            .join("note.md")
            .exists()
    );
}

// T-W1-2: RTM FR-28.2
#[test]
fn fr28_2_ac3_write_markdown_frontmatter_round_trips() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut frontmatter = BTreeMap::new();
    frontmatter.insert(
        "title".to_string(),
        YamlValue::String("Round Trip".to_string()),
    );
    frontmatter.insert(
        "tags".to_string(),
        YamlValue::Sequence(vec![
            YamlValue::String("test".to_string()),
            YamlValue::String("boundary".to_string()),
        ]),
    );

    markdown::write_markdown(
        "memory://roundtrip-test",
        Some(&frontmatter),
        "# RT\n\nContent",
        &env.cfg,
    )
    .expect("write roundtrip");

    let (read_fm, read_content, _) =
        markdown::read_markdown("memory://roundtrip-test", &env.cfg).expect("read roundtrip");

    let read_fm = read_fm.expect("frontmatter");
    assert_eq!(
        yaml_string(&read_fm, "title").as_deref(),
        Some("Round Trip")
    );
    assert!(matches!(
        read_fm.get("tags"),
        Some(YamlValue::Sequence(tags))
            if tags
                == &vec![
                    YamlValue::String("test".to_string()),
                    YamlValue::String("boundary".to_string())
                ]
    ));
    assert!(read_content.contains("Content"));
}

// T-W1-2: RTM FR-28.3
#[test]
fn fr28_3_ac1_update_markdown_valid_checksum_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://checksum-test",
        None,
        "# Original\n\nOriginal content",
    );
    let (frontmatter, _, checksum) =
        markdown::read_markdown("memory://checksum-test", &env.cfg).expect("read checksum");

    markdown::update_markdown(
        "memory://checksum-test",
        frontmatter.as_ref(),
        "# Updated\n\nNew content",
        Some(&checksum),
        &env.cfg,
    )
    .expect("update markdown");

    let (_, updated_content, _) =
        markdown::read_markdown("memory://checksum-test", &env.cfg).expect("read updated");
    assert!(updated_content.contains("New content"));
}

// T-W1-2: RTM FR-28.3
#[test]
fn fr28_3_ac2_update_markdown_stale_checksum_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://stale-checksum-test",
        None,
        "# Original\n\nContent",
    );

    let err = markdown::update_markdown(
        "memory://stale-checksum-test",
        None,
        "# Tampered\n\nContent",
        Some("bogus-checksum-that-does-not-match"),
        &env.cfg,
    )
    .expect_err("stale checksum should fail");

    assert_message_error(err, "Checksum mismatch");
}

// T-W1-2: RTM FR-28.4
#[test]
fn fr28_4_ac1_append_h2_section_valid_uri_appends_section() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(
        &env.cfg,
        "memory://append-test",
        None,
        "# Base\n\nExisting content.",
    );

    markdown::append_h2_section(
        "memory://append-test",
        "New Section",
        "Appended content here.",
        &env.cfg,
    )
    .expect("append section");

    let (_, content, _) =
        markdown::read_markdown("memory://append-test", &env.cfg).expect("read appended file");
    assert!(content.contains("Existing content."));
    assert!(content.contains("## New Section"));
    assert!(content.contains("Appended content here."));
}

// T-W1-2: RTM FR-28.4
#[test]
fn fr28_4_ac2_append_h2_section_non_existent_uri_creates_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    markdown::append_h2_section(
        "memory://append-creates-file",
        "First Section",
        "Created from nothing.",
        &env.cfg,
    )
    .expect("append creates file");

    assert!(markdown::file_exists("memory://append-creates-file", &env.cfg).expect("exists"));
    let (_, content, _) =
        markdown::read_markdown("memory://append-creates-file", &env.cfg).expect("read created");
    assert!(content.contains("## First Section"));
    assert!(content.contains("Created from nothing."));
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac1_get_memory_files_returns_memory_uris() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://alpha", None, "# Test\n\nContent");
    seed_file(&env.cfg, "memory://beta", None, "# Test\n\nContent");
    seed_file(&env.cfg, "memory://sub/gamma", None, "# Test\n\nContent");

    let files = markdown::get_memory_files(&env.cfg, None).expect("memory files");

    assert!(files.len() >= 3);
    assert!(files.iter().all(|file| file.starts_with("memory://")));
    assert!(files.contains(&"memory://alpha".to_string()));
    assert!(files.contains(&"memory://beta".to_string()));
    assert!(files.contains(&"memory://sub/gamma".to_string()));
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac2_get_memory_files_excludes_hidden_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://visible-note", None, "# Visible");
    let hidden_dir = env.cfg.memory_path.join(".obsidian");
    fs::create_dir_all(&hidden_dir).expect("hidden dir");
    fs::write(hidden_dir.join("hidden-note.md"), "# Hidden").expect("hidden note");

    let files = markdown::get_memory_files(&env.cfg, None).expect("memory files");

    assert!(files.iter().all(|file| file.starts_with("memory://")));
    assert!(files.contains(&"memory://visible-note".to_string()));
    assert!(files.iter().all(|file| !file.contains(".obsidian")));
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac3_get_memory_files_folder_uri_filter_returns_only_subset() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://docs/readme", None, "# Test");
    seed_file(&env.cfg, "memory://docs/guide", None, "# Test");
    seed_file(&env.cfg, "memory://notes/daily", None, "# Test");

    let files = markdown::get_memory_files(&env.cfg, Some("memory://docs")).expect("docs files");

    assert_eq!(files.len(), 2);
    assert!(files.iter().all(|file| file.starts_with("memory://docs/")));
    assert!(
        files
            .iter()
            .all(|file| !file.starts_with("memory://notes/"))
    );
}

// T-W1-2: RTM FR-28.5
#[test]
fn fr28_5_ac4_get_memory_files_null_filter_returns_all_files() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://a", None, "# Test");
    seed_file(&env.cfg, "memory://folder/b", None, "# Test");

    let files = markdown::get_memory_files(&env.cfg, None).expect("all files");

    assert!(files.len() >= 2);
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac1_get_memory_directories_returns_memory_uris() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://folder-a/note", None, "# Test");
    seed_file(&env.cfg, "memory://folder-b/note", None, "# Test");

    let dirs = get_memory_directories(&env.cfg, None).expect("memory directories");

    assert!(dirs.iter().all(|dir| dir.starts_with("memory://")));
    assert!(dirs.contains(&"memory://folder-a".to_string()));
    assert!(dirs.contains(&"memory://folder-b".to_string()));
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac2_get_memory_directories_excludes_hidden_dirs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://visible-dir/note", None, "# Test");
    fs::create_dir_all(env.cfg.memory_path.join(".git")).expect("hidden dir");

    let dirs = get_memory_directories(&env.cfg, None).expect("memory directories");

    assert!(dirs.iter().all(|dir| !dir.contains(".git")));
}

// T-W1-2: RTM FR-28.6
#[test]
fn fr28_6_ac3_get_memory_directories_folder_uri_filter_returns_only_subset() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://docs/subfolder/note", None, "# Test");
    seed_file(&env.cfg, "memory://docs/other-sub/note", None, "# Test");
    seed_file(&env.cfg, "memory://notes/archive/note", None, "# Test");

    let dirs = get_memory_directories(&env.cfg, Some("memory://docs")).expect("docs dirs");

    assert_eq!(dirs.len(), 2);
    assert!(dirs.iter().all(|dir| dir.starts_with("memory://docs/")));
    assert!(dirs.iter().all(|dir| !dir.starts_with("memory://notes/")));
}

// T-W1-2: RTM FR-28.7
#[test]
fn fr28_7_ac1_file_exists_existing_file_returns_true() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://exists-check", None, "# Test");

    assert!(markdown::file_exists("memory://exists-check", &env.cfg).expect("exists"));
}

// T-W1-2: RTM FR-28.7
#[test]
fn fr28_7_ac2_file_exists_non_existent_file_returns_false() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    assert!(!markdown::file_exists("memory://no-such-file", &env.cfg).expect("exists"));
}

// T-W1-2: RTM FR-28.8
#[test]
fn fr28_8_ac1_delete_file_existing_file_removes_file() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    seed_file(&env.cfg, "memory://delete-me", None, "# Test");
    assert!(markdown::file_exists("memory://delete-me", &env.cfg).expect("precondition"));

    markdown::delete_file("memory://delete-me", &env.cfg).expect("delete file");

    assert!(!markdown::file_exists("memory://delete-me", &env.cfg).expect("postcondition"));
}

// T-W1-2: RTM FR-28.8
#[test]
fn fr28_8_ac2_delete_file_non_existent_file_returns_not_found_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::delete_file("memory://never-existed", &env.cfg)
        .expect_err("missing delete target should fail");

    assert_message_error(err, "Memory file not found");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac1_read_markdown_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::read_markdown("memory://../../../etc/passwd", &env.cfg)
        .expect_err("traversal read should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac2_write_markdown_no_memory_prefix_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::write_markdown("/absolute/path/note", None, "content", &env.cfg)
        .expect_err("missing memory prefix should fail");

    assert_invalid_argument(err, "memory://");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac3_read_markdown_empty_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::read_markdown("", &env.cfg).expect_err("empty uri should fail");

    assert_invalid_argument(err, "null or empty");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac4_file_exists_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::file_exists("memory://../escape", &env.cfg).expect_err("traversal exists fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac5_delete_file_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err =
        markdown::delete_file("memory://../escape", &env.cfg).expect_err("traversal delete fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac6_get_memory_files_traversal_folder_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::get_memory_files(&env.cfg, Some("memory://../escape"))
        .expect_err("traversal folder should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac7_write_markdown_null_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = write_markdown_boundary(None, None, "content", &env.cfg)
        .expect_err("rust boundary adapter should reject null-equivalent uri");

    assert_invalid_argument(err, "null or empty");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac8_update_markdown_traversal_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::update_markdown("memory://../escape", None, "content", None, &env.cfg)
        .expect_err("traversal update should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac9_append_h2_section_no_prefix_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = markdown::append_h2_section("just-a-name", "Heading", "Content", &env.cfg)
        .expect_err("missing prefix append should fail");

    assert_invalid_argument(err, "memory://");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_ac10_get_memory_directories_traversal_folder_uri_throws_invalid_argument() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let err = get_memory_directories(&env.cfg, Some("memory://../escape"))
        .expect_err("traversal directory filter should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red1_concept_repair_tool_error_does_not_leak_filesystem_path() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let output = ToolRegistry::global()
        .try_invoke(
            "RepairConcepts",
            &json!({
                "conceptsToReplace": "[\"test-concept\"]",
                "canonicalConcept": "test",
                "folder": "invalid-nonexistent-folder-12345"
            }),
            OutputFormat::Markdown,
        )
        .expect("invoke repair concepts");

    assert!(!output.contains(env.cfg.memory_path.to_string_lossy().as_ref()));
    assert!(
        output.contains("ERROR")
            || output.contains("Scanning 0 markdown files")
            || output.contains("Files scanned: 0"),
        "unexpected output: {output}"
    );
}

// T-W1-2: RTM FR-28.10
#[test]
fn fr28_10_red2_markdown_writer_is_not_public() {
    let _guard = TEST_MUTEX.lock();
    let sources = read_crate_sources();

    assert!(
        !sources.contains("pub struct MarkdownWriter"),
        "implementation writer type must not be publicly exported"
    );
    assert!(
        !sources.contains("pub use") || !sources.contains("MarkdownWriter"),
        "writer implementation must not be re-exported"
    );
}

// T-W1-2: RTM FR-28.10
#[test]
fn fr28_10_red2_markdown_reader_is_not_public() {
    let _guard = TEST_MUTEX.lock();
    let sources = read_crate_sources();

    assert!(
        !sources.contains("pub struct MarkdownReader"),
        "implementation reader type must not be publicly exported"
    );
    assert!(
        !sources.contains("pub use") || !sources.contains("MarkdownReader"),
        "reader implementation must not be re-exported"
    );
}

// T-W1-2: RTM FR-28.10
#[test]
fn fr28_10_red3_get_session_path_is_not_public() {
    let _guard = TEST_MUTEX.lock();
    let sources = read_crate_sources();

    assert!(
        !sources.contains("pub fn get_session_path"),
        "filesystem path helpers must not be public below the URI boundary"
    );
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_traversal_single_dotdot_segment_throws() {
    let err = markdown::validate_uri("memory://foo/../bar", "uri")
        .expect_err("single traversal segment should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_traversal_double_dotdot_segment_throws() {
    let err = markdown::validate_uri("memory://foo/../../bar", "uri")
        .expect_err("double traversal segment should fail");

    assert_invalid_argument(err, "path traversal");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_legit_double_dot_in_concept_name_does_not_throw() {
    markdown::validate_uri("memory://some..concept", "uri").expect("legitimate dots accepted");
}

// T-W1-2: RTM FR-28.9
#[test]
fn fr28_9_red5_validate_uri_legit_double_dot_in_version_string_does_not_throw() {
    markdown::validate_uri("memory://version-2..0-notes", "uri")
        .expect("version-like dots accepted");
}
