use super::*;

#[test]
fn cancelled_worker_error_returns_cancelled_stop_reason() {
    let payload = prompt_result_payload(
        json!(7),
        Err(jockey::acp::AcpExecuteError {
            message: "worker disconnected after cancellation".to_string(),
            requested_resume: false,
            resumed: false,
            resume_failure_kind: None,
        }),
        true,
    );

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 7);
    assert_eq!(payload["result"]["stopReason"], "cancelled");
    assert!(payload.get("error").is_none());
}

#[test]
fn cancelled_worker_success_returns_cancelled_stop_reason() {
    let payload = prompt_result_payload(
        json!(8),
        Ok(jockey::acp::AcpExecuteSuccess {
            output: String::new(),
            session_id: "session-1".to_string(),
            resumed: false,
        }),
        true,
    );

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 8);
    assert_eq!(payload["result"]["stopReason"], "cancelled");
    assert!(payload.get("error").is_none());
}

#[test]
fn non_cancelled_worker_error_returns_json_rpc_error() {
    let payload = prompt_result_payload(
        json!(9),
        Err(jockey::acp::AcpExecuteError {
            message: "agent failed".to_string(),
            requested_resume: false,
            resumed: false,
            resume_failure_kind: None,
        }),
        false,
    );

    assert_eq!(payload["jsonrpc"], "2.0");
    assert_eq!(payload["id"], 9);
    assert_eq!(payload["error"]["code"], -32000);
    assert_eq!(payload["error"]["message"], "agent failed");
    assert!(payload.get("result").is_none());
}

#[test]
fn socket_close_worker_message_resets_session_connection() {
    let relay = RelayConfig {
        agent_name: "Copilot".to_string(),
        adapter: jockey::acp::StdioAdapterSpec {
            runtime_key: "copilot".to_string(),
            binary: "copilot".to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        default_cwd: "/repo".to_string(),
        configured_mcp_servers: Vec::new(),
        keep_session_alive: false,
    };
    let (result_tx, _result_rx) = tokio::sync::oneshot::channel();

    match relay_reset_message(&relay, "acp-relay:local-1", result_tx) {
        jockey::acp::WorkerMsg::Reset {
            runtime_key,
            role_name,
            app_session_id,
            ..
        } => {
            assert_eq!(runtime_key, "copilot");
            assert_eq!(role_name, "Copilot");
            assert_eq!(app_session_id, "acp-relay:local-1");
        }
        jockey::acp::WorkerMsg::Cancel { .. } => panic!("socket close must reset, not cancel"),
        _ => panic!("socket close must use WorkerMsg::Reset"),
    }
}

#[test]
fn keepalive_idle_cleanup_ttl_defaults_to_five_minutes() {
    assert_eq!(
        relay_idle_cleanup_ttl(),
        std::time::Duration::from_secs(5 * 60)
    );
}

#[test]
fn explicit_close_worker_message_resets_session_connection() {
    let relay = RelayConfig {
        agent_name: "Copilot".to_string(),
        adapter: jockey::acp::StdioAdapterSpec {
            runtime_key: "copilot".to_string(),
            binary: "copilot".to_string(),
            args: Vec::new(),
            env: Vec::new(),
        },
        default_cwd: "/repo".to_string(),
        configured_mcp_servers: Vec::new(),
        keep_session_alive: true,
    };
    let (result_tx, _result_rx) = tokio::sync::oneshot::channel();

    match relay_reset_message(&relay, "acp-relay:local-2", result_tx) {
        jockey::acp::WorkerMsg::Reset {
            runtime_key,
            role_name,
            app_session_id,
            ..
        } => {
            assert_eq!(runtime_key, "copilot");
            assert_eq!(role_name, "Copilot");
            assert_eq!(app_session_id, "acp-relay:local-2");
        }
        _ => panic!("explicit close must use WorkerMsg::Reset"),
    }
}

#[test]
fn explicit_close_endpoint_message_resets_by_acp_session_id() {
    let (result_tx, _result_rx) = tokio::sync::oneshot::channel();

    match relay_reset_by_session_message(
        "copilot".to_string(),
        "Copilot".to_string(),
        "session-123".to_string(),
        result_tx,
    ) {
        jockey::acp::WorkerMsg::ResetBySession {
            runtime_key,
            role_name,
            session_id,
            ..
        } => {
            assert_eq!(runtime_key, "copilot");
            assert_eq!(role_name, "Copilot");
            assert_eq!(session_id, "session-123");
        }
        _ => panic!("explicit close endpoint must use WorkerMsg::ResetBySession"),
    }
}
