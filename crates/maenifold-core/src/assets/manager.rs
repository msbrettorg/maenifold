use super::support::{
    asset_files_in_root, asset_is_multi_agent, emit_asset_degraded_diagnostic,
    resolve_packaged_assets_root,
};
use super::types::{AssetCatalog, AssetKind};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AssetManager {
    root: PathBuf,
}

impl AssetManager {
    pub fn new(config: &Config) -> Self {
        Self {
            root: config.assets_path.clone(),
        }
    }

    // T-ASSET-PATH: RTM FR-3, FR-20 — use runtime asset resolution instead
    // of compile-time CARGO_MANIFEST_DIR so the binary works on any machine.
    pub fn initialize_assets(config: &Config) -> Result<()> {
        config.ensure_directories()?;
        let packaged_root = match resolve_packaged_assets_root() {
            Some(p) => p,
            None => return Ok(()), // No packaged assets found — graceful
        };

        for entry in walkdir::WalkDir::new(&packaged_root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            let src = entry.path();
            let relative = match src.strip_prefix(&packaged_root) {
                Ok(relative) if !relative.as_os_str().is_empty() => relative,
                _ => continue,
            };
            if should_skip_packaged_symlink_entry(src)? {
                continue;
            }
            let dst = config.assets_path.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&dst)?;
                continue;
            }
            if dst.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(error) = fs::copy(src, &dst) {
                if should_skip_initialize_asset_copy(src, &error) {
                    continue;
                }
                return Err(error.into());
            }
        }
        Ok(())
    }

    pub fn list_assets(&self, kind: Option<AssetKind>) -> Result<Value> {
        if let Some(kind) = kind {
            let values = self.load_kind_metadata(kind)?;
            Ok(Value::Array(values))
        } else {
            Ok(Value::Array(
                AssetKind::all()
                    .into_iter()
                    .map(|kind| Value::String(kind.as_str().to_string()))
                    .collect(),
            ))
        }
    }

    pub fn catalog(&self) -> Result<AssetCatalog> {
        let workflows = self.load_kind_metadata_from_root(&self.root, AssetKind::Workflow)?;
        // T-0.2: RTM NFR-3 - catalog shape mirrors the C# oracle by splitting workflows on `multiAgent`.
        let (multi_agent_workflows, single_agent_workflows): (Vec<_>, Vec<_>) =
            workflows.into_iter().partition(asset_is_multi_agent);
        Ok(AssetCatalog {
            single_agent_workflows,
            multi_agent_workflows,
            roles: self.load_kind_metadata_from_root(&self.root, AssetKind::Role)?,
            colors: self.load_kind_metadata_from_root(&self.root, AssetKind::Color)?,
            perspectives: self.load_kind_metadata_from_root(&self.root, AssetKind::Perspective)?,
        })
    }

    pub fn read_resource(&self, uri: &str) -> Result<Value> {
        if uri.eq_ignore_ascii_case("asset://catalog") {
            let catalog = self.catalog()?;
            return serde_json::to_value(catalog).map_err(Into::into);
        }
        let Some(rest) = uri.strip_prefix("asset://") else {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Expected 'asset://type/id'"
            )));
        };
        let mut parts = rest.splitn(2, '/');
        let kind = parts.next().ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Expected 'asset://type/id'"))
        })?;
        let id = parts.next().ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Expected 'asset://type/id'"))
        })?;
        let kind = AssetKind::parse(kind)?;
        self.read_asset(kind, id)
    }

    pub fn read_asset(&self, kind: AssetKind, id: &str) -> Result<Value> {
        let candidates = self.asset_files(kind)?;
        let target = id.to_ascii_lowercase();
        for path in &candidates {
            // T-0.3g: RTM FR-3, RTM-047 - keep asset lookup behavior while satisfying clippy.
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && stem.eq_ignore_ascii_case(&target)
            {
                return Ok(serde_json::from_str(&fs::read_to_string(path)?)?);
            }
        }

        // T-0.2: RTM NFR-3 - ReadMcpResource must fall back to the JSON `id` field like the C# oracle.
        for path in candidates {
            let content = fs::read_to_string(&path)?;
            let value: Value = match serde_json::from_str(&content) {
                Ok(value) => value,
                Err(error) => {
                    // T-0.1: RTM FR-25.2 preserves degraded asset lookup diagnostics when malformed JSON is skipped.
                    emit_asset_degraded_diagnostic("FindAssetFileById", &path, &error);
                    continue;
                }
            };
            if value
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|asset_id| asset_id.eq_ignore_ascii_case(&target))
            {
                return Ok(value);
            }
        }

        Err(MaenifoldError::AssetNotFound(
            kind.as_str().to_string(),
            id.to_string(),
        ))
    }

    fn load_kind_metadata(&self, kind: AssetKind) -> Result<Vec<Value>> {
        self.load_kind_metadata_from_root(&self.root, kind)
    }

    fn load_kind_metadata_from_root(&self, root: &Path, kind: AssetKind) -> Result<Vec<Value>> {
        let mut items = Vec::new();
        for path in asset_files_in_root(root, kind)? {
            let text = fs::read_to_string(&path)?;
            let value: Value = match serde_json::from_str(&text) {
                Ok(value) => value,
                Err(error) => {
                    // T-0.1: RTM FR-25.2 preserves degraded metadata listing diagnostics when malformed JSON is skipped.
                    emit_asset_degraded_diagnostic("GetAssetMetadata", &path, &error);
                    continue;
                }
            };
            let mut metadata = serde_json::Map::new();
            for field in ["id", "name", "emoji", "description"] {
                if let Some(v) = value.get(field).cloned() {
                    metadata.insert(field.to_string(), v);
                }
            }
            // T-0.2: RTM NFR-3 - C# metadata always includes `multiAgent`, defaulting to false.
            metadata.insert(
                "multiAgent".to_string(),
                Value::Bool(
                    value
                        .get("multiAgent")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                ),
            );
            items.push(Value::Object(metadata));
        }
        Ok(items)
    }

    fn asset_files(&self, kind: AssetKind) -> Result<Vec<PathBuf>> {
        asset_files_in_root(&self.root, kind)
    }

    pub(crate) fn read_adopt_asset(&self, kind: AssetKind, identifier: &str) -> Result<String> {
        let identifier = super::support::validate_adopt_identifier(identifier)?;

        // T-1.1: RTM FR-1 - prefer direct filename lookup before scanning asset metadata.
        let fast_path = self
            .root
            .join(kind.dir_name())
            .join(format!("{identifier}.json"));
        if fast_path.is_file() {
            return Ok(fs::read_to_string(fast_path)?);
        }

        // T-1.1: RTM FR-1 - fall back to matching the JSON `id` field, skipping malformed JSON.
        for path in self.asset_files(kind)? {
            let content = fs::read_to_string(&path)?;
            let Ok(value) = serde_json::from_str::<Value>(&content) else {
                continue;
            };

            if value.get("id").and_then(Value::as_str) == Some(identifier) {
                return Ok(content);
            }
        }

        Err(MaenifoldError::AssetNotFound(
            kind.as_str().to_string(),
            identifier.to_string(),
        ))
    }
}

