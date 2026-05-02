use super::{AcpEvent, complete_already_live_prewarm, complete_failed_prewarm};
use tokio::sync::{mpsc, oneshot};

#[tokio::test]
async fn already_live_resume_emits_replay_complete() {
    let (delta_tx, mut delta_rx) = mpsc::channel(1);
    let (result_tx, result_rx) = oneshot::channel();

    complete_already_live_prewarm(
        "missing-key",
        "copilot",
        Some(delta_tx),
        Some(result_tx),
        true,
    )
    .await;

    assert!(matches!(
        delta_rx.try_recv().expect("replay sentinel"),
        AcpEvent::ReplayComplete
    ));
    let (_, _, session_id) = result_rx.await.expect("prewarm result");
    assert_eq!(session_id, "");
}

#[tokio::test]
async fn failed_resume_emits_replay_complete_before_empty_result() {
    let (delta_tx, mut delta_rx) = mpsc::channel(1);
    let (result_tx, result_rx) = oneshot::channel();

    complete_failed_prewarm(
        Some(delta_tx),
        Some(result_tx),
        true,
    )
    .await;

    assert!(matches!(
        delta_rx.try_recv().expect("replay sentinel"),
        AcpEvent::ReplayComplete
    ));
    let (_, _, session_id) = result_rx.await.expect("prewarm result");
    assert_eq!(session_id, "");
}
