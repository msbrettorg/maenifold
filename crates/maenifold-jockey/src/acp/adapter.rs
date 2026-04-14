use dashmap::DashMap;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

fn load_agents_config(root: &Path) -> Value {
    let path = root.join("maenifold.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_else(|| Value::Object(Default::default()))
}

fn find_agent_entry<'a>(config: &'a Value, runtime: &str) -> Result<&'a Value, String> {
    config
        .get("agents")
        .and_then(Value::as_object)
        .and_then(|agents| {
            agents
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(runtime))
                .map(|(_, entry)| entry)
        })
        .ok_or_else(|| format!("agent '{}' not found in maenifold.json", runtime))
}

fn command_parts(entry: &Value) -> (&str, Vec<&str>) {
    let mut parts = entry
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split_whitespace();
    let binary = parts.next().unwrap_or("");
    let args = parts.collect();
    (binary, args)
}

fn entry_env(entry: &Value) -> Vec<(String, String)> {
    entry
        .get("env")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
        .collect()
}

fn env_passthrough(config: &Value) -> impl Iterator<Item = &str> {
    config
        .get("settings")
        .and_then(|settings| settings.get("envPassthrough"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

static ADAPTER_CACHE: std::sync::OnceLock<
    DashMap<(String, PathBuf), Result<(String, Vec<String>, Vec<(String, String)>), String>>,
> = std::sync::OnceLock::new();
fn adapter_cache(
) -> &'static DashMap<(String, PathBuf), Result<(String, Vec<String>, Vec<(String, String)>), String>>
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

pub fn config_root() -> PathBuf {
    CONFIG_ROOT
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Clone)]
pub struct StdioAdapterSpec {
    pub runtime_key: String,
    pub binary: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub fn build_stdio_adapter(runtime: &str) -> Result<Option<StdioAdapterSpec>, String> {
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
    let entry = find_agent_entry(&config, runtime)?;
    let (binary, args) = command_parts(entry);
    if binary.is_empty() {
        return Err(format!("agent '{}' has an empty command", runtime));
    }
    let mut env = entry_env(entry);
    for var_name in env_passthrough(&config) {
        if !env.iter().any(|(key, _)| key == var_name) {
            if let Ok(value) = std::env::var(var_name) {
                env.push((var_name.to_string(), value));
            }
        }
    }
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

pub fn resolve_cwd(cwd: &str) -> String {
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
