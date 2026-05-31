// Default agents config bootstrap. Used by `Config::ensure_directories()`
// to write `$MAENIFOLD_ROOT/maenifold.json` on first run when no config
// exists yet. Both the maenifold daemon AND the Maenifold Tauri UI read
// the same file, so the defaults need to be valid for both: the daemon
// requires `validate_agent_entry` (absolute command path that exists),
// and the UI just needs a command + optional env.
//
// We resolve `npx` on PATH at bootstrap time so the generated file uses
// an absolute path the daemon will accept. If `npx` isn't on PATH we
// still write the defaults with a bare `npx` command — the UI's Tauri
// shell wrapping (cmd /c on Windows, sh -c on unix) can still spawn it,
// and the daemon will simply warn-and-skip until the user installs Node
// or edits the file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::loader::AgentsConfig;
use super::types::AgentEntry;

/// Build the default agents config that ships on first run.
///
/// Mirrors the agent set the Maenifold Tauri UI used to hardcode in
/// `ui/src-tauri/src/config.rs::AgentsConfig::default`. Single source of
/// truth lives here so the UI can stop maintaining its own copy.
pub fn default_agents_config() -> AgentsConfig {
    let npx = locate_npx().unwrap_or_else(|| PathBuf::from("npx"));
    let npx_str = npx.to_string_lossy().into_owned();

    let stdio = |trailing_args: &str, env: HashMap<String, String>| AgentEntry {
        command: format!("{npx_str} {trailing_args}"),
        env,
        env_from_file: HashMap::new(),
        env_from_command: HashMap::new(),
        mcp_servers: HashMap::new(),
    };

    let mut agents: HashMap<String, AgentEntry> = HashMap::new();
    agents.insert(
        "GitHub Copilot".to_string(),
        stdio(
            "@github/copilot-language-server@latest --acp",
            HashMap::new(),
        ),
    );
    agents.insert(
        "Claude Code".to_string(),
        stdio(
            "@agentclientprotocol/claude-agent-acp@latest",
            HashMap::new(),
        ),
    );
    agents.insert(
        "Gemini CLI".to_string(),
        stdio(
            "@google/gemini-cli@latest --experimental-acp",
            HashMap::new(),
        ),
    );
    agents.insert(
        "Codex CLI".to_string(),
        stdio("@zed-industries/codex-acp@latest", HashMap::new()),
    );
    agents.insert(
        "Qwen Code".to_string(),
        stdio(
            "@qwen-code/qwen-code@latest --acp --experimental-skills",
            HashMap::new(),
        ),
    );
    agents.insert(
        "Auggie CLI".to_string(),
        stdio(
            "@augmentcode/auggie@latest --acp",
            HashMap::from([(
                "AUGMENT_DISABLE_AUTO_UPDATE".to_string(),
                "1".to_string(),
            )]),
        ),
    );
    agents.insert(
        "Qoder CLI".to_string(),
        stdio("@qoder-ai/qodercli@latest --acp", HashMap::new()),
    );
    agents.insert(
        "OpenCode".to_string(),
        stdio("opencode-ai@latest acp", HashMap::new()),
    );
    agents.insert(
        "OpenClaw".to_string(),
        stdio("openclaw acp", HashMap::new()),
    );

    AgentsConfig {
        mcp_servers: HashMap::new(),
        agents,
        settings: None,
    }
}

/// Find `npx` (or `npx.cmd` on Windows) on PATH, returning an absolute
/// path the daemon's `validate_agent_entry` will accept. Returns `None`
/// if Node.js isn't installed — caller falls back to bare `npx`.
fn locate_npx() -> Option<PathBuf> {
    let candidates: &[&str] = if cfg!(windows) {
        &["npx.cmd", "npx.exe", "npx"]
    } else {
        &["npx"]
    };
    let path = std::env::var_os("PATH")?;
    let sep = if cfg!(windows) { ';' } else { ':' };
    let path_str = path.to_string_lossy();
    for dir in path_str.split(sep) {
        if dir.is_empty() {
            continue;
        }
        for candidate in candidates {
            let p = Path::new(dir).join(candidate);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_contains_expected_agents() {
        let config = default_agents_config();
        // The 9 ACP agents the UI used to hardcode.
        for name in [
            "GitHub Copilot",
            "Claude Code",
            "Gemini CLI",
            "Codex CLI",
            "Qwen Code",
            "Auggie CLI",
            "Qoder CLI",
            "OpenCode",
            "OpenClaw",
        ] {
            assert!(
                config.agents.contains_key(name),
                "default config missing agent: {name}"
            );
        }
        assert_eq!(config.agents.len(), 9);
        assert!(config.mcp_servers.is_empty());
        // Settings stays unset so the daemon's normal defaulting path
        // (see `MaenifoldSettings::default`) drives runtime tuning rather
        // than the bootstrap freezing in stale numbers.
        assert!(config.settings.is_none());
    }

    #[test]
    fn auggie_carries_disable_auto_update_env() {
        let config = default_agents_config();
        let auggie = config.agents.get("Auggie CLI").expect("auggie present");
        assert_eq!(
            auggie.env.get("AUGMENT_DISABLE_AUTO_UPDATE").map(String::as_str),
            Some("1"),
        );
    }
}
