use super::trouter_handle::ReRegisterHandle;
use super::trouter_protocol::{
    ack_message, build_auth_frame, build_connection_url, ensure_tls_provider, message_id,
    next_text_frame_cancellable, parse_frame, register, send_event_ack, sleep_or_cancel,
    websocket_text, TrouterConnected, WsStream, CONNECT_TIMEOUT, MAX_FRAME_SIZE, ORIGIN,
};
use super::{EventMessage, TrouterFrame, set_teams_connected, set_teams_connecting};
use crate::error::{MaenifoldError, Result};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::{mpsc, watch}, time};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message as WsMessage, client::IntoClientRequest, http::HeaderValue},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const DEFAULT_GATEWAY: &str = "go-msit.trouter.teams.microsoft.com";
const REGISTRAR_URL: &str = "https://teams.cloud.microsoft/registrar/prod/V2/registrations";
const HEARTBEAT: Duration = Duration::from_secs(30);
const INITIAL_BACKOFF: Duration = Duration::from_secs(5);
const MAX_BACKOFF: Duration = Duration::from_secs(300);
const HANDSHAKE_FRAMES: usize = 10;
/// Re-register every 45 minutes; Trouter TTL is 3600s (1h). Matches agency-cowork reference.
const REREGISTER_INTERVAL: Duration = Duration::from_secs(45 * 60);

#[derive(Debug, Clone)]
pub struct TrouterClientConfig {
    pub gateway: String,
    pub registrar_url: String,
    pub app_id: String,
    pub heartbeat: Duration,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for TrouterClientConfig {
    fn default() -> Self {
        Self {
            gateway: DEFAULT_GATEWAY.to_string(),
            registrar_url: REGISTRAR_URL.to_string(),
            app_id: "TeamsCDLWebWorker".to_string(),
            heartbeat: HEARTBEAT,
            initial_backoff: INITIAL_BACKOFF,
            max_backoff: MAX_BACKOFF,
        }
    }
}

pub struct TrouterClient {
    config: TrouterClientConfig,
    token_rx: watch::Receiver<String>,
    cancel: CancellationToken,
    ws: Option<WsStream>,
    epid: Uuid,
    seq: u64,
    connected: Arc<RwLock<Option<TrouterConnected>>>,
}

/// Cloneable handle for re-registering with Trouter using a fresh token,
/// without owning `&mut TrouterClient`. Used by the token-refresh loop
/// (T-238.50). See `trouter_handle::ReRegisterHandle`.

impl TrouterClient {
    pub fn with_config(
        token_rx: watch::Receiver<String>,
        cancel: CancellationToken,
        config: TrouterClientConfig,
    ) -> Self {
        Self {
            config,
            token_rx,
            cancel,
            ws: None,
            epid: Uuid::new_v4(),
            seq: 0,
            connected: Arc::new(RwLock::new(None)),
        }
    }

    /// Cloneable handle the refresh loop uses to immediately re-register after
    /// acquiring a fresh token.
    pub fn re_register_handle(&self) -> ReRegisterHandle {
        ReRegisterHandle {
            config: self.config.clone(),
            epid: self.epid,
            connected: Arc::clone(&self.connected),
        }
    }

    fn current_token(&self) -> String {
        self.token_rx.borrow().clone()
    }

    pub async fn connect(&mut self) -> Result<()> {
        ensure_tls_provider();
        self.epid = Uuid::new_v4();
        self.seq = 0;
        *self.connected.write() = None;
        let mut request = build_connection_url(&self.config.gateway, &self.epid)
            .into_client_request()
            .map_err(|e| MaenifoldError::Message(format!("invalid trouter request: {e}")))?;
        request
            .headers_mut()
            .insert("Origin", HeaderValue::from_static(ORIGIN));
        tracing::info!("trouter connecting to {}", self.config.gateway);
        let (mut ws, _) = connect_async(request)
            .await
            .map_err(|e| MaenifoldError::Message(format!("trouter connect failed: {e}")))?;
        let connect = next_text_frame_cancellable(&mut ws, CONNECT_TIMEOUT, &self.cancel).await?;
        tracing::debug!(frame = %connect, "trouter recv");
        if !matches!(parse_frame(&connect), TrouterFrame::Connect) {
            return Err(MaenifoldError::Message(format!(
                "expected trouter connect frame, received {connect}"
            )));
        }
        let token = self.current_token();
        let auth = build_auth_frame(&token, &self.epid);
        tracing::debug!(len = auth.len(), "trouter sending auth");
        ws.send(WsMessage::Text(auth.into()))
            .await
            .map_err(|e| MaenifoldError::Message(format!("trouter auth send failed: {e}")))?;
        let connected = self.await_connected(&mut ws).await?;
        tracing::info!(surl_len = connected.surl.len(), "trouter.connected received");
        tokio::select! {
            result = register(&token, &connected, &self.epid, &self.config) => {
                tracing::info!("trouter registrar ok");
                result?
            },
            _ = self.cancel.cancelled() => {
                return Err(MaenifoldError::Message("trouter registration cancelled".to_string()));
            }
        }
        *self.connected.write() = Some(connected);
        self.ws = Some(ws);
        Ok(())
    }

