use agent_client_protocol as acp;
use sacp::{ConnectionTo, schema as sacps};

use super::{
    client::HeadlessClient,
    convert::bridge,
    format_stop_reason,
    types::{AgentError, AgentResponse, Result},
    usage_to_response,
};

fn trace_enabled() -> bool {
    std::env::var("MAENIFOLD_TRACE")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off"
            )
        })
        .unwrap_or(true)
}

fn stop_reason_error(stop_reason: Option<&str>) -> Option<String> {
    match stop_reason {
        Some("end_turn" | "max_tokens" | "max_turn_requests" | "refusal" | "cancelled") | None => {
            None
        }
        Some(other) => Some(other.to_string()),
    }
}

pub(super) async fn prompt_session(
    conn: &ConnectionTo<sacp::Agent>,
    client: &HeadlessClient,
    session_id: String,
    prompt: String,
) -> Result<AgentResponse> {
    prompt_session_with_blocks(conn, client, session_id, prompt, vec![]).await
}

pub(super) async fn prompt_session_with_blocks(
    conn: &ConnectionTo<sacp::Agent>,
    client: &HeadlessClient,
    session_id: String,
    prompt: String,
    content_blocks: Vec<acp::ContentBlock>,
) -> Result<AgentResponse> {
    client.begin_turn(&session_id).await;
    if trace_enabled() {
        eprintln!("[acp] prompt turn start");
    }

    let mut content = content_blocks
        .into_iter()
        .map(bridge)
        .collect::<Result<Vec<sacps::ContentBlock>>>()?;
    content.push(sacps::ContentBlock::Text(sacps::TextContent::new(prompt)));

    let result = conn
        .send_request(sacps::PromptRequest::new(session_id.clone(), content))
        .block_task()
        .await;

    let mut response = client.finish_turn(&session_id).await;
    match result {
        Ok(prompt_response) => {
            response.stop_reason = Some(format_stop_reason(prompt_response.stop_reason));
            response.error = stop_reason_error(response.stop_reason.as_deref());
            response.usage = prompt_response.usage.map(usage_to_response);
            if trace_enabled() {
                eprintln!(
                    "[acp] prompt turn complete ({} chars)",
                    response.text.chars().count()
                );
            }
            Ok(response)
        }
        Err(error) => Err(AgentError::Acp(error.to_string())),
    }
}
