use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase, markdown, memory, search};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// T-W1-4: RTM NFR-2 — Ported MemoryPathTests.cs coverage for nested memory paths and URI/path conversion.
static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
const TEST_FOLDER: &str = "memory-path-tests";

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl TestEnv {
    fn ctx(&self) -> ToolContext {
        ToolContext::new(self.cfg.clone(), OutputFormat::Markdown)
    }

    fn test_folder_path(&self) -> std::path::PathBuf {
        self.cfg.memory_path.join(TEST_FOLDER)
    }
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
    fs::create_dir_all(cfg.memory_path.join(TEST_FOLDER)).expect("create test folder");
    TestEnv { _tmp: tmp, cfg }
}

fn write_memory_output(env: &TestEnv, title: &str, content: &str, folder: Option<&str>) -> String {
    memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": content,
            "folder": folder,
        }),
    )
    .expect("write memory")
}

fn read_memory_output(
    env: &TestEnv,
    identifier: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    memory::read_memory(&env.ctx(), &json!({ "identifier": identifier }))
}

fn move_memory_output(
    env: &TestEnv,
    source: &str,
    destination: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    memory::move_memory(
        &env.ctx(),
        &json!({
            "source": source,
            "destination": destination,
        }),
    )
}

fn extract_concepts_output(
    env: &TestEnv,
    identifier: &str,
) -> Result<String, maenifold_core::MaenifoldError> {
    search::extract_concepts_from_file(&env.ctx(), &json!({ "identifier": identifier }))
}

// T-W1-4: RTM NFR-2 — Nested directory creation parity with the C# oracle.
#[test]
fn write_memory_creates_nested_directories_automatically() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let nested_folder = format!("{TEST_FOLDER}/level1/level2/level3");

    let result = write_memory_output(
        &env,
        "Deep Nested Test",
        "Testing [[nested directory]] creation in deep folders.",
        Some(&nested_folder),
    );

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(result.contains("memory://memory-path-tests/level1/level2/level3/deep-nested-test"));

    let expected_dir = env
        .test_folder_path()
        .join("level1")
        .join("level2")
        .join("level3");
    assert!(
        expected_dir.exists(),
        "nested directories should be created"
    );

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/level1/level2/level3/deep-nested-test",
    )
    .expect("read nested memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[nested directory]] creation in deep folders."));
}

#[test]
fn move_memory_creates_nested_destination_directories() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let write_result = write_memory_output(
        &env,
        "Move Source Test",
        "Testing [[move to nested]] directory functionality.",
        Some(TEST_FOLDER),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let nested_destination = format!("{TEST_FOLDER}/categories/subcategory/moved-file");
    let move_result = move_memory_output(
        &env,
        "memory://memory-path-tests/move-source-test",
        &nested_destination,
    )
    .expect("move memory");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://memory-path-tests/categories/subcategory/moved-file"));

    let expected_dir = env
        .test_folder_path()
        .join("categories")
        .join("subcategory");
    assert!(
        expected_dir.exists(),
        "nested destination directories should be created"
    );

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/categories/subcategory/moved-file",
    )
    .expect("read moved memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[move to nested]] directory functionality."));
}

#[test]
fn extract_concepts_from_file_works_with_nested_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let nested_folder = format!("{TEST_FOLDER}/concepts/nested");

    let write_result = write_memory_output(
        &env,
        "Concept Test",
        "This contains [[machine learning]] and [[data science]] concepts for testing.",
        Some(&nested_folder),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let extract_result = extract_concepts_output(
        &env,
        "memory://memory-path-tests/concepts/nested/concept-test",
    )
    .expect("extract concepts");
    assert!(
        !extract_result.starts_with("ERROR:"),
        "unexpected: {extract_result}"
    );
    assert!(extract_result.contains("[[machine-learning]]"));
    assert!(extract_result.contains("[[data-science]]"));
}

// T-W1-4: RTM NFR-2 — Path traversal rejection parity with the C# oracle.
#[test]
fn write_memory_rejects_path_traversal_attempts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_folder in [
        "../outside-memory",
        "../../etc",
        "folder/../../../escape",
        "valid/../../invalid",
        "memory-path-tests/../escape-test-folder",
    ] {
        let result = write_memory_output(
            &env,
            "Test Title",
            "Content with [[required concept]] for testing.",
            Some(malicious_folder),
        );

        assert!(
            result.starts_with("ERROR:"),
            "path traversal should be rejected for folder {malicious_folder}: {result}"
        );
    }
}

