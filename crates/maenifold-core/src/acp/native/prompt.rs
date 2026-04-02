use agent_client_protocol::{self as acp, Agent as _};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use super::{
    accumulator::SessionAccumulator,
    format_stop_reason,
    types::{AgentError, AgentResponse, Result},
    usage_to_response,
};

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
) -> Result<AgentResponse> {
    prompt_session_with_blocks(conn, sessions, session_id, prompt, vec![]).await
}

pub(super) async fn prompt_session_with_blocks(
    conn: &acp::ClientSideConnection,
    sessions: &Arc<Mutex<HashMap<String, SessionAccumulator>>>,
    session_id: String,
    prompt: String,
    content_blocks: Vec<acp::ContentBlock>,
) -> Result<AgentResponse> {
    sessions
        .lock()
        .await
        .insert(session_id.clone(), SessionAccumulator::default());

    let mut content = content_blocks;
    content.push(acp::ContentBlock::Text(acp::TextContent::new(prompt)));


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
