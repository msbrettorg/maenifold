use crate::acp_config::AgentsConfig;
use crate::config::Config;
use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Read-only mirror of maenifold.json kept hot for the HTTP API surface.
// Mutations now flow through the desktop app via Electron IPC; the daemon
// only reads + reloads through the file watcher (see api::config_watch).
#[derive(Clone)]
pub(crate) struct ConfigManager {
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            config: Arc::new(RwLock::new(
                AgentsConfig::load_or_default(&config.maenifold_root).unwrap_or_default(),
            )),
            config_path: config.maenifold_root.join("maenifold.json"),
        }
    }

    pub(crate) fn get_config(&self) -> AgentsConfig {
        self.config.read().clone()
    }

    pub(crate) fn reload(&self) -> Result<AgentsConfig, String> {
        let new_config = load_config(&self.config_path)?;
        *self.config.write() = new_config.clone();
        Ok(new_config)
    }

    pub(crate) fn get_config_path(&self) -> PathBuf {
        self.config_path.clone()
    }
}

fn load_config(path: &Path) -> Result<AgentsConfig, String> {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AgentsConfig::default()),
        Err(error) => Err(error.to_string()),
    }
}
