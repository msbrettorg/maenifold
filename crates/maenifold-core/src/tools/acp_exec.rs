use super::context::ToolContext;
use crate::acp_config::{AgentEntry, AgentsConfig};
use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
};
use agent_client_protocol as acp;
use jockey::acp::AcpPromptResult;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::runtime::{Builder, Handle, Runtime, RuntimeFlavor};
use tokio::task;

static ACP_EXEC_RUNTIME: LazyLock<std::result::Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|error| format!("Failed to build ACP exec runtime: {error}"))
});

fn acp_exec_runtime() -> crate::error::Result<&'static Runtime> {
    ACP_EXEC_RUNTIME
        .as_ref()
        .map_err(|message| MaenifoldError::Message(message.clone()))
}

/// Unified ACP invocation tool backed by Jockey.
///
/// - **prompt** – send a single prompt and return the response text.
/// - **think** – translate the request into a structured prompt and return the response text.
/// - **workflow** – translate the request into a structured prompt and return the response text.
pub fn acp_exec(context: &ToolContext, payload: &Value) -> Result<String> {
    let agent = required_str(payload, "agent")?;
    let mode = mode_or_default(payload)?;
    let model = optional_string(payload, "model");
    let started_at = Instant::now();

    let result = exec_with_jockey(context, payload, agent, mode);

    log_acp_exec_result(
        agent,
        model.as_deref(),
        mode_for_log(payload),
        started_at,
        &result,
    );

    result
}

fn exec_with_jockey(
    context: &ToolContext,
    payload: &Value,
    agent: &str,
    mode: &str,
) -> Result<String> {
    let resolved_agent = resolve_agent(&context.config.maenifold_root, agent)?;
    let request = request_from_payload(payload, mode)?;
    let mut options = request_options_from(context.config.maenifold_root.clone(), payload)?;
    let timeout = options.timeout_seconds.take();

    let runtime_kind = resolved_agent.runtime_kind;
    let agent_name = agent.to_string();
    let cwd = options.cwd.to_string_lossy().to_string();
    let role_mode = options.role_mode.clone();
    let role_config_options = options.role_config_options.clone();
    let app_session_id = options.app_session_id.clone();
    let auto_approve = options.auto_approve;
    let mut context_pairs = request.context;
    context_pairs.extend(options.extra_context.clone());

    run_acp_future(
        async move {
            let (event_tx, _event_rx) = tokio::sync::broadcast::channel::<jockey::JockeyEvent>(32);
            let result = jockey::acp::execute_runtime(
                &runtime_kind,
                &agent_name,
                &request.prompt,
                &context_pairs,
                &cwd,
                &event_tx,
                auto_approve,
                resolved_agent.mcp_servers,
                role_mode,
                role_config_options,
                None,
                &app_session_id,
            )
            .await;
            acp_prompt_result_into_result(result)
        },
        timeout,
    )
}

fn run_acp_future<F, T>(future: F, timeout: Option<u64>) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let future = async move {
        match timeout {
            Some(secs) => tokio::time::timeout(Duration::from_secs(secs), future)
                .await
                .map_err(|_| MaenifoldError::Message(format!("AcpExec timed out after {secs}s")))?,
            None => future.await,
        }
    };
    match Handle::try_current() {
        Ok(handle) => {
            if !matches!(handle.runtime_flavor(), RuntimeFlavor::MultiThread) {
                return Err(MaenifoldError::Message(
                    "AcpExec requires a multi-thread Tokio runtime when invoked from async context"
                        .to_string(),
                ));
            }
            task::block_in_place(|| handle.block_on(future))
        }
        Err(_) => acp_exec_runtime()?.block_on(future),
    }
}

#[derive(Debug)]
struct ResolvedAgent {
    runtime_kind: String,
    mcp_servers: Vec<acp::McpServer>,
}

