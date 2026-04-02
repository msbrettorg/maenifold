use agent_client_protocol::{self as acp, Agent as _};
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, time::Duration};

use super::{
    accumulator::SessionAccumulator,
    format_stop_reason,
    types::{AgentError, AgentResponse, Result, DEFAULT_SESSION_TIMEOUT_SECONDS},
    usage_to_response,
};

/// WikiLink tagging instructions injected as context for all ACP sessions.
const WIKILINK_CONTEXT: &str = r#"Tag important concepts as [[WikiLinks]] using these rules:
- Double brackets: [[WikiLink]] never [WikiLink]
- Normalized to lowercase-with-hyphens internally
- PRIMARY concept only: [[MCP]] not [[MCP-server]]
- GENERAL terms: [[authentication]] not [[auth-system]]
- NO file paths, code elements, or trivial words
- TAG substance: [[machine-learning]], [[GraphRAP]], [[vector-embeddings]]
- REUSE existing concepts before inventing near-duplicates
- HYPHENATE multiword: [[null-reference-exception]] not [[Null Reference Exception]]
WikiLinks become knowledge graph nodes. Bad tagging = graph corruption.
Only tag concepts worth a wiki page."#;

pub(super) async fn prompt_session(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    timeout_overrides: &Arc<Mutex<HashMap<String, u64>>>,
    session_id: String,
    prompt: String,
) -> Result<AgentResponse> {
    prompt_session_with_blocks(
        conn,
        sessions,
        timeout_overrides,
        session_id,
        prompt,
        vec![],
    )
    .await
}

pub(super) async fn prompt_session_with_blocks(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    timeout_overrides: &Arc<Mutex<HashMap<String, u64>>>,
    session_id: String,
    prompt: String,
    content_blocks: Vec<acp::ContentBlock>,
) -> Result<AgentResponse> {
    let timeout_seconds = timeout_overrides
        .lock()
        .await
        .get(&session_id)
        .copied()
        .unwrap_or(DEFAULT_SESSION_TIMEOUT_SECONDS);

    sessions
        .lock()
        .await
        .insert(session_id.clone(), SessionAccumulator::default());

    let mut content = vec![acp::ContentBlock::Text(acp::TextContent::new(WIKILINK_CONTEXT))];
    content.extend(content_blocks);
    content.push(acp::ContentBlock::Text(acp::TextContent::new(prompt)));

    let result = tokio::time::timeout(
        Duration::from_secs(timeout_seconds),
        conn.prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.clone()),
            content,
        )),
    )
    .await;

    let mut response = sessions
        .lock()
        .await
        .remove(&session_id)
        .unwrap_or_default()
        .into_response();

    match result {
        Ok(Ok(prompt_response)) => {
            response.stop_reason = Some(format_stop_reason(prompt_response.stop_reason));
            response.usage = prompt_response.usage.map(usage_to_response);
            Ok(response)
        }
        Ok(Err(error)) => Err(AgentError::from(error)),
        Err(_) => {
            if let Err(error) = conn
                .cancel(acp::CancelNotification::new(acp::SessionId::new(
                    session_id.clone(),
                )))
                .await
            {
                tracing::warn!(
                    session_id = %session_id,
                    "Failed to cancel timed out ACP prompt: {error}"
                );
            }
            Err(AgentError::Timeout {
                message: format!("prompt timed out after {timeout_seconds}s"),
            })
        }
    }
}
