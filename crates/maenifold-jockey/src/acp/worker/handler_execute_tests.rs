use super::*;

#[test]
fn cancelled_prompt_error_is_normalized_to_success() {
    let session_id = acp::SessionId::from("session-1".to_string());
    let result = normalize_prompt_result_after_cancel(
        Err("transport aborted after cancel".to_string()),
        true,
        &session_id,
    );

    assert_eq!(result, Ok((String::new(), "session-1".to_string())));
}

#[test]
fn non_cancelled_prompt_error_is_preserved() {
    let session_id = acp::SessionId::from("session-1".to_string());
    let result = normalize_prompt_result_after_cancel(
        Err("transport aborted".to_string()),
        false,
        &session_id,
    );

    assert_eq!(result, Err("transport aborted".to_string()));
}
