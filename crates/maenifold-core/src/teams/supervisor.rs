use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use chrono::Utc;
use serde_json::Value;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::error::Result;

use super::{
    EventMessage, MessageHandler, MessageHandlerConfig, TokenInfo, TrouterClient,
    TrouterClientConfig, runtime::update_teams_runtime_state, set_teams_connecting,
    set_teams_disabled, set_teams_error, teams_runtime_snapshot,
    token_provider::TeamsTokenProvider, token_refresh::spawn_refresh_loop,
};

pub type DispatchFn = Arc<dyn Fn(Value) -> std::result::Result<String, String> + Send + Sync>;
pub type RunnerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
/// Runner factory: receives a pre-built `TrouterClient` (so the supervisor
/// can pre-extract a `ReRegisterHandle` for the refresh loop) along with the
/// outbound message channel.
pub type RunnerFn = Arc<
    dyn Fn(TrouterClient, mpsc::Sender<EventMessage>) -> RunnerFuture + Send + Sync,
>;

const MESSAGE_CHANNEL_CAPACITY: usize = 64;

pub struct TrouterSupervisor {
    cancel: CancellationToken,
    join_handle: Option<JoinHandle<()>>,
}

impl TrouterSupervisor {
    pub fn start(
        initial_token: TokenInfo,
        provider: Arc<TeamsTokenProvider>,
        fallback_refresh: Duration,
        trouter_config: TrouterClientConfig,
        handler_config: MessageHandlerConfig,
        dispatch_agent: Option<String>,
        dispatch_fn: DispatchFn,
    ) -> Self {
        Self::start_with_runner(
            initial_token,
            provider,
            fallback_refresh,
            trouter_config,
            handler_config,
            dispatch_agent,
            dispatch_fn,
            default_runner(),
        )
    }

    pub fn start_with_runner(
        initial_token: TokenInfo,
        provider: Arc<TeamsTokenProvider>,
        fallback_refresh: Duration,
        trouter_config: TrouterClientConfig,
        handler_config: MessageHandlerConfig,
        dispatch_agent: Option<String>,
        dispatch_fn: DispatchFn,
        runner_fn: RunnerFn,
    ) -> Self {
        let cancel = CancellationToken::new();
        let task_cancel = cancel.clone();
        let join_handle = tokio::spawn(async move {
            run_supervisor(
                initial_token,
                provider,
                fallback_refresh,
                trouter_config,
                handler_config,
                dispatch_agent,
                dispatch_fn,
                runner_fn,
                task_cancel,
            )
            .await;
        });

        Self {
            cancel,
            join_handle: Some(join_handle),
        }
    }

    pub async fn stop(&mut self) {
        self.cancel.cancel();
        if let Some(join_handle) = self.join_handle.take() {
            if let Err(error) = join_handle.await {
                set_teams_error(format!("Teams Trouter supervisor task failed: {error}"));
            }
        }
    }
}

pub fn default_dispatch_fn() -> DispatchFn {
    Arc::new(|payload| {
        let agent = payload
            .get("agent")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let prompt = payload
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let adapter = jockey::acp::build_stdio_adapter(&agent)
            .map_err(|e| format!("Agent setup failed: {e}"))?
            .ok_or_else(|| format!("agent '{agent}' is not configured"))?;

        let root = jockey::acp::config_root();
        let cwd = root.join("memory").to_string_lossy().to_string();

        let (event_tx, _) = tokio::sync::mpsc::channel::<jockey::acp::AcpEvent>(64);
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();

        jockey::acp::worker_tx()
            .send(jockey::acp::WorkerMsg::Execute {
                runtime_key: adapter.runtime_key,
                binary: adapter.binary,
                args: adapter.args,
                env: adapter.env,
                role_name: agent,
                app_session_id: TEAMS_SESSION_ID.to_string(),
                prompt,
                context: vec![],
                cwd,
                delta_tx: event_tx,
                result_tx,
                auto_approve: false,
                mcp_servers: vec![],
                client_capabilities: agent_client_protocol::ClientCapabilities::default(),
                role_mode: None,
                role_config_options: vec![],
                resume_session_id: None,
            })
            .map_err(|_| "ACP worker unavailable".to_string())?;

        match result_rx.blocking_recv() {
            Ok(Ok(success)) => Ok(success.output),
            Ok(Err(error)) => Err(error.message),
            Err(_) => Err("ACP worker dropped result channel".to_string()),
        }
    })
}

