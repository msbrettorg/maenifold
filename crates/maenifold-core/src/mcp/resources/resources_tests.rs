use super::*;
use crate::config::Config;
use crate::mcp::initialize_runtime;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Mutex;
use tempfile::TempDir;

// T-10.2: RTM FR-11 — serialise MCP resources tests to avoid concurrent Config override races.
static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestRoot {
    _guard: std::sync::MutexGuard<'static, ()>,
    _tmp: TempDir,
}

impl TestRoot {
    fn new() -> Self {
        // T-10.2: Recover from a poisoned mutex so that a prior test panic does not
        // cascade-fail every subsequent test in this module.
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let tmp = TempDir::new().expect("temp dir");
        // T-10.2: Explicitly set both root AND database_path overrides, then snapshot
        // Config::current() immediately.  This prevents a race where another test
        // module (e.g. decay::scoring) calls Config::reset_overrides() between our
        // override_root and our Config::current() read.
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        initialize_runtime(&config).expect("runtime init should succeed");
        Self {
            _guard: guard,
            _tmp: tmp,
        }
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-10.2: RTM FR-11 / RTM-069 — extracted MCP resources module enumerates all asset resources.
#[test]
fn t_10_2_list_asset_resources_covers_all_asset_types() {
    let _root = TestRoot::new();

    let resources = list_asset_resources(current_asset_manager().expect("asset manager"))
        .expect("resources should list");

    assert!(
        resources
            .iter()
            .any(|r| r.uri.starts_with("asset://workflows/"))
    );
    assert!(
        resources
            .iter()
            .any(|r| r.uri.starts_with("asset://roles/"))
    );
    assert!(
        resources
            .iter()
            .any(|r| r.uri.starts_with("asset://colors/"))
    );
    assert!(
        resources
            .iter()
            .any(|r| r.uri.starts_with("asset://perspectives/"))
    );
}

// T-10.2: RTM FR-11 / RTM-070 — extracted MCP resources module resolves asset:// URIs to JSON.
#[test]
fn t_10_2_current_asset_manager_reads_asset_uri() {
    let _root = TestRoot::new();

    let resource = current_asset_manager()
        .expect("asset manager")
        .read_resource("asset://workflows/deductive-reasoning")
        .expect("resource should read");

    assert_eq!(
        resource.get("id").and_then(Value::as_str),
        Some("deductive-reasoning")
    );
}
