use indexmap::IndexMap;
#[cfg(desktop)]
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tauri::AppHandle;
#[cfg(desktop)]
use tauri::Emitter;
#[cfg(not(desktop))]
use tauri::Manager;

/// Transport kind for an ACP agent.
///
/// `stdio` (default, desktop only) launches a subprocess and exchanges
/// JSON-RPC over stdin/stdout. `websocket` and `http` connect to a remote
/// endpoint advertised by an agent that natively speaks ACP over the wire,
/// and require the corresponding `url`/`headers` fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentTransport {
    #[default]
    Stdio,
    Websocket,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnvFromCommand {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Transport kind. Defaults to `stdio` for backward compatibility with
    /// existing `agents.json` files that only contain `command`/`args`.
    #[serde(default, skip_serializing_if = "is_default_transport")]
    pub transport: AgentTransport,

    // ----- stdio-only fields (optional when transport != stdio) -----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub env: std::collections::HashMap<String, String>,
    #[serde(
        default,
        rename = "envFromFile",
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    pub env_from_file: std::collections::HashMap<String, String>,
    #[serde(
        default,
        rename = "envFromCommand",
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    pub env_from_command: std::collections::HashMap<String, EnvFromCommand>,

    // ----- remote-only fields (optional, used when transport != stdio) -----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

impl AgentConfig {
    pub fn stdio(
        command: String,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            transport: AgentTransport::Stdio,
            command: Some(command),
            args: Some(args),
            env,
            env_from_file: std::collections::HashMap::new(),
            env_from_command: std::collections::HashMap::new(),
            url: None,
            headers: None,
        }
    }

    pub fn command_parts(&self) -> Option<(String, Vec<String>)> {
        let command = self.command.clone()?;
        if let Some(args) = self.args.clone() {
            return Some((expand_home_prefix(&command), args));
        }
        let mut parts = command.split_whitespace();
        let binary = parts.next()?;
        let args = parts.map(str::to_string).collect();
        Some((expand_home_prefix(binary), args))
    }
}

pub(crate) fn resolve_env(config: &AgentConfig) -> Result<HashMap<String, String>, String> {
    let mut resolved = HashMap::new();

    for (key, path) in &config.env_from_file {
        let expanded_path = expand_home_prefix(path);
        let value = std::fs::read_to_string(&expanded_path).map_err(|error| {
            format!(
                "{key}: failed to read envFromFile path {}: {error}",
                expanded_path
            )
        })?;
        resolved.insert(
            key.clone(),
            value
                .trim_end_matches(|ch| ch == '\n' || ch == '\r')
                .to_string(),
        );
    }

    for (key, spec) in &config.env_from_command {
        let output = Command::new(&spec.command)
            .args(&spec.args)
            .output()
            .map_err(|error| {
                format!(
                    "{key}: failed to run envFromCommand {}: {error}",
                    spec.command
                )
            })?;
        if !output.status.success() {
            return Err(format!(
                "{key}: envFromCommand {} exited with status {}",
                spec.command, output.status
            ));
        }
        let value = String::from_utf8_lossy(&output.stdout);
        resolved.insert(
            key.clone(),
            value
                .trim_end_matches(|ch| ch == '\n' || ch == '\r')
                .to_string(),
        );
    }

    for (key, value) in &config.env {
        resolved.insert(key.clone(), value.clone());
    }

    Ok(resolved)
}

fn expand_home_prefix(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    value.to_string()
}

fn is_default_transport(t: &AgentTransport) -> bool {
    *t == AgentTransport::Stdio
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub agents: IndexMap<String, AgentConfig>,
    /// Daemon-owned settings block. Stored as a free-form JSON object so
    /// the UI doesn't have to track every field the daemon ships -- only
    /// the keys the UI actually reads (today: `defaultAgent`) need explicit
    /// extraction at the call site. Round-tripped unchanged on save so we
    /// don't lose daemon-only keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        // Agent defaults now live in maenifold-core
        // (`crate::acp_config::default_agents_config`) so the daemon and
        // the UI bootstrap the same set. The UI's empty default is just
        // the fallback when no file exists yet and the daemon hasn't run
        // — first daemon launch (or first UI launch hitting
        // `save_config`) writes the populated file.
        AgentsConfig {
            agents: IndexMap::new(),
            settings: None,
        }
    }
}

