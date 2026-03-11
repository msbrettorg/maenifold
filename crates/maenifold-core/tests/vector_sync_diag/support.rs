//! T-9.6: RTM FR-7, FR-8 — shared vector sync diagnostics test support.

// T-9.6: RTM FR-7, FR-8 — shared helpers for vector sync diagnostics split tests.
use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct ConfigOverrideReset;

impl Drop for ConfigOverrideReset {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

pub struct EnvVarReset {
    key: &'static str,
    value: Option<String>,
}

impl EnvVarReset {
    pub fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var(key).ok();
        // T-9.6: RTM FR-7, FR-8 — preserve process env across vector diagnostics tests.
        unsafe { std::env::set_var(key, value) };
        Self {
            key,
            value: previous,
        }
    }
}

impl Drop for EnvVarReset {
    fn drop(&mut self) {
        match self.value.as_deref() {
            Some(value) => {
                // T-9.6: RTM FR-7, FR-8 — restore process env after vector diagnostics tests.
                unsafe { std::env::set_var(self.key, value) };
            }
            None => {
                // T-9.6: RTM FR-7, FR-8 — restore cleared process env after vector diagnostics tests.
                unsafe { std::env::remove_var(self.key) };
            }
        }
    }
}

pub fn isolated_config() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    (tmp, cfg)
}

pub fn write_memory_fixture(cfg: &Config, relative_path: &str, content: &str) -> PathBuf {
    let path = cfg.memory_path.join(relative_path);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, content).expect("write fixture");
    path
}

pub fn repo_source_path(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}
