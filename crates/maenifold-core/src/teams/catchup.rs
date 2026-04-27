//! Catch-up polling for the offline gap (T-238.54).
//!
//! Ports `_catchup_poll()` from agency-cowork
//! (`agency-cowork/skills/teams/scripts/monitor/service.py` L349-443).
//! On supervisor startup we fetch recent messages for each monitored
//! conversation via chatsvc REST and feed them into the same
//! `mpsc::Sender<EventMessage>` the Trouter runner uses. The existing
//! `MessageDedup` in `process_messages` keeps duplicates from
//! double-dispatching, so this is purely additive. Per-conversation errors
//! are swallowed at debug level — one bad chat must not poison the sweep
//! (parity with python service.py L437-438). URL encoding mirrors
//! `reply.rs::reply_url` (`urlencoding::encode`); the transitively-available
//! `percent_encoding` crate is intentionally avoided to keep `Cargo.toml`
//! untouched.
use std::collections::BTreeSet;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::sync::mpsc;

use super::types::EventMessage;

/// 10-minute look-back. Matches `_CATCHUP_WINDOW_SECONDS` (service.py L367).
pub(crate) const CATCHUP_WINDOW_SECS: u64 = 600;
/// Page size. Matches service.py L393.
pub(crate) const CATCHUP_PAGE_SIZE: u32 = 30;

const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Poll chatsvc for recent messages from each non-wildcard conversation
/// in `conversations` and forward them through `msg_tx`. Returns the
/// number of messages forwarded. Wildcards (`"*"`) are filtered (chatsvc
/// has no "all my conversations" endpoint — service.py L373-379).
pub(crate) async fn run_catchup_poll(
    token: &str,
    region: &str,
    conversations: &[String],
    msg_tx: &mpsc::Sender<EventMessage>,
) -> usize {
    let conv_ids: BTreeSet<&String> = conversations
        .iter()
        .filter(|c| c.as_str() != "*" && !c.is_empty())
        .collect();
    if conv_ids.is_empty() {
        tracing::debug!("Catch-up: no specific conversation IDs to poll (wildcard only)");
        return 0;
    }

    let client = match reqwest::Client::builder().timeout(HTTP_TIMEOUT).build() {
        Ok(c) => c,
        Err(error) => {
            tracing::debug!(%error, "Catch-up: failed to build HTTP client");
            return 0;
        }
    };

    let cutoff = Utc::now() - chrono::Duration::seconds(CATCHUP_WINDOW_SECS as i64);
    let bearer = format!("Bearer {}", token.trim().trim_start_matches("Bearer "));
    let mut total = 0usize;

    for conv_id in conv_ids {
        match poll_one(&client, &bearer, region, conv_id, cutoff, msg_tx).await {
            Ok(n) => total += n,
            Err(error) => {
                let preview: String = conv_id.chars().take(30).collect();
                tracing::debug!(%error, conversation = %preview, "Catch-up poll error");
            }
        }
    }

    if total > 0 {
        tracing::info!(count = total, "Catch-up: processed missed messages");
    } else {
        tracing::debug!("Catch-up: no missed messages found");
    }
    total
}

async fn poll_one(
    client: &reqwest::Client,
    bearer: &str,
    region: &str,
    conv_id: &str,
    cutoff: DateTime<Utc>,
    msg_tx: &mpsc::Sender<EventMessage>,
) -> Result<usize, String> {
    // urllib.parse.quote(..., safe="") equivalent (service.py L389).
    let encoded = urlencoding::encode(conv_id);
    let url = format!(
        "https://teams.cloud.microsoft/api/chatsvc/{region}/v1/users/ME/conversations/{encoded}/messages?view=msnp24Equivalent&pageSize={CATCHUP_PAGE_SIZE}"
    );

    let response = client
        .get(&url)
        .header("Authorization", bearer)
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        // Non-200 → debug + skip (service.py L396-398).
        let preview: String = conv_id.chars().take(30).collect();
        tracing::debug!(conversation = %preview, status = %status.as_u16(), "Catch-up: non-200");
        return Ok(0);
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("json decode failed: {e}"))?;

    // {"messages": [...]} or top-level array (service.py L400).
    let messages: &[Value] = match &body {
        Value::Object(map) => map
            .get("messages")
            .and_then(Value::as_array)
            .map_or(&[][..], |v| v.as_slice()),
        Value::Array(arr) => arr.as_slice(),
        _ => &[][..],
    };

    let mut count = 0usize;
    for msg in messages {
        // id → clientmessageid fallback; skip if absent (service.py L403-405).
        let message_id = msg
            .get("id")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| msg.get("clientmessageid").and_then(Value::as_str))
            .unwrap_or("");
        if message_id.is_empty() {
            continue;
        }

        // composetime → originalarrivaltime; if parseable and older than
        // cutoff, skip. Unparseable/missing falls through (service.py L408-417:
        // "safer to double-process than to silently drop").
        let compose_time = msg
            .get("composetime")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| msg.get("originalarrivaltime").and_then(Value::as_str))
            .unwrap_or("");
        if !compose_time.is_empty() {
            if let Ok(ts) = DateTime::parse_from_rfc3339(compose_time) {
                if ts.with_timezone(&Utc) < cutoff {
                    continue;
                }
            }
        }

        let event = EventMessage {
            message_id: message_id.to_string(),
            conversation_id: conv_id.to_string(),
            content: msg
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            sender_mri: msg
                .get("from")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .or_else(|| msg.get("fromUserId").and_then(Value::as_str))
                .unwrap_or("")
                .to_string(),
            sender_name: msg
                .get("imdisplayname")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            compose_time: compose_time.to_string(),
            resource_type: "NewMessage".to_string(),
        };

        // Downstream MessageDedup handles overlap with live Trouter delivery.
        if msg_tx.send(event).await.is_err() {
            return Ok(count); // receiver gone → supervisor shutting down
        }
        count += 1;
    }
    Ok(count)
}