pub struct ConfigManager {
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
    /// File watcher is desktop-only. Mobile builds rely on explicit
    /// `add_agent` / `update_agent` IPC calls (no external editing path).
    #[cfg(desktop)]
    #[allow(dead_code)]
    watcher: Option<RecommendedWatcher>,
}

impl ConfigManager {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let config_path = get_config_path(app)?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // The maenifold daemon owns first-run bootstrap of
        // `maenifold.json` (see `Config::ensure_default_config_file` in
        // maenifold-core). If the file is missing here it usually means
        // the daemon hasn't been launched yet — fall back to an empty
        // in-memory config and let the file watcher pick up the daemon's
        // write when it lands. Writing an empty file here would clobber
        // any defaults the daemon is about to write.
        let config = if config_path.exists() {
            load_config(&config_path)?
        } else {
            AgentsConfig::default()
        };

        let config = Arc::new(RwLock::new(config));

        // Set up file watcher (desktop only — `notify` doesn't have a useful
        // backend on iOS/Android, and mobile users can't edit the file
        // outside the app anyway).
        #[cfg(desktop)]
        let watcher = {
            let config_clone = Arc::clone(&config);
            let config_path_clone = config_path.clone();
            let app_handle = app.clone();
            Some(setup_watcher(config_clone, config_path_clone, app_handle)?)
        };
        #[cfg(not(desktop))]
        {
            // Touch `app` so the parameter isn't reported as unused on mobile.
            let _ = app;
        }

        Ok(Self {
            config,
            config_path,
            #[cfg(desktop)]
            watcher,
        })
    }

    pub fn get_config(&self) -> AgentsConfig {
        self.config.read().clone()
    }

    pub fn reload(&self) -> Result<AgentsConfig, String> {
        let new_config = load_config(&self.config_path)?;
        *self.config.write() = new_config.clone();
        Ok(new_config)
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.config_path.clone()
    }

    pub fn save(&self) -> Result<(), String> {
        let config = self.config.read();
        save_config(&self.config_path, &config)
    }

    pub fn add_agent(&self, name: String, config: AgentConfig) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.insert(name, config);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn remove_agent(&self, name: &str) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            agents_config.agents.shift_remove(name);
        }
        self.save()?;
        Ok(self.get_config())
    }

    pub fn update_agent(&self, name: String, config: AgentConfig) -> Result<AgentsConfig, String> {
        {
            let mut agents_config = self.config.write();
            if agents_config.agents.contains_key(&name) {
                agents_config.agents.insert(name, config);
            } else {
                return Err(format!("Agent '{}' not found", name));
            }
        }
        self.save()?;
        Ok(self.get_config())
    }
}

fn get_config_path(_app: &AppHandle) -> Result<PathBuf, String> {
    // Single source of truth: the maenifold daemon's `maenifold.json`
    // lives at `$MAENIFOLD_ROOT/maenifold.json` (default `~/maenifold/`).
    // The daemon bootstraps this file with sane agent defaults on first
    // run (`Config::ensure_default_config_file`), so both surfaces see
    // the same agents from the very first launch instead of the UI
    // maintaining its own historical `~/.config/acp-ui/agents.json`.
    #[cfg(desktop)]
    {
        let root: PathBuf = match std::env::var_os("MAENIFOLD_ROOT") {
            Some(value) => PathBuf::from(value),
            None => dirs::home_dir()
                .ok_or_else(|| "Could not resolve home directory".to_string())?
                .join("maenifold"),
        };
        return Ok(root.join("maenifold.json"));
    }
    // Mobile builds have no shared filesystem with a maenifold daemon —
    // keep the sandbox path so the UI still functions standalone.
    #[cfg(not(desktop))]
    {
        return _app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Could not resolve app config dir: {}", e))
            .map(|p| p.join("maenifold.json"));
    }
}

fn load_config(path: &PathBuf) -> Result<AgentsConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_maenifold_config(&content)
}

