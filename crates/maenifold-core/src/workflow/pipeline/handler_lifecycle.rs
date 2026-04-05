use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use serde_json::Value;

#[path = "handler_lifecycle_rewind.rs"]
mod rewind;

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
fn run_pipeline(_context: &ToolContext, session_id: &str, _prompt: Option<&str>) -> Result<String> {
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: run=true requires an external pipeline driver; \
         use the pipeline-ui or `gate dispatch` to start autonomous execution"
    )))
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
    Err(MaenifoldError::Message(format!(
        "Pipeline {session_id}: cancel requires an external pipeline driver; \
         the in-process supervisor has been removed"
    )))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use once_cell::sync::Lazy;
    use serde_json::json;
    use tokio_util::sync::CancellationToken;

    use crate::util::json::OutputFormat;

    use super::super::test_support::{
        pipeline_test_context, seed_pipeline_session_with_node_sessions,
    };
    use super::super::{NodeState, NodeStatus, PipelineDefinition, PipelineNode};
    use super::*;

    static TEST_RUNNING_PIPELINE_TOKENS: Lazy<Mutex<HashMap<String, CancellationToken>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

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
            },
        )]);
        (definition, node_states)
    }

    fn register_test_token(session_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        TEST_RUNNING_PIPELINE_TOKENS
            .lock()
            .expect("lock test token registry")
            .insert(session_id.to_string(), token.clone());
        token
    }

    fn test_token_is_registered(session_id: &str) -> bool {
        TEST_RUNNING_PIPELINE_TOKENS
            .lock()
            .expect("lock test token registry")
            .contains_key(session_id)
    }

    fn clear_test_tokens() {
        TEST_RUNNING_PIPELINE_TOKENS
            .lock()
            .expect("lock test token registry")
            .clear();
    }

    #[tokio::test]
    async fn run_pipeline_no_longer_requires_external_driver() {
        clear_test_tokens();
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
    #[ignore = "T-104.2 implementation pending: requires in-process run registry"]
    async fn run_pipeline_rejects_duplicate_in_process_execution() {
        todo!(
            "Start one in-process pipeline run, assert the immediate second run rejects the duplicate session"
        );
    }

    #[tokio::test]
    async fn cancel_pipeline_cancels_registered_token_and_cleans_up_lookup() {
        clear_test_tokens();
        let (_tmp, context, _guard) = pipeline_test_context(OutputFormat::Json);
        let session_id = "pipeline-1700000000000-cancel";
        let token = register_test_token(session_id);

        let result = handle_lifecycle_operation(&context, session_id, &json!({ "cancel": true }));

        assert!(
            result.is_ok() && token.is_cancelled() && !test_token_is_registered(session_id),
            "expected cancel=true to cancel the registered [[CancellationToken]] and remove its [[pipeline]] lookup entry, got result={result:?}, cancelled={}, still_registered={}",
            token.is_cancelled(),
            test_token_is_registered(session_id),
        );
    }
}