    pub async fn run(&mut self, msg_tx: mpsc::Sender<EventMessage>) -> Result<()> {
        let mut backoff = self.config.initial_backoff;
        loop {
            if self.cancel.is_cancelled() {
                self.close_ws().await;
                return Ok(());
            }
            match self.connect().await {
                Ok(()) => {
                    set_teams_connected();
                    backoff = self.config.initial_backoff;
                    let Some(ws) = self.ws.take() else {
                        return Err(MaenifoldError::Message(
                            "trouter socket missing after connect".into(),
                        ));
                    };
                    if self.listen(ws, &msg_tx).await? {
                        self.close_ws().await;
                        return Ok(());
                    }
                    *self.connected.write() = None;
                    set_teams_connecting();
                }
                Err(error) => tracing::warn!("trouter connect failed: {error}"),
            }
            if !sleep_or_cancel(&self.cancel, backoff).await {
                self.close_ws().await;
                return Ok(());
            }
            backoff = backoff.saturating_mul(2).min(self.config.max_backoff);
        }
    }

    async fn await_connected(&mut self, ws: &mut WsStream) -> Result<TrouterConnected> {
        for _ in 0..HANDSHAKE_FRAMES {
            match parse_frame(
                &next_text_frame_cancellable(ws, CONNECT_TIMEOUT, &self.cancel).await?,
            ) {
                TrouterFrame::Event { name, args } if name == "trouter.connected" => {
                    let first = args.first();
                    return Ok(TrouterConnected {
                        surl: first
                            .and_then(|v| v.get("surl"))
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        registrar_url: first
                            .and_then(|v| v.get("registrarUrl"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                    });
                }
                TrouterFrame::Event { name, args } if name == "trouter.message_loss" => {
                    send_event_ack(ws, &mut self.seq, "trouter.processed_message_loss", args)
                        .await?;
                }
                _ => {}
            }
        }
        Err(MaenifoldError::Message(
            "did not receive trouter.connected during handshake".into(),
        ))
    }

    async fn listen(
        &mut self,
        mut ws: WsStream,
        msg_tx: &mpsc::Sender<EventMessage>,
    ) -> Result<bool> {
        let mut heartbeat = time::interval(self.config.heartbeat);
        heartbeat.tick().await;
        let mut reregister = time::interval(REREGISTER_INTERVAL);
        reregister.tick().await;
        let mut reregister_handle: Option<tokio::task::JoinHandle<Result<()>>> = None;
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    if let Some(h) = reregister_handle.take() { h.abort(); }
                    let _ = ws.close(None).await;
                    return Ok(true);
                }
                _ = heartbeat.tick() => {
                    self.seq += 1;
                    let ping = format!("5:{}+::{}", self.seq, json!({"name":"ping"}));
                    ws.send(WsMessage::Text(ping.into())).await.map_err(|e| MaenifoldError::Message(format!("trouter heartbeat failed: {e}")))?;
                }
                _ = reregister.tick() => {
                    if reregister_handle.as_ref().is_some_and(|h| h.is_finished()) {
                        if let Some(h) = reregister_handle.take() {
                            match h.await {
                                Ok(Ok(())) => tracing::info!("trouter re-registration ok"),
                                Ok(Err(e)) => tracing::warn!(error = %e, "trouter re-registration failed"),
                                Err(e) => tracing::warn!(error = %e, "trouter re-registration task join failed"),
                            }
                        }
                    } else if reregister_handle.is_some() {
                        tracing::warn!("trouter re-registration still in flight, skipping tick");
                    }
                    if reregister_handle.is_none() {
                        let handle = self.re_register_handle();
                        let token = self.current_token();
                        let cancel = self.cancel.clone();
                        reregister_handle = Some(tokio::spawn(async move {
                            tokio::select! {
                                result = handle.re_register(&token) => result,
                                _ = cancel.cancelled() => Err(MaenifoldError::Message("trouter re-registration cancelled".into())),
                            }
                        }));
                    }
                }
                frame = ws.next() => {
                    let Some(frame) = frame else {
                        tracing::warn!("trouter stream ended");
                        if let Some(h) = reregister_handle.take() { h.abort(); }
                        return Ok(false);
                    };
                    let raw = websocket_text(frame.map_err(|e| MaenifoldError::Message(format!("trouter read failed: {e}")))?)?;
                    if raw.is_empty() { continue; }
                    tracing::debug!(len = raw.len(), prefix = &raw[..raw.len().min(80)], "trouter frame");
                    if raw.len() > MAX_FRAME_SIZE {
                        tracing::warn!(len = raw.len(), "trouter frame exceeds size limit, dropping");
                        continue;
                    }
                    if let Some(id) = message_id(&raw) { ack_message(&mut ws, &id).await?; }
                    match parse_frame(&raw) {
                        TrouterFrame::Event { name, args } if name == "trouter.message_loss" =>
                            send_event_ack(&mut ws, &mut self.seq, "trouter.processed_message_loss", args).await?,
                        TrouterFrame::Event { name, .. } if name == "trouter.reconnect" => {
                            if let Some(h) = reregister_handle.take() { h.abort(); }
                            return Ok(false);
                        }
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

    async fn close_ws(&mut self) {
        *self.connected.write() = None;
        if let Some(mut ws) = self.ws.take() {
            let _ = ws.close(None).await;
        }
    }
}
