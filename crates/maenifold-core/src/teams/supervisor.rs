use std::{
    fs::{File, OpenOptions},
    future::Future,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::error::{MaenifoldError, Result};

use super::{
    EventMessage, MessageHandler, MessageHandlerConfig, TeamsDispatchRequest, TeamsReplyRequest,
    TokenInfo, TrouterClient, TrouterClientConfig, chunking, credential_guard,
    dispatcher::{DispatchCtx, DispatchOutcome, run_dispatch},
    runtime::update_teams_runtime_state, send_reply, set_teams_connecting, set_teams_disabled,
    set_teams_error, teams_runtime_snapshot, token_provider::TeamsTokenProvider,
    token_refresh::spawn_refresh_loop,
};

pub use super::dispatcher::DispatchFn;
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
    // T-238.56: held for lifetime of supervisor; released on Drop.
    // Field order matters: declared last so it drops last (after join_handle is taken via stop()).
    _singleton: MonitorSingleton,
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
        lock_path: PathBuf,
    ) -> Result<Self> {
        Self::start_with_runner(
            initial_token,
            provider,
            fallback_refresh,
            trouter_config,
            handler_config,
            dispatch_agent,
            dispatch_fn,
            default_runner(),
            lock_path,
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
        lock_path: PathBuf,
    ) -> Result<Self> {
        // T-238.56: acquire OS-level singleton BEFORE spawning the supervisor task,
        // so contention short-circuits without leaking the runner.
        let singleton = MonitorSingleton::acquire(&lock_path)?;

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

        Ok(Self {
            cancel,
            join_handle: Some(join_handle),
            _singleton: singleton,
        })
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
    Arc::new(|req: TeamsDispatchRequest| {
        Box::pin(async move {
            let agent = req.agent.clone();
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
                    prompt: req.prompt,
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

            match result_rx.await {
                Ok(Ok(success)) => Ok(success.output),
                Ok(Err(error)) => Err(error.message),
                Err(_) => Err("ACP worker dropped result channel".to_string()),
            }
        })
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
    let token_rx_for_dispatch = token_rx.clone();

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
        token_rx_for_dispatch,
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
    dispatch_agent: Option<String>,
    dispatch_fn: DispatchFn,
    token_rx: watch::Receiver<String>,
) {
    let mut handler = MessageHandler::new(handler_config.clone());
    // Dedup is a correctness gate against Trouter redelivery (reconnects,
    // re-registration). Distinct from the handler's per-conversation cooldown,
    // which is a rate limit. See teams::dedup.
    // TODO(T-238.54): share this buffer with the catch-up polling worker.
    let mut dedup = super::MessageDedup::with_capacity(super::DEFAULT_DEDUP_CAPACITY);

    let timeout_minutes = handler_config.dispatch_timeout_minutes.max(1);
    let dispatch_timeout = Duration::from_secs(timeout_minutes.saturating_mul(60));
    let from_mri = handler_config.self_mri.clone().unwrap_or_default();
    let chatsvc_region = handler_config.chatsvc_region.clone();
    let reply_prefix = handler_config.reply_prefix.clone();
    let reply_behavior = handler_config.reply_behavior.clone();

    while let Some(message) = msg_rx.recv().await {
        record_message_received();
        ensure_connected_state();

        if !dedup.check_and_record(&message.message_id) {
            tracing::debug!(
                message_id = %message.message_id,
                "skipping duplicate Trouter delivery"
            );
            continue;
        }

        let Some(result) = handler.handle(&message) else {
            continue;
        };

        // T-238.65: fire-and-forget processing receipt before dispatch.
        // Body is hardcoded — never echo prompt content back to the user.
        if handler_config.send_receipt {
            if let Some(receipt_from_mri) = handler_config.self_mri.clone() {
                let req = TeamsReplyRequest {
                    token: token_rx.borrow().clone(),
                    conversation_id: result.message.conversation_id.clone(),
                    body: "⏳ Processing...".to_string(),
                    from_mri: receipt_from_mri,
                    chatsvc_region: chatsvc_region.clone(),
                    reply_prefix: reply_prefix.clone(),
                };
                tokio::spawn(async move {
                    if let Err(error) = send_reply(req).await {
                        tracing::warn!(%error, "teams dispatch receipt failed");
                    }
                });
            } else {
                tracing::debug!("send_receipt enabled but self_mri is None; skipping receipt");
            }
        }

        super::push_teams_prompt(super::TeamsPrompt {
            prompt: result.prompt.clone(),
            conversation_id: result.message.conversation_id.clone(),
            sender_name: result.message.sender_name.clone(),
            matched_keyword: result.matched_keyword.clone(),
        });

        let Some(agent) = dispatch_agent.as_ref() else {
            tracing::info!(
                conversation_id = %result.message.conversation_id,
                "Teams match received but no dispatch_agent configured; skipping dispatch"
            );
            continue;
        };

        let request = TeamsDispatchRequest::from_match(
            &result,
            agent,
            timeout_minutes,
            &reply_behavior,
        );
        let ctx = DispatchCtx {
            token: token_rx.clone(),
            from_mri: from_mri.clone(),
            chatsvc_region: chatsvc_region.clone(),
            reply_prefix: reply_prefix.clone(),
            dispatch_timeout,
        };
        let dispatch_fn = dispatch_fn.clone();

        // Spawn so a slow agent does not back-pressure the receive loop.
        tokio::spawn(async move {
            let conversation_id = request.metadata.reply_conversation_id.clone();
            let outcome = run_dispatch(request, &ctx, &dispatch_fn).await;
            let body = match outcome {
                DispatchOutcome::Success { reply, .. } => {
                    tracing::info!(%conversation_id, "Teams dispatch succeeded");
                    reply
                }
                DispatchOutcome::Timeout => {
                    tracing::warn!(%conversation_id, "Teams dispatch timed out");
                    format!("⏳ Timed out after {timeout_minutes} minutes.")
                }
                DispatchOutcome::Failed(error) => {
                    tracing::error!(%conversation_id, %error, "Teams dispatch failed");
                    format!("❌ Failed: {error}")
                }
            };
            if let Err(error) = deliver_reply(&ctx, &conversation_id, &body).await {
                tracing::warn!(%conversation_id, %error, "Teams reply delivery failed");
            }
        });
    }
}

/// Apply the reply prefix to the first chunk only, then send chunks
/// sequentially with `INTER_CHUNK_DELAY` between sends. First failure
/// short-circuits remaining chunks (matches python `_reply_to_chat`).
async fn deliver_reply(ctx: &DispatchCtx, conversation_id: &str, body: &str) -> Result<()> {
    let token = ctx.token.borrow().clone();
    let scan = credential_guard::scan_for_credentials(body);
    let effective_body = if credential_guard::has_findings(&scan) {
        let previews: Vec<String> = scan
            .findings
            .iter()
            .map(|f| format!("{}={}", f.pattern_name, f.match_preview))
            .collect();
        tracing::warn!(
            %conversation_id,
            finding_count = scan.findings.len(),
            findings = %previews.join(","),
            "Credential guard blocked reply — potential credential leak in agent output"
        );
        "⚠️ Response blocked — potential credential leak detected. The agent's output contained patterns matching known credential formats.".to_string()
    } else {
        body.to_string()
    };
    let chunks = chunking::chunk_message(&effective_body, chunking::DEFAULT_MAX_CHUNK);
    let total = chunks.len();
    for (i, chunk) in chunks.into_iter().enumerate() {
        let prefix = if i == 0 {
            ctx.reply_prefix.clone()
        } else {
            String::new()
        };
        send_reply(TeamsReplyRequest {
            token: token.clone(),
            conversation_id: conversation_id.to_string(),
            body: chunk,
            from_mri: ctx.from_mri.clone(),
            chatsvc_region: ctx.chatsvc_region.clone(),
            reply_prefix: prefix,
        })
        .await?;
        if i + 1 < total {
            tokio::time::sleep(chunking::INTER_CHUNK_DELAY).await;
        }
    }
    Ok(())
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

// ─── T-238.56: OS-level singleton lock for the Teams monitor ──────────────────
//
// `fd-lock` (POSIX `flock` / Windows `LockFileEx`) gives us "released by the
// kernel on process exit/death" for free, so no PID-liveness probe is needed.
// The lock file contains "{hostname}-{pid}" purely for operator diagnostics.
// Reference pattern: crates/maenifold-pipeline-driver/src/main.rs.

/// RAII guard ensuring at most one Teams monitor runs per maenifold root.
///
/// Holds an exclusive advisory file lock until dropped. The owning
/// `fd_lock::RwLock<File>` is heap-allocated so its address is stable; the
/// guard's lifetime is unsafely extended to `'static` and the field drop
/// order (`guard` declared first → drops first → releases lock → THEN box
/// drops) keeps this sound.
pub struct MonitorSingleton {
    guard: Option<fd_lock::RwLockWriteGuard<'static, File>>,
    // Boxed so address is stable across moves of the outer struct.
    // SAFETY contract: must outlive `guard`.
    _rw_lock: Box<fd_lock::RwLock<File>>,
}

impl MonitorSingleton {
    pub fn acquire(lock_path: &Path) -> Result<Self> {
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                MaenifoldError::Message(format!(
                    "Cannot create Teams monitor lock directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|e| {
                MaenifoldError::Message(format!(
                    "Cannot open Teams monitor lock file {}: {e}",
                    lock_path.display()
                ))
            })?;

        let mut rw_lock = Box::new(fd_lock::RwLock::new(file));
        // SAFETY: `rw_lock` is heap-allocated (stable address) and lives in
        // `Self::_rw_lock`. The guard lives in `Self::guard`, declared first,
        // so it drops before `_rw_lock`. We never expose `&mut self` paths
        // that could move the box while the guard is alive.
        let rw_lock_ptr: *mut fd_lock::RwLock<File> = &mut *rw_lock;
        let guard = unsafe { (*rw_lock_ptr).try_write() }.map_err(|_| {
            let holder = read_holder(lock_path).unwrap_or_else(|| "<unknown>".to_string());
            MaenifoldError::Message(format!(
                "Teams monitor already running (lock held at {}; holder: {holder})",
                lock_path.display()
            ))
        })?;
        let mut guard: fd_lock::RwLockWriteGuard<'static, File> =
            unsafe { std::mem::transmute(guard) };

        let holder = format!("{}-{}", hostname_string(), std::process::id());
        let _ = guard.set_len(0);
        let _ = guard.seek(SeekFrom::Start(0));
        let _ = guard.write_all(holder.as_bytes());
        let _ = guard.flush();

        tracing::info!(
            holder = %holder,
            path = %lock_path.display(),
            "teams monitor singleton acquired"
        );

        Ok(Self {
            guard: Some(guard),
            _rw_lock: rw_lock,
        })
    }
}

impl Drop for MonitorSingleton {
    fn drop(&mut self) {
        // Drop guard first → releases kernel lock → THEN _rw_lock box drops.
        self.guard.take();
        tracing::info!("teams monitor singleton released");
    }
}

fn read_holder(lock_path: &Path) -> Option<String> {
    let mut f = std::fs::File::open(lock_path).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn hostname_string() -> String {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    if result != 0 {
        return "unknown".to_string();
    }
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..nul]).into_owned()
}

#[cfg(test)]
mod singleton_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_succeeds_and_writes_holder() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("teams-monitor.lock");
        let _g = MonitorSingleton::acquire(&path).expect("first acquire");
        let holder = read_holder(&path).expect("holder file present");
        assert!(
            holder.contains(&std::process::id().to_string()),
            "holder {holder} should contain current PID"
        );
    }

    #[test]
    fn second_acquire_fails_with_holder_in_message() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("teams-monitor.lock");
        let _first = MonitorSingleton::acquire(&path).expect("first acquire");
        let err = match MonitorSingleton::acquire(&path) {
            Ok(_) => panic!("second acquire must fail"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("already running"), "msg: {msg}");
        assert!(msg.contains(&std::process::id().to_string()), "msg: {msg}");
    }

    #[test]
    fn drop_releases_lock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("teams-monitor.lock");
        {
            let _g = MonitorSingleton::acquire(&path).expect("first acquire");
        }
        let _g2 = MonitorSingleton::acquire(&path).expect("re-acquire after drop");
    }
}