pub const TEAMS_SESSION_ID: &str = "teams-monitor";

pub fn prewarm_teams_session(agent: &str) {
    let Ok(Some(adapter)) = jockey::acp::build_stdio_adapter(agent) else {
        tracing::warn!(agent, "cannot prewarm teams session: agent not configured");
        return;
    };
    let root = jockey::acp::config_root();
    let cwd = root.join("memory").to_string_lossy().to_string();

    if jockey::acp::worker_tx()
        .send(jockey::acp::WorkerMsg::Prewarm {
            runtime_key: adapter.runtime_key,
            binary: adapter.binary,
            args: adapter.args,
            env: adapter.env,
            client_capabilities: agent_client_protocol::ClientCapabilities::default(),
            role_name: agent.to_string(),
            app_session_id: TEAMS_SESSION_ID.to_string(),
            cwd,
            auto_approve: false,
            mcp_servers: vec![],
            role_mode: None,
            role_config_options: vec![],
            delta_tx: None,
            result_tx: None,
            resume_session_id: None,
        })
        .is_err()
    {
        tracing::warn!("ACP worker unavailable for teams session prewarm");
    } else {
        tracing::info!(agent, "teams ACP session prewarmed");
    }
}

fn default_runner() -> RunnerFn {
    Arc::new(|mut client, msg_tx| Box::pin(async move { client.run(msg_tx).await }))
}

#[allow(clippy::too_many_arguments)]
async fn run_supervisor(
    initial_token: TokenInfo,
    provider: Arc<TeamsTokenProvider>,
    fallback_refresh: Duration,
    trouter_config: TrouterClientConfig,
    handler_config: MessageHandlerConfig,
    dispatch_agent: Option<String>,
    dispatch_fn: DispatchFn,
    runner_fn: RunnerFn,
    cancel: CancellationToken,
) {
    set_teams_connecting();

    let (msg_tx, msg_rx) = mpsc::channel::<EventMessage>(MESSAGE_CHANNEL_CAPACITY);
    let (token_tx, token_rx) = watch::channel(initial_token.access_token.clone());

    let client = TrouterClient::with_config(token_rx, cancel.child_token(), trouter_config);
    let re_register = client.re_register_handle();

    let refresh_join = spawn_refresh_loop(
        initial_token,
        provider,
        fallback_refresh,
        token_tx,
        re_register,
        cancel.child_token(),
    );

    let runner_handle = tokio::spawn(async move { runner_fn(client, msg_tx).await });
    let processor_handle = tokio::spawn(process_messages(
        msg_rx,
        handler_config,
        dispatch_agent,
        dispatch_fn,
    ));

    let runner_result = match runner_handle.await {
        Ok(result) => result,
        Err(error) => Err(crate::error::MaenifoldError::Message(format!(
            "Teams Trouter runner task failed: {error}"
        ))),
    };

    cancel.cancel();
    let _ = refresh_join.await;

    if let Err(error) = processor_handle.await {
        set_teams_error(format!("Teams message processor task failed: {error}"));
        return;
    }

    match runner_result {
        Ok(()) => set_teams_disabled(),
        Err(error) => set_teams_error(error.to_string()),
    }
}

async fn process_messages(
    mut msg_rx: mpsc::Receiver<EventMessage>,
    handler_config: MessageHandlerConfig,
    _dispatch_agent: Option<String>,
    _dispatch_fn: DispatchFn,
) {
    let mut handler = MessageHandler::new(handler_config.clone());

    while let Some(message) = msg_rx.recv().await {
        record_message_received();
        ensure_connected_state();

        let Some(result) = handler.handle(&message) else {
            continue;
        };

        super::push_teams_prompt(super::TeamsPrompt {
            prompt: result.prompt,
            conversation_id: result.message.conversation_id,
            sender_name: result.message.sender_name,
            matched_keyword: result.matched_keyword,
        });
    }
}

fn record_message_received() {
    update_teams_runtime_state(|state| {
        state.last_message_at = Some(Utc::now().to_rfc3339());
    });
}

fn ensure_connected_state() {
    let snapshot = teams_runtime_snapshot();
    if snapshot.connection_state != "connected" {
        update_teams_runtime_state(|state| {
            state.connection_state = "connected".to_string();
            state.last_error = None;
        });
    }
}
