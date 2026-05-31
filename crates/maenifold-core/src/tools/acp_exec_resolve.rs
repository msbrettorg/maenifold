use crate::acp_config::{AgentEntry, AgentsConfig};
use crate::error::{MaenifoldError, Result};
use agent_client_protocol::schema::{self as acp};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug)]
pub(super) struct ResolvedAgent {
    pub(super) agent_key: String,
    pub(super) runtime_kind: String,
    pub(super) mcp_servers: Vec<acp::McpServer>,
}

pub(super) fn resolve_agent(root: &Path, agent: &str) -> Result<ResolvedAgent> {
    let config = AgentsConfig::load(root)?;
    let (agent_key, entry) = if let Some(entry) = config.agents.get(agent) {
        (agent.to_string(), entry)
    } else {
        let requested = normalized_agent_display_name(agent);
        let mut matches = config
            .agents
            .iter()
            .filter(|(name, _)| normalized_agent_display_name(name) == requested)
            .collect::<Vec<_>>();
        matches.sort_by(|(left, _), (right, _)| left.cmp(right));

        match matches.len() {
            1 => {
                let (matched_key, entry) = matches[0];
                (matched_key.to_string(), entry)
            }
            0 => {
                return Err(MaenifoldError::AgentSetup(format!(
                    "agent '{agent}' not configured in maenifold.json"
                )));
            }
            _ => {
                let keys = matches
                    .iter()
                    .map(|(key, _)| key.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(MaenifoldError::AgentSetup(format!(
                    "agent '{agent}' is ambiguous: matches config keys {keys}"
                )));
            }
        }
    };

    let runtime_kind = infer_runtime_kind(entry)?;
    let mut seen = HashSet::new();
    let mut mcp_servers = Vec::new();

    for (name, server) in config.mcp_servers.iter().chain(entry.mcp_servers.iter()) {
        if seen.insert(name.to_ascii_lowercase()) {
            mcp_servers.push(server.to_acp(name));
        }
    }

    Ok(ResolvedAgent {
        agent_key,
        runtime_kind,
        mcp_servers,
    })
}

fn normalized_agent_display_name(agent: &str) -> String {
    let normalized = agent
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();
    let mut parts = normalized
        .split_whitespace()
        .filter(|part| *part != "cli")
        .collect::<Vec<_>>();
    if parts.last() == Some(&"code") {
        parts.pop();
    }
    parts.join(" ")
}

pub(super) fn infer_runtime_kind(entry: &AgentEntry) -> Result<String> {
    let (binary, args) = entry.command_parts();
    if binary.trim().is_empty() {
        return Err(MaenifoldError::AgentSetup(
            "agent command is empty; configure a real ACP runtime command".to_string(),
        ));
    }

    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(binary)
        .to_ascii_lowercase();

    for candidate in std::iter::once(binary_name.as_str()).chain(args.into_iter()) {
        let normalized = candidate.to_ascii_lowercase();
        if normalized == "mock" {
            return Err(MaenifoldError::AgentSetup(
                "agent command maps to disallowed ACP runtime 'mock'; configure a real ACP runtime"
                    .to_string(),
            ));
        }
        if normalized.contains("claude") {
            return Ok("claude-code".to_string());
        }
        if normalized.contains("codex") {
            return Ok("codex-cli".to_string());
        }
        if normalized.contains("gemini") {
            return Ok("gemini-cli".to_string());
        }
    }

    Err(MaenifoldError::AgentSetup(format!(
        "agent command '{}' does not map to a supported Jockey runtime",
        entry.command
    )))
}

#[cfg(test)]
mod tests {
    use super::{infer_runtime_kind, resolve_agent};
    use crate::acp_config::AgentEntry;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn infer_runtime_kind_maps_command_binary() {
        let entry = AgentEntry {
            command: "/usr/local/bin/claude --print".to_string(),
            env: HashMap::new(),
            env_from_file: HashMap::new(),
            env_from_command: HashMap::new(),
            mcp_servers: HashMap::new(),
        };

        assert_eq!(
            infer_runtime_kind(&entry).expect("runtime kind"),
            "claude-code"
        );
    }

    #[test]
    fn infer_runtime_kind_rejects_empty_command() {
        let entry = AgentEntry {
            command: "  ".to_string(),
            env: HashMap::new(),
            env_from_file: HashMap::new(),
            env_from_command: HashMap::new(),
            mcp_servers: HashMap::new(),
        };

        let err = infer_runtime_kind(&entry).expect_err("empty command should be rejected");
        assert!(
            err.to_string().contains("agent command is empty"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn infer_runtime_kind_rejects_mock_runtime() {
        let entry = AgentEntry {
            command: "mock".to_string(),
            env: HashMap::new(),
            env_from_file: HashMap::new(),
            env_from_command: HashMap::new(),
            mcp_servers: HashMap::new(),
        };

        let err = infer_runtime_kind(&entry).expect_err("mock runtime should be rejected");
        assert!(
            err.to_string().contains("disallowed ACP runtime 'mock'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn resolve_agent_maps_codex_display_name_to_runtime_id() {
        let root = tempdir().expect("temp root");
        // Use a real executable path so `validate_agent_entry` (which checks
        // absolute + exists + executable) accepts the agent on every platform.
        // The first arg ("codex") drives `infer_runtime_kind` to "codex-cli".
        let stub = crate::test_util::fake_exit_zero_binary();
        let config = serde_json::to_string(&serde_json::json!({
            "agents": {
                "codex-cli": {
                    "command": format!("{} codex", stub.display())
                }
            }
        }))
        .expect("config json");
        std::fs::write(root.path().join("maenifold.json"), config).expect("write config");

        assert_eq!(
            resolve_agent(root.path(), "Codex")
                .expect("display name should resolve")
                .runtime_kind,
            "codex-cli"
        );
    }
}