/// Parse the daemon-shape `maenifold.json` into the UI's `AgentsConfig`.
///
/// The daemon stores each agent's invocation as a single space-joined
/// `command` string (`"npx @scope/foo@latest --acp"`); the UI's spawn
/// path takes a separate `command` binary and `args` vector. We split on
/// whitespace at load time to bridge the two shapes without forcing the
/// daemon to mirror our richer struct. Unknown top-level keys
/// (`mcpServers`, `settings`) are preserved on save (see `save_config`).
fn parse_maenifold_config(content: &str) -> Result<AgentsConfig, String> {
    let value: serde_json::Value = serde_json::from_str(content).map_err(|e| e.to_string())?;
    let agents_value = value
        .get("agents")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let raw_agents: IndexMap<String, serde_json::Value> =
        serde_json::from_value(agents_value).map_err(|e| e.to_string())?;

    let mut agents: IndexMap<String, AgentConfig> = IndexMap::with_capacity(raw_agents.len());
    for (name, raw) in raw_agents {
        let entry = agent_from_value(&name, &raw)?;
        agents.insert(name, entry);
    }

    // Tunnel the entire `settings` block through to the frontend so
    // settings-driven UI behavior (e.g. `defaultAgent` for the picker)
    // works without code changes when new keys land. Unknown sub-keys
    // are preserved as opaque JSON.
    let settings = value.get("settings").and_then(|v| v.as_object()).cloned();

    Ok(AgentsConfig { agents, settings })
}

fn agent_from_value(name: &str, raw: &serde_json::Value) -> Result<AgentConfig, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| format!("agent '{name}' must be an object"))?;

    let transport = match obj.get("transport").and_then(|v| v.as_str()) {
        Some("websocket") => AgentTransport::Websocket,
        Some("http") => AgentTransport::Http,
        // Default + explicit "stdio" + anything unknown collapses to the
        // canonical stdio path so the UI never silently drops an agent.
        _ => AgentTransport::Stdio,
    };

    let env: std::collections::HashMap<String, String> = obj
        .get("env")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("agent '{name}' has invalid env: {e}"))?
        .unwrap_or_default();
    let env_from_file: std::collections::HashMap<String, String> = obj
        .get("envFromFile")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("agent '{name}' has invalid envFromFile: {e}"))?
        .unwrap_or_default();
    let env_from_command: std::collections::HashMap<String, EnvFromCommand> = obj
        .get("envFromCommand")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("agent '{name}' has invalid envFromCommand: {e}"))?
        .unwrap_or_default();

    let url = obj
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let headers: Option<std::collections::HashMap<String, String>> = obj
        .get("headers")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| format!("agent '{name}' has invalid headers: {e}"))?;

    // stdio agents accept either the daemon shape (single `command` string)
    // or the legacy UI shape (separate `command` + `args`). Split on
    // whitespace when no explicit `args` is present so the spawn path can
    // always invoke `Command::new(command).args(args)`.
    let (command, args) = match transport {
        AgentTransport::Stdio => {
            let raw_command = obj
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let raw_args: Option<Vec<String>> = obj
                .get("args")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| format!("agent '{name}' has invalid args: {e}"))?;
            split_command_and_args(raw_command, raw_args)
        }
        _ => (None, None),
    };

    Ok(AgentConfig {
        transport,
        command,
        args,
        env,
        env_from_file,
        env_from_command,
        url,
        headers,
    })
}

fn split_command_and_args(
    command: Option<String>,
    args: Option<Vec<String>>,
) -> (Option<String>, Option<Vec<String>>) {
    if args.is_some() {
        return (command, args);
    }
    let Some(raw) = command else {
        return (None, None);
    };
    let mut parts = raw.split_whitespace().map(|s| s.to_string());
    let Some(binary) = parts.next() else {
        return (None, None);
    };
    let rest: Vec<String> = parts.collect();
    let args_field = if rest.is_empty() { None } else { Some(rest) };
    (Some(binary), args_field)
}

fn save_config(path: &PathBuf, config: &AgentsConfig) -> Result<(), String> {
    // Preserve unknown top-level keys the daemon owns (`mcpServers`,
    // `settings`, future additions) by merging into whatever is currently
    // on disk rather than serializing the UI's view wholesale.
    let mut root: serde_json::Map<String, serde_json::Value> = match fs::read_to_string(path) {
        Ok(existing) => serde_json::from_str::<serde_json::Value>(&existing)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(err) => return Err(err.to_string()),
    };

    let mut agents_map = serde_json::Map::with_capacity(config.agents.len());
    for (name, entry) in &config.agents {
        agents_map.insert(name.clone(), agent_to_daemon_value(entry));
    }
    root.insert("agents".to_string(), serde_json::Value::Object(agents_map));

    let body = serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .map_err(|e| e.to_string())?;
    fs::write(path, format!("{body}\n")).map_err(|e| e.to_string())
}

