use super::*;

#[test]
fn rejects_session_new_and_load_client_mcp_server_injection() {
    for (method, request_id) in [("session/new", json!(7)), ("session/load", json!("load"))] {
        let error = reject_client_mcp_servers(
            &json!({
                "sessionId": "session-1",
                "mcpServers": [{
                    "type": "stdio",
                    "name": "evil",
                    "command": "malicious"
                }]
            }),
            request_id.clone(),
            method,
        )
        .expect_err("client mcpServers must be rejected");
        assert_eq!(error["id"], request_id);
        assert_eq!(error["error"]["code"], json!(-32602));
        assert_eq!(
            error["error"]["message"],
            format!("{method} does not accept client-provided mcpServers")
        );
    }
}

fn relay_for(agent_name: &str) -> RelayConfig {
    RelayConfig {
        agent_name: agent_name.to_string(),
        adapter: jockey::acp::StdioAdapterSpec {
            runtime_key: agent_name.to_string(),
            binary: agent_name.to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        default_cwd: ".".to_string(),
        configured_mcp_servers: Vec::new(),
        keep_session_alive: false,
    }
}

#[test]
fn initialize_handshake_for_claude_reports_image_not_audio() {
    let relay = relay_for("claude");
    let payload = initialize_response(&relay, caps_for_agent(&relay.agent_name, &relay.adapter));
    let caps = &payload["agentCapabilities"]["promptCapabilities"];
    assert_eq!(caps["image"], json!(true));
    assert_eq!(caps["audio"], json!(false));
    assert_eq!(caps["embeddedContext"], json!(true));
}

#[test]
fn initialize_handshake_for_ollama_reports_no_image_or_audio() {
    let relay = relay_for("ollama-llama3");
    let payload = initialize_response(&relay, caps_for_agent(&relay.agent_name, &relay.adapter));
    let caps = &payload["agentCapabilities"]["promptCapabilities"];
    assert_eq!(caps["image"], json!(false));
    assert_eq!(caps["audio"], json!(false));
    assert_eq!(caps["embeddedContext"], json!(true));
}

// T-330.3 regression: in the resume (session/load) path the upstream forwarder
// task sends `ReplayComplete` and then drops its sender clone — closing the
// event channel — *before* the worker's result oneshot fires. The wait loop
// must not interpret that channel closure as a fatal "replay channel closed
// before completion"; it must keep waiting for the result.
#[tokio::test]
async fn wait_for_worker_result_returns_result_when_oneshot_fires() {
    use tokio::sync::oneshot;

    let (result_tx, mut result_rx) = oneshot::channel::<(Vec<Value>, Vec<String>, String)>();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let _ = result_tx.send((vec![], vec![], "resumed-session-1".to_string()));
    });

    let result = wait_for_worker_result(&mut result_rx)
        .await
        .expect("worker result must be delivered after ReplayComplete");
    assert_eq!(result.2, "resumed-session-1");
}

#[tokio::test]
async fn wait_for_worker_result_reports_worker_disconnect_on_sender_drop() {
    use tokio::sync::oneshot;

    let (result_tx, mut result_rx) = oneshot::channel::<(Vec<Value>, Vec<String>, String)>();
    drop(result_tx);

    let err = wait_for_worker_result(&mut result_rx)
        .await
        .expect_err("dropped result sender must surface WorkerDisconnected");
    assert_eq!(err, ReplayWaitError::WorkerDisconnected);
}

// Integration-style: simulate the select-loop body and prove that the
// post-ReplayComplete wait survives the event channel closing.
// This mirrors the structure of `handle_session_load` after the T-330.3 fix.
#[tokio::test]
async fn session_load_loop_survives_event_channel_close_after_replay_complete() {
    use tokio::sync::{mpsc, oneshot};

    let (event_tx, mut event_rx) = mpsc::channel::<jockey::acp::AcpEvent>(8);
    let (result_tx, mut rx) = oneshot::channel::<(Vec<Value>, Vec<String>, String)>();

    // Upstream: send ReplayComplete, then drop sender so channel closes.
    event_tx
        .send(jockey::acp::AcpEvent::ReplayComplete)
        .await
        .expect("send ReplayComplete");
    drop(event_tx);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let _ = result_tx.send((vec![], vec![], "resumed-session-2".to_string()));
    });

    // Mirror the fixed loop in handle_session_load.
    let mut replay_complete = false;
    let mut replay_result: Option<(Vec<Value>, Vec<String>, String)> = None;
    let mut spurious_close = false;
    let outcome = loop {
        if replay_complete {
            if let Some(result) = replay_result.take() {
                break Ok::<_, &'static str>(result);
            }
            match wait_for_worker_result(&mut rx).await {
                Ok(result) => break Ok(result),
                Err(_) => break Err("worker disconnected"),
            }
        }
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else {
                    spurious_close = true;
                    break Err("ACP replay channel closed before completion");
                };
                if matches!(event, jockey::acp::AcpEvent::ReplayComplete) {
                    replay_complete = true;
                    if let Some(result) = replay_result.take() { break Ok(result); }
                    continue;
                }
            }
            result = &mut rx => {
                let result = match result {
                    Ok(r) => r,
                    Err(_) => break Err("worker disconnected"),
                };
                if replay_complete { break Ok(result); }
                replay_result = Some(result);
            }
        }
    };

    assert!(
        !spurious_close,
        "post-ReplayComplete channel close must not surface as a spurious error"
    );
    let result = outcome.expect("session/load loop must succeed");
    assert_eq!(result.2, "resumed-session-2");
}
