use dashmap::DashMap;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

// Minimal config types to read maenifold.json without depending on maenifold-core.
#[derive(Debug, Default, Deserialize)]
struct AgentsConfig {
    #[serde(default)]
    agents: HashMap<String, AgentEntry>,
}

#[derive(Debug, Deserialize)]
struct AgentEntry {
    command: String,
    #[serde(default)]
    env: HashMap<String, String>,
}

impl AgentEntry {
    fn command_parts(&self) -> (&str, Vec<&str>) {
        let mut parts = self.command.split_whitespace();
        let binary = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();
        (binary, args)
    }
}

fn load_agents_config(root: &Path) -> AgentsConfig {
    let path = root.join("maenifold.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

static ADAPTER_CACHE: std::sync::OnceLock<
    DashMap<(String, PathBuf), Result<(String, Vec<String>, Vec<(String, String)>), String>>,
> = std::sync::OnceLock::new();
fn adapter_cache()
-> &'static DashMap<(String, PathBuf), Result<(String, Vec<String>, Vec<(String, String)>), String>>
{
    ADAPTER_CACHE.get_or_init(DashMap::new)
}

pub fn clear_adapter_cache() {
    adapter_cache().clear();
}

static CONFIG_ROOT: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn set_config_root(path: PathBuf) {
    if let Ok(mut guard) = CONFIG_ROOT.write() {
        *guard = Some(path);
    }
}

pub(super) fn config_root() -> PathBuf {
    CONFIG_ROOT
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) struct StdioAdapterSpec {
    pub(super) runtime_key: String,
    pub(super) binary: String,
    pub(super) args: Vec<String>,
    pub(super) env: Vec<(String, String)>,
}

pub(super) fn build_stdio_adapter(runtime: &str) -> Result<Option<StdioAdapterSpec>, String> {
    build_stdio_adapter_with_root(runtime, &config_root())
}

pub(super) fn build_stdio_adapter_with_root(
    runtime: &str,
    root: &PathBuf,
) -> Result<Option<StdioAdapterSpec>, String> {
    let normalized = runtime.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    let cache_key = (normalized.clone(), root.clone());
    let cached = adapter_cache().get(&cache_key).map(|r| r.clone());
    let resolved = if let Some(result) = cached {
        result
    } else {
        let result = resolve_adapter_for_name(&normalized, root);
        adapter_cache().insert(cache_key, result.clone());
        result
    };
    let (binary, args, env) = resolved?;
    Ok(Some(StdioAdapterSpec {
        runtime_key: normalized,
        binary,
        args,
        env,
    }))
}

fn resolve_adapter_for_name(
    runtime: &str,
    root: &PathBuf,
) -> Result<(String, Vec<String>, Vec<(String, String)>), String> {
    let config = load_agents_config(root);
    let entry = config
        .agents
        .get(runtime)
        .ok_or_else(|| format!("agent '{}' not found in maenifold.json", runtime))?;
    let (binary, args) = entry.command_parts();
    if binary.is_empty() {
        return Err(format!("agent '{}' has an empty command", runtime));
    }
    let env = entry
        .env
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    Ok((
        binary.to_string(),
        args.into_iter().map(ToString::to_string).collect(),
        env,
    ))
}

pub fn probe_runtime(runtime_kind: &str) -> Option<(bool, String)> {
    let n = runtime_kind.trim().to_ascii_lowercase();
    if n.is_empty() {
        return None;
    }
    match build_stdio_adapter(&n) {
        Ok(Some(a)) => Some((true, a.binary)),
        Ok(None) => Some((false, format!("unsupported runtime: {}", n))),
        Err(e) => Some((false, e)),
    }
}

pub(super) fn friendly_error_message(runtime: &str, raw: &str) -> String {
    let l = raw.to_ascii_lowercase();
    if l.contains("429")
        || l.contains("rate limit")
        || l.contains("quota")
        || l.contains("resource_exhausted")
        || l.contains("too many requests")
    {
        return format!("[{runtime}] Rate limit exceeded — please wait and retry.");
    }
    if l.contains("epipe") || l.contains("broken pipe") || l.contains("transport closed") {
        return format!(
            "[{runtime}] Agent process exited unexpectedly. It will restart on next message."
        );
    }
    if l.contains("timeout") {
        return format!("[{runtime}] Operation timed out — please retry.");
    }
    if l.contains("agent process exited") || l.contains("no longer alive") {
        return format!(
            "[{runtime}] Agent process exited unexpectedly. It will restart on next message."
        );
    }
    if l.contains("binary not found")
        || l.contains("adapter unavailable")
        || l.contains("not found in maenifold.json")
    {
        return format!("[{runtime}] Agent command not found. Check maenifold.json.");
    }
    format!("[{runtime}] {raw}")
}

pub(super) fn resolve_cwd(cwd: &str) -> String {
    let p = std::path::Path::new(cwd);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    };
    abs.canonicalize()
        .unwrap_or(abs)
        .to_string_lossy()
        .to_string()
}

pub(super) fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub(super) fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n).collect()
}

pub(super) fn acp_log(event: &str, payload: Value) {
    eprintln!("[jockey.acp] {} {} {}", now_ms(), event, payload);
}
