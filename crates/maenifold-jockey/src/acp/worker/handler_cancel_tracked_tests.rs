use super::*;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::super::pool::{CANCEL_HANDLES, CONN_MAP, CancelHandle, LiveConnection};

#[derive(Clone, Default)]
struct CompletingAgent;

#[async_trait::async_trait(?Send)]
impl acp::Agent for CompletingAgent {
    async fn initialize(
        &self,
        request: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        Ok(acp::InitializeResponse::new(request.protocol_version))
    }

    async fn authenticate(
        &self,
        _request: acp::AuthenticateRequest,
    ) -> acp::Result<acp::AuthenticateResponse> {
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        _request: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        Ok(acp::NewSessionResponse::new(acp::SessionId::new(
            "timeout-session",
        )))
    }

    async fn prompt(&self, _request: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, _request: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

fn insert_tracked_connection(key: &str, session_id: acp::SessionId) {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (agent_read, agent_write) = tokio::io::split(agent_stream);

    let delta_slot = Arc::new(Mutex::new(None));
    let (client_conn, client_io_task) = acp::ClientSideConnection::new(
        super::super::super::client::JockeyUiClient {
            delta_slot,
            auto_approve: true,
            workspace_root: PathBuf::from("/"),
        },
        client_write.compat_write(),
        client_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    let conn = Rc::new(client_conn);

    let (_agent_conn, agent_io_task) = acp::AgentSideConnection::new(
        CompletingAgent,
        agent_write.compat_write(),
        agent_read.compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    tokio::task::spawn_local(agent_io_task);

    let (health_tx, health_rx) = tokio::sync::watch::channel(true);
    drop(health_tx);
    let child = tokio::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("spawn killable child process for tracked ACP test");
    let child_pid = child.id();

    CONN_MAP.with(|map| {
        map.borrow_mut().insert(
            key.to_string(),
            LiveConnection {
                instance_id: 1,
                conn: conn.clone(),
                session_id: session_id.clone(),
                resumed: false,
                cwd: "/".to_string(),
                delta_slot: Arc::new(Mutex::new(None::<mpsc::Sender<_>>)),
                available_modes: Vec::new(),
                current_mode: None,
                child_pid,
                _child: child,
                _io_task: tokio::task::spawn_local(async move {
                    let _ = client_io_task.await;
                }),
                health_rx,
            },
        );
    });
    CANCEL_HANDLES.with(|handles| {
        handles.borrow_mut().insert(
            key.to_string(),
            CancelHandle {
                conn,
                session_id,
                cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            },
        );
    });
}

#[tokio::test(flavor = "current_thread")]
async fn cancel_tracked_session_falls_through_to_pg_kill_on_timeout() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let key = super::super::pool::pool_key("app-session", "mock-runtime", "mock-role");
            let session_id = acp::SessionId::new("timeout-session");
            CONN_MAP.with(|map| map.borrow_mut().clear());
            CANCEL_HANDLES.with(|handles| handles.borrow_mut().clear());
            set_test_clean_cancel_hangs(true);
            insert_tracked_connection(&key, session_id.clone());

            let started = Instant::now();
            let outcome = cancel_tracked_session(&session_id.to_string())
                .await
                .expect("tracked session cancel should return after timeout");
            let elapsed = started.elapsed();

            set_test_clean_cancel_hangs(false);
            assert!(elapsed < Duration::from_secs(4), "elapsed={elapsed:?}");
            assert!(outcome.acp_session);
            assert!(outcome.killed_process_group);
            assert!(!outcome.clean_cancel_completed);
            CONN_MAP.with(|map| assert!(!map.borrow().contains_key(&key)));
            CANCEL_HANDLES.with(|handles| assert!(!handles.borrow().contains_key(&key)));
        })
        .await;
}
