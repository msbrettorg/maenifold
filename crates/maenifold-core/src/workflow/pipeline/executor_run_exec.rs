use super::super::external_mcp::require_tool_allowed;
use super::*;
use std::sync::Arc;

use super::super::super::agent_session::AcpSessionCapturedCallback;
use super::executor_run_persist::persist_active_node_session;

/// T-328.15 — Wall-clock budget for a single prompt node await.
/// Catches alive-but-hung ACP agents (T-328.14 only catches process death).
pub(crate) const PROMPT_BUDGET_SECS: u64 = 600;
/// Per-thought budget for sequentialThinking nodes (multiplied by total_thoughts).
pub(crate) const THINKING_PER_THOUGHT_BUDGET_SECS: u64 = 600;
/// Budget for workflow nodes (many steps, so longer than single prompt).
pub(crate) const WORKFLOW_BUDGET_SECS: u64 = 1800;

/// T-328.15 — Run `fut` with a wall-clock budget. On expiry, return a
/// `MaenifoldError::Message` with the canonical "[<agent>] timed out after <N>s"
/// shape so the pipeline node abandons with a clear diagnostic message.
pub(crate) async fn await_with_budget<F, T>(
    agent: &str,
    budget: std::time::Duration,
    fut: F,
) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match tokio::time::timeout(budget, fut).await {
        Ok(res) => res,
        Err(_) => Err(MaenifoldError::Message(format!(
            "[{agent}] timed out after {}s",
            budget.as_secs()
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_prompt_node(
    context: &ToolContext,
    event_tx: &EventSender,
    agent: &str,
    node_id: &str,
    pipeline_session_id: &str,
    session_id: &str,
    prompt: &str,
    _tool_mode: &str,
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    keep_session_alive: bool,
) -> Result<PromptExecutionResult> {
    let _ = session_id;
    require_adopted_role("prompt invocation", requires_role, adopt_role)?;
    let mut session = open_agent_session(agent, &session_config, event_tx).await?;
    let acp_session_id = session.remote_id();
    persist_active_node_session(
        context,
        pipeline_session_id,
        node_id,
        Some(&acp_session_id),
        None,
        session_config.model.as_deref(),
    )
    .await?;
    let callback_context = ToolContext::new(context.config.clone(), context.output_format)
        .with_stdio_pool(context.stdio_pool.clone());
    let callback_session_id = pipeline_session_id.to_string();
    let callback_node_id = node_id.to_string();
    let callback_model = session_config.model.clone();
    let callback: AcpSessionCapturedCallback = Arc::new(move |acp_session_id| {
        let context = ToolContext::new(
            callback_context.config.clone(),
            callback_context.output_format,
        )
        .with_stdio_pool(callback_context.stdio_pool.clone());
        let session_id = callback_session_id.clone();
        let node_id = callback_node_id.clone();
        let model = callback_model.clone();
        Box::pin(async move {
            persist_active_node_session(
                &context,
                &session_id,
                &node_id,
                Some(&acp_session_id),
                None,
                model.as_deref(),
            )
            .await
        })
    });
    session.set_acp_session_captured_callback(callback);
    apply_adopt_instructions(&session, adopt_instructions).await?;
    let response_result = await_with_budget(
        agent,
        std::time::Duration::from_secs(PROMPT_BUDGET_SECS),
        session.prompt(prompt),
    )
    .await;
    let prompt_result = match response_result {
        Ok(response) => {
            if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
                Ok(PromptExecutionResult {
                    response: encode_prompt_failure(&error),
                    acp_session_id: Some(acp_session_id),
                    input_tokens: response.input_tokens,
                    output_tokens: response.output_tokens,
                    model: response.model,
                })
            } else {
                Ok(PromptExecutionResult {
                    response: response.text.trim().to_string(),
                    acp_session_id: Some(acp_session_id),
                    input_tokens: response.input_tokens,
                    output_tokens: response.output_tokens,
                    model: response.model,
                })
            }
        }
        Err(error) => Ok(PromptExecutionResult {
            response: encode_prompt_failure(&error.to_string()),
            acp_session_id: Some(acp_session_id),
            input_tokens: None,
            output_tokens: None,
            model: None,
        }),
    };
    if keep_session_alive {
        session.keep_alive();
        prompt_result
    } else {
        let close_result = session.close().await;
        match (prompt_result, close_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(error)) => {
                let response = match decode_prompt_failure(&value.response) {
                    Some(reason) => encode_prompt_failure(
                        &runner::append_close_error_context(MaenifoldError::Message(reason), error)
                            .to_string(),
                    ),
                    None => return Err(error),
                };
                Ok(PromptExecutionResult {
                    response,
                    acp_session_id: value.acp_session_id,
                    input_tokens: value.input_tokens,
                    output_tokens: value.output_tokens,
                    model: value.model,
                })
            }
            (Err(error), Ok(())) => Err(error),
            (Err(error), Err(close_error)) => {
                Err(runner::append_close_error_context(error, close_error))
            }
        }
    }
}

pub(crate) async fn apply_adopt_instructions(
    session: &AgentSession,
    adopt_instructions: &[String],
) -> Result<()> {
    for instruction in adopt_instructions {
        let response = session.prompt(instruction).await?;
        if let Some(error) = response.error.filter(|value| !value.trim().is_empty()) {
            return Err(MaenifoldError::Message(error));
        }
    }
    Ok(())
}

pub(crate) async fn run_tool_node(
    context: &ToolContext,
    tool_name: &str,
    tool_args: Option<&JsonValue>,
    agent: Option<&str>,
    allowed_tools: Option<&[String]>,
    cwd: &Path,
) -> Result<ToolExecutionResult> {
    info!(tool_name, "executing pipeline tool node");
    let arguments = tool_args
        .cloned()
        .unwrap_or_else(|| JsonValue::Object(Default::default()));

    let response = async {
        if let Some((server_name, remote_tool_name)) = split_namespaced_tool_name(tool_name) {
            if server_name == "maenifold" {
                require_tool_allowed(tool_name, allowed_tools)?;
                Ok(invoke_local_tool(remote_tool_name, &arguments)?)
            } else {
                let agents_config = AgentsConfig::load(&context.config.maenifold_root)?;
                invoke_external_tool(
                    context,
                    &agents_config,
                    server_name,
                    remote_tool_name,
                    tool_args,
                    agent,
                    allowed_tools,
                    cwd,
                )
                .await
            }
        } else {
            require_tool_allowed(tool_name, allowed_tools)?;
            Ok(invoke_local_tool(tool_name, &arguments)?)
        }
    }
    .await;

    match response {
        Ok(response) => {
            let response_summary = truncate_chars(&response, MAX_CONCLUSION_CHARS);
            Ok(ToolExecutionResult {
                response,
                provenance: ToolProvenance {
                    tool_name: tool_name.to_string(),
                    resolved_args: Some(arguments),
                    response_summary,
                },
            })
        }
        Err(e) => {
            error!(tool_name, error = %e, "pipeline tool node failed");
            Err(e)
        }
    }
}

fn invoke_local_tool(tool_name: &str, arguments: &JsonValue) -> Result<String> {
    ToolRegistry::global()
        .try_invoke(tool_name, &arguments, OutputFormat::Json)
        .map_err(|error| {
            if matches!(
                error,
                MaenifoldError::UnknownTool(_)
                    | MaenifoldError::InvalidPayload(_)
                    | MaenifoldError::InvalidArgument(_)
                    | MaenifoldError::InvalidResourceUri(_)
                    | MaenifoldError::AssetNotFound(_, _)
                    | MaenifoldError::PathTraversal(_)
                    | MaenifoldError::MissingProperty(_)
            ) {
                error
            } else {
                MaenifoldError::Message(format!("Error invoking {tool_name}: {error}"))
            }
        })
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    // T-328.15 — Sleeper-agent stand-in: a future that never resolves.
    // Asserts the wall-clock budget fires and yields the canonical
    // "[<agent>] timed out after <N>s" message that drives node abandonment.
    #[tokio::test(start_paused = true)]
    async fn await_with_budget_times_out_hung_agent() {
        let budget = std::time::Duration::from_secs(600);
        let sleeper = async {
            tokio::time::sleep(std::time::Duration::from_secs(86_400)).await;
            Ok::<(), MaenifoldError>(())
        };
        let result = await_with_budget("copilot", budget, sleeper).await;
        let err = result.expect_err("hung agent must surface as Err");
        assert_eq!(err.to_string(), "[copilot] timed out after 600s");
    }

    #[tokio::test(start_paused = true)]
    async fn await_with_budget_passes_through_fast_success() {
        let budget = std::time::Duration::from_secs(600);
        let quick = async { Ok::<u32, MaenifoldError>(42) };
        let result = await_with_budget("copilot", budget, quick).await;
        assert_eq!(result.unwrap(), 42);
    }
}
