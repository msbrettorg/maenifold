use chrono::{SecondsFormat, Utc};
use serde_json::{Value, json};
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{MaenifoldError, Result};

use super::region::extract_region_from_url;

const TEAMS_CLOUD_ORIGIN: &str = "https://teams.cloud.microsoft";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamsReplyRequest {
    pub token: String,
    pub conversation_id: String,
    pub body: String,
    pub from_mri: String,
    pub chatsvc_region: String,
    pub reply_prefix: String,
}

pub async fn send_reply(request: TeamsReplyRequest) -> Result<()> {
    let client = reqwest::Client::new();
    let response = post_once(&client, &request, &request.chatsvc_region).await?;
    let status = response.status().as_u16();

    match status {
        201 | 202 => Ok(()),
        200 => {
            // 200 from the wrong region returns a "messages" echo (conversation
            // history) instead of delivering. Detect and auto-correct.
            let body_text = response.text().await.unwrap_or_default();
            let parsed: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
            let is_echo = parsed.is_array()
                || parsed
                    .as_object()
                    .is_some_and(|o| o.contains_key("messages"));
            if !is_echo {
                return Ok(());
            }
            if let Some(new_region) = extract_region_from_url(&body_text) {
                if new_region != request.chatsvc_region.to_lowercase() {
                    warn!(
                        "chatsvc region mismatch: sent to '{}' but conversation lives in '{}' — retrying",
                        request.chatsvc_region, new_region
                    );
                    return retry_once(&client, &request, &new_region).await;
                }
            }
            Err(MaenifoldError::Message(format!(
                "chatsvc returned 200 echo with no alternate region (region={}, bodyLen={})",
                request.chatsvc_region,
                body_text.len()
            )))
        }
        301 | 302 | 404 => {
            let location = response
                .headers()
                .get("Location")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
                .unwrap_or_default();
            let body_text = response.text().await.unwrap_or_default();
            if !location.is_empty() || body_text.to_lowercase().contains("different cloud") {
                let hint = if location.is_empty() {
                    body_text.as_str()
                } else {
                    location.as_str()
                };
                if let Some(new_region) = extract_region_from_url(hint) {
                    if new_region != request.chatsvc_region.to_lowercase() {
                        info!(
                            "chatsvc auto-discovered region '{new_region}' (status {status}) — retrying"
                        );
                        return retry_once(&client, &request, &new_region).await;
                    }
                }
            }
            Err(MaenifoldError::Message(format!(
                "chatsvc reply returned {status}: {}",
                body_text.chars().take(500).collect::<String>()
            )))
        }
        _ => {
            let body = response.text().await.unwrap_or_default();
            Err(MaenifoldError::Message(format!(
                "chatsvc reply returned {status}: {}",
                body.chars().take(500).collect::<String>()
            )))
        }
    }
}

