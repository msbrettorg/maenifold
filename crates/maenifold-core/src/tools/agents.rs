use super::context::{ToolContext, render};
use crate::acp_config::{AgentEntry, AgentsConfig, McpServerEntry};
use crate::error::{MaenifoldError, Result};
use futures_util::future::join_all;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;
use tokio::time::timeout;

const PER_AGENT_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

static AGENTS_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build agents runtime: {error}"))
});

fn agents_runtime() -> std::result::Result<&'static Runtime, String> {
    AGENTS_RUNTIME.as_ref().map_err(|message| message.clone())
}

pub fn list_agents(context: &ToolContext, payload: &Value) -> Result<String> {
    let probe = payload
        .get("probe")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let agents = list_agents_value(&context.config.maenifold_root, probe)?;
    Ok(render(context.output_format, agents))
}

fn list_agents_value(root: &std::path::Path, probe: bool) -> Result<Value> {
    let config = AgentsConfig::load(root);
    let mut names = config.agents.keys().cloned().collect::<Vec<_>>();
    names.sort();

    let mut agents = Vec::with_capacity(names.len());
    let mut probe_entries = Vec::with_capacity(names.len());
    let top_level_mcp_servers = config.mcp_servers.clone();
    for name in &names {
        let entry = config
            .agents
            .get(name)
            .expect("sorted key should resolve to an agent entry");
        let value = list_agent_entry(name, entry)?;
        if probe {
            probe_entries.push(ProbeAgent::from_entry(
                name.clone(),
                entry.clone(),
                &top_level_mcp_servers,
            ));
        }
        agents.push(value);
    }

    if probe {
        let capabilities = probe_agents_value(root, probe_entries, PER_AGENT_PROBE_TIMEOUT)
            .map_err(MaenifoldError::Message)?;
        for (value, caps) in agents.iter_mut().zip(capabilities) {
            if let Value::Object(obj) = value {
                obj.insert("capabilities".to_string(), caps);
            }
        }
    }

    Ok(Value::Array(agents))
}

fn list_agent_entry(name: &str, entry: &crate::acp_config::AgentEntry) -> Result<Value> {
    let mut object = match serde_json::to_value(entry)? {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    let (command, args) = entry.command_parts();

    object.insert("name".to_string(), json!(name));
    object.insert("command".to_string(), json!(command));
    object.insert(
        "args".to_string(),
        Value::Array(args.into_iter().map(|arg| json!(arg)).collect()),
    );

    Ok(Value::Object(object))
}

#[derive(Clone)]
struct ProbeAgent {
    name: String,
    entry: AgentEntry,
    mcp_servers: Vec<McpServerEntry>,
}

impl ProbeAgent {
    fn from_entry(
        name: String,
        entry: AgentEntry,
        top_level_mcp_servers: &[McpServerEntry],
    ) -> Self {
        let mut mcp_servers = top_level_mcp_servers.to_vec();
        mcp_servers.extend(entry.mcp_servers.clone());
        Self {
            name,
            entry,
            mcp_servers,
        }
    }
}

fn build_probe_capabilities(modes: Vec<String>, config_options: Vec<Value>) -> Value {
    let models = extract_model_ids(&config_options);
    json!({
        "style": "modern",
        "agentInfo": Value::Null,
        "agentCapabilities": Value::Null,
        "modes": modes,
        "models": models,
        "configOptions": config_options,
    })
}

fn run_agent_future<F, T>(future: F) -> std::result::Result<T, String>
where
    F: Future<Output = std::result::Result<T, String>>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(
                    "agent probing requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                );
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => agents_runtime()?.block_on(future),
    }
}

fn probe_error_value(error: impl Into<String>) -> Value {
    json!({ "error": error.into() })
}

fn block_on_probe_future<F, T>(future: F) -> std::result::Result<T, String>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    match Handle::try_current() {
        Ok(handle) if matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) => {
            Ok(task::block_in_place(|| handle.block_on(future)))
        }
        Ok(_) => {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let result = agents_runtime().map(|runtime| runtime.block_on(future));
                let _ = sender.send(result);
            });
            receiver
                .recv()
                .map_err(|_| "failed to join probe runtime thread".to_string())?
        }
        Err(_) => Ok(agents_runtime()?.block_on(future)),
    }
}

