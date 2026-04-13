use super::context::{ToolContext, render};
use crate::acp_config::{AgentEntry, AgentsConfig};
use crate::acp_native::{GLOBAL_AGENT_POOL, SessionInfo, SessionOpts};
use crate::error::{MaenifoldError, Result};
use agent_client_protocol as acp;
use futures_util::future::join_all;
use serde_json::{Map, Value, json};
use std::future::Future;
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
    for name in &names {
        let entry = config
            .agents
            .get(name)
            .expect("sorted key should resolve to an agent entry");
        let value = list_agent_entry(name, entry)?;
        if probe {
            probe_entries.push((name.clone(), entry.clone()));
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

fn serialize_agent_info(agent_info: &Option<acp::Implementation>) -> Value {
    match agent_info {
        Some(info) => json!({
            "name": info.name,
            "version": info.version,
            "title": info.title,
        }),
        None => Value::Null,
    }
}

fn serialize_agent_capabilities(capabilities: &Option<acp::AgentCapabilities>) -> Value {
    match capabilities {
        Some(capabilities) => json!({
            "promptCapabilities": {
                "image": capabilities.prompt_capabilities.image,
                "audio": capabilities.prompt_capabilities.audio,
                "embeddedContext": capabilities.prompt_capabilities.embedded_context,
            },
            "sessionCapabilities": {
                "list": capabilities.session_capabilities.list.is_some(),
                "loadSession": capabilities.load_session,
            },
        }),
        None => Value::Null,
    }
}

fn build_probe_capabilities(
    info: &SessionInfo,
    agent_info: &Option<acp::Implementation>,
    agent_capabilities: &Option<acp::AgentCapabilities>,
) -> Value {
    let has_config_options = !info.config_options.is_empty();
    json!({
        "style": if has_config_options { "modern" } else { "legacy" },
        "agentInfo": serialize_agent_info(agent_info),
        "agentCapabilities": serialize_agent_capabilities(agent_capabilities),
        "modes": info.modes.iter().map(|mode| {
            json!({
                "id": mode.id.to_string(),
                "name": mode.name,
                "description": mode.description,
            })
        }).collect::<Vec<_>>(),
        "models": info.models.iter().map(|model| {
            json!({
                "modelId": model.model_id.to_string(),
                "name": model.name,
                "description": model.description,
            })
        }).collect::<Vec<_>>(),
        "configOptions": info.config_options.iter().map(|opt| {
            serde_json::to_value(opt).unwrap_or_default()
        }).collect::<Vec<_>>(),
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

fn probe_agent_capabilities(
    root: std::path::PathBuf,
    name: String,
    entry: AgentEntry,
    per_agent_timeout: Duration,
) -> Value {
    match run_agent_future(async move {
        Ok(collect_probe_result(probe_agent_once(root, name, entry), per_agent_timeout).await)
    }) {
        Ok(value) => value,
        Err(error) => probe_error_value(error),
    }
}

fn probe_agents_value(
    root: &std::path::Path,
    agents: Vec<(String, AgentEntry)>,
    per_agent_timeout: Duration,
) -> std::result::Result<Vec<Value>, String> {
    let root = root.to_path_buf();
    block_on_probe_future(async move {
        join_all(agents.into_iter().map(|(name, entry)| {
            let root = root.clone();
            task::spawn_blocking(move || {
                probe_agent_capabilities(root, name, entry, per_agent_timeout)
            })
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

/// Spawn an agent, initialize, create a session, and capture advertised
/// capabilities (modes, models, configOptions).
async fn probe_agent_once(
    root: std::path::PathBuf,
    name: String,
    entry: AgentEntry,
) -> std::result::Result<Value, String> {
    let command = entry.command;
    let env = entry.env;
    let handle = GLOBAL_AGENT_POOL
        .get_or_create(&name, &command, &root, env, true)
        .await;

    let info = handle
        .new_session(SessionOpts {
            session_id: None,
            cwd: root.clone(),
            auto_grant_permissions: true,
            acp_connection_timeout_seconds: None,
            model: None,
            mode: None,
            thought_level: None,
            config_options: None,
            allowed_tools: None,
            max_turns: None,
            timeout_seconds: None,
            mcp_servers: vec![],
        })
        .await
        .map_err(|e| e.to_string())?;

    let session_id = info.session_id.clone();
    let agent_info = handle.agent_info();
    let agent_capabilities = handle.capabilities();
    let _ = handle.close_session(&session_id).await;
    Ok(build_probe_capabilities(
        &info,
        &agent_info,
        &agent_capabilities,
    ))
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
    fn probe_capabilities_ignore_unsupported_close_session_errors() {
        let capabilities = build_probe_capabilities(&SessionInfo::default(), &None, &None);

        assert_eq!(capabilities["style"], json!("legacy"));
        assert!(
            capabilities.get("closeSessionError").is_none(),
            "unsupported close_session cleanup must not poison otherwise-valid probe results"
        );
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
