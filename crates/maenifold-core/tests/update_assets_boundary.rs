use maenifold_core::json::OutputFormat;
use maenifold_core::{AssetManager, Config, ToolRegistry};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn test_mutex() -> &'static Mutex<()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

fn workspace_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("assets")
}

fn unique_asset_relative(dir: &str, prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{dir}/{prefix}-{nanos}.json")
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, content).expect("write file");
}

fn remove_empty_ancestors(mut current: Option<&Path>, stop_at: &Path) {
    while let Some(path) = current {
        if path == stop_at {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(_) => break,
        }
    }
}

struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

struct PackagedAssetGuard {
    path: PathBuf,
    assets_root: PathBuf,
}

impl PackagedAssetGuard {
    fn create(relative: &str, content: &str) -> Self {
        let assets_root = workspace_assets_root();
        let path = assets_root.join(relative);
        write_file(&path, content);
        Self { path, assets_root }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PackagedAssetGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        remove_empty_ancestors(self.path.parent(), &self.assets_root);
    }
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_dry_run_detects_files_missing_from_target() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_asset_relative("workflows", "t14c-added");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(
        &relative,
        r#"{"id":"t14c-added","name":"Added boundary workflow"}"#,
    );

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Added files: 1"),
        "expected one added file in summary: {output}"
    );
    assert!(
        output.contains(&file_name),
        "expected missing file name in summary: {output}"
    );
    assert!(
        !cfg.assets_path.join(&relative).exists(),
        "dry-run should not copy missing files"
    );
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_dry_run_detects_sha_changes_for_existing_files() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_asset_relative("roles", "t14c-updated");
    let source = PackagedAssetGuard::create(&relative, r#"{"id":"t14c-updated","version":1}"#);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    write_file(source.path(), r#"{"id":"t14c-updated","version":2}"#);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Updated files: 1"),
        "expected one updated file in summary: {output}"
    );
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_non_dry_run_copies_added_files_into_runtime_assets() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_asset_relative("workflows", "t14c-copy");
    let source_content = r#"{"id":"t14c-copy","name":"Copied workflow"}"#;
    let _source = PackagedAssetGuard::create(&relative, source_content);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": false}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Added files: 1") || output.contains("Updated files: 1"),
        "expected copy summary for changed file: {output}"
    );
    let target_content = fs::read_to_string(cfg.assets_path.join(&relative)).expect("target file");
    assert_eq!(target_content, source_content);
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_reports_no_changes_when_source_and_target_match() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("No changes detected"),
        "expected no-change summary: {output}"
    );
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_learn_mode_returns_help_instead_of_update_summary() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"learn": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("# UpdateAssets") || output.contains("Refreshes"),
        "expected help documentation: {output}"
    );
    assert!(
        !output.contains("Asset Update Summary"),
        "learn mode should not render update summary: {output}"
    );
}

// T-1.4c: RTM FR-3
#[test]
fn update_assets_uses_sha256_not_file_size_for_equal_length_content() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_asset_relative("roles", "t14c-same-size");
    let source = PackagedAssetGuard::create(&relative, r#"{"a":1}"#);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    write_file(&cfg.assets_path.join(&relative), r#"{"b":2}"#);

    let output = ToolRegistry::global()
        .try_invoke(
            "UpdateAssets",
            &json!({"dryRun": true}),
            OutputFormat::Markdown,
        )
        .expect("invoke update assets");

    assert!(
        output.contains("Updated files: 1"),
        "same-size content changes must still be reported: {output}"
    );
    assert!(
        output.contains(
            source
                .path()
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .as_ref()
        ),
        "updated same-size file should be listed: {output}"
    );
}
