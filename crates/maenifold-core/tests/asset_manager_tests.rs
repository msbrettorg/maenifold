use maenifold_core::assets::update_assets;
use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, Result};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
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

fn executable_assets_root() -> PathBuf {
    std::env::current_exe()
        .expect("current exe")
        .parent()
        .expect("exe parent")
        .join("assets")
}

fn unique_suffix(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

fn unique_relative_path(dir: &str, prefix: &str, extension: &str) -> String {
    format!("{dir}/{}.{}", unique_suffix(prefix), extension)
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
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
    root: PathBuf,
}

impl PackagedAssetGuard {
    fn create(relative: &str, content: &str) -> Self {
        let root = workspace_assets_root();
        let path = root.join(relative);
        write_file(&path, content);
        Self { path, root }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PackagedAssetGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        remove_empty_ancestors(self.path.parent(), &self.root);
    }
}

struct HiddenDirGuard {
    original: PathBuf,
    hidden: Option<PathBuf>,
}

impl HiddenDirGuard {
    fn hide(path: PathBuf) -> Self {
        if !path.exists() {
            return Self {
                original: path,
                hidden: None,
            };
        }

        let file_name = path
            .file_name()
            .expect("hidden dir filename")
            .to_string_lossy()
            .into_owned();
        let hidden =
            path.with_file_name(format!("{file_name}.{}.hidden", unique_suffix("assetmgr")));
        fs::rename(&path, &hidden).expect("hide directory");
        Self {
            original: path,
            hidden: Some(hidden),
        }
    }
}

impl Drop for HiddenDirGuard {
    fn drop(&mut self) {
        if let Some(hidden) = &self.hidden {
            if self.original.exists() {
                let _ = fs::remove_dir_all(&self.original);
            }
            let _ = fs::rename(hidden, &self.original);
        }
    }
}

fn hide_packaged_asset_roots() -> Vec<HiddenDirGuard> {
    let mut paths = vec![workspace_assets_root(), executable_assets_root()];
    paths.sort();
    paths.dedup();
    paths.into_iter().map(HiddenDirGuard::hide).collect()
}

fn invoke_update_assets(config: &Config, dry_run: bool) -> Result<String> {
    let context = ToolContext::new(config.clone(), OutputFormat::Markdown);
    update_assets(&context, &json!({ "dryRun": dry_run }))
}

fn get_asset_path(config: &Config, relative_path: &str) -> PathBuf {
    config.assets_path.join(relative_path)
}

fn asset_exists(config: &Config, relative_path: &str) -> bool {
    get_asset_path(config, relative_path).exists()
}

fn load_asset_text(config: &Config, relative_path: &str) -> Option<String> {
    fs::read_to_string(get_asset_path(config, relative_path)).ok()
}

fn load_asset_json<T: DeserializeOwned>(config: &Config, relative_path: &str) -> Option<T> {
    load_asset_text(config, relative_path).and_then(|text| serde_json::from_str(&text).ok())
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs initialize copy coverage.
#[test]
fn initialize_assets_with_source_files_copies_files_to_target() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-init-copy"));
    let expected = "hello from InitializeAssets test";
    let _source = PackagedAssetGuard::create(&relative, expected);
    let target = cfg.assets_path.join(&relative);
    let _ = fs::remove_file(&target);

    AssetManager::initialize_assets(&cfg).expect("initialize assets");

    assert!(
        target.exists(),
        "expected copied file at {}",
        target.display()
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        expected,
        "copied file content should match source content"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs missing source bootstrap coverage.
#[test]
fn initialize_assets_when_source_dir_missing_returns_gracefully() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    let _hidden_roots = hide_packaged_asset_roots();

    AssetManager::initialize_assets(&cfg).expect("missing packaged assets should be ignored");
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs non-overwrite coverage.
#[test]
fn initialize_assets_skips_existing_files_and_does_not_overwrite() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-init-skip"));
    let _source = PackagedAssetGuard::create(&relative, "source content");
    let target = cfg.assets_path.join(&relative);
    write_file(&target, "original target content");

    AssetManager::initialize_assets(&cfg).expect("initialize assets");

    assert_eq!(
        fs::read_to_string(&target).expect("read target"),
        "original target content",
        "initialize_assets must not overwrite an existing target file"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs dry-run reporting coverage.
#[test]
fn update_assets_dry_run_reports_new_files_without_copying() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_relative_path("workflows", "asset-manager-dryrun-new", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(&relative, r#"{"id":"dry-run-source"}"#);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains(&file_name),
        "expected missing file in dry-run summary: {output}"
    );
    assert!(
        !cfg.assets_path.join(&relative).exists(),
        "dry-run should not copy files into runtime assets"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs apply coverage.
#[test]
fn update_assets_apply_copies_new_files_to_target() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_relative_path("workflows", "asset-manager-apply-new", "json");
    let expected = r#"{"id":"apply-source","name":"apply source content"}"#;
    let _source = PackagedAssetGuard::create(&relative, expected);

    let output = invoke_update_assets(&cfg, false).expect("apply update assets");

    assert!(
        output.contains("Added files: 1") || output.contains("Updated files: 1"),
        "expected copy summary: {output}"
    );
    assert_eq!(
        fs::read_to_string(cfg.assets_path.join(&relative)).expect("read copied file"),
        expected,
        "applied file content must match source content"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs new-file detection coverage.
#[test]
fn update_assets_new_file_in_source_appears_in_added_files() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let relative = unique_relative_path("roles", "asset-manager-added", "json");
    let file_name = Path::new(&relative)
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();
    let _source = PackagedAssetGuard::create(&relative, r#"{"id":"brand-new"}"#);

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("### Added Files"),
        "expected added files section: {output}"
    );
    assert!(
        output.contains(&file_name),
        "expected added file in summary: {output}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs changed-file detection coverage.
#[test]
fn update_assets_changed_file_appears_in_updated_files() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("roles", "asset-manager-updated", "json");
    let source = PackagedAssetGuard::create(&relative, r#"{"id":"updated","version":2}"#);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    write_file(
        &cfg.assets_path.join(&relative),
        r#"{"id":"updated","version":1}"#,
    );

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("### Updated Files"),
        "expected updated files section: {output}"
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
        "expected updated file in summary: {output}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs identical-file coverage.
#[test]
fn update_assets_identical_file_is_not_reported_as_added_or_updated() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = unique_relative_path("colors", "asset-manager-identical", "json");
    let content = r#"{"id":"same","value":1}"#;
    let _source = PackagedAssetGuard::create(&relative, content);
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("No changes detected"),
        "identical source and target should produce a no-change summary: {output}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs source-missing error coverage.
#[test]
fn update_assets_when_source_dir_missing_returns_error() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    let _hidden_roots = hide_packaged_asset_roots();

    let err = invoke_update_assets(&cfg, true).expect_err("missing packaged assets should error");

    assert!(
        err.to_string()
            .contains("Source assets directory not found"),
        "unexpected error: {err}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs target-missing error coverage.
#[test]
fn update_assets_when_target_dir_missing_returns_error() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");
    fs::remove_dir_all(&cfg.assets_path).expect("remove runtime assets dir");

    let err = invoke_update_assets(&cfg, true).expect_err("missing runtime assets should error");

    assert!(
        err.to_string()
            .contains("Target assets directory does not exist. Run initialization first."),
        "unexpected error: {err}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs GetAssetPath coverage.
#[test]
fn get_asset_path_returns_combined_assets_path() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = Path::new("workflows").join("my-workflow.json");
    let actual = get_asset_path(&cfg, relative.to_str().expect("utf-8 path"));

    assert_eq!(
        actual,
        cfg.assets_path.join(relative),
        "get_asset_path helper should mirror Config.assets_path joining"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs AssetExists coverage.
#[test]
fn asset_exists_is_true_for_existing_file_and_false_for_missing_file() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-exists"));
    write_file(&cfg.assets_path.join(&relative), "exists");

    assert!(
        asset_exists(&cfg, &relative),
        "expected existing file to be found"
    );
    assert!(
        !asset_exists(
            &cfg,
            &format!("{}.txt", unique_suffix("asset-manager-missing"))
        ),
        "expected missing file to return false"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs LoadAssetText coverage.
#[test]
fn load_asset_text_returns_content_or_none_for_missing_file() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.txt", unique_suffix("asset-manager-load-text"));
    let expected = "some asset text content\nwith multiple lines";
    write_file(&cfg.assets_path.join(&relative), expected);

    assert_eq!(
        load_asset_text(&cfg, &relative),
        Some(expected.to_string()),
        "expected to load existing asset text"
    );
    assert_eq!(
        load_asset_text(
            &cfg,
            &format!("{}.txt", unique_suffix("asset-manager-missing-text"))
        ),
        None,
        "expected missing asset text to return None"
    );
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct TestAssetDto {
    name: String,
    version: String,
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs LoadAssetJson coverage.
#[test]
fn load_asset_json_deserializes_json_or_returns_none_for_missing_file() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();

    let relative = format!("{}.json", unique_suffix("asset-manager-load-json"));
    write_file(
        &cfg.assets_path.join(&relative),
        r#"{"name":"test-asset","version":"1.0"}"#,
    );

    assert_eq!(
        load_asset_json::<TestAssetDto>(&cfg, &relative),
        Some(TestAssetDto {
            name: "test-asset".to_string(),
            version: "1.0".to_string(),
        }),
        "expected JSON asset to deserialize successfully"
    );
    assert_eq!(
        load_asset_json::<TestAssetDto>(
            &cfg,
            &format!("{}.json", unique_suffix("asset-manager-missing-json"))
        ),
        None,
        "expected missing JSON asset to return None"
    );
}

#[cfg(unix)]
// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs AssetUpdateResult summary coverage.
#[test]
fn update_assets_summary_formats_counts_sections_and_errors() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let added_relative = unique_relative_path("workflows", "asset-manager-summary-added", "json");
    let updated_relative = unique_relative_path("roles", "asset-manager-summary-updated", "json");
    let skipped_relative = unique_relative_path("colors", "asset-manager-summary-skipped", "json");

    let _added = PackagedAssetGuard::create(&added_relative, r#"{"id":"summary-added"}"#);
    let _updated =
        PackagedAssetGuard::create(&updated_relative, r#"{"id":"summary-updated","version":2}"#);
    let _skipped = PackagedAssetGuard::create(&skipped_relative, r#"{"id":"summary-skipped"}"#);

    write_file(
        &cfg.assets_path.join(&updated_relative),
        r#"{"id":"summary-updated","version":1}"#,
    );

    let skipped_target = cfg.assets_path.join(&skipped_relative);
    let skipped_target_parent = skipped_target.parent().expect("skipped target parent");
    fs::create_dir_all(skipped_target_parent).expect("create skipped target dir");
    let symlink_target = cfg
        .assets_path
        .join(format!("{}.txt", unique_suffix("symlink-target")));
    write_file(&symlink_target, "symlink target content");
    symlink(&symlink_target, &skipped_target).expect("create target symlink");

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    assert!(
        output.contains("Asset Update Summary"),
        "missing summary header: {output}"
    );
    assert!(
        output.contains("Added files: 1"),
        "missing added count: {output}"
    );
    assert!(
        output.contains("Updated files: 1"),
        "missing updated count: {output}"
    );
    assert!(
        output.contains("Skipped symlinks: 1"),
        "missing skipped count: {output}"
    );
    assert!(
        output.contains("Errors: 1"),
        "missing error count: {output}"
    );
    assert!(
        output.contains("### Added Files"),
        "missing Added Files section: {output}"
    );
    assert!(
        output.contains("### Updated Files"),
        "missing Updated Files section: {output}"
    );
    assert!(
        output.contains("### Skipped Symlink Targets"),
        "missing skipped symlink section: {output}"
    );
    assert!(
        output.contains("### Errors"),
        "missing Errors section: {output}"
    );
    assert!(
        output.contains(
            Path::new(&added_relative)
                .file_name()
                .expect("added file name")
                .to_string_lossy()
                .as_ref()
        ),
        "missing added file name: {output}"
    );
    assert!(
        output.contains(
            Path::new(&updated_relative)
                .file_name()
                .expect("updated file name")
                .to_string_lossy()
                .as_ref()
        ),
        "missing updated file name: {output}"
    );
    assert!(
        output.contains("skipped symlink target"),
        "missing symlink error detail: {output}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs error formatting coverage.
#[test]
fn update_assets_source_missing_error_surfaces_without_summary_counts() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    let _hidden_roots = hide_packaged_asset_roots();

    let err = invoke_update_assets(&cfg, true).expect_err("missing packaged assets should error");
    let text = err.to_string();

    assert!(
        text.contains("Source assets directory not found"),
        "missing source error should surface clearly: {text}"
    );
    assert!(
        !text.contains("Added files:"),
        "top-level error should not render summary counts: {text}"
    );
}

// T-1.4: RTM FR-3 - ported from C# AssetManagerTests.cs source-target mapping coverage.
#[test]
fn update_assets_source_target_mapping_discovers_all_four_rust_asset_subdirectories() {
    let _guard = test_mutex().lock().expect("mutex");
    let _reset = ConfigResetGuard;
    let (_tmp, cfg) = isolated_config();
    AssetManager::initialize_assets(&cfg).expect("seed runtime assets");

    let categories = ["workflows", "roles", "colors", "perspectives"];
    let mut expected_names = Vec::new();
    let mut _guards = Vec::new();

    for category in categories {
        let relative =
            unique_relative_path(category, &format!("asset-manager-map-{category}"), "json");
        expected_names.push(
            Path::new(&relative)
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned(),
        );
        _guards.push(PackagedAssetGuard::create(
            &relative,
            &format!(r#"{{"id":"{category}-mapping"}}"#),
        ));
    }

    let output = invoke_update_assets(&cfg, true).expect("dry-run update assets");

    for file_name in expected_names {
        assert!(
            output.contains(&file_name),
            "expected mapped file {file_name} to appear in summary: {output}"
        );
    }
}
