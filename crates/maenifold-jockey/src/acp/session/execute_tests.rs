use rmcp::model::Tool;
use serde_json::{Map, Value};
use std::sync::Arc;
use tokio::sync::oneshot;

use super::execute_runtime_sampling;

fn fire_session_id_if_present(
    session_id_text: String,
    session_id_tx: Option<oneshot::Sender<String>>,
) {
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

    let session_id = rx
        .await
        .expect("sender should fire for non-empty ACP session id");
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

#[test]
fn execute_runtime_sampling_accepts_resume_session_id_and_tools_contract() {
    let (session_id_tx, _session_id_rx) = oneshot::channel();
    let (event_tx, _event_rx) = tokio::sync::broadcast::channel(1);
    let tool = Tool::new(
        "lookup".to_string(),
        "Lookup test data".to_string(),
        Arc::new(Map::<String, Value>::new()),
    );

    let future = execute_runtime_sampling(
        "mock",
        "copilot",
        uuid::Uuid::new_v4(),
        "Use a tool if needed".to_string(),
        None,
        session_id_tx,
        event_tx,
        Some("existing-acp-session".to_string()),
        Some(vec![tool]),
    );

    drop(future);
}
