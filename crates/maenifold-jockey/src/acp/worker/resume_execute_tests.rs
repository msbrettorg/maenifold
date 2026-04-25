use super::*;

#[test]
fn cancelled_prompt_error_is_normalized_to_success() {
    let session_id = acp::SessionId::from("session-1".to_string());
    let result = normalize_prompt_result_after_cancel(
        Err(AcpExecuteError {
            message: "transport aborted after cancel".to_string(),
            requested_resume: true,
            resumed: true,
            resume_failure_kind: None,
        }),
        true,
        &session_id,
        true,
    );

    let success = result.expect("cancelled prompt should normalize to success");
    assert_eq!(success.output, "");
    assert_eq!(success.session_id, "session-1");
    assert!(success.resumed);
}

#[test]
fn non_cancelled_prompt_error_is_preserved() {
    let session_id = acp::SessionId::from("session-1".to_string());
    let result = normalize_prompt_result_after_cancel(
        Err(AcpExecuteError {
            message: "transport aborted".to_string(),
            requested_resume: false,
            resumed: false,
            resume_failure_kind: None,
        }),
        false,
        &session_id,
        false,
    );

    let err = result.expect_err("non-cancelled prompt error should be preserved");
    assert_eq!(err.message, "transport aborted");
    assert!(!err.requested_resume);
    assert!(!err.resumed);
}