fn resolve_agent(root: &Path, agent: &str) -> Result<ResolvedAgent> {
    let config = AgentsConfig::load(root);
    let entry = config.agents.get(agent).ok_or_else(|| {
        MaenifoldError::AgentSetup(format!("agent '{agent}' not configured in maenifold.json"))
    })?;

    let runtime_kind = infer_runtime_kind(entry)?;
    let mut seen = HashSet::new();
    let mut mcp_servers = Vec::new();

    for server in config
        .mcp_servers
        .iter()
        .chain(entry.mcp_servers.iter())
        .cloned()
    {
        let name = server.name().to_ascii_lowercase();
        if seen.insert(name) {
            mcp_servers.push(server.to_acp());
        }
    }

    Ok(ResolvedAgent {
        runtime_kind,
        mcp_servers,
    })
}

fn infer_runtime_kind(entry: &AgentEntry) -> Result<String> {
    let (binary, args) = entry.command_parts();
    let binary_name = Path::new(binary)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(binary)
        .to_ascii_lowercase();

    for candidate in std::iter::once(binary_name.as_str()).chain(args.into_iter()) {
        let normalized = candidate.to_ascii_lowercase();
        if normalized.contains("claude") {
            return Ok("claude-code".to_string());
        }
        if normalized.contains("codex") {
            return Ok("codex-cli".to_string());
        }
        if normalized.contains("gemini") {
            return Ok("gemini-cli".to_string());
        }
        if normalized == "mock" {
            return Ok("mock".to_string());
        }
    }

    Err(MaenifoldError::AgentSetup(format!(
        "agent command '{}' does not map to a supported Jockey runtime",
        entry.command
    )))
}

#[derive(Debug)]
struct AcpExecRequest {
    prompt: String,
    context: Vec<(String, String)>,
}

fn request_from_payload(payload: &Value, mode: &str) -> Result<AcpExecRequest> {
    let mut context = optional_context_pairs(payload)?;
    match mode {
        "prompt" => {
            context.push(("acp_exec_mode".to_string(), "prompt".to_string()));
            Ok(AcpExecRequest {
                prompt: required_str(payload, "prompt")?.to_string(),
                context,
            })
        }
        "think" => {
            let topic = required_str(payload, "topic")?;
            let total_thoughts = payload
                .get("totalThoughts")
                .and_then(Value::as_u64)
                .unwrap_or(6);
            context.push(("acp_exec_mode".to_string(), "think".to_string()));
            context.push(("total_thoughts".to_string(), total_thoughts.to_string()));
            Ok(AcpExecRequest {
                prompt: format!(
                    "Use sequential thinking for the following topic and return only the final conclusion.\n\nTopic:\n{topic}"
                ),
                context,
            })
        }
        "workflow" => {
            let workflow_id = required_str(payload, "workflowId")?;
            let workflow_prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
            context.push(("acp_exec_mode".to_string(), "workflow".to_string()));
            context.push(("workflow_id".to_string(), workflow_id.to_string()));
            Ok(AcpExecRequest {
                prompt: if workflow_prompt.is_empty() {
                    format!(
                        "Execute workflow '{workflow_id}' and return only the final conclusion."
                    )
                } else {
                    format!(
                        "Execute workflow '{workflow_id}' using the following input and return only the final conclusion.\n\nInput:\n{workflow_prompt}"
                    )
                },
                context,
            })
        }
        other => Err(MaenifoldError::InvalidArgument(format!(
            "unknown mode '{other}'; expected prompt, think, or workflow"
        ))),
    }
}

#[derive(Debug, PartialEq)]
struct RequestOptions {
    app_session_id: String,
    cwd: PathBuf,
    auto_approve: bool,
    role_mode: Option<String>,
    role_config_options: Vec<(String, String)>,
    timeout_seconds: Option<u64>,
    extra_context: Vec<(String, String)>,
}

fn request_options_from(cwd: PathBuf, payload: &Value) -> Result<RequestOptions> {
    let role_config_options = payload_config_options(payload)?;
    let extra_context = payload_extra_context(payload)?;
    let app_session_id = optional_string(payload, "sessionId").unwrap_or_else(fresh_session_id);

    Ok(RequestOptions {
        app_session_id,
        cwd: optional_string(payload, "cwd")
            .map(PathBuf::from)
            .unwrap_or(cwd),
        auto_approve: required_bool(payload, "autoGrantPermissions")?,
        role_mode: optional_string(payload, "sessionMode"),
        role_config_options,
        timeout_seconds: payload.get("timeoutSeconds").and_then(Value::as_u64),
        extra_context,
    })
}