#[test]
fn read_memory_rejects_path_traversal_attempts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside-memory",
        "memory://../../etc/hosts",
        "memory://folder/../../../system-file",
    ] {
        match read_memory_output(&env, malicious_uri) {
            Ok(result) => assert!(
                result.starts_with("ERROR:"),
                "path traversal read should be rejected for {malicious_uri}: {result}"
            ),
            Err(err) => assert!(
                err.to_string().contains("Path traversal")
                    || err.to_string().contains("path traversal")
                    || err.to_string().contains("Invalid URI")
                    || err.to_string().contains("escape"),
                "unexpected error for {malicious_uri}: {err}"
            ),
        }
    }
}

#[test]
fn move_memory_rejects_path_traversal_in_destination() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let write_result = write_memory_output(
        &env,
        "Valid Source",
        "Content to move with [[required concept]] for validation.",
        Some(TEST_FOLDER),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    for malicious_destination in [
        "../outside-memory",
        "../../escape-folder/file",
        "memory-path-tests/../escape-test-folder/file",
    ] {
        match move_memory_output(
            &env,
            "memory://memory-path-tests/valid-source",
            malicious_destination,
        ) {
            Ok(result) => assert!(
                result.starts_with("ERROR:")
                    || result.contains("Invalid")
                    || result.contains("escape"),
                "path traversal move should be rejected for {malicious_destination}: {result}"
            ),
            Err(err) => assert!(
                err.to_string().contains("Path traversal")
                    || err.to_string().contains("Invalid")
                    || err.to_string().contains("escape"),
                "unexpected error for {malicious_destination}: {err}"
            ),
        }
    }
}

#[test]
fn uri_to_path_directly_rejects_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside",
        "memory://../../system",
        "memory://folder/../../../escape",
    ] {
        let err = markdown::uri_to_path(malicious_uri, &env.cfg.memory_path)
            .expect_err("uri_to_path should reject traversal");
        assert!(
            err.to_string().contains("Path traversal")
                || err.to_string().contains("path traversal")
                || err.to_string().contains("Invalid URI")
                || err.to_string().contains("escape"),
            "unexpected error for {malicious_uri}: {err}"
        );
    }
}

// T-W1-4: RTM NFR-2 — Cross-platform path conversion parity with the C# oracle.
#[test]
fn path_handling_works_with_unix_style_separators() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let unix_style_folder = format!("{TEST_FOLDER}/unix/style");

    let write_result = write_memory_output(
        &env,
        "Unix Style Path",
        "Testing [[unix paths]] with forward slashes in nested directories.",
        Some(&unix_style_folder),
    );

    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );
    assert!(write_result.contains("memory://memory-path-tests/unix/style/unix-style-path"));

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/unix/style/unix-style-path",
    )
    .expect("read unix path memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(
        read_result.contains("Testing [[unix paths]] with forward slashes in nested directories.")
    );

    let expected_dir = env.test_folder_path().join("unix").join("style");
    assert!(
        expected_dir.exists(),
        "directory should exist with correct platform separators"
    );
}

#[test]
fn path_to_uri_produces_consistent_uri_format() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let test_cases = [
        env.test_folder_path().join("simple.md"),
        env.test_folder_path().join("nested").join("file.md"),
        env.test_folder_path()
            .join("deep")
            .join("nesting")
            .join("levels")
            .join("file.md"),
    ];

    for test_path in test_cases {
        if let Some(dir) = test_path.parent() {
            fs::create_dir_all(dir).expect("create test dirs");
        }
        fs::write(&test_path, "# Test File\n\nContent").expect("write fixture");

        let uri = markdown::path_to_uri(&test_path, &env.cfg.memory_path).expect("path to uri");

        assert!(uri.starts_with("memory://"));
        assert!(!uri.contains('\\'), "URI should not contain backslashes");
        assert!(
            !uri.ends_with(".md"),
            "URI should not include .md extension"
        );

        let converted_path =
            markdown::uri_to_path(&uri, &env.cfg.memory_path).expect("uri to path");
        assert!(
            converted_path.exists(),
            "converted path should exist: {:?}",
            converted_path
        );
    }
}