fn should_skip_packaged_symlink_entry(src: &Path) -> Result<bool> {
    if !src.is_symlink() {
        return Ok(false);
    }

    match fs::metadata(src) {
        Ok(metadata) => Ok(metadata.is_dir()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

fn should_skip_initialize_asset_copy(src: &Path, error: &std::io::Error) -> bool {
    if !src.is_symlink() {
        return false;
    }

    if error.kind() == std::io::ErrorKind::NotFound {
        return true;
    }

    if error.kind() != std::io::ErrorKind::InvalidInput {
        return false;
    }

    fs::metadata(src)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false)
}

// ── T-205.716: Concurrent access regression tests ────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::test_util::ConfigResetGuard;
    use std::sync::Arc;
    use std::thread;

    /// T-205.716: Regression test for ListAssets deadlock under concurrent access.
    ///
    /// This test verifies that multiple threads can safely call list_assets()
    /// concurrently without triggering "Resource deadlock avoided (os error 11)".
    /// The bug was originally observed when multiple agents accessed the asset
    /// system simultaneously.
    ///
    /// EXPECTED BEHAVIOR:
    /// - Before fix: This test should demonstrate the concurrency hazard
    /// - After fix: All threads complete successfully without deadlock errors
    #[test]
    fn concurrent_list_assets_does_not_deadlock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let assets_path = dir.path().join("assets");

        // Set up asset directories with some sample files
        for kind in AssetKind::all() {
            let kind_dir = assets_path.join(kind.dir_name());
            fs::create_dir_all(&kind_dir).expect("create asset dir");

            // Create a few asset files per kind
            for i in 0..5 {
                let file_path = kind_dir.join(format!("test-asset-{i}.json"));
                let content = format!(
                    r#"{{"id":"test-asset-{i}","name":"Test Asset {i}","description":"Test"}}"#
                );
                fs::write(&file_path, content).expect("write asset file");
            }
        }

        Config::override_root(dir.path());
        let _guard = ConfigResetGuard;

        let config = Arc::new(Config::current());
        let thread_count = 8;
        let iterations_per_thread = 50;

        let mut handles = Vec::with_capacity(thread_count);

        for thread_id in 0..thread_count {
            let config = Arc::clone(&config);
            let handle = thread::spawn(move || {
                for i in 0..iterations_per_thread {
                    let manager = AssetManager::new(&config);

                    // Alternate between typed and untyped calls
                    let result = if i % 2 == 0 {
                        manager.list_assets(None)
                    } else {
                        let kind = AssetKind::all()[i % 4];
                        manager.list_assets(Some(kind))
                    };

                    match result {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = e.to_string();
                            // Fail explicitly if we see the deadlock error
                            if err_str.contains("Resource deadlock avoided")
                                || err_str.contains("os error 11")
                            {
                                panic!(
                                    "Thread {thread_id} iteration {i}: Deadlock detected: {err_str}"
                                );
                            }
                            // Other errors may be acceptable during concurrent access
                            // but log them for debugging
                            eprintln!("Thread {thread_id} iteration {i}: Error: {err_str}");
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Join all threads and propagate any panics
        for (id, handle) in handles.into_iter().enumerate() {
            handle
                .join()
                .unwrap_or_else(|e| panic!("Thread {id} panicked: {e:?}"));
        }
    }

    /// T-205.716: Verify untyped list_assets returns exactly the four asset kinds.
    #[test]
    fn untyped_list_assets_returns_four_kinds() {
        let dir = tempfile::tempdir().expect("tempdir");
        Config::override_root(dir.path());
        let _guard = ConfigResetGuard;

        let config = Config::current();
        let manager = AssetManager::new(&config);

        let result = manager.list_assets(None).expect("list_assets should succeed");
        let Value::Array(kinds) = result else {
            panic!("Expected array, got: {result:?}");
        };

        let kind_names: Vec<&str> = kinds
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert_eq!(kind_names.len(), 4, "Should return exactly 4 asset kinds");
        assert!(kind_names.contains(&"workflow"), "Missing workflow");
        assert!(kind_names.contains(&"role"), "Missing role");
        assert!(kind_names.contains(&"color"), "Missing color");
        assert!(kind_names.contains(&"perspective"), "Missing perspective");
    }

    /// T-205.716: Verify typed list_assets returns metadata array for each kind.
    #[test]
    fn typed_list_assets_returns_metadata_array() {
        let dir = tempfile::tempdir().expect("tempdir");
        let assets_path = dir.path().join("assets");

        // Create a test asset
        let workflows_dir = assets_path.join("workflows");
        fs::create_dir_all(&workflows_dir).expect("create workflows dir");
        fs::write(
            workflows_dir.join("test-workflow.json"),
            r#"{"id":"test-workflow","name":"Test Workflow","description":"A test"}"#,
        )
        .expect("write workflow");

        Config::override_root(dir.path());
        let _guard = ConfigResetGuard;

        let config = Config::current();
        let manager = AssetManager::new(&config);

        let result = manager
            .list_assets(Some(AssetKind::Workflow))
            .expect("list_assets should succeed");

        let Value::Array(items) = result else {
            panic!("Expected array, got: {result:?}");
        };

        assert_eq!(items.len(), 1, "Should return exactly 1 workflow");
        let item = &items[0];
        assert_eq!(item["id"], "test-workflow");
        assert_eq!(item["name"], "Test Workflow");
    }

    /// T-205.716: Regression test simulating MCP server concurrency pattern.
    ///
    /// The actual MCP server uses `tokio::task::spawn_blocking` for list_resources.
    /// This test simulates multiple concurrent "agents" making asset listing calls
    /// while also performing file-system modifications (simulating asset updates).
    ///
    /// The deadlock in production occurs with "Resource deadlock avoided (os error 11)"
    /// which indicates a circular lock dependency at the kernel level.
    #[test]
    fn concurrent_list_and_modify_assets_does_not_deadlock() {
        use notify::{RecursiveMode, Watcher};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        let dir = tempfile::tempdir().expect("tempdir");
        let assets_path = dir.path().join("assets");

        // Set up asset directories with sample files
        for kind in AssetKind::all() {
            let kind_dir = assets_path.join(kind.dir_name());
            fs::create_dir_all(&kind_dir).expect("create asset dir");
            for i in 0..10 {
                let file_path = kind_dir.join(format!("asset-{i}.json"));
                let content = format!(
                    r#"{{"id":"asset-{i}","name":"Asset {i}","description":"Test"}}"#
                );
                fs::write(&file_path, content).expect("write asset file");
            }
        }

        Config::override_root(dir.path());
        let _guard = ConfigResetGuard;

        let config = Arc::new(Config::current());
        let stop_flag = Arc::new(AtomicBool::new(false));

        // Create a file watcher to simulate the MCP asset watcher
        let stop_flag_watcher = Arc::clone(&stop_flag);
        let _watcher = notify::recommended_watcher(move |_event: notify::Result<notify::Event>| {
            // Just consume events, simulating the MCP watcher
            if stop_flag_watcher.load(Ordering::Relaxed) {
                return;
            }
        })
        .expect("create watcher");

        // Watch the assets directory
        let mut watcher = notify::recommended_watcher(|_| {}).expect("watcher");
        let _ = watcher.watch(&assets_path, RecursiveMode::Recursive);

        let mut handles = Vec::new();

        // Thread 1: Continuous asset modifications (simulating UpdateAssets)
        let stop_flag_mod = Arc::clone(&stop_flag);
        let assets_path_mod = assets_path.clone();
        let modifier_handle = thread::spawn(move || {
            let mut counter = 0_u32;
            while !stop_flag_mod.load(Ordering::Relaxed) {
                for kind in AssetKind::all() {
                    let kind_dir = assets_path_mod.join(kind.dir_name());
                    let temp_path = kind_dir.join(format!("temp-{counter}.json"));
                    let content = format!(
                        r#"{{"id":"temp-{counter}","name":"Temp {counter}"}}"#
                    );
                    // Create and immediately remove to trigger fs events
                    if fs::write(&temp_path, content).is_ok() {
                        let _ = fs::remove_file(&temp_path);
                    }
                    counter = counter.wrapping_add(1);
                }
                thread::sleep(Duration::from_micros(100));
            }
        });
        handles.push(modifier_handle);

        // Threads 2-9: Concurrent list_assets calls
        for thread_id in 0..8 {
            let config = Arc::clone(&config);
            let stop_flag_reader = Arc::clone(&stop_flag);
            let handle = thread::spawn(move || {
                let mut iteration = 0_u32;
                while !stop_flag_reader.load(Ordering::Relaxed) && iteration < 100 {
                    let manager = AssetManager::new(&config);

                    // Alternate between typed and untyped calls
                    let result = if iteration % 2 == 0 {
                        manager.list_assets(None)
                    } else {
                        let kind = AssetKind::all()[(iteration as usize) % 4];
                        manager.list_assets(Some(kind))
                    };

                    match result {
                        Ok(_) => {}
                        Err(e) => {
                            let err_str = e.to_string();
                            if err_str.contains("Resource deadlock avoided")
                                || err_str.contains("os error 11")
                                || err_str.contains("EDEADLK")
                            {
                                panic!(
                                    "Thread {thread_id} iteration {iteration}: DEADLOCK: {err_str}"
                                );
                            }
                        }
                    }
                    iteration += 1;
                    thread::yield_now();
                }
            });
            handles.push(handle);
        }

        // Wait for reader threads to complete (should finish in ~1 second)
        thread::sleep(Duration::from_secs(2));
        stop_flag.store(true, Ordering::Relaxed);

        for (id, handle) in handles.into_iter().enumerate() {
            handle
                .join()
                .unwrap_or_else(|e| panic!("Thread {id} panicked: {e:?}"));
        }

        // Final verification: list_assets should still work after concurrent stress
        let manager = AssetManager::new(&config);
        let result = manager.list_assets(None);
        assert!(
            result.is_ok(),
            "list_assets should succeed after concurrent stress: {:?}",
            result.err()
        );
    }
}
