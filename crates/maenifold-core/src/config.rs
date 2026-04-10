use crate::acp_config::{AgentsConfig, MaenifoldSettings};
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
    pub settings: MaenifoldSettings,
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
        let settings = AgentsConfig::load(&maenifold_root)
            .settings
            .unwrap_or_default();
        Self {
            maenifold_root,
            memory_path,
            assets_path,
            thinking_path,
            sequential_path,
            database_path,
            settings,
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
        env::var("MAENIFOLD_LOG_ROTATION")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.settings.log_rotation.clone())
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_sequential(&self) -> i64 {
        env::var("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(self.settings.decay.grace_days_sequential)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_workflows(&self) -> i64 {
        env::var("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(self.settings.decay.grace_days_workflows)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_grace_days_default(&self) -> i64 {
        env::var("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(self.settings.decay.grace_days_default)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_half_life_days(&self) -> i64 {
        env::var("MAENIFOLD_DECAY_HALF_LIFE_DAYS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or(self.settings.decay.half_life_days)
    }

    // T-GRAPH-DECAY-001.2-6: RTM NFR-7.5.x
    pub fn decay_function(&self) -> String {
        env::var("MAENIFOLD_DECAY_FUNCTION")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.settings.decay.function.clone())
    }

    // T-10.9: RTM FR-15
    pub fn enable_auto_sync(&self) -> bool {
        env_bool("MAENIFOLD_AUTO_SYNC", self.settings.auto_sync)
    }

    // T-10.9: RTM FR-15
    pub fn incremental_optimize_every(&self) -> u32 {
        env::var("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(self.settings.incremental_optimize_every)
            .max(1)
    }

    // T-10.9: RTM FR-15
    pub fn incremental_vacuum_minutes(&self) -> u32 {
        env::var("MAENIFOLD_INCREMENTAL_VACUUM_MINUTES")
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(self.settings.incremental_vacuum_minutes)
    }

    // T-10.9: RTM FR-15
    pub fn enable_session_cleanup(&self) -> bool {
        env_bool("MAENIFOLD_SESSION_CLEANUP", self.settings.session_cleanup)
    }

    // T-10.9: RTM FR-15
    pub fn enable_sync_logging(&self) -> bool {
        env_bool("MAENIFOLD_SYNC_LOGGING", self.settings.sync_logging)
    }

    // T-10.9: RTM FR-15
    pub fn watcher_buffer_size(&self) -> usize {
        env::var("MAENIFOLD_WATCHER_BUFFER")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(self.settings.watcher_buffer_size as usize)
    }

    // T-10.9: RTM FR-15
    pub fn sqlite_busy_timeout_ms(&self) -> u64 {
        env::var("MAENIFOLD_SQLITE_BUSY_TIMEOUT")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(self.settings.sqlite_busy_timeout_ms)
    }

    // T-10.9: RTM FR-15
    pub fn enable_embedding_logs(&self) -> bool {
        env_bool(
            "MAENIFOLD_EMBEDDING_LOGS",
            self.settings.vector.embedding_logs,
        )
    }

    // T-10.9: RTM FR-15
    pub fn enable_vector_logs(&self) -> bool {
        env_bool("MAENIFOLD_VECTOR_LOGS", self.settings.vector.vector_logs)
    }

    // T-11.2: RTM FR-15
    pub fn sync_lease_ms(&self) -> u64 {
        env::var("MAENIFOLD_SYNC_LEASE_MS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(self.settings.sync_lease_ms)
            .clamp(1_000, 60_000)
    }
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
