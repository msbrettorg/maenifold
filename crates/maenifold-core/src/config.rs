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

    pub fn enable_incremental_sync(&self) -> bool {
        env::var("MAENIFOLD_ENABLE_INCREMENTAL_SYNC")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true)
    }
}
