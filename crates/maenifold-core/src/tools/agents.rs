use super::context::{ToolContext, render};
use crate::acp_config::AgentsConfig;
use crate::acp_native::{GLOBAL_AGENT_POOL, SessionOpts};
use crate::error::Result;
use agent_client_protocol as acp;
use serde_json::{Map, Value, json};
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

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
    for name in &names {
        let entry = config
            .agents
            .get(name)
            .expect("sorted key should resolve to an agent entry");
        let mut value = list_agent_entry(name, entry)?;
        if probe {
            let caps = probe_agent(root, name, entry);
            if let Value::Object(ref mut obj) = value {
                obj.insert("capabilities".to_string(), caps);
            }
        }
        agents.push(value);
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

fn serialize_category(category: &Option<acp::SessionConfigOptionCategory>) -> Value {
    match category {
        Some(acp::SessionConfigOptionCategory::Model) => json!("model"),
        Some(acp::SessionConfigOptionCategory::Mode) => json!("mode"),
        Some(acp::SessionConfigOptionCategory::ThoughtLevel) => json!("thought_level"),
        Some(acp::SessionConfigOptionCategory::Other(value)) => json!(value),
        Some(_) | None => Value::Null,
    }
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

/// Spawn an agent, initialize, create a session, and capture advertised
/// capabilities (modes, models, configOptions).
fn probe_agent(root: &std::path::Path, name: &str, entry: &crate::acp_config::AgentEntry) -> Value {
    let command = entry.command.clone();
    let env = entry.env.clone();
    let cwd = root.to_path_buf();
    let agent_name = name.to_string();

    let result: std::result::Result<Value, String> = run_agent_future(async move {
        let handle = GLOBAL_AGENT_POOL
            .get_or_create(&agent_name, &command, &cwd, env, true)
            .await;

        let info = handle
            .new_session(SessionOpts {
                cwd: cwd.clone(),
                auto_grant_permissions: true,
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
        let has_config_options = !info.config_options.is_empty();
        let agent_info = handle.agent_info();
        let agent_capabilities = handle.capabilities();

        let mut capabilities = json!({
            "style": if has_config_options { "modern" } else { "legacy" },
            "agentInfo": serialize_agent_info(&agent_info),
            "agentCapabilities": serialize_agent_capabilities(&agent_capabilities),
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
                json!({
                    "id": opt.id.to_string(),
                    "name": opt.name,
                    "description": opt.description,
                    "category": serialize_category(&opt.category),
                })
            }).collect::<Vec<_>>(),
        });

        if let Err(error) = handle.close_session(&session_id).await {
            if let Value::Object(ref mut object) = capabilities {
                object.insert("closeSessionError".to_string(), json!(error.to_string()));
            }
        }
        Ok(capabilities)
    });

    match result {
        Ok(value) => value,
        Err(error) => json!({ "error": error }),
    }
}