fn payload_config_options(payload: &Value) -> Result<Vec<(String, String)>> {
    let mut options = Vec::new();

    if let Some(map) = payload.get("configOptions").and_then(Value::as_object) {
        for (key, value) in map {
            options.push((key.clone(), config_value_to_string(key, value)?));
        }
    }

    push_option_if_present(&mut options, "model", optional_string(payload, "model"));
    push_option_if_present(
        &mut options,
        "thoughtLevel",
        optional_string(payload, "thoughtLevel"),
    );

    Ok(options)
}

fn push_option_if_present(options: &mut Vec<(String, String)>, key: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };

    if let Some(existing) = options
        .iter_mut()
        .find(|(existing_key, _)| existing_key == key)
    {
        existing.1 = value;
    } else {
        options.push((key.to_string(), value));
    }
}

fn config_value_to_string(key: &str, value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Null => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must not be null"
        ))),
        other => serde_json::to_string(other).map_err(MaenifoldError::from),
    }
}

fn payload_extra_context(payload: &Value) -> Result<Vec<(String, String)>> {
    let mut context = Vec::new();

    if let Some(allowed_tools) = payload.get("allowedTools").and_then(Value::as_array) {
        let tools = allowed_tools
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !tools.is_empty() {
            context.push(("allowed_tools".to_string(), tools.join(", ")));
        }
    }

    if let Some(max_turns) = payload.get("maxTurns").and_then(Value::as_u64) {
        context.push(("max_turns".to_string(), max_turns.to_string()));
    }

    Ok(context)
}

fn fresh_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("acp-exec-{millis}")
}

fn acp_prompt_result_into_result(result: AcpPromptResult) -> Result<String> {
    if result.ok {
        return Ok(result.output);
    }

    let mut message = result.output.trim().to_string();
    if message.is_empty() {
        message = result
            .error_code
            .clone()
            .unwrap_or_else(|| "ACP execution failed".to_string());
    }
    Err(MaenifoldError::Message(message))
}

fn required_str<'a>(payload: &'a Value, field: &'static str) -> Result<&'a str> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty(field))
}

fn mode_or_default(payload: &Value) -> Result<&str> {
    match payload.get("mode") {
        Some(value) => value.as_str().ok_or_else(|| {
            MaenifoldError::InvalidArgument(
                "mode must be a string; expected prompt, think, or workflow".to_string(),
            )
        }),
        None => Ok("prompt"),
    }
}

fn mode_for_log(payload: &Value) -> &str {
    match payload.get("mode") {
        Some(Value::String(mode)) => mode.as_str(),
        Some(_) => "<invalid>",
        None => "prompt",
    }
}

fn optional_string(payload: &Value, field: &str) -> Option<String> {
    payload.get(field).and_then(Value::as_str).map(String::from)
}

fn required_bool(payload: &Value, field: &'static str) -> Result<bool> {
    payload
        .get(field)
        .and_then(Value::as_bool)
        .ok_or(MaenifoldError::MissingProperty(field))
}

pub(crate) fn optional_context_pairs(payload: &Value) -> Result<Vec<(String, String)>> {
    let Some(arr) = payload.get("context").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    arr.iter()
        .enumerate()
        .map(|(i, item)| {
            let block = serde_json::from_value::<AcpExecContextBlock>(item.clone())
                .map_err(|e| MaenifoldError::InvalidArgument(format!("context[{i}]: {e}")))?;
            context_block_into_pair(i, block)
        })
        .collect()
}