/// Serialize a single agent in the daemon's wire shape.
pub(crate) fn agent_to_daemon_value(entry: &AgentConfig) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    match entry.transport {
        AgentTransport::Stdio => {
            if let Some(command) = entry.command.as_ref() {
                obj.insert(
                    "command".to_string(),
                    serde_json::Value::String(command.clone()),
                );
            }
            if let Some(args) = entry.args.as_ref() {
                obj.insert(
                    "args".to_string(),
                    serde_json::to_value(args).unwrap_or(serde_json::Value::Null),
                );
            }
        }
        AgentTransport::Websocket | AgentTransport::Http => {
            obj.insert(
                "transport".to_string(),
                serde_json::Value::String(match entry.transport {
                    AgentTransport::Websocket => "websocket".to_string(),
                    AgentTransport::Http => "http".to_string(),
                    AgentTransport::Stdio => unreachable!(),
                }),
            );
            if let Some(url) = entry.url.as_ref() {
                obj.insert("url".to_string(), serde_json::Value::String(url.clone()));
            }
            if let Some(headers) = entry.headers.as_ref() {
                if !headers.is_empty() {
                    obj.insert(
                        "headers".to_string(),
                        serde_json::to_value(headers).unwrap_or(serde_json::Value::Null),
                    );
                }
            }
        }
    }
    if !entry.env.is_empty() {
        obj.insert(
            "env".to_string(),
            serde_json::to_value(&entry.env).unwrap_or(serde_json::Value::Null),
        );
    }
    if !entry.env_from_file.is_empty() {
        obj.insert(
            "envFromFile".to_string(),
            serde_json::to_value(&entry.env_from_file).unwrap_or(serde_json::Value::Null),
        );
    }
    if !entry.env_from_command.is_empty() {
        obj.insert(
            "envFromCommand".to_string(),
            serde_json::to_value(&entry.env_from_command).unwrap_or(serde_json::Value::Null),
        );
    }
    serde_json::Value::Object(obj)
}

