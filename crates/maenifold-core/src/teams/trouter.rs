use std::time::{Duration, SystemTime, UNIX_EPOCH};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{net::TcpStream, sync::mpsc, time};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream, tungstenite::{client::IntoClientRequest, http::HeaderValue, Message as WsMessage}};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use crate::error::{MaenifoldError, Result};
use super::{EventMessage, TrouterFrame};

const DEFAULT_GATEWAY: &str = "go-msit.trouter.teams.microsoft.com";
const REGISTRAR_URL: &str = "https://teams.cloud.microsoft/registrar/prod/V2/registrations";
const ORIGIN: &str = "https://teams.cloud.microsoft";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HEARTBEAT: Duration = Duration::from_secs(30);
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(300);
const MAX_FRAME_SIZE: usize = 256 * 1024;
const HANDSHAKE_FRAMES: usize = 10;
type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct TrouterClient { gateway: String, token: String, cancel: CancellationToken, ws: Option<WsStream>, epid: Uuid, seq: u64 }

impl TrouterClient {
    pub fn new(token: String, cancel: CancellationToken) -> Self {
        Self { gateway: DEFAULT_GATEWAY.to_string(), token, cancel, ws: None, epid: Uuid::new_v4(), seq: 0 }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.epid = Uuid::new_v4();
        self.seq = 0;
        let mut request = build_connection_url(&self.gateway, &self.epid)
            .into_client_request()
            .map_err(|e| MaenifoldError::Message(format!("invalid trouter request: {e}")))?;
        request.headers_mut().insert("Origin", HeaderValue::from_static(ORIGIN));
        let (mut ws, _) = connect_async(request).await.map_err(|e| MaenifoldError::Message(format!("trouter connect failed: {e}")))?;
        let connect = next_text_frame_cancellable(&mut ws, CONNECT_TIMEOUT, &self.cancel).await?;
        if !matches!(parse_frame(&connect), TrouterFrame::Connect) {
            return Err(MaenifoldError::Message(format!("expected trouter connect frame, received {connect}")));
        }
        ws.send(WsMessage::Text(build_auth_frame(&self.token, &self.epid).into()))
            .await
            .map_err(|e| MaenifoldError::Message(format!("trouter auth send failed: {e}")))?;
        let surl = self.await_connected(&mut ws).await?;
        tokio::select! {
            result = register(&self.token, &surl, &self.epid) => result?,
            _ = self.cancel.cancelled() => {
                return Err(MaenifoldError::Message("trouter registration cancelled".to_string()));
            }
        }
        self.ws = Some(ws);
        Ok(())
    }

    pub async fn run(&mut self, msg_tx: mpsc::Sender<EventMessage>) -> Result<()> {
        let mut backoff = INITIAL_BACKOFF;
        loop {
            if self.cancel.is_cancelled() { self.close_ws().await; return Ok(()); }
            match self.connect().await {
                Ok(()) => {
                    backoff = INITIAL_BACKOFF;
                    let Some(ws) = self.ws.take() else {
                        return Err(MaenifoldError::Message("trouter socket missing after connect".into()));
                    };
                    if self.listen(ws, &msg_tx).await? { self.close_ws().await; return Ok(()); }
                }
                Err(error) => tracing::warn!("trouter connect failed: {error}"),
            }
            if !sleep_or_cancel(&self.cancel, backoff).await { self.close_ws().await; return Ok(()); }
            backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
        }
    }

    async fn await_connected(&mut self, ws: &mut WsStream) -> Result<String> {
        for _ in 0..HANDSHAKE_FRAMES {
            match parse_frame(&next_text_frame_cancellable(ws, CONNECT_TIMEOUT, &self.cancel).await?) {
                TrouterFrame::Event { name, args } if name == "trouter.connected" => {
                    return Ok(args.first().and_then(|v| v.get("surl")).and_then(Value::as_str).unwrap_or_default().to_string());
                }
                TrouterFrame::Event { name, args } if name == "trouter.message_loss" => {
                    send_event_ack(ws, &mut self.seq, "trouter.processed_message_loss", args).await?;
                }
                _ => {}
            }
        }
        Err(MaenifoldError::Message("did not receive trouter.connected during handshake".into()))
    }

