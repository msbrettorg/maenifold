use maenifold_core::json::OutputFormat;
use maenifold_core::memory;
use maenifold_core::tools::ToolContext;
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::fs;
use std::path::Path;
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

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
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

fn list_memories_output(env: &TestEnv, path: &str) -> String {
    memory::list_memories(&env.ctx(), &json!({ "path": path })).expect("list memories")
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

// T-QUAL-002.6h: RTM FR-25.5
#[test]
fn fr25_5_ac1_shared_path_validator_absolute_path_rejected_by_memory_and_system_tools() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let absolute_path = env.cfg.memory_path.join("..").join("blocked-absolute");
    let absolute_path = absolute_path
        .canonicalize()
        .unwrap_or_else(|_| absolute_path.clone());
    let absolute_path = absolute_path.to_string_lossy().to_string();
    let expected_reason = "Absolute paths are not allowed. Use relative paths only.";

    let write_result = write_memory_output(
        &env,
        "Absolute Path Rejection",
        "Path security test with [[path-validation]].",
        Some(&absolute_path),
    );
    let list_result = list_memories_output(&env, &absolute_path);

    assert!(
        write_result.starts_with("ERROR:"),
        "unexpected: {write_result}"
    );
    assert!(
        write_result.contains(expected_reason),
        "unexpected: {write_result}"
    );
    assert!(
        list_result.starts_with("ERROR:"),
        "unexpected: {list_result}"
    );
    assert!(
        list_result.contains(expected_reason),
        "unexpected: {list_result}"
    );
}

// T-QUAL-002.6h: RTM FR-25.6
#[test]
fn fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let traversal_path = "safe/../escape";
    let expected_reason = "Path traversal components (dot sequences) are not allowed.";

    let write_result = write_memory_output(
        &env,
        "Traversal Rejection",
        "Traversal protection test with [[path-traversal]].",
        Some(traversal_path),
    );
    let list_result = list_memories_output(&env, traversal_path);

    assert!(
        write_result.starts_with("ERROR:"),
        "unexpected: {write_result}"
    );
    assert!(
        write_result.contains(expected_reason),
        "unexpected: {write_result}"
    );
    assert!(
        list_result.starts_with("ERROR:"),
        "unexpected: {list_result}"
    );
    assert!(
        list_result.contains(expected_reason),
        "unexpected: {list_result}"
    );
}

// T-QUAL-002.6h: RTM FR-25.5
#[test]
fn fr25_5_ac2_path_traversal_protection_memory_tools_still_allows_valid_relative_folder() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let test_folder = "fr25-5-path-validator-valid-memory";
    let valid_folder = format!("{test_folder}/nested");

    let write_result = write_memory_output(
        &env,
        "Valid Relative Path",
        "Valid path operation with [[memorytools]].",
        Some(&valid_folder),
    );

    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );
    assert!(
        write_result.contains(&format!("memory://{valid_folder}/valid-relative-path")),
        "unexpected: {write_result}"
    );
}

// T-QUAL-002.6h: RTM FR-25.6
#[test]
fn fr25_6_ac2_path_traversal_protection_system_tools_still_allows_valid_relative_folder() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let test_folder = "fr25-5-path-validator-valid-system";
    let valid_folder = format!("{test_folder}/system");

    let write_result = write_memory_output(
        &env,
        "System List Target",
        "Listing target with [[systemtools]].",
        Some(&valid_folder),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let list_result = list_memories_output(&env, &valid_folder);

    assert!(
        list_result.starts_with("# Directory:"),
        "unexpected: {list_result}"
    );
    assert!(
        list_result.contains("system-list-target.md"),
        "unexpected: {list_result}"
    );
    assert!(
        !list_result.contains("Directory traversal detected"),
        "unexpected: {list_result}"
    );
}

// T-QUAL-002.6h: RTM FR-25.5
#[test]
fn fr25_5_ac3_shared_path_validator_rejects_dot_space_traversal_variants_consistently() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let traversal_variants = [
        ".. ",
        ". .",
        ".. .",
        "safe/.. /escape",
        "safe/. ./escape",
        "safe/.. ./escape",
    ];
    let expected_reason = "Path traversal components (dot sequences) are not allowed.";

    for traversal_variant in traversal_variants {
        let write_result = write_memory_output(
            &env,
            "Dot Space Traversal Rejection",
            "Dot space traversal test with [[path-security]].",
            Some(traversal_variant),
        );
        let list_result = list_memories_output(&env, traversal_variant);

        assert!(
            write_result.starts_with("ERROR:"),
            "WriteMemory should reject traversal variant '{traversal_variant}': {write_result}"
        );
        assert!(
            write_result.contains(expected_reason),
            "WriteMemory should use shared validator reason for '{traversal_variant}': {write_result}"
        );
        assert!(
            list_result.starts_with("ERROR:"),
            "ListMemories should reject traversal variant '{traversal_variant}': {list_result}"
        );
        assert!(
            list_result.contains(expected_reason),
            "ListMemories should use shared validator reason for '{traversal_variant}': {list_result}"
        );
    }
}

// T-QUAL-002.6h: RTM FR-25.6
#[test]
fn fr25_6_ac4_list_memories_rejects_symlink_escaping_memory_path() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let test_folder = "fr25-5-path-validator-symlink";
    let test_folder_path = env.cfg.memory_path.join(test_folder);
    let symlink_folder = test_folder_path.join("link-outside");
    let outside_tmp = TempDir::new().expect("outside tmp");

    fs::create_dir_all(&test_folder_path).expect("create test folder");
    if let Err(error) = create_directory_symlink(outside_tmp.path(), &symlink_folder) {
        eprintln!("Skipping symlink test: {error}");
        return;
    }

    let list_result = list_memories_output(&env, &format!("{test_folder}/link-outside"));

    assert!(
        list_result.starts_with("ERROR:"),
        "unexpected: {list_result}"
    );
    assert!(
        list_result.contains("Resolved path escapes memory root"),
        "unexpected: {list_result}"
    );
}
