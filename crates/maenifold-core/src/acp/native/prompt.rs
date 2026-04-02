use agent_client_protocol::{self as acp, Agent as _};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use super::{
    accumulator::SessionAccumulator,
    format_stop_reason,
    types::{AgentError, AgentResponse, Result},
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

fn stop_reason_error(stop_reason: Option<&str>) -> Option<String> {
    match stop_reason {
        Some("end_turn" | "max_tokens" | "max_turn_requests" | "refusal" | "cancelled") | None => None,
        Some(other) => Some(other.to_string()),
    }
}

pub(super) async fn prompt_session(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: String,
    prompt: String,
    inject_wikilinks: bool,
) -> Result<AgentResponse> {
    prompt_session_with_blocks(
        conn,
        sessions,
        session_id,
        prompt,
        vec![],
        inject_wikilinks,
    )
    .await
}

pub(super) async fn prompt_session_with_blocks(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: String,
    prompt: String,
    content_blocks: Vec<acp::ContentBlock>,
    inject_wikilinks: bool,
) -> Result<AgentResponse> {
    sessions
        .lock()
        .await
        .insert(session_id.clone(), SessionAccumulator::default());

    let final_prompt = if inject_wikilinks {
        format!("{prompt}\n\n{WIKILINK_CONTEXT}")
    } else {
        prompt
    };
    let mut content = content_blocks;
    content.push(acp::ContentBlock::Text(acp::TextContent::new(final_prompt)));


    let result = conn
        .prompt(acp::PromptRequest::new(
            acp::SessionId::new(session_id.clone()),
            content,
        ))
        .await;


    let mut response = sessions
        .lock()
        .await
        .remove(&session_id)
        .unwrap_or_default()
        .into_response();


    match result {
        Ok(prompt_response) => {
            response.stop_reason = Some(format_stop_reason(prompt_response.stop_reason));
            response.error = stop_reason_error(response.stop_reason.as_deref());
            response.usage = prompt_response.usage.map(usage_to_response);
            Ok(response)
        }
        Err(error) => Err(AgentError::from(error)),
    }
}