    async fn listen(&mut self, mut ws: WsStream, msg_tx: &mpsc::Sender<EventMessage>) -> Result<bool> {
        let mut heartbeat = time::interval(HEARTBEAT);
        heartbeat.tick().await;
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => { let _ = ws.close(None).await; return Ok(true); }
                _ = heartbeat.tick() => {
                    self.seq += 1;
                    let ping = format!("5:{}+::{}", self.seq, json!({"name":"ping"}));
                    ws.send(WsMessage::Text(ping.into())).await.map_err(|e| MaenifoldError::Message(format!("trouter heartbeat failed: {e}")))?;
                }
                frame = ws.next() => {
                    let Some(frame) = frame else { return Ok(false); };
                    let raw = websocket_text(frame.map_err(|e| MaenifoldError::Message(format!("trouter read failed: {e}")))?)?;
                    if raw.is_empty() { continue; }
                    if raw.len() > MAX_FRAME_SIZE {
                        tracing::warn!(len = raw.len(), "trouter frame exceeds size limit, dropping");
                        continue;
                    }
                    if let Some(id) = message_id(&raw) { ack_message(&mut ws, &id).await?; }
                    match parse_frame(&raw) {
                        TrouterFrame::Event { name, args } if name == "trouter.message_loss" =>
                            send_event_ack(&mut ws, &mut self.seq, "trouter.processed_message_loss", args).await?,
                        TrouterFrame::Event { name, .. } if name == "trouter.reconnect" => return Ok(false),
                        TrouterFrame::Event { name, .. } if name == "ping" => {
                            self.seq += 1;
                            ws.send(WsMessage::Text(format!("6:::{}[\"pong\"]", self.seq).into()))
                                .await
                                .map_err(|e| MaenifoldError::Message(format!("trouter pong failed: {e}")))?;
                        }
                        TrouterFrame::Message(message) => msg_tx.send(message).await.map_err(|_| MaenifoldError::Message("trouter message receiver dropped".into()))?,
                        TrouterFrame::Ack(payload) | TrouterFrame::Unknown(payload) => tracing::debug!("trouter frame ignored: {payload}"),
                        TrouterFrame::Connect | TrouterFrame::Event { .. } => {}
                    }
                }
            }
        }
    }

    async fn close_ws(&mut self) { if let Some(mut ws) = self.ws.take() { let _ = ws.close(None).await; } }
}

pub fn parse_frame(raw: &str) -> TrouterFrame {
    if raw.starts_with("1::") { return TrouterFrame::Connect; }
    if let Some(payload) = raw.strip_prefix("6:::") { return TrouterFrame::Ack(payload.to_string()); }
    if let Some(payload) = raw.strip_prefix("3:::") {
        return serde_json::from_str::<Value>(payload).ok().and_then(parse_event_message).map(TrouterFrame::Message).unwrap_or_else(|| TrouterFrame::Unknown(raw.to_string()));
    }
    if raw.starts_with('5') {
        let Some(split) = raw.find("::") else { return TrouterFrame::Unknown(raw.to_string()); };
        let body = raw[split + 2..].strip_prefix(':').unwrap_or(&raw[split + 2..]);
        if let Ok(value) = serde_json::from_str::<Value>(body) {
            if let (Some(name), Some(args)) = (value.get("name").and_then(Value::as_str), value.get("args").and_then(Value::as_array)) {
                return TrouterFrame::Event { name: name.to_string(), args: args.clone() };
            }
        }
    }
    TrouterFrame::Unknown(raw.to_string())
}

pub fn build_connection_url(gateway: &str, epid: &Uuid) -> String {
    let host = gateway.trim_start_matches("wss://").trim_start_matches("ws://").trim_end_matches('/');
    let config = json!({"cv":"2026.03.01.1","ua":"TeamsCDL","hr":"","v":"1415/26020101120"}).to_string();
    let tc = urlencoding::encode(&config);
    format!("wss://{host}/v4/c?tc={tc}&timeout=40&epid={epid}&ccid=&dom=teams.cloud.microsoft&cor_id={}&con_num={}_0", Uuid::new_v4(), unix_millis())
}

pub fn build_auth_frame(token: &str, epid: &Uuid) -> String {
    let now = unix_millis() as i64;
    let auth = json!({"name":"user.authenticate","args":[{"headers":{"Authorization": bearer_token(token), "X-Ms-Test-User":"False", "X-MS-Migration":"True"},"connectparams":{"issuer":"","scae":"1","sig":"","sr":epid.to_string(),"sp":"","se":now + 600_000,"st":now}}]});
    format!("5:::{auth}")
}

async fn register(token: &str, surl: &str, epid: &Uuid) -> Result<()> {
    let response = reqwest::Client::new()
        .post(REGISTRAR_URL)
        .header("Authorization", bearer_token(token))
        .header("Content-Type", "application/json")
        .header("Origin", ORIGIN)
        .json(&json!({"clientDescription":{"appId":"TeamsCDLWebWorker","aesKey":"","languageId":"en-US","platform":"edge","templateKey":"TeamsCDLWebWorker_2.6","platformUIVersion":"1415/26020101120"},"registrationId":epid,"nodeId":"","transports":{"TROUTER":[{"context":"","path":surl,"ttl":3600}]}}))
        .send().await.map_err(|e| MaenifoldError::Message(format!("trouter register failed: {e}")))?;
    if response.status().is_success() || response.status().as_u16() == 202 { return Ok(()); }
    let status = response.status();
    let body = response.text().await.unwrap_or_else(|_| String::new());
    Err(MaenifoldError::Message(format!("trouter registrar returned {status}: {body}")))
}

