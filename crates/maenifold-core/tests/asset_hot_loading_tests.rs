use maenifold_core::{AssetKind, AssetManager, Config, MaenifoldError};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn test_mutex() -> &'static Mutex<()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

struct AssetHotLoadingHarness {
    _tmp: TempDir,
    config: Config,
    workflows_path: PathBuf,
    roles_path: PathBuf,
    colors_path: PathBuf,
    perspectives_path: PathBuf,
}

impl AssetHotLoadingHarness {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("dirs");

        let workflows_path = config.assets_path.join("workflows");
        let roles_path = config.assets_path.join("roles");
        let colors_path = config.assets_path.join("colors");
        let perspectives_path = config.assets_path.join("perspectives");

        for path in [
            &workflows_path,
            &roles_path,
            &colors_path,
            &perspectives_path,
        ] {
            fs::create_dir_all(path).expect("asset dir");
        }

        Self {
            _tmp: tmp,
            config,
            workflows_path,
            roles_path,
            colors_path,
            perspectives_path,
        }
    }

    fn asset_manager(&self) -> AssetManager {
        AssetManager::new(&self.config)
    }
}

impl Drop for AssetHotLoadingHarness {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileCreatedAddsResourceToMcpServer.
#[test]
fn file_created_adds_resource_to_asset_manager() {
    let _guard = test_mutex().lock().expect("mutex");
    let harness = AssetHotLoadingHarness::new();

    let test_file = harness.workflows_path.join("test-workflow.json");
    let test_content = r#"{"name":"test-workflow","version":"1.0.0"}"#;

    fs::write(&test_file, test_content).expect("create workflow asset");
    thread::sleep(Duration::from_millis(200));

    assert!(test_file.exists(), "file should exist after creation");
    assert_eq!(
        fs::read_to_string(&test_file).expect("read workflow asset"),
        test_content,
        "file content should match written content"
    );

    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Workflow, "test-workflow")
        .expect("workflow asset should be readable immediately");
    assert_eq!(asset["name"], "test-workflow");
    assert_eq!(asset["version"], "1.0.0");
}

// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileChangedUpdatesResourceInMcpServer.
#[test]
fn file_changed_updates_resource_in_asset_manager() {
    let _guard = test_mutex().lock().expect("mutex");
    let harness = AssetHotLoadingHarness::new();

    let test_file = harness.roles_path.join("modify-test.json");
    let initial_content = r#"{"name":"original-role","version":"1.0.0"}"#;
    fs::write(&test_file, initial_content).expect("write initial role asset");
    thread::sleep(Duration::from_millis(200));

    let updated_content = r#"{"name":"modified-role","version":"2.0.0"}"#;
    fs::write(&test_file, updated_content).expect("update role asset");
    thread::sleep(Duration::from_millis(200));

    let read_content = fs::read_to_string(&test_file).expect("read updated role asset");
    assert_eq!(
        read_content, updated_content,
        "file should contain updated content"
    );
    assert!(
        read_content.contains("modified-role"),
        "updated content should be present"
    );
    assert!(
        !read_content.contains("original-role"),
        "old content should be replaced"
    );

    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Role, "modify-test")
        .expect("updated role asset should be readable immediately");
    assert_eq!(asset["name"], "modified-role");
    assert_eq!(asset["version"], "2.0.0");
}

// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileDeletedRemovesResourceFromMcpServer.
#[test]
fn file_deleted_removes_resource_from_asset_manager() {
    let _guard = test_mutex().lock().expect("mutex");
    let harness = AssetHotLoadingHarness::new();

    let test_file = harness.colors_path.join("delete-test.json");
    let test_content = r##"{"name":"delete-color","hex":"#FF0000"}"##;
    fs::write(&test_file, test_content).expect("write color asset");
    thread::sleep(Duration::from_millis(200));

    assert!(test_file.exists(), "file should exist before deletion");
    assert!(
        harness
            .asset_manager()
            .read_asset(AssetKind::Color, "delete-test")
            .is_ok(),
        "asset should be readable before deletion"
    );

    fs::remove_file(&test_file).expect("delete color asset");
    thread::sleep(Duration::from_millis(200));

    assert!(!test_file.exists(), "file should not exist after deletion");

    let error = harness
        .asset_manager()
        .read_asset(AssetKind::Color, "delete-test")
        .expect_err("deleted asset should no longer be readable");
    assert!(
        matches!(&error, MaenifoldError::AssetNotFound(kind, id) if kind == "color" && id == "delete-test"),
        "expected missing asset error, got: {error:?}"
    );
}

// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs AssetReloadLatencyIsUnder500ms.
#[test]
fn asset_reload_latency_is_under_500ms() {
    let _guard = test_mutex().lock().expect("mutex");
    let harness = AssetHotLoadingHarness::new();

    let test_file = harness.perspectives_path.join("latency-test.json");
    let test_content = r#"{"name":"test-perspective","description":"Performance test"}"#;

    let started = Instant::now();
    fs::write(&test_file, test_content).expect("write perspective asset");
    thread::sleep(Duration::from_millis(200));
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_millis(500),
        "asset reload latency was {}ms, expected < 500ms",
        elapsed.as_millis()
    );
    assert!(
        test_file.exists(),
        "file should exist for latency verification"
    );

    let asset = harness
        .asset_manager()
        .read_asset(AssetKind::Perspective, "latency-test")
        .expect("latency test asset should be readable immediately");
    assert_eq!(asset["name"], "test-perspective");
    assert_eq!(asset["description"], "Performance test");
}
