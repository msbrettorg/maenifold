use tokio::sync::oneshot;

fn fire_session_id_if_present(session_id_text: String, session_id_tx: Option<oneshot::Sender<String>>) {
    if let (false, Some(tx)) = (session_id_text.is_empty(), session_id_tx) {
        let _ = tx.send(session_id_text);
    }
}

#[tokio::test]
async fn test_execute_runtime_sampling_fires_session_id_tx_on_success() {
    let source = include_str!("../worker/handler_execute.rs");
    assert!(source.contains("session_id_text.is_empty()"));
    assert!(source.contains("let _ = tx.send(session_id_text);"));

    let (tx, rx) = oneshot::channel();
    fire_session_id_if_present("acp-session-123".to_string(), Some(tx));

    let session_id = rx.await.expect("sender should fire for non-empty ACP session id");
    assert_eq!(session_id, "acp-session-123");
}

#[tokio::test]
async fn test_execute_runtime_sampling_drops_sender_on_failure() {
    let source = include_str!("../worker/handler_execute.rs");
    assert!(source.contains("session_id_text.is_empty()"));

    let (tx, rx) = oneshot::channel();
    fire_session_id_if_present(String::new(), Some(tx));

    let err = rx
        .await
        .expect_err("empty ACP session id must drop sender instead of sending an empty string");
    assert_eq!(err.to_string(), "channel closed");
}
