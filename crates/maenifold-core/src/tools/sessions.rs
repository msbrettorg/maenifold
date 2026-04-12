use super::context::{ToolContext, load_help_doc, render};
use crate::acp_config::AgentsConfig;
use crate::acp_native::GLOBAL_AGENT_POOL;
use crate::error::{MaenifoldError, Result};
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static SESSIONS_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build sessions runtime: {error}"))
});

fn sessions_runtime() -> std::result::Result<&'static Runtime, String> {
    SESSIONS_RUNTIME.as_ref().map_err(|message| message.clone())
}

pub fn list_sessions(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "ListSessions")
            .unwrap_or_else(|| "ERROR: Help file not found for ListSessions".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let sessions = run_sessions_future(list_sessions_value(context.config.maenifold_root.clone()))?;
    Ok(render(context.output_format, sessions))
}

fn run_sessions_future<F, T>(future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "ListSessions requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                ));
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => sessions_runtime()
            .map_err(MaenifoldError::Message)?
            .block_on(future),
    }
}

async fn list_sessions_value(root: std::path::PathBuf) -> Result<Value> {
    let config = AgentsConfig::load(&root);
    let mut names = config.agents.keys().cloned().collect::<Vec<_>>();
    names.sort();

    let mut sessions = Vec::new();
    let mut unsupported = Vec::new();
    for name in names {
        let entry = config
            .agents
            .get(&name)
            .expect("sorted key should resolve to an agent entry");
        let handle = GLOBAL_AGENT_POOL
            .get_or_create(&name, &entry.command, &root, entry.env.clone(), true)
            .await;
        let listed = match handle.list_sessions(Some(root.clone())).await {
            Ok(listed) => listed,
            Err(error) => {
                tracing::debug!(
                    agent = %name,
                    error = %error,
                    "skipping agent: session/list unsupported or unavailable"
                );
                unsupported.push(json!({
                    "agent": name.as_str(),
                    "status": "unsupported",
                    "error": error.to_string(),
                }));
                continue;
            }
        };

        for session in listed {
            let status = if handle.has_tracked_session(&session.session_id).await {
                "active"
            } else {
                "idle"
            };
            sessions.push(json!({
                "agent": name.as_str(),
                "sessionId": session.session_id,
                "status": status,
                "title": session.title,
                "updatedAt": session.updated_at,
            }));
        }
    }

    let mut result = json!({ "sessions": sessions });
    if !unsupported.is_empty() {
        result["unsupportedAgents"] = Value::Array(unsupported);
    }
    Ok(result)
}
