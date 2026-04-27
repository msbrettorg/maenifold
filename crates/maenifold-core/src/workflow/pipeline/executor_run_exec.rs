use super::super::external_mcp::require_tool_allowed;
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_prompt_node(
    _context: &ToolContext,
    event_tx: &EventSender,
    agent: &str,
    _session_id: &str,
    prompt: &str,
    _tool_mode: &str,
    adopt_instructions: &[String],
    adopt_role: Option<&str>,
    requires_role: Option<&str>,
    session_config: AgentSessionConfig,
    keep_session_alive: bool,
) -> Result<PromptExecutionResult> {
    require_adopted_role("prompt invocation", requires_role, adopt_role)?;
    let session = open_agent_session(agent, &session_config, event_tx).await?;
    apply_adopt_instructions(&session, adopt_instructions).await?;
    let response_result = session.prompt(prompt).await;
    let acp_session_id = session.remote_id();
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
