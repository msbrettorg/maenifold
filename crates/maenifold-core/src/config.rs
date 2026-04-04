use crate::error::{MaenifoldError, Result};
use parking_lot::RwLock;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

#[derive(Debug, Clone, Default)]
struct Overrides {
    root: Option<PathBuf>,
    database_path: Option<PathBuf>,
}

static OVERRIDES: LazyLock<RwLock<Overrides>> = LazyLock::new(|| RwLock::new(Overrides::default()));

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
        for path in [
            &self.maenifold_root,
            &self.memory_path,
            &self.assets_path,
            &self.maenifold_root.join("logs"),
        ] {
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

    // T-92.2: rolling log rotation strategy.
    pub fn log_rotation(&self) -> String {
        env_string("MAENIFOLD_LOG_ROTATION", "daily")
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

    // T-10.9: RTM FR-15
    pub fn enable_auto_sync(&self) -> bool {
        env_bool("MAENIFOLD_AUTO_SYNC", true)
    }

    // T-10.9: RTM FR-15
    pub fn incremental_optimize_every(&self) -> u32 {
        env_u32("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", 40).max(1)
    }

    // T-10.9: RTM FR-15
    pub fn incremental_vacuum_minutes(&self) -> u32 {
        env_u32("MAENIFOLD_INCREMENTAL_VACUUM_MINUTES", 720)
    }

    // T-10.9: RTM FR-15
    pub fn enable_session_cleanup(&self) -> bool {
        env_bool("MAENIFOLD_SESSION_CLEANUP", true)
    }

    // T-10.9: RTM FR-15
    pub fn enable_sync_logging(&self) -> bool {
        env_bool("MAENIFOLD_SYNC_LOGGING", true)
    }

    // T-10.9: RTM FR-15
    pub fn watcher_buffer_size(&self) -> usize {
        env_usize("MAENIFOLD_WATCHER_BUFFER", 65_536)
    }

    // T-10.9: RTM FR-15
    pub fn sqlite_busy_timeout_ms(&self) -> u64 {
        env_u64("MAENIFOLD_SQLITE_BUSY_TIMEOUT", 5_000)
    }

    // T-10.9: RTM FR-15
    pub fn enable_embedding_logs(&self) -> bool {
        env_bool("MAENIFOLD_EMBEDDING_LOGS", false)
    }

    // T-10.9: RTM FR-15
    pub fn enable_vector_logs(&self) -> bool {
        env_bool("MAENIFOLD_VECTOR_LOGS", false)
    }

    // T-11.2: RTM FR-15
    pub fn sync_lease_ms(&self) -> u64 {
        env_u64("MAENIFOLD_SYNC_LEASE_MS", 10_000).clamp(1_000, 60_000)
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

fn env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

// T-11.2: RTM FR-15
pub(crate) fn env_bool(name: &str, default: bool) -> bool {
    env::var(name)
        .ok()
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        })
        .unwrap_or(default)
}
