use crate::error::{MaenifoldError, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
struct Overrides {
    root: Option<PathBuf>,
    database_path: Option<PathBuf>,
}

static OVERRIDES: Lazy<RwLock<Overrides>> = Lazy::new(|| RwLock::new(Overrides::default()));

#[derive(Debug, Clone)]
pub struct Config {
    pub maenifold_root: PathBuf,
    pub memory_path: PathBuf,
    pub assets_path: PathBuf,
    pub thinking_path: PathBuf,
    pub sequential_path: PathBuf,
    pub database_path: PathBuf,
}

impl Config {
    pub const SQLITE_BUSY_TIMEOUT_MS: u64 = 5_000;

    pub fn current() -> Self {
        let overrides = OVERRIDES.read().clone();
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let maenifold_root = overrides
            .root
            .or_else(|| env::var_os("MAENIFOLD_ROOT").map(PathBuf::from))
            .unwrap_or_else(|| home.join("maenifold"));
        let database_path = overrides
            .database_path
            .or_else(|| env::var_os("MAENIFOLD_DATABASE_PATH").map(PathBuf::from))
            .unwrap_or_else(|| maenifold_root.join("memory.db"));
        let memory_path = maenifold_root.join("memory");
        let assets_path = maenifold_root.join("assets");
        let thinking_path = memory_path.join("thinking");
        let sequential_path = thinking_path.join("sequential");
        Self {
            maenifold_root,
            memory_path,
            assets_path,
            thinking_path,
            sequential_path,
            database_path,
        }
    }

    pub fn override_root(path: impl AsRef<Path>) {
        OVERRIDES.write().root = Some(path.as_ref().to_path_buf());
    }

    pub fn set_database_path(path: impl AsRef<Path>) {
        OVERRIDES.write().database_path = Some(path.as_ref().to_path_buf());
    }

    pub fn reset_overrides() {
        *OVERRIDES.write() = Overrides::default();
    }

    pub fn ensure_directories(&self) -> Result<()> {
        for path in [&self.maenifold_root, &self.memory_path, &self.assets_path] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    pub fn database_connection_string(&self) -> Result<String> {
        let path = self
            .database_path
            .to_str()
            .ok_or_else(|| MaenifoldError::NonUtf8Path(self.database_path.clone()))?;
        Ok(format!("file:{path}?mode=rwc"))
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_sequential(&self) -> i64 {
        env_i64("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", 7)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_workflows(&self) -> i64 {
        env_i64("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", 14)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_default(&self) -> i64 {
        env_i64("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", 28)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_half_life_days(&self) -> i64 {
        env_i64("MAENIFOLD_DECAY_HALF_LIFE_DAYS", 30)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_function(&self) -> String {
        env_string("MAENIFOLD_DECAY_FUNCTION", "power-law")
    }

    pub fn enable_incremental_sync(&self) -> bool {
        env::var("MAENIFOLD_ENABLE_INCREMENTAL_SYNC")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true)
    }
}

fn env_i64(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_string(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}
