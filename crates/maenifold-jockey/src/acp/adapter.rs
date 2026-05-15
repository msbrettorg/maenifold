use dashmap::DashMap;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "adapter_env.rs"]
mod adapter_env;
#[cfg(test)]
#[path = "adapter_env_t301_31_tests.rs"]
mod adapter_env_t301_31_tests;
#[cfg(test)]
#[path = "adapter_env_tests.rs"]
mod adapter_env_tests;
#[path = "adapter_resolve.rs"]
mod adapter_resolve;
#[cfg(test)]
#[path = "adapter_t301_31_tests.rs"]
mod adapter_t301_31_tests;

use adapter_env::validate_env;
use adapter_resolve::resolve_adapter_for_name;

fn load_agents_config(root: &Path) -> Result<Value, String> {
    let path = root.join("maenifold.json");
    let contents = std::fs::read_to_string(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("maenifold.json not found at {}", path.display())
        } else {
            format!("Failed to read maenifold.json: {error}")
        }
    })?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("Failed to parse maenifold.json: {error}"))
}

fn find_agent_entry(root: &Path, runtime: &str) -> Result<(Value, Value), String> {
    let config = load_agents_config(root)?;
    let entry = config
        .get("agents")
        .and_then(Value::as_object)
        .and_then(|agents| {
            agents
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(runtime))
                .map(|(_, entry)| entry.clone())
        })
        .ok_or_else(|| format!("Agent '{runtime}' not found in maenifold.json"))?;
    Ok((config, entry))
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

fn env_contains(env: &[(String, String)], var_name: &str) -> bool {
    env.iter().any(|(key, _)| key == var_name)
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
    if l.contains("timeout") {
        return format!("[{runtime}] Operation timed out — please retry.");
    }
    if l.contains("not found in maenifold.json") {
        return format!("[{runtime}] {raw}");
    }
    if l.contains("binary not found") {
        return format!("[{runtime}] {raw}");
    }
    if l.contains("failed to start agent") {
        return format!("[{runtime}] {raw}");
    }
    if l.contains("agent process exited")
        || l.contains("no longer alive")
        || l.contains("transport closed")
        || l.contains("broken pipe")
        || l.contains("epipe")
    {
        if l.contains("agent '") && l.contains(" exited:") {
            return format!("[{runtime}] {raw}");
        }
        return format!("[{runtime}] Agent '{runtime}' exited: {raw}");
    }
    if l.contains("adapter unavailable") {
        return format!("[{runtime}] {raw}");
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

#[cfg(test)]
mod tests {
    use super::{build_stdio_adapter_with_root, clear_adapter_cache};

    #[test]
    fn stdio_adapter_resolves_agent_display_name_not_runtime_kind() {
        let root = tempfile::tempdir().expect("create temp config root");
        std::fs::write(
            root.path().join("maenifold.json"),
            r#"{"agents":{"Codex":{"command":"/some/path/to/codex-acp --flag"}}}"#,
        )
        .expect("write maenifold.json");
        let root = root.path().to_path_buf();

        clear_adapter_cache();
        let adapter = build_stdio_adapter_with_root("Codex", &root)
            .expect("display-name key should resolve")
            .expect("adapter should exist");

        assert_eq!(adapter.binary, "/some/path/to/codex-acp");
        assert_eq!(adapter.args, vec!["--flag"]);

        let err = match build_stdio_adapter_with_root("codex-cli", &root) {
            Ok(_) => panic!("runtime kind should not match display-name keyed agents"),
            Err(err) => err,
        };
        assert_eq!(err, "Agent 'codex-cli' not found in maenifold.json");
    }
}
