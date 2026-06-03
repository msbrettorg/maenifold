// T-01.16: config-loading helpers extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use std::collections::HashMap;
#[cfg(unix)]
use std::ffi::CString;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::error::{MaenifoldError, Result};

use super::gateway_settings::IntentResolverSettings;
use super::settings::MaenifoldSettings;
use super::types::{AgentEntry, McpServerEntry};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mcp_servers: HashMap<String, McpServerEntry>,
    #[serde(default)]
    pub agents: HashMap<String, AgentEntry>,
    #[serde(default, rename = "mcpAllowedHosts", skip_serializing_if = "Vec::is_empty")]
    pub mcp_allowed_hosts: Vec<String>,
    #[serde(default, rename = "mcpAllowedOrigins", skip_serializing_if = "Vec::is_empty")]
    pub mcp_allowed_origins: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<MaenifoldSettings>,
}

impl AgentsConfig {
    pub fn load(root: &Path) -> Result<AgentsConfig> {
        let config_path = root.join("maenifold.json");
        let contents = read_config_contents(&config_path)?;

        reject_legacy_mcp_servers_array(&contents, &config_path)?;
        let parsed: AgentsConfig = parse_agents_config(&contents, &config_path)?;

        let mut agents = HashMap::new();
        for (name, entry) in parsed.agents {
            if validate_agent_entry(&name, &entry) {
                agents.insert(name, entry);
            }
        }

        let config = Self {
            mcp_servers: parsed.mcp_servers,
            agents,
            mcp_allowed_hosts: parsed.mcp_allowed_hosts,
            mcp_allowed_origins: parsed.mcp_allowed_origins,
            settings: parsed.settings,
        };

        if let Some(settings) = config.settings.as_ref() {
            validate_intent_resolver_config(&settings.gateway.intent_resolver, &config)?;
        }

        Ok(config)
    }

    pub fn load_or_default(root: &Path) -> Result<AgentsConfig> {
        let config_path = root.join("maenifold.json");
        let contents = match read_config_contents(&config_path) {
            Ok(contents) => contents,
            Err(error) => {
                warn!(root = %root.display(), error = %error, "using default agents config");
                return Ok(Self::default());
            }
        };

        let parsed: AgentsConfig = match reject_legacy_mcp_servers_array(&contents, &config_path)
            .and_then(|_| parse_agents_config(&contents, &config_path))
        {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!(root = %root.display(), error = %error, "using default agents config");
                return Ok(Self::default());
            }
        };

        let mut agents = HashMap::new();
        for (name, entry) in parsed.agents {
            if validate_agent_entry(&name, &entry) {
                agents.insert(name, entry);
            }
        }

        let config = Self {
            mcp_servers: parsed.mcp_servers,
            agents,
            mcp_allowed_hosts: parsed.mcp_allowed_hosts,
            mcp_allowed_origins: parsed.mcp_allowed_origins,
            settings: parsed.settings,
        };

        if let Some(settings) = config.settings.as_ref() {
            validate_intent_resolver_config(&settings.gateway.intent_resolver, &config)?;
        }

        Ok(config)
    }
}

fn parse_agents_config(contents: &str, config_path: &Path) -> Result<AgentsConfig> {
    serde_json::from_str(contents).map_err(|error| {
        MaenifoldError::AgentSetup(format!(
            "failed to parse agents config '{}': {error}",
            config_path.display()
        ))
    })
}

fn reject_legacy_mcp_servers_array(contents: &str, config_path: &Path) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(contents).map_err(|error| {
        MaenifoldError::AgentSetup(format!(
            "failed to parse agents config '{}': {error}",
            config_path.display()
        ))
    })?;
    if value
        .get("mcpServers")
        .is_some_and(serde_json::Value::is_array)
    {
        return Err(MaenifoldError::AgentSetup(format!(
            "failed to parse agents config '{}': mcpServers legacy array form is not supported; migration T-334.4 requires the new object-keyed shape: {{\"mcpServers\": {{\"server-name\": {{\"command\": \"...\"}}}}}}",
            config_path.display()
        )));
    }
    Ok(())
}

fn read_config_contents(config_path: &Path) -> Result<String> {
    match fs::read_to_string(config_path) {
        Ok(contents) => {
            info!(path = %config_path.display(), "loaded agents config file");
            Ok(contents)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(MaenifoldError::AgentSetup(format!(
                "agents config '{}' not found",
                config_path.display()
            )))
        }
        Err(error) => Err(MaenifoldError::AgentSetup(format!(
            "failed to read agents config '{}': {error}",
            config_path.display()
        ))),
    }
}

pub(crate) fn validate_intent_resolver_config(
    intent_resolver: &IntentResolverSettings,
    _agents_config: &AgentsConfig,
) -> Result<()> {
    if !(0.0..=1.0).contains(&intent_resolver.auto_execute_confidence_threshold) {
        return Err(MaenifoldError::AgentSetup(
            "gateway intent resolver autoExecuteConfidenceThreshold must be between 0.0 and 1.0"
                .to_string(),
        ));
    }
    if intent_resolver.timeout_seconds == Some(0) {
        return Err(MaenifoldError::AgentSetup(
            "gateway intent resolver timeoutSeconds must be greater than 0".to_string(),
        ));
    }
    if intent_resolver.max_candidates == Some(0) {
        return Err(MaenifoldError::AgentSetup(
            "gateway intent resolver maxCandidates must be greater than 0".to_string(),
        ));
    }
    if let Some(level) = intent_resolver.effective_thought_level()
        && !matches!(level, "low" | "medium" | "high" | "xhigh")
    {
        return Err(MaenifoldError::AgentSetup(format!(
            "gateway intent resolver thoughtLevel '{level}' is unsupported; expected none, low, medium, high, or xhigh"
        )));
    }
    if !intent_resolver.enabled {
        return Ok(());
    }
    Ok(())
}

pub(crate) fn validate_agent_entry(name: &str, entry: &AgentEntry) -> bool {
    let (command_path, _) = entry.command_parts();
    if command_path.is_empty() {
        warn!(agent = name, "excluding agent: command must not be empty");
        return false;
    }
    let command_path = Path::new(command_path);
    if !command_path.is_absolute() {
        warn!(
            agent = name,
            command = %entry.command,
            "excluding agent: command path must be absolute"
        );
        return false;
    }

    match fs::metadata(command_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            warn!(
                agent = name,
                command = %entry.command,
                "excluding agent: command does not exist"
            );
            return false;
        }
        Err(error) => {
            warn!(
                agent = name,
                command = %entry.command,
                error = %error,
                "excluding agent: failed to inspect command"
            );
            return false;
        }
    };

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let c_path = match CString::new(command_path.as_os_str().as_bytes()) {
            Ok(path) => path,
            Err(_) => {
                warn!(
                    agent = name,
                    command = %entry.command,
                    "excluding agent: command path contains interior NUL"
                );
                return false;
            }
        };

        // SAFETY: c_path is a valid NUL-terminated C string that lives for the duration
        // of the call, and libc::access is a read-only POSIX syscall.
        let accessible = unsafe { libc::access(c_path.as_ptr(), libc::X_OK) };
        if accessible != 0 {
            warn!(
                agent = name,
                command = %entry.command,
                "excluding agent: command is not executable by the current user"
            );
            return false;
        }
    }

    true
}
