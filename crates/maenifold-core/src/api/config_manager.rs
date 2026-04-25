use crate::acp_config::{AgentEntry, AgentsConfig, MaenifoldSettings, McpServerEntry};
use crate::config::Config;
use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Reuses the daemon's existing maenifold.json pathing model while matching the
// desktop config mutation flow closely enough for the HTTP API surface.
#[derive(Clone)]
pub(crate) struct ConfigManager {
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub(crate) fn new(config: &Config) -> Self {
        Self {
            config: Arc::new(RwLock::new(AgentsConfig::load_or_default(
                &config.maenifold_root,
            ))),
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

    pub(crate) fn save(&self) -> Result<(), String> {
        let config = self.config.read();
        save_config(&self.config_path, &config)
    }

    pub(crate) fn add_agent(
        &self,
        name: String,
        config: AgentEntry,
    ) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.insert(name, config);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn remove_agent(&self, name: &str) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.remove(name);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn update_agent(
        &self,
        name: String,
        config: AgentEntry,
    ) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            if agents_config.agents.contains_key(&name) {
                agents_config.agents.insert(name.clone(), config);
            } else {
                return Err(format!("Agent '{name}' not found"));
            }
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn add_mcp_server(&self, entry: McpServerEntry) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.mcp_servers.push(entry);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn remove_mcp_server(&self, name: &str) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.mcp_servers.retain(|server| server.name() != name);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn update_mcp_server(
        &self,
        name: &str,
        entry: McpServerEntry,
    ) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            if let Some(existing) = config
                .mcp_servers
                .iter_mut()
                .find(|server| server.name() == name)
            {
                *existing = entry;
            } else {
                return Err(format!("MCP server '{name}' not found"));
            }
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub(crate) fn update_settings(
        &self,
        settings: MaenifoldSettings,
    ) -> Result<AgentsConfig, String> {
        {
            let mut config = self.config.write();
            config.settings = Some(settings);
        }
        self.save()?;
        Ok(self.get_config())
    }
}

fn load_config(path: &Path) -> Result<AgentsConfig, String> {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AgentsConfig::default()),
        Err(error) => Err(error.to_string()),
    }
}

fn save_config(path: &Path, config: &AgentsConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}
