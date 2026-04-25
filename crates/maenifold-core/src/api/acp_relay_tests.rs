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