fn describe_timeout(duration: Duration) -> String {
    if duration.subsec_nanos() == 0 && duration.as_secs() > 0 {
        format!("{}s", duration.as_secs())
    } else if duration.as_millis() > 0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

async fn collect_probe_result<F>(probe: F, per_agent_timeout: Duration) -> Value
where
    F: Future<Output = std::result::Result<Value, String>>,
{
    match timeout(per_agent_timeout, probe).await {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => probe_error_value(error),
        Err(_) => probe_error_value(format!(
            "probe timed out after {}",
            describe_timeout(per_agent_timeout)
        )),
    }
}

fn probe_agents_value(
    root: &std::path::Path,
    agents: Vec<ProbeAgent>,
    per_agent_timeout: Duration,
) -> std::result::Result<Vec<Value>, String> {
    // Ensure the Jockey adapter knows where to find maenifold.json.
    jockey::acp::set_config_root(root.to_path_buf());
    let root = root.to_path_buf();
    block_on_probe_future(async move {
        join_all(agents.into_iter().map(|agent| {
            let root = root.clone();
            task::spawn_blocking(move || probe_agent_capabilities(root, agent, per_agent_timeout))
        }))
        .await
        .into_iter()
        .map(|result| match result {
            Ok(value) => value,
            Err(error) => probe_error_value(error.to_string()),
        })
        .collect()
    })
}

async fn probe_agent_once(root: PathBuf, agent: ProbeAgent) -> std::result::Result<Value, String> {
    // The agent name is the runtime key — our adapter resolves from AgentsConfig
    let runtime_kind = &agent.name;
    let cwd = root.to_string_lossy().into_owned();
    let mcp_servers = agent
        .mcp_servers
        .iter()
        .map(McpServerEntry::to_acp)
        .collect::<Vec<_>>();
    let (config_options, discovered_modes) =
        jockey::acp::prewarm_role_for_config(runtime_kind, &agent.name, &cwd, None).await;
    let modes = if discovered_modes.is_empty() {
        extract_mode_ids(&config_options)
    } else {
        normalize_strings(discovered_modes)
    };

    Ok(build_probe_capabilities(modes, config_options))
}

fn probe_agent_capabilities(
    root: PathBuf,
    agent: ProbeAgent,
    per_agent_timeout: Duration,
) -> Value {
    match run_agent_future(async move {
        Ok(collect_probe_result(probe_agent_once(root, agent), per_agent_timeout).await)
    }) {
        Ok(value) => value,
        Err(error) => probe_error_value(error),
    }
}

fn infer_runtime_kind(entry: &AgentEntry) -> Option<&'static str> {
    let (command, args) = entry.command_parts();
    let binary = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command);
    let mut haystacks = vec![entry.command.to_ascii_lowercase()];
    haystacks.push(binary.to_ascii_lowercase());
    haystacks.extend(args.into_iter().map(|arg| arg.to_ascii_lowercase()));

    if haystacks
        .iter()
        .any(|value| value.contains("claude-agent-acp") || value.contains("claude-acp"))
        || haystacks.iter().any(|value| {
            value == "claude"
                || value == "claude-code"
                || value.ends_with("/claude")
                || value.ends_with("\\claude")
        })
    {
        return Some("claude-code");
    }

    if haystacks.iter().any(|value| {
        value == "gemini"
            || value == "gemini-cli"
            || value.contains("@google/gemini-cli")
            || value.ends_with("/gemini")
            || value.ends_with("\\gemini")
    }) {
        return Some("gemini-cli");
    }

    if haystacks
        .iter()
        .any(|value| value.contains("codex-acp") || value.contains("@zed-industries/codex-acp"))
        || haystacks.iter().any(|value| {
            value == "codex"
                || value == "codex-cli"
                || value.ends_with("/codex")
                || value.ends_with("\\codex")
        })
    {
        return Some("codex-cli");
    }

    None
}

