use super::acp_exec_payload::{
    mode_for_log, mode_or_default, optional_string, request_from_payload, request_options_from,
    required_str,
};
use super::context::ToolContext;
use crate::acp_config::{AgentEntry, AgentsConfig};
use crate::error::{MaenifoldError, Result};
use agent_client_protocol as acp;
use jockey::acp::AcpPromptResult;
use serde_json::Value;
use std::collections::HashSet;
use std::future::Future;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
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
                None,
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

    for server in config.mcp_servers.iter().chain(entry.mcp_servers.iter()) {
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
    use super::infer_runtime_kind;
    use crate::acp_config::AgentEntry;
    use std::collections::HashMap;

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
}
