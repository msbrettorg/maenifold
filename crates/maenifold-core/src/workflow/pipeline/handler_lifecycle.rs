use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use dashmap::{DashMap, mapref::entry::Entry};
use once_cell::sync::Lazy;
use serde_json::Value;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use super::{execute_pipeline, finalize_failed_pipeline_execution, persist_final_pipeline_status};

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

static RUNNING_PIPELINE_TOKENS: Lazy<DashMap<String, CancellationToken>> = Lazy::new(DashMap::new);

pub(super) fn handle_lifecycle_operation(
    context: &ToolContext,
    session_id: &str,
    payload: &Value,
) -> Result<Option<String>> {
    if payload.get("rewind").is_some() {
        return rewind::handle_rewind_operation(context, session_id, payload).map(Some);
    }
    if payload.get("run").and_then(Value::as_bool).unwrap_or(false) {
        let prompt = payload.get("prompt").and_then(Value::as_str);
        return run_pipeline(context, session_id, prompt).map(Some);
    }
    if payload
        .get("pause")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return pause_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("resume")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return resume_pipeline(context, session_id).map(Some);
    }
    if payload
        .get("cancel")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return cancel_pipeline(context, session_id).map(Some);
    }

    Ok(None)
}

fn run_pipeline(context: &ToolContext, session_id: &str, prompt: Option<&str>) -> Result<String> {
    Handle::try_current().map_err(|error| {
        MaenifoldError::Message(format!(
            "Pipeline {session_id}: run=true requires an active Tokio runtime: {error}"
        ))
    })?;

    let token = CancellationToken::new();
    match RUNNING_PIPELINE_TOKENS.entry(session_id.to_string()) {
        Entry::Occupied(_) => {
            return Err(MaenifoldError::Message(format!(
                "Pipeline {session_id} is already running"
            )));
        }
        Entry::Vacant(entry) => {
            entry.insert(token.clone());
        }
    }

    let task_context = ToolContext::new(context.config.clone(), context.output_format);
    let session_id_owned = session_id.to_string();
    let prompt_owned = prompt.unwrap_or_default().to_string();
    tokio::spawn(async move {
        let execution =
            execute_pipeline(&task_context, &session_id_owned, &prompt_owned, token).await;
        match execution {
            Ok(result) => {
                if let Err(error) =
                    persist_final_pipeline_status(&task_context, &session_id_owned, &result.status)
                {
                    tracing::error!(
                        session_id = %session_id_owned,
                        error = %error,
                        "[pipeline] failed to persist final in-process status"
                    );
                }
            }
            Err(error) => {
                tracing::error!(
                    session_id = %session_id_owned,
                    error = %error,
                    "[pipeline] in-process execution failed"
                );
                finalize_failed_pipeline_execution(&task_context, &session_id_owned);
            }
        }
        RUNNING_PIPELINE_TOKENS.remove(&session_id_owned);
    });

    Ok(format!("Pipeline {session_id} started"))
}

fn pause_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: pause requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}

fn resume_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: resume requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}

fn cancel_pipeline(_context: &ToolContext, session_id: &str) -> Result<String> {
    if let Some((_, token)) = RUNNING_PIPELINE_TOKENS.remove(session_id) {
        token.cancel();
        Ok(format!("Pipeline {session_id} cancellation requested"))
    } else {
        Err(MaenifoldError::Message(format!(
            "Pipeline {session_id} is not running"
        )))
    }
}

#[cfg(test)]
fn register_running_pipeline_token_for_test(session_id: &str) -> CancellationToken {
    let token = CancellationToken::new();
    RUNNING_PIPELINE_TOKENS.insert(session_id.to_string(), token.clone());
    token
}

#[cfg(test)]
fn running_pipeline_token_registered_for_test(session_id: &str) -> bool {
    RUNNING_PIPELINE_TOKENS.contains_key(session_id)
}

#[cfg(test)]
fn clear_running_pipeline_tokens_for_test() {
    RUNNING_PIPELINE_TOKENS.clear();
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use crate::util::json::OutputFormat;

    use super::super::test_support::{
        pipeline_test_context, seed_pipeline_session_with_node_sessions,
    };
    use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
    use super::*;

    fn ready_tool_pipeline() -> (PipelineDefinition, HashMap<String, NodeState>) {
        let definition = PipelineDefinition {
            nodes: vec![PipelineNode::Tool {
                id: "tool".to_string(),
                tool_name: "RecentActivity".to_string(),
                tool_args: Some(json!({ "limit": 1 })),
                agent: None,
                model: None,
                thought_level: None,
                allowed_tools: None,
                max_turns: None,
                agent_config: None,
            }],
            edges: vec![],
        };
        let node_states = HashMap::from([(
            "tool".to_string(),
            NodeState {
                session_id: "workflow-1700000000000-tool".to_string(),
                status: NodeStatus::Ready,
                attempts: 0,
                thinking_session_id: None,
                output: None,
                failure_reason: None,
                retry_after: None,
                retry_count: None,
            },
        )]);
        (definition, node_states)
    }

    #[tokio::test]
    async fn run_pipeline_no_longer_requires_external_driver() {
        clear_running_pipeline_tokens_for_test();
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-run";
        let (definition, node_states) = ready_tool_pipeline();
        seed_pipeline_session_with_node_sessions(&context, session_id, definition, node_states, 1);

        let result = handle_lifecycle_operation(
            &context,
            session_id,
            &json!({ "run": true, "prompt": "go" }),
        );

        assert!(
            result.is_ok(),
            "expected run=true to return immediately from the in-process pipeline path, got {result:?}"
        );
    }

    #[tokio::test]
    async fn cancel_pipeline_cancels_registered_token_and_cleans_up_lookup() {
        clear_running_pipeline_tokens_for_test();
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-cancel";
        let token = register_running_pipeline_token_for_test(session_id);

        let result = handle_lifecycle_operation(&context, session_id, &json!({ "cancel": true }));

        assert!(
            result.is_ok()
                && token.is_cancelled()
                && !running_pipeline_token_registered_for_test(session_id),
            "expected cancel=true to cancel the registered [[CancellationToken]] and remove its [[pipeline]] lookup entry, got result={result:?}, cancelled={}, still_registered={}",
            token.is_cancelled(),
            running_pipeline_token_registered_for_test(session_id),
        );
    }
}