async fn next_text_frame(ws: &mut WsStream, timeout: Duration) -> Result<String> {
    let next = time::timeout(timeout, ws.next()).await.map_err(|_| MaenifoldError::Message("trouter frame timeout".into()))?;
    let Some(frame) = next else { return Err(MaenifoldError::Message("trouter socket closed".into())); };
    let text = websocket_text(frame.map_err(|e| MaenifoldError::Message(format!("trouter read failed: {e}")))? )?;
    if text.len() > MAX_FRAME_SIZE {
        return Err(MaenifoldError::Message(format!("trouter frame exceeds size limit: {}", text.len())));
    }
    Ok(text)
}

async fn next_text_frame_cancellable(ws: &mut WsStream, timeout: Duration, cancel: &CancellationToken) -> Result<String> {
    tokio::select! {
        result = next_text_frame(ws, timeout) => result,
        _ = cancel.cancelled() => Err(MaenifoldError::Message("trouter connect cancelled".to_string())),
    }
}

fn websocket_text(frame: WsMessage) -> Result<String> {
    match frame {
        WsMessage::Text(text) => Ok(text.to_string()),
        WsMessage::Binary(bytes) => String::from_utf8(bytes.to_vec()).map_err(|e| MaenifoldError::Message(format!("invalid trouter binary frame: {e}"))),
        WsMessage::Close(_) => Err(MaenifoldError::Message("trouter socket closed".into())),
        _ => Ok(String::new()),
    }
}

async fn ack_message(ws: &mut WsStream, message_id: &str) -> Result<()> {
    let ack = json!({"id": message_id, "status": 200, "headers": {}, "body": ""});
    ws.send(WsMessage::Text(format!("3:::{ack}").into())).await.map_err(|e| MaenifoldError::Message(format!("trouter message ack failed: {e}")))
}

async fn send_event_ack(ws: &mut WsStream, seq: &mut u64, name: &str, args: Vec<Value>) -> Result<()> {
    *seq += 1;
    let payload = json!({"name": name, "args": args});
    ws.send(WsMessage::Text(format!("5:{}+::{payload}", *seq).into())).await.map_err(|e| MaenifoldError::Message(format!("trouter event ack failed: {e}")))
}

fn message_id(raw: &str) -> Option<String> {
    raw.strip_prefix("3:::").and_then(|payload| serde_json::from_str::<Value>(payload).ok()).and_then(|value| value.get("id").and_then(Value::as_str).map(ToOwned::to_owned))
}

fn parse_event_message(value: Value) -> Option<EventMessage> {
    let body = match value.get("body")? { Value::String(text) => serde_json::from_str::<Value>(text).ok()?, other => other.clone() };
    let resource = body.get("resource")?;
    Some(EventMessage {
        message_id: resource.get("id")?.as_str()?.to_string(),
        conversation_id: conversation_id(resource),
        content: resource.get("content").and_then(Value::as_str).unwrap_or_default().to_string(),
        sender_mri: sender_mri(resource),
        sender_name: resource.get("imdisplayname").or_else(|| resource.get("displayName")).and_then(Value::as_str).unwrap_or_default().to_string(),
        compose_time: resource.get("composetime").or_else(|| resource.get("composeTime")).and_then(Value::as_str).unwrap_or_default().to_string(),
        resource_type: body.get("resourceType").and_then(Value::as_str).unwrap_or_default().to_string(),
    }).filter(|message| message.resource_type == "NewMessage")
}

fn conversation_id(resource: &Value) -> String {
    resource.get("conversationLink").and_then(Value::as_str).and_then(|link| link.split("/conversations/").nth(1)).or_else(|| resource.get("to").and_then(Value::as_str)).unwrap_or_default().to_string()
}

fn sender_mri(resource: &Value) -> String {
    resource.get("from").and_then(Value::as_str).map(|value| value.rsplit("/contacts/").next().unwrap_or(value).to_string()).unwrap_or_default()
}

fn bearer_token(token: &str) -> String { let trimmed = token.trim(); if trimmed.starts_with("Bearer ") { trimmed.to_string() } else { format!("Bearer {trimmed}") } }

fn unix_millis() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }

async fn sleep_or_cancel(cancel: &CancellationToken, delay: Duration) -> bool {
    tokio::select! { _ = cancel.cancelled() => false, _ = time::sleep(delay) => true }
}
