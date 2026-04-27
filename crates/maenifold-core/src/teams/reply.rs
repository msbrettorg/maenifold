use chrono::{SecondsFormat, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{MaenifoldError, Result};

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
    let response = reqwest::Client::new()
        .post(reply_url(&request.chatsvc_region, &request.conversation_id))
        .header("Authorization", bearer_token(&request.token))
        .header("Content-Type", "application/json")
        .header("Origin", TEAMS_CLOUD_ORIGIN)
        .header("X-MS-Migration", "True")
        .header("X-Ms-Test-User", "False")
        .header("clientinfo", client_info())
        .json(&reply_payload(&request))
        .send()
        .await
        .map_err(|error| MaenifoldError::Message(format!("chatsvc reply failed: {error}")))?;

    if matches!(response.status().as_u16(), 200 | 201 | 202) {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(MaenifoldError::Message(format!(
        "chatsvc reply returned {status}: {}",
        body.chars().take(500).collect::<String>()
    )))
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