fn context_block_into_pair(index: usize, block: AcpExecContextBlock) -> Result<(String, String)> {
    Ok(match block {
        AcpExecContextBlock::Text(AcpExecTextContext { text }) => {
            (format!("context_text_{index}"), text)
        }
        AcpExecContextBlock::Image(AcpExecImageContext {
            data,
            media_type,
            uri,
        }) => (
            format!("context_image_{index}"),
            format!(
                "media_type={}; uri={}; bytes={}",
                media_type,
                uri.unwrap_or_default(),
                data.len()
            ),
        ),
        AcpExecContextBlock::Resource(AcpExecResourceContext {
            uri,
            text,
            blob,
            media_type,
        }) => {
            let value = match (text, blob) {
                (Some(text), None) => {
                    let mut resource = Map::new();
                    resource.insert("uri".to_string(), Value::String(uri));
                    resource.insert(
                        "mediaType".to_string(),
                        media_type.map(Value::String).unwrap_or(Value::Null),
                    );
                    resource.insert("text".to_string(), Value::String(text));
                    Value::Object(resource)
                }
                (None, Some(blob)) => {
                    let mut resource = Map::new();
                    resource.insert("uri".to_string(), Value::String(uri));
                    resource.insert(
                        "mediaType".to_string(),
                        media_type.map(Value::String).unwrap_or(Value::Null),
                    );
                    resource.insert(
                        "blobBytes".to_string(),
                        Value::Number(serde_json::Number::from(blob.len() as u64)),
                    );
                    Value::Object(resource)
                }
                (Some(_), Some(_)) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires exactly one of 'text' or 'blob'"
                    )));
                }
                (None, None) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires either 'text' or 'blob'"
                    )));
                }
            };

            (
                format!("context_resource_{index}"),
                serde_json::to_string(&value)?,
            )
        }
    })
}

fn log_acp_exec_result<T>(
    agent: &str,
    model: Option<&str>,
    mode: &str,
    started_at: Instant,
    result: &Result<T>,
) {
    let duration_ms = started_at.elapsed().as_millis();
    match result {
        Ok(_) => tracing::info!(
            agent = %agent,
            model = %model.unwrap_or(""),
            mode = %mode,
            duration_ms = duration_ms,
            "[acp-exec] invocation completed"
        ),
        Err(error) => tracing::error!(
            agent = %agent,
            model = %model.unwrap_or(""),
            mode = %mode,
            duration_ms = duration_ms,
            error = %error,
            "[acp-exec] invocation failed"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{context_block_into_pair, infer_runtime_kind, request_options_from};
    use crate::acp_config::AgentEntry;
    use crate::mcp::schema::{AcpExecContextBlock, AcpExecResourceContext};
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn request_options_from_uses_payload_cwd_when_present() {
        let root = PathBuf::from("/repo/root");
        let opts = request_options_from(
            root,
            &json!({
                "autoGrantPermissions": true,
                "sessionId": "sess-123",
                "cwd": "/repo/feature"
            }),
        )
        .expect("request options");

        assert_eq!(opts.app_session_id, "sess-123");
        assert_eq!(opts.cwd, PathBuf::from("/repo/feature"));
    }

    #[test]
    fn request_options_from_defaults_cwd_to_root_when_omitted() {
        let root = PathBuf::from("/repo/root");
        let opts = request_options_from(
            root.clone(),
            &json!({
                "autoGrantPermissions": true
            }),
        )
        .expect("request options");

        assert!(opts.app_session_id.starts_with("acp-exec-"));
        assert_eq!(opts.cwd, root);
    }

    #[test]
    fn infer_runtime_kind_maps_command_binary() {
        let entry = AgentEntry {
            command: "/usr/local/bin/claude --print".to_string(),
            env: HashMap::new(),
            mcp_servers: vec![],
        };

        assert_eq!(
            infer_runtime_kind(&entry).expect("runtime kind"),
            "claude-code"
        );
    }

    #[test]
    fn resource_context_with_text_is_serialized() {
        let pair = context_block_into_pair(
            0,
            AcpExecContextBlock::Resource(AcpExecResourceContext {
                uri: "memory://note".to_string(),
                text: Some("hello".to_string()),
                blob: None,
                media_type: Some("text/plain".to_string()),
            }),
        )
        .expect("context pair");

        assert_eq!(pair.0, "context_resource_0");
        assert!(pair.1.contains("memory://note"));
        assert!(pair.1.contains("hello"));
    }
}
