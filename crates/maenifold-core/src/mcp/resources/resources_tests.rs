use super::*;
use crate::config::Config;
use crate::mcp::initialize_runtime;
use serde_json::Value;
use serial_test::serial;
use tempfile::TempDir;

struct TestRoot {
    _tmp: TempDir,
}

impl TestRoot {
    fn new() -> Self {
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
        Self { _tmp: tmp }
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

// T-10.2: RTM FR-11 / RTM-069 — extracted MCP resources module enumerates all asset resources.
#[test]
#[serial(config)]
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
#[serial(config)]
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