#[cfg(desktop)]
fn setup_watcher(
    config: Arc<RwLock<AgentsConfig>>,
    config_path: PathBuf,
    app_handle: AppHandle,
) -> Result<RecommendedWatcher, String> {
    let config_path_for_watcher = config_path.clone();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        if event.paths.iter().any(|p| p == &config_path_for_watcher) {
                            if let Ok(new_config) = load_config(&config_path_for_watcher) {
                                *config.write() = new_config.clone();
                                let _ = app_handle.emit("config-changed", new_config);
                            }
                        }
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;

    // Watch the config directory
    if let Some(parent) = config_path.parent() {
        watcher
            .watch(parent, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
    }

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_daemon_combined_command_form() {
        // Maenifold daemon stores agents as a single space-joined command
        // string. The UI's loader must split it into binary + args so the
        // spawn path can call `Command::new(binary).args(args)`.
        let json = r#"{"agents":{"GitHub Copilot":{"command":"npx @github/copilot-language-server@latest --acp"}}}"#;
        let cfg = parse_maenifold_config(json).expect("daemon shape must parse");
        let entry = cfg.agents.get("GitHub Copilot").expect("agent present");
        assert_eq!(entry.transport, AgentTransport::Stdio);
        assert_eq!(entry.command.as_deref(), Some("npx"));
        assert_eq!(
            entry.args.as_deref(),
            Some(["@github/copilot-language-server@latest", "--acp"].as_slice())
                .map(|s| s.iter().map(|x| x.to_string()).collect::<Vec<_>>())
                .as_deref()
        );
    }

    #[test]
    fn parses_legacy_split_command_args_form() {
        // Older `agents.json` files used separate command + args fields;
        // keep deserializing them unchanged so a user migrating from the
        // old UI-owned config file doesn't lose their entries.
        let json = r#"{"agents":{"Legacy":{"command":"npx","args":["x"],"env":{}}}}"#;
        let cfg = parse_maenifold_config(json).expect("legacy shape must parse");
        let entry = cfg.agents.get("Legacy").expect("agent present");
        assert_eq!(entry.transport, AgentTransport::Stdio);
        assert_eq!(entry.command.as_deref(), Some("npx"));
        assert_eq!(entry.args.as_deref().map(|v| v.len()), Some(1));
    }

    #[test]
    fn parses_remote_websocket_agent() {
        let json = r#"{"agents":{"Remote":{"transport":"websocket","url":"wss://x/v1","headers":{"Authorization":"Bearer abc"}}}}"#;
        let cfg = parse_maenifold_config(json).expect("remote shape must parse");
        let entry = cfg.agents.get("Remote").expect("agent present");
        assert_eq!(entry.transport, AgentTransport::Websocket);
        assert_eq!(entry.url.as_deref(), Some("wss://x/v1"));
        assert_eq!(entry.command, None);
        assert!(entry.args.is_none());
        assert_eq!(
            entry
                .headers
                .as_ref()
                .and_then(|h| h.get("Authorization"))
                .map(String::as_str),
            Some("Bearer abc"),
        );
    }

    #[test]
    fn ignores_daemon_specific_top_level_keys() {
        // The shared maenifold.json carries `mcpServers` and `settings`
        // sections the daemon owns. The UI loader must tolerate them
        // rather than refusing to parse.
        let json = r#"{
            "mcpServers": {"x": {"command": "/bin/y"}},
            "agents": {"A": {"command": "/bin/a"}},
            "settings": {"autoSync": true}
        }"#;
        let cfg = parse_maenifold_config(json).expect("daemon-shape config must parse");
        assert_eq!(cfg.agents.len(), 1);
    }

    #[test]
    fn save_preserves_unknown_top_level_keys() {
        // The UI must not wipe `mcpServers` / `settings` when the user
        // adds or edits an agent.
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let path = tmp.path().to_path_buf();
        std::fs::write(
            &path,
            r#"{
                "mcpServers": {"x": {"command": "/bin/y"}},
                "agents": {"Old": {"command": "/bin/old"}},
                "settings": {"autoSync": true}
            }"#,
        )
        .expect("seed");

        let mut config = AgentsConfig {
            agents: IndexMap::new(),
            settings: None,
        };
        config.agents.insert(
            "New".to_string(),
            AgentConfig::stdio(
                "/bin/new".to_string(),
                vec!["--flag".to_string()],
                std::collections::HashMap::new(),
            ),
        );
        save_config(&path, &config).expect("save");

        let written = std::fs::read_to_string(&path).expect("read");
        let value: serde_json::Value = serde_json::from_str(&written).expect("parse");
        assert!(value.get("mcpServers").is_some(), "mcpServers preserved");
        assert!(value.get("settings").is_some(), "settings preserved");
        let new_agent = value
            .get("agents")
            .and_then(|a| a.get("New"))
            .expect("New agent");
        assert_eq!(
            new_agent.get("command").and_then(|c| c.as_str()),
            Some("/bin/new")
        );
        assert_eq!(
            new_agent.get("args").and_then(|a| a.as_array()).map(|v| v
                .iter()
                .map(|x| x.as_str().unwrap_or(""))
                .collect::<Vec<_>>()),
            Some(vec!["--flag"]),
        );
        assert!(
            value.get("agents").and_then(|a| a.get("Old")).is_none(),
            "save replaces agents wholesale (Old should be gone)"
        );
    }

    #[test]
    fn roundtrips_stdio_through_daemon_shape() {
        let json =
            r#"{"agents":{"GH":{"command":"npx @github/copilot-language-server@latest --acp"}}}"#;
        let parsed = parse_maenifold_config(json).expect("parse");
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        save_config(&tmp.path().to_path_buf(), &parsed).expect("save");
        let reparsed = parse_maenifold_config(&std::fs::read_to_string(tmp.path()).expect("read"))
            .expect("reparse");
        let entry = reparsed.agents.get("GH").expect("agent");
        assert_eq!(entry.command.as_deref(), Some("npx"));
        assert_eq!(entry.args.as_deref().map(|v| v.len()), Some(2));
    }

    #[test]
    fn default_keeps_zero_agents_until_daemon_bootstraps() {
        // The UI no longer ships its own default agent set — the daemon
        // owns the bootstrap (see `Config::ensure_default_config_file`).
        // Empty default is the correct fallback when no file exists.
        let cfg = AgentsConfig::default();
        assert!(cfg.agents.is_empty());
    }
}