#[test]
fn uri_to_path_handles_platform_specific_separators() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for uri in [
        "memory://memory-path-tests/simple-file",
        "memory://memory-path-tests/nested/subdirectory/file",
        "memory://memory-path-tests/deep/multi/level/nesting/file",
    ] {
        let path = markdown::uri_to_path(uri, &env.cfg.memory_path).expect("uri to path");

        if cfg!(windows) && uri.contains('/') {
            let rendered = path.to_string_lossy();
            assert!(
                rendered.contains('\\') || !uri.contains('/'),
                "windows paths should use backslashes for nested directories: {rendered}"
            );
        }

        assert!(
            path.starts_with(&env.cfg.memory_path),
            "converted path should stay within memory directory"
        );
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("md"));
    }
}

#[test]
fn file_operations_work_with_mixed_path_styles() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let platform_path = env
        .test_folder_path()
        .join("mixed")
        .join("platform")
        .join("file.md");

    if let Some(dir) = platform_path.parent() {
        fs::create_dir_all(dir).expect("create mixed path dirs");
    }

    let content = "# Mixed Path Test\n\nTesting [[cross platform]] compatibility.";
    fs::write(&platform_path, content).expect("write mixed path file");

    let uri = markdown::path_to_uri(&platform_path, &env.cfg.memory_path).expect("path to uri");
    let converted_path = markdown::uri_to_path(&uri, &env.cfg.memory_path).expect("uri to path");

    let original_full = platform_path
        .canonicalize()
        .unwrap_or(platform_path.clone());
    let converted_full = converted_path
        .canonicalize()
        .unwrap_or_else(|_| converted_path.clone());
    assert_eq!(
        converted_full, original_full,
        "round-trip conversion should preserve path identity"
    );

    let read_result = read_memory_output(&env, &uri).expect("read mixed path file");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[cross platform]] compatibility."));
}

// T-W1-4: RTM NFR-2 — Additional path edge-case parity with the C# oracle.
#[test]
fn nested_paths_handle_special_characters() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for (title, folder, uri) in [
        (
            "File With Hyphens",
            "memory-path-tests/special-chars",
            "file-with-hyphens",
        ),
        (
            "File With Underscores",
            "memory-path-tests/special_chars",
            "file-with-underscores",
        ),
        (
            "File With Numbers456",
            "memory-path-tests/numbers123",
            "file-with-numbers456",
        ),
        (
            "Final File",
            "memory-path-tests/mixed-chars_123",
            "final-file",
        ),
    ] {
        let content = format!("Testing [[special characters]] in path: {folder}");
        let write_result = write_memory_output(&env, title, &content, Some(folder));

        assert!(
            write_result.starts_with("Created memory FILE:"),
            "special character path should work for {folder}: {write_result}"
        );

        let read_result = read_memory_output(&env, &format!("memory://{folder}/{uri}"))
            .expect("read special character path");
        assert!(
            !read_result.starts_with("ERROR:"),
            "reading special character path should work for {folder}: {read_result}"
        );
        assert!(read_result.contains(&content));
    }
}

#[test]
fn deep_nesting_limits_are_respected() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut deep_folder = TEST_FOLDER.to_string();
    for level in 1..=10 {
        deep_folder.push_str(&format!("/level{level}"));
    }

    let write_result = write_memory_output(
        &env,
        "Deep File",
        "Testing [[very deep]] nesting in directory structures.",
        Some(&deep_folder),
    );

    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let read_result = read_memory_output(&env, &format!("memory://{deep_folder}/deep-file"))
        .expect("read deep nesting file");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[very deep]] nesting in directory structures."));
}

#[test]
fn empty_directory_names_are_handled_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for problematic_folder in [
        "memory-path-tests//double-slash",
        "memory-path-tests/normal//double-middle",
        "memory-path-tests/trailing-slash//",
    ] {
        let result = write_memory_output(
            &env,
            "Empty Directory Test",
            "Testing [[empty segments]] in folder paths for validation.",
            Some(problematic_folder),
        );

        assert!(
            !result.contains("Exception"),
            "empty directory segments should be handled gracefully for {problematic_folder}: {result}"
        );
    }
}
