// T-01.16: config-loading helpers extracted from acp/config.rs to keep
// each module ≤300 LOC. No behavioural changes.

use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::settings::MaenifoldSettings;
use super::types::{AgentEntry, McpServerEntry};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
    #[serde(default)]
    pub agents: HashMap<String, AgentEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<MaenifoldSettings>,
}

impl AgentsConfig {
    pub fn load(root: &Path) -> AgentsConfig {
        let config_path = root.join("maenifold.json");
        let Some(contents) = read_config_contents(&config_path) else {
            return Self::default();
        };

        let parsed: AgentsConfig = match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(error) => {
                warn!(
                    path = %config_path.display(),
                    error = %error,
                    "failed to parse agents config"
                );
                return Self::default();
            }
        };

        let mut agents = HashMap::new();
        for (name, entry) in parsed.agents {
            if validate_agent_entry(&name, &entry) {
                agents.insert(name, entry);
            }
        }

        Self {
            mcp_servers: parsed.mcp_servers,
            agents,
            settings: parsed.settings,
        }
    }
}

fn read_config_contents(config_path: &Path) -> Option<String> {
    match fs::read_to_string(config_path) {
        Ok(contents) => {
            info!(path = %config_path.display(), "loaded agents config file");
            Some(contents)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            warn!(
                path = %config_path.display(),
                error = %error,
                "failed to read agents config"
            );
            None
        }
    }
}

fn validate_agent_entry(name: &str, entry: &AgentEntry) -> bool {
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

    let c_path = match CString::new(command_path.as_os_str().as_encoded_bytes()) {
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

    // SAFETY: c_path is a valid NUL-terminated C string that lives for the duration of
    // the call, and libc::access is a read-only POSIX syscall with no memory side-effects.
    let accessible = unsafe { libc::access(c_path.as_ptr(), libc::X_OK) };
    if accessible != 0 {
        warn!(
            agent = name,
            command = %entry.command,
            "excluding agent: command is not executable by the current user"
        );
        return false;
    }

    true
}