/// Single retry after region recovery; only 201/202 counts as delivered.
async fn retry_once(
    client: &reqwest::Client,
    request: &TeamsReplyRequest,
    region: &str,
) -> Result<()> {
    let response = post_once(client, request, region).await?;
    let status = response.status().as_u16();
    if matches!(status, 201 | 202) {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    Err(MaenifoldError::Message(format!(
        "chatsvc retry returned {status} (region={region}): {}",
        body.chars().take(200).collect::<String>()
    )))
}

/// Build and send the chatsvc POST against `region`. Shared by initial send and
/// the single region-recovery retry to keep header/payload construction in one place.
async fn post_once(
    client: &reqwest::Client,
    request: &TeamsReplyRequest,
    region: &str,
) -> Result<reqwest::Response> {
    let mut scoped = request.clone();
    scoped.chatsvc_region = region.to_string();
    client
        .post(reply_url(region, &scoped.conversation_id))
        .header("Authorization", bearer_token(&scoped.token))
        .header("Content-Type", "application/json")
        .header("Origin", TEAMS_CLOUD_ORIGIN)
        .header("X-MS-Migration", "True")
        .header("X-Ms-Test-User", "False")
        .header("clientinfo", client_info())
        .json(&reply_payload(&scoped))
        .send()
        .await
        .map_err(|error| MaenifoldError::Message(format!("chatsvc reply failed: {error}")))
}

pub fn reply_url(region: &str, conversation_id: &str) -> String {
    let encoded = urlencoding::encode(conversation_id);
    format!(
        "{TEAMS_CLOUD_ORIGIN}/api/chatsvc/{region}/v1/users/ME/conversations/{encoded}/messages"
    )
}

pub fn reply_payload(request: &TeamsReplyRequest) -> Value {
    let encoded = urlencoding::encode(&request.conversation_id);
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    json!({
        "id": "-1",
        "type": "Message",
        "conversationid": request.conversation_id,
        "conversationLink": format!("{TEAMS_CLOUD_ORIGIN}/api/chatsvc/{}/v1/users/ME/conversations/{encoded}", request.chatsvc_region),
        "from": request.from_mri,
        "fromUserId": request.from_mri,
        "composetime": now,
        "originalarrivaltime": now,
        "content": html_content(&prefixed_body(&request.body, &request.reply_prefix)),
        "messagetype": "RichText/Html",
        "contenttype": "Text",
        "imdisplayname": display_name_from_prefix(&request.reply_prefix),
        "clientmessageid": Uuid::new_v4().as_u128().to_string().chars().take(19).collect::<String>(),
        "callId": "",
        "state": 0,
        "version": "0",
        "amsreferences": [],
        "properties": {
            "importance": "",
            "subject": "",
            "title": "",
            "cards": "[]",
            "links": "[]",
            "mentions": "[]",
            "onbehalfof": null,
            "files": "[]",
            "policyViolation": null,
            "formatVariant": "TEAMS",
        },
        "crossPostChannels": [],
    })
}

fn prefixed_body(body: &str, prefix: &str) -> String {
    if prefix.is_empty() || body.starts_with(prefix) {
        body.to_string()
    } else {
        format!("{prefix}{body}")
    }
}

fn html_content(body: &str) -> String {
    let escaped = body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "<br/>");
    if escaped.starts_with('<') {
        escaped
    } else {
        format!("<p>{escaped}</p>")
    }
}

fn display_name_from_prefix(prefix: &str) -> String {
    let display_name = prefix.trim_end_matches([':', ' ']);
    if display_name.is_empty() {
        "maenifold".to_string()
    } else {
        display_name.to_string()
    }
}

fn bearer_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.starts_with("Bearer ") {
        trimmed.to_string()
    } else {
        format!("Bearer {trimmed}")
    }
}

fn client_info() -> &'static str {
    "os=macos; osVer=14; proc=arm64; lcid=en-us; deviceType=1; country=us; clientName=skypeteams; clientVer=1415/26020101120; utcOffset=0; timezone=UTC"
}

#[cfg(test)]
mod tests {
    use super::{TeamsReplyRequest, reply_payload, reply_url};

    #[test]
    fn reply_payload_matches_chatsvc_shape() {
        let payload = reply_payload(&TeamsReplyRequest {
            token: "token".to_string(),
            conversation_id: "48:notes".to_string(),
            body: "hello\nworld".to_string(),
            from_mri: "8:orgid:self".to_string(),
            chatsvc_region: "amer".to_string(),
            reply_prefix: "maenifold: ".to_string(),
        });

        assert_eq!(payload["type"], "Message");
        assert_eq!(payload["from"], "8:orgid:self");
        assert_eq!(payload["fromUserId"], "8:orgid:self");
        assert_eq!(payload["messagetype"], "RichText/Html");
        assert_eq!(payload["content"], "<p>maenifold: hello<br/>world</p>");
        assert_eq!(payload["properties"]["formatVariant"], "TEAMS");
    }

    #[test]
    fn reply_url_encodes_conversation_id() {
        assert_eq!(
            reply_url("amer", "19:thread with spaces"),
            "https://teams.cloud.microsoft/api/chatsvc/amer/v1/users/ME/conversations/19%3Athread%20with%20spaces/messages"
        );
    }
}
