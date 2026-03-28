#![allow(dead_code, unused_imports)]

use maenifold_core::assets::update_assets;
use maenifold_core::json::OutputFormat;
use maenifold_core::tools::ToolContext;
use maenifold_core::{AssetManager, Config, Result};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

// T-12.7: RTM NFR-1 - shared helpers keep split asset-manager integration tests behaviorally identical.
//
// NOTE: Cross-binary mutex gap (latent). This mutex serializes tests within a single integration
// test binary, but other binaries (e.g. update_assets_boundary, asset_hot_loading_tests) declare
// their own static mutexes. PackagedAssetGuard writes into the shared workspace `assets/` directory
// and HiddenDirGuard renames it, so cross-binary races are theoretically possible when `cargo test`
// runs integration binaries in parallel. In practice the overlap probability is low, but callers
// accept this risk. A cross-binary coordination mechanism (e.g. file lock) would eliminate it.
pub fn test_mutex() -> &'static Mutex<()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX.get_or_init(|| Mutex::new(()))
}

pub fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

pub fn workspace_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("assets")
}

pub fn executable_assets_root() -> PathBuf {
    std::env::current_exe()
        .expect("current exe")
        .parent()
        .expect("exe parent")
        .join("assets")
}

pub fn unique_suffix(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    format!("{prefix}-{nanos}")
}

pub fn unique_relative_path(dir: &str, prefix: &str, extension: &str) -> String {
    format!("{dir}/{}.{}", unique_suffix(prefix), extension)
}

pub fn write_file(path: &Path, content: &str) {
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

pub struct ConfigResetGuard;

impl Drop for ConfigResetGuard {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct PackagedAssetGuard {
    path: PathBuf,
    root: PathBuf,
}

impl PackagedAssetGuard {
    pub fn create(relative: &str, content: &str) -> Self {
        let root = workspace_assets_root();
        let path = root.join(relative);
        write_file(&path, content);
        Self { path, root }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PackagedAssetGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        remove_empty_ancestors(self.path.parent(), &self.root);
    }
}

pub struct HiddenDirGuard {
    original: PathBuf,
    hidden: Option<PathBuf>,
}

impl HiddenDirGuard {
    pub fn hide(path: PathBuf) -> Self {
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

pub fn hide_packaged_asset_roots() -> Vec<HiddenDirGuard> {
    let mut paths = vec![workspace_assets_root(), executable_assets_root()];
    paths.sort();
    paths.dedup();
    paths.into_iter().map(HiddenDirGuard::hide).collect()
}

pub fn invoke_update_assets(config: &Config, dry_run: bool) -> Result<String> {
    let context = ToolContext::new(config.clone(), OutputFormat::Markdown);
    update_assets(&context, &json!({ "dryRun": dry_run }))
}

pub fn get_asset_path(config: &Config, relative_path: &str) -> PathBuf {
    config.assets_path.join(relative_path)
}

pub fn asset_exists(config: &Config, relative_path: &str) -> bool {
    get_asset_path(config, relative_path).exists()
}

pub fn load_asset_text(config: &Config, relative_path: &str) -> Option<String> {
    fs::read_to_string(get_asset_path(config, relative_path)).ok()
}

pub fn load_asset_json<T: DeserializeOwned>(config: &Config, relative_path: &str) -> Option<T> {
    load_asset_text(config, relative_path).and_then(|text| serde_json::from_str(&text).ok())
}

pub fn initialize_assets(config: &Config) {
    AssetManager::initialize_assets(config).expect("initialize assets");
}
