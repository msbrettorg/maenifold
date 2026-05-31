use super::*;
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::super::pool::{CANCEL_HANDLES, CONN_MAP, CancelHandle, LiveConnection};

async fn insert_tracked_connection(key: &str, session_id: acp::SessionId) {
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (agent_read, agent_write) = tokio::io::split(agent_stream);

    let delta_slot = Arc::new(Mutex::new(None));
    let ui_client = Arc::new(super::super::super::client::JockeyUiClient {
        delta_slot,
        auto_approve: true,
        workspace_root: PathBuf::from("/"),
    });
    let client_transport = ByteStreams::new(client_write.compat_write(), client_read.compat());
    let (connection_tx, connection_rx) = tokio::sync::oneshot::channel::<ConnectionTo<Agent>>();
    let client_io_task = Client
        .builder()
        .name("jockey-test")
        .on_receive_request(
            {
                let ui_client = ui_client.clone();
                async move |req: acp::RequestPermissionRequest, responder, _connection| {
                    let response = ui_client.request_permission(req).await?;
                    responder.respond(response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let ui_client = ui_client.clone();
                async move |notif: acp::SessionNotification, _connection| {
                    ui_client.session_notification(notif).await
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            client_transport,
            move |connection: ConnectionTo<Agent>| async move {
                connection_tx.send(connection).map_err(|_| {
                    acp::Error::new(
                        acp::ErrorCode::InternalError.into(),
                        "failed to receive ACP connection handle",
                    )
                })?;
                std::future::pending::<acp::Result<()>>().await
            },
        );

    let agent_transport = ByteStreams::new(agent_write.compat_write(), agent_read.compat());
    let agent_io_task = Agent
        .builder()
        .on_receive_request(
            async move |request: acp::InitializeRequest, responder, _connection| {
                responder.respond(acp::InitializeResponse::new(request.protocol_version))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: acp::AuthenticateRequest, responder, _connection| {
                responder.respond(acp::AuthenticateResponse::default())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: acp::NewSessionRequest, responder, _connection| {
                responder.respond(acp::NewSessionResponse::new(acp::SessionId::new(
                    "timeout-session",
                )))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: acp::PromptRequest, responder, _connection| {
                responder.respond(acp::PromptResponse::new(acp::StopReason::EndTurn))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |_request: acp::CancelNotification, _connection| Ok(()),
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(
            agent_transport,
            move |_connection: ConnectionTo<Client>| async move {
                std::future::pending::<acp::Result<()>>().await
            },
        );
    tokio::task::spawn_local(agent_io_task);
    tokio::task::spawn_local({
        let client_io_task = client_io_task;
        async move {
            let _ = client_io_task.await;
        }
    });
    let conn = Rc::new(
        connection_rx
            .await
            .expect("receive ACP connection handle for mock client"),
    );

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
                child_pid,
                _io_task: tokio::task::spawn_local(async {}),
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
            insert_tracked_connection(&key, session_id.clone()).await;

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
