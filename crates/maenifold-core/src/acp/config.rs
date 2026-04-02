use agent_client_protocol as acp;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpServerEntry {
    #[serde(rename = "stdio")]
    Stdio {
        name: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    #[serde(rename = "http")]
    Http { name: String, url: String },
}

impl McpServerEntry {
    pub fn to_acp(&self) -> acp::McpServer {
        match self {
            McpServerEntry::Stdio {
                name,
                command,
                args,
            } => acp::McpServer::Stdio(
                acp::McpServerStdio::new(name.clone(), command.clone()).args(args.clone()),
            ),
            McpServerEntry::Http { name, url } => {
                acp::McpServer::Http(acp::McpServerHttp::new(name.clone(), url.clone()))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    pub command: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServerEntry>,
}

impl AgentEntry {
    pub fn command_parts(&self) -> (&str, Vec<&str>) {
        let mut parts = self.command.split_whitespace();
        let binary = parts.next().unwrap_or("");
        let args = parts.collect();
        (binary, args)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub agents: HashMap<String, AgentEntry>,
}

impl AgentsConfig {
    pub fn load(root: &Path) -> AgentsConfig {
        let primary_path = root.join("agents.json");
        let fallback_path = root.join(".acpxrc.json");
        let Some((config_path, contents)) = read_config_contents(&primary_path, &fallback_path)
        else {
            return Self::default();
        };

        #[cfg(unix)]
        ensure_acpx_symlink(root, &config_path);

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
}

fn read_config_contents(primary_path: &Path, fallback_path: &Path) -> Option<(PathBuf, String)> {
    match fs::read_to_string(primary_path) {
        Ok(contents) => Some((primary_path.to_path_buf(), contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::read_to_string(fallback_path) {
                Ok(contents) => Some((fallback_path.to_path_buf(), contents)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    eprintln!(
                        "Warning: failed to read agents config '{}': {error}",
                        fallback_path.display()
                    );
                    None
                }
            }
        }
        Err(error) => {
            eprintln!(
                "Warning: failed to read agents config '{}': {error}",
                primary_path.display()
            );
            None
        }
    }
}

#[cfg(unix)]
fn ensure_acpx_symlink(root: &Path, loaded_path: &Path) {
    use std::os::unix::fs::symlink;

    let agents_path = root.join("agents.json");
    let acpx_path = root.join(".acpxrc.json");
    if loaded_path != agents_path || acpx_path.exists() {
        return;
    }

    if let Err(error) = symlink("agents.json", &acpx_path) {
        eprintln!(
            "Warning: failed to create ACP config symlink '{}': {error}",
            acpx_path.display()
        );
    }
}

fn validate_agent_entry(name: &str, entry: &AgentEntry) -> bool {
    let (command_path, _) = entry.command_parts();
    if command_path.is_empty() {
        eprintln!("Warning: excluding agent '{name}': command must not be empty");
        return false;
    }
    let command_path = Path::new(command_path);
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

    let c_path = match CString::new(command_path.as_os_str().as_encoded_bytes()) {
        Ok(path) => path,
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