fn extract_mode_ids(config_options: &[Value]) -> Vec<String> {
    extract_option_values(config_options, |option| {
        option
            .get("category")
            .and_then(Value::as_str)
            .is_some_and(|category| category.eq_ignore_ascii_case("mode"))
            || option
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.eq_ignore_ascii_case("mode"))
    })
}

fn extract_model_ids(config_options: &[Value]) -> Vec<String> {
    extract_option_values(config_options, |option| {
        option
            .get("category")
            .and_then(Value::as_str)
            .is_some_and(|category| category.eq_ignore_ascii_case("model"))
            || option
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| id.eq_ignore_ascii_case("model"))
    })
}

fn extract_option_values<F>(config_options: &[Value], matches_option: F) -> Vec<String>
where
    F: Fn(&Map<String, Value>) -> bool,
{
    let mut values = BTreeSet::new();
    for option in config_options {
        let Some(option) = option.as_object() else {
            continue;
        };
        if !matches_option(option) {
            continue;
        }
        if let Some(current_value) = option.get("currentValue").and_then(Value::as_str) {
            values.insert(current_value.to_string());
        }
        if let Some(raw_current_value) = option.get("rawCurrentValue").and_then(Value::as_str) {
            values.insert(raw_current_value.to_string());
        }
        if let Some(options) = option.get("options") {
            collect_select_option_values(options, &mut values);
        }
    }
    values.into_iter().collect()
}

fn collect_select_option_values(value: &Value, out: &mut BTreeSet<String>) {
    let Some(items) = value.as_array() else {
        return;
    };

    for item in items {
        if let Some(value) = item.as_str() {
            out.insert(value.to_string());
            continue;
        }

        let Some(object) = item.as_object() else {
            continue;
        };
        if let Some(value) = object.get("value").and_then(Value::as_str) {
            out.insert(value.to_string());
        }
        if let Some(options) = object.get("options") {
            collect_select_option_values(options, out);
        }
    }
}

fn normalize_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
fn delayed_probe(
    delay_ms: u64,
    result: std::result::Result<Value, String>,
) -> impl Future<Output = std::result::Result<Value, String>> {
    async move {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_capabilities_return_modern_jockey_payload() {
        let capabilities = build_probe_capabilities(vec![], vec![]);

        assert_eq!(capabilities["style"], json!("modern"));
        assert_eq!(capabilities["configOptions"], json!([]));
        assert_eq!(capabilities["modes"], json!([]));
        assert_eq!(capabilities["models"], json!([]));
    }

    #[tokio::test]
    async fn join_all_preserves_sorted_order_for_probe_results() {
        let results = join_all(vec![
            collect_probe_result(
                delayed_probe(5, Ok(json!({"id": "alpha"}))),
                Duration::from_secs(5),
            ),
            collect_probe_result(
                delayed_probe(200, Ok(json!({"id": "bravo"}))),
                Duration::from_secs(5),
            ),
            collect_probe_result(
                delayed_probe(20, Ok(json!({"id": "charlie"}))),
                Duration::from_secs(5),
            ),
        ])
        .await;

        assert_eq!(results[0], json!({"id": "alpha"}));
        assert_eq!(results[1], json!({"id": "bravo"}));
        assert_eq!(results[2], json!({"id": "charlie"}));
    }

    #[tokio::test]
    async fn collect_probe_result_preserves_per_agent_timeout_and_failure_errors() {
        let results = join_all(vec![
            collect_probe_result(
                delayed_probe(50, Ok(json!({"id": "slow"}))),
                Duration::from_millis(10),
            ),
            collect_probe_result(
                delayed_probe(1, Err("probe failed".to_string())),
                Duration::from_secs(1),
            ),
        ])
        .await;

        assert_eq!(results[0], json!({"error": "probe timed out after 10ms"}));
        assert_eq!(results[1], json!({"error": "probe failed"}));
    }
}
