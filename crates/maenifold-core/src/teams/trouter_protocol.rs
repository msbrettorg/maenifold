use super::types::{EventMessage, TrouterFrame};
use super::trouter::TrouterClientConfig;
use crate::error::{MaenifoldError, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::{net::TcpStream, time};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::Message as WsMessage,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(crate) const ORIGIN: &str = "https://teams.cloud.microsoft";
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const MAX_FRAME_SIZE: usize = 256 * 1024;

pub(crate) type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone)]
pub struct TrouterConnected {
    pub surl: String,
    pub registrar_url: Option<String>,
}

pub fn parse_frame(raw: &str) -> TrouterFrame {
    if raw.starts_with("1::") {
        return TrouterFrame::Connect;
    }
    if let Some(payload) = raw.strip_prefix("6:::") {
        return TrouterFrame::Ack(payload.to_string());
    }
    if let Some(payload) = raw.strip_prefix("3:::") {
        return serde_json::from_str::<Value>(payload)
            .ok()
            .and_then(parse_event_message)
            .map(TrouterFrame::Message)
            .unwrap_or_else(|| TrouterFrame::Unknown(raw.to_string()));
    }
    if raw.starts_with('5') {
        let Some(split) = raw.find("::") else {
            return TrouterFrame::Unknown(raw.to_string());
        };
        let body = raw[split + 2..]
            .strip_prefix(':')
            .unwrap_or(&raw[split + 2..]);
        if let Ok(value) = serde_json::from_str::<Value>(body) {
            if let (Some(name), Some(args)) = (
                value.get("name").and_then(Value::as_str),
                value.get("args").and_then(Value::as_array),
            ) {
                return TrouterFrame::Event {
                    name: name.to_string(),
                    args: args.clone(),
                };
            }
        }
    }
    TrouterFrame::Unknown(raw.to_string())
}

pub fn build_connection_url(gateway: &str, epid: &Uuid) -> String {
    let host = gateway
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .trim_end_matches('/');
    let config =
        json!({"cv":"2026.03.01.1","ua":"TeamsCDL","hr":"","v":"1415/26020101120"}).to_string();
    let tc = urlencoding::encode(&config);
    format!(
        "wss://{host}/v4/c?tc={tc}&timeout=40&epid={epid}&ccid=&dom=teams.cloud.microsoft&cor_id={}&con_num={}_0",
        Uuid::new_v4(),
        unix_millis()
    )
}

pub fn build_auth_frame(token: &str, epid: &Uuid) -> String {
    let now = unix_millis() as i64;
    let se = (now + 600_000).to_string();
    let st = now.to_string();
    let auth = json!({"name":"user.authenticate","args":[{"headers":{"X-Ms-Test-User":"False","Authorization": bearer_token(token),"X-MS-Migration":"True"},"connectparams":{"issuer":"","scae":"1","sig":"","sr":epid.to_string(),"sp":"","se":se,"st":st}}]});
    format!("5:::{auth}")
}

pub(crate) const REGISTER_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn register(
    token: &str,
    connected: &TrouterConnected,
    epid: &Uuid,
    config: &TrouterClientConfig,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(REGISTER_HTTP_TIMEOUT)
        .build()
        .map_err(|e| MaenifoldError::Message(format!("trouter registrar client init failed: {e}")))?;
    let payload = json!({"clientDescription":{"appId":"TeamsCDLWebWorker","aesKey":"","languageId":"en-US","platform":"edge","templateKey":"TeamsCDLWebWorker_2.6","platformUIVersion":"1415/26020101120"},"registrationId":epid,"nodeId":"","transports":{"TROUTER":[{"context":"","path":connected.surl,"ttl":3600}]}});
    let mut urls = Vec::new();
    if let Some(server_url) = connected.registrar_url.as_ref() {
        urls.push(server_url.clone());
    }
    if !urls.iter().any(|url| url == &config.registrar_url) {
        urls.push(config.registrar_url.clone());
    }

    let mut any_success = false;
    for url in &urls {
        let response = client
            .post(url)
            .header("Authorization", bearer_token(token))
            .header("Content-Type", "application/json")
            .header("Origin", ORIGIN)
            .json(&payload)
            .send()
            .await;
        match response {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 202 => {
                tracing::info!(url = &url[..url.len().min(60)], status = %resp.status(), "registrar ok");
                any_success = true;
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!(url = &url[..url.len().min(60)], %status, body = &body[..body.len().min(200)], "registrar rejected");
            }
            Err(e) => {
                tracing::warn!(url = &url[..url.len().min(60)], error = %e, "registrar request failed");
            }
        }
    }
    if any_success {
        Ok(())
    } else {
        Err(MaenifoldError::Message(format!(
            "trouter registration failed on all {} URLs",
            urls.len()
        )))
    }
}

