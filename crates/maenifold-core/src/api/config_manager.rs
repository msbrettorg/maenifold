use crate::acp_config::AgentsConfig;
use crate::acp_config::{AgentEntry, MaenifoldSettings, McpServerEntry};
use crate::config::Config;
use indexmap::IndexMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Read/write mirror of maenifold.json kept hot for the HTTP API and MCP
// gateway surfaces. Gateway config mutations persist through this manager so
// the daemon and config watch continue to share one file-authoritative source
// of truth.
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

    pub(crate) fn render_config(&self) -> Result<String, String> {
        match fs::read_to_string(&self.config_path) {
            Ok(content) => Ok(content.trim_end().to_string()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                serde_json::to_string_pretty(&*self.config.read())
                    .map_err(|error| error.to_string())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    pub(crate) fn update_settings(
        &self,
        settings: MaenifoldSettings,
    ) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        config.settings = Some(settings);
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn add_agent(
        &self,
        name: String,
        entry: AgentEntry,
    ) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if config.agents.contains_key(&name) {
            return Err(format!("duplicate agent name: {name}"));
        }
        config.agents.insert(name, entry);
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn update_agent(
        &self,
        name: String,
        entry: AgentEntry,
    ) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if !config.agents.contains_key(&name) {
            return Err(format!("agent not found: {name}"));
        }
        config.agents.insert(name, entry);
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn remove_agent(&self, name: &str) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if config.agents.remove(name).is_none() {
            return Err(format!("agent not found: {name}"));
        }
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn add_mcp_server(
        &self,
        name: String,
        entry: McpServerEntry,
    ) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if config.mcp_servers.contains_key(&name) {
            return Err(format!("duplicate mcp server name: {name}"));
        }
        config.mcp_servers.insert(name, entry);
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn update_mcp_server(
        &self,
        name: &str,
        entry: McpServerEntry,
    ) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if !config.mcp_servers.contains_key(name) {
            return Err(format!("mcp server not found: {name}"));
        }
        config.mcp_servers.insert(name.to_string(), entry);
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }

    pub(crate) fn remove_mcp_server(&self, name: &str) -> Result<AgentsConfig, String> {
        let mut config = self.config.write();
        if config.mcp_servers.remove(name).is_none() {
            return Err(format!("mcp server not found: {name}"));
        }
        persist_config(&self.config_path, &config)?;
        Ok(config.clone())
    }
}

fn persist_config(path: &Path, config: &AgentsConfig) -> Result<(), String> {
    let content = ordered_config_json(path, config)?;
    fs::write(path, format!("{content}\n")).map_err(|error| error.to_string())
}

fn load_config(path: &Path) -> Result<AgentsConfig, String> {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AgentsConfig::default()),
        Err(error) => Err(error.to_string()),
    }
}

fn ordered_config_json(path: &Path, config: &AgentsConfig) -> Result<String, String> {
    let mut value = serde_json::to_value(config).map_err(|error| error.to_string())?;
    let Some(root) = value.as_object_mut() else {
        return serde_json::to_string_pretty(config).map_err(|error| error.to_string());
    };

    let Some(Value::Object(servers)) = root.remove("mcpServers") else {
        return serde_json::to_string_pretty(&value).map_err(|error| error.to_string());
    };

    let prior_order = read_mcp_server_order(path)?;
    let mut remaining: IndexMap<String, Value> = servers.into_iter().collect();
    let mut ordered = IndexMap::new();

    for key in prior_order {
        if let Some(entry) = remaining.shift_remove(&key) {
            ordered.insert(key, entry);
        }
    }
    ordered.extend(remaining);
    root.insert(
        "mcpServers".to_string(),
        Value::Object(ordered.into_iter().collect()),
    );

    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}

fn read_mcp_server_order(path: &Path) -> Result<Vec<String>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.to_string()),
    };

    let value: Value = serde_json::from_str(&contents).map_err(|error| error.to_string())?;
    Ok(value
        .get("mcpServers")
        .and_then(Value::as_object)
        .map(|servers| servers.keys().cloned().collect())
        .unwrap_or_default())
}
