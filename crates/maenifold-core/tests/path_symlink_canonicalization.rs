use maenifold_core::{Config, assets, paths};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_directory_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

#[cfg(unix)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_file_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, link)
}

// T-14.5: RTM FR-12 - normal nested memory paths still resolve inside the configured memory root.
#[test]
fn path_validation_accepts_normal_relative_memory_uri() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();

    let path =
        paths::memory_uri_to_path("memory://notes/canonicalization-check", &cfg).expect("uri path");

    assert_eq!(
        path,
        cfg.memory_path
            .join("notes")
            .join("canonicalization-check.md")
    );
    Config::reset_overrides();
}

// T-14.5: RTM FR-12 - dot-sequence traversal remains rejected before any filesystem resolution.
#[test]
fn path_validation_rejects_dot_dot_components() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let err = paths::memory_uri_to_path("memory://notes/../escape", &cfg)
        .expect_err("dot-sequence traversal should be rejected");

    assert!(
        err.to_string().contains("Path traversal"),
        "unexpected error: {err}"
    );
    Config::reset_overrides();
}

// T-14.5: RTM FR-12 - canonicalization must reject symlinked memory subdirectories that point outside root.
#[test]
fn path_validation_rejects_symlink_escape_for_nonexistent_leaf() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let outside_tmp = TempDir::new().expect("outside tmp");
    let symlink_path = cfg.memory_path.join("outside-link");

    if let Err(error) = create_directory_symlink(outside_tmp.path(), &symlink_path) {
        eprintln!("Skipping symlink test: {error}");
        Config::reset_overrides();
        return;
    }

    let err = paths::memory_uri_to_path("memory://outside-link/new-note", &cfg)
        .expect_err("symlink escape should be rejected");

    assert!(
        err.to_string().contains("escapes base directory"),
        "unexpected error: {err}"
    );
    Config::reset_overrides();
}

// T-14.5: RTM FR-12 - asset watcher filtering must reject symlinked JSON files outside the assets root.
#[test]
fn path_validation_rejects_symlinked_asset_json_outside_assets_root() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let role_dir = cfg.assets_path.join("roles");
    let outside_tmp = TempDir::new().expect("outside tmp");
    let outside_file = outside_tmp.path().join("outside.json");
    let symlink_path = role_dir.join("linked.json");

    fs::create_dir_all(&role_dir).expect("create role dir");
    fs::write(&outside_file, r#"{"id":"outside"}"#).expect("write outside json");

    if let Err(error) = create_file_symlink(&outside_file, &symlink_path) {
        eprintln!("Skipping asset symlink test: {error}");
        Config::reset_overrides();
        return;
    }

    assert!(
        !assets::is_watchable_asset_json_path(&cfg.assets_path, &symlink_path),
        "symlinked asset outside root should not be watchable"
    );
    Config::reset_overrides();
}