async fn next_text_frame(ws: &mut WsStream, timeout: Duration) -> Result<String> {
    let next = time::timeout(timeout, ws.next())
        .await
        .map_err(|_| MaenifoldError::Message("trouter frame timeout".into()))?;
    let Some(frame) = next else {
        return Err(MaenifoldError::Message("trouter socket closed".into()));
    };
    let text = websocket_text(
        frame.map_err(|e| MaenifoldError::Message(format!("trouter read failed: {e}")))?,
    )?;
    if text.len() > MAX_FRAME_SIZE {
        return Err(MaenifoldError::Message(format!(
            "trouter frame exceeds size limit: {}",
            text.len()
        )));
    }
    Ok(text)
}

pub(crate) async fn next_text_frame_cancellable(
    ws: &mut WsStream,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<String> {
    tokio::select! {
        result = next_text_frame(ws, timeout) => result,
        _ = cancel.cancelled() => Err(MaenifoldError::Message("trouter connect cancelled".to_string())),
    }
}

pub(crate) fn websocket_text(frame: WsMessage) -> Result<String> {
    match frame {
        WsMessage::Text(text) => Ok(text.to_string()),
        WsMessage::Binary(bytes) => String::from_utf8(bytes.to_vec())
            .map_err(|e| MaenifoldError::Message(format!("invalid trouter binary frame: {e}"))),
        WsMessage::Close(_) => Err(MaenifoldError::Message("trouter socket closed".into())),
        _ => Ok(String::new()),
    }
}

pub(crate) async fn ack_message(ws: &mut WsStream, message_id: &str) -> Result<()> {
    let ack = json!({"id": message_id, "status": 200, "headers": {}, "body": ""});
    ws.send(WsMessage::Text(format!("3:::{ack}").into()))
        .await
        .map_err(|e| MaenifoldError::Message(format!("trouter message ack failed: {e}")))
}

pub(crate) async fn send_event_ack(
    ws: &mut WsStream,
    seq: &mut u64,
    name: &str,
    args: Vec<Value>,
) -> Result<()> {
    *seq += 1;
    let payload = json!({"name": name, "args": args});
    ws.send(WsMessage::Text(format!("5:{}+::{payload}", *seq).into()))
        .await
        .map_err(|e| MaenifoldError::Message(format!("trouter event ack failed: {e}")))
}

pub(crate) fn message_id(raw: &str) -> Option<String> {
    raw.strip_prefix("3:::")
        .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
        .and_then(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn parse_event_message(value: Value) -> Option<EventMessage> {
    let body = match value.get("body")? {
        Value::String(text) => serde_json::from_str::<Value>(text).ok()?,
        other => other.clone(),
    };
    let resource = body.get("resource")?;
    Some(EventMessage {
        message_id: resource.get("id")?.as_str()?.to_string(),
        conversation_id: conversation_id(resource),
        content: resource
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        sender_mri: sender_mri(resource),
        sender_name: resource
            .get("imdisplayname")
            .or_else(|| resource.get("displayName"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        compose_time: resource
            .get("composetime")
            .or_else(|| resource.get("composeTime"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        resource_type: body
            .get("resourceType")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
    .filter(|message| message.resource_type == "NewMessage")
}

fn conversation_id(resource: &Value) -> String {
    resource
        .get("conversationLink")
        .and_then(Value::as_str)
        .and_then(|link| link.split("/conversations/").nth(1))
        .or_else(|| resource.get("to").and_then(Value::as_str))
        .unwrap_or_default()
        .to_string()
}

fn sender_mri(resource: &Value) -> String {
    resource
        .get("from")
        .and_then(Value::as_str)
        .map(|value| {
            value
                .rsplit("/contacts/")
                .next()
                .unwrap_or(value)
                .to_string()
        })
        .unwrap_or_default()
}

pub(crate) fn bearer_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.starts_with("Bearer ") {
        trimmed.to_string()
    } else {
        format!("Bearer {trimmed}")
    }
}

pub(crate) fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) async fn sleep_or_cancel(cancel: &CancellationToken, delay: Duration) -> bool {
    tokio::select! { _ = cancel.cancelled() => false, _ = time::sleep(delay) => true }
}

pub(crate) fn ensure_tls_provider() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}
