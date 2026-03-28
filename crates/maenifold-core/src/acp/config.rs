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
}

/// Generate `.acpxrc.json` in `root` from `agents.json` so `acpx` resolves the
/// same agents as maenifold and pipeline-ui. The acpx format stores the full
/// command string (binary + args) under each agent's `command` key.
pub fn sync_acpx_config(root: &Path) {
    let config = AgentsConfig::load(root);
    if config.agents.is_empty() {
        return;
    }

    let acpx_agents: HashMap<String, serde_json::Value> = config
        .agents
        .iter()
        .map(|(name, entry)| {
            let full_command = if entry.args.is_empty() {
                entry.command.clone()
            } else {
                format!("{} {}", entry.command, entry.args.join(" "))
            };
            (name.clone(), serde_json::json!({ "command": full_command }))
        })
        .collect();

    let payload = serde_json::json!({ "agents": acpx_agents });
    let target = root.join(".acpxrc.json");
    let tmp = root.join(".acpxrc.json.tmp");

    match serde_json::to_string_pretty(&payload) {
        Ok(content) => {
            if let Err(error) =
                fs::write(&tmp, format!("{content}\n")).and_then(|()| fs::rename(&tmp, &target))
            {
                eprintln!(
                    "Warning: failed to sync .acpxrc.json at '{}': {error}",
                    target.display()
                );
                let _ = fs::remove_file(&tmp);
            }
        }
        Err(error) => {
            eprintln!("Warning: failed to serialize .acpxrc.json: {error}");
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_agents_json(root: &Path, content: &str) {
        fs::write(root.join("agents.json"), content).expect("write agents.json");
    }

    #[test]
    fn sync_acpx_config_generates_acpxrc_from_agents_json() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        write_agents_json(
            root,
            r#"{
                "agents": {
                    "TestAgent": {
                        "command": "/usr/bin/test-agent",
                        "args": ["serve", "--acp"],
                        "env": {}
                    }
                }
            }"#,
        );

        // Create a dummy executable so AgentsConfig::load validates it
        let dummy = root.join("test-agent");
        fs::write(&dummy, "#!/bin/sh\n").expect("write dummy");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dummy, fs::Permissions::from_mode(0o755)).expect("chmod");
        }

        // Rewrite agents.json with the dummy path
        let json = serde_json::json!({
            "agents": {
                "TestAgent": {
                    "command": dummy.display().to_string(),
                    "args": ["serve", "--acp"],
                    "env": {}
                }
            }
        });
        write_agents_json(root, &serde_json::to_string_pretty(&json).unwrap());

        sync_acpx_config(root);

        let acpxrc = root.join(".acpxrc.json");
        assert!(acpxrc.exists(), ".acpxrc.json must be created");

        let content: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&acpxrc).expect("read"))
                .expect("parse .acpxrc.json");

        let expected_cmd = format!("{} serve --acp", dummy.display());
        assert_eq!(
            content["agents"]["TestAgent"]["command"]
                .as_str()
                .expect("command"),
            expected_cmd
        );
    }

    #[test]
    fn sync_acpx_config_skips_when_no_agents() {
        let tmp = tempdir().expect("tempdir");
        sync_acpx_config(tmp.path());
        assert!(
            !tmp.path().join(".acpxrc.json").exists(),
            ".acpxrc.json must not be created when agents.json is absent"
        );
    }
}
