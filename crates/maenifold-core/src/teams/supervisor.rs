use std::{future::Future, pin::Pin, sync::Arc};

use chrono::Utc;
use serde_json::Value;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    error::Result,
    registry::ToolRegistry,
    util::json::OutputFormat,
};

use super::{
    runtime::update_teams_runtime_state, EventMessage, MessageHandler, MessageHandlerConfig,
    TeamsDispatchRequest, TrouterClient, set_teams_authenticated, set_teams_connecting,
    set_teams_disabled, set_teams_error, teams_runtime_snapshot,
};

pub type DispatchFn = Arc<dyn Fn(Value) -> std::result::Result<String, String> + Send + Sync>;
pub type RunnerFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
pub type RunnerFn =
    Arc<dyn Fn(String, CancellationToken, mpsc::Sender<EventMessage>) -> RunnerFuture + Send + Sync>;

const MESSAGE_CHANNEL_CAPACITY: usize = 64;

pub struct TrouterSupervisor {
    cancel: CancellationToken,
    join_handle: Option<JoinHandle<()>>,
}

impl TrouterSupervisor {
    pub fn start(
        token: String,
        handler_config: MessageHandlerConfig,
        dispatch_agent: Option<String>,
        dispatch_fn: DispatchFn,
    ) -> Self {
        Self::start_with_runner(
            token,
            handler_config,
            dispatch_agent,
            dispatch_fn,
            default_runner(),
        )
    }

    pub fn start_with_runner(
        token: String,
        handler_config: MessageHandlerConfig,
        dispatch_agent: Option<String>,
        dispatch_fn: DispatchFn,
        runner_fn: RunnerFn,
    ) -> Self {
        let cancel = CancellationToken::new();
        let task_cancel = cancel.clone();
        let join_handle = tokio::spawn(async move {
            run_supervisor(
                token,
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
        ToolRegistry::global()
            .try_invoke("AcpExec", &payload, OutputFormat::Json)
            .map_err(|error| error.to_string())
    })
}

fn default_runner() -> RunnerFn {
    Arc::new(|token, cancel, msg_tx| {
        Box::pin(async move {
            let mut client = TrouterClient::new(token, cancel);
            client.run(msg_tx).await
        })
    })
}

async fn run_supervisor(
    token: String,
    handler_config: MessageHandlerConfig,
    dispatch_agent: Option<String>,
    dispatch_fn: DispatchFn,
    runner_fn: RunnerFn,
    cancel: CancellationToken,
) {
    set_teams_connecting();

    let (msg_tx, msg_rx) = mpsc::channel::<EventMessage>(MESSAGE_CHANNEL_CAPACITY);
    let runner_cancel = cancel.child_token();
    let runner_handle = tokio::spawn(async move { runner_fn(token, runner_cancel, msg_tx).await });
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
    dispatch_agent: Option<String>,
    dispatch_fn: DispatchFn,
) {
    let mut handler = MessageHandler::new(handler_config);

    while let Some(message) = msg_rx.recv().await {
        record_message_received();
        mark_authenticated_on_first_message();

        let Some(result) = handler.handle(&message) else {
            continue;
        };
        record_match();

        if let Some(agent) = dispatch_agent.as_deref() {
            let payload = TeamsDispatchRequest::from_match(&result, agent).to_acp_payload();
            spawn_dispatch(payload, dispatch_fn.clone());
        }
    }
}

fn record_message_received() {
    update_teams_runtime_state(|state| {
        state.last_message_at = Some(Utc::now().to_rfc3339());
    });
}

fn mark_authenticated_on_first_message() {
    let snapshot = teams_runtime_snapshot();
    if snapshot.connection_state == "authenticated" {
        return;
    }
    if snapshot.auth_state == "authenticated" {
        update_teams_runtime_state(|state| {
            state.connection_state = "authenticated".to_string();
            state.last_error = None;
        });
        return;
    }
    set_teams_authenticated();
}

fn record_match() {
    update_teams_runtime_state(|state| {
        state.last_match_at = Some(Utc::now().to_rfc3339());
        state.match_count = state.match_count.saturating_add(1);
    });
}

fn spawn_dispatch(payload: Value, dispatch_fn: DispatchFn) {
    tokio::task::spawn_blocking(move || {
        let _guard = ActiveDispatchGuard::new();
        if let Err(error) = dispatch_fn(payload) {
            tracing::warn!("Teams monitor dispatch failed: {error}");
        }
    });
}

struct ActiveDispatchGuard;

impl ActiveDispatchGuard {
    fn new() -> Self {
        update_teams_runtime_state(|state| {
            state.active_dispatches = state.active_dispatches.saturating_add(1);
        });
        Self
    }
}

impl Drop for ActiveDispatchGuard {
    fn drop(&mut self) {
        update_teams_runtime_state(|state| {
            state.active_dispatches = state.active_dispatches.saturating_sub(1);
        });
    }
}
