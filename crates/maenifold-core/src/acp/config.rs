use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub agents: HashMap<String, AgentEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub status: String,
}

impl AgentsConfig {
    pub fn load(root: &Path) -> AgentsConfig {
        let config_path: PathBuf = root.join("agents.json");
        let contents = match fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Self::default();
            }
            Err(error) => {
                eprintln!(
                    "Warning: failed to read agents config '{}': {error}",
                    config_path.display()
                );
                return Self::default();
            }
        };

        let parsed: AgentsConfig = match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(error) => {
                eprintln!(
                    "Warning: failed to parse agents config '{}': {error}",
                    config_path.display()
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

        Self { agents }
    }

    pub fn list_agents(&self) -> Vec<AgentInfo> {
        let mut agents = self
            .agents
            .keys()
            .cloned()
            .map(|name| AgentInfo {
                name,
                status: "available".to_string(),
            })
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.name.cmp(&right.name));
        agents
    }
}

fn validate_agent_entry(name: &str, entry: &AgentEntry) -> bool {
    let command_path = Path::new(&entry.command);
    if !command_path.is_absolute() {
        eprintln!(
            "Warning: excluding agent '{name}': command path must be absolute ('{}')",
            entry.command
        );
        return false;
    }

    match fs::metadata(command_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "Warning: excluding agent '{name}': command does not exist ('{}')",
                entry.command
            );
            return false;
        }
        Err(error) => {
            eprintln!(
                "Warning: excluding agent '{name}': failed to inspect command '{}': {error}",
                entry.command
            );
            return false;
        }
    };

    // FR-32-SEC: Check real user execute permission via access(2) instead of
    // inspecting mode bits, which don't account for file ownership or ACLs.
    let c_path = match CString::new(command_path.as_os_str().as_encoded_bytes()) {
        Ok(p) => p,
        Err(_) => {
            eprintln!(
                "Warning: excluding agent '{name}': command path contains interior NUL ('{}')",
                entry.command
            );
            return false;
        }
    };

    // SAFETY: c_path is a valid NUL-terminated C string that lives for the duration of
    // the call, and libc::access is a read-only POSIX syscall with no memory side-effects.
    let accessible = unsafe { libc::access(c_path.as_ptr(), libc::X_OK) };
    if accessible != 0 {
        eprintln!(
            "Warning: excluding agent '{name}': command is not executable by the current user ('{}')",
            entry.command
        );
        return false;
    }

    true
}
