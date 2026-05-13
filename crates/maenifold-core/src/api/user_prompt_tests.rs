use super::*;
use crate::mcp::pending_requests::{
    ElicitationResponse, SamplingAction, SamplingDecision, UserPromptAction, UserPromptResponse,
};
use uuid::Uuid;

#[test]
fn http_decoder_accepts_kind_elicitation() {
    let id = Uuid::parse_str("11111111-1111-1111-1111-111111111111")
        .expect("static UUID should parse");
    let body = serde_json::from_str::<UserPromptResponse>(
        r#"{"kind":"elicitation","id":"11111111-1111-1111-1111-111111111111","response":{"action":"cancel"}}"#,
    )
    .expect("elicitation response should decode");

    assert_eq!(
        body,
        UserPromptResponse::Elicitation {
            id,
            response: ElicitationResponse {
                action: UserPromptAction::Cancel,
                content: None,
            },
        }
    );
}

#[test]
fn http_decoder_accepts_kind_sampling() {
    let id = Uuid::parse_str("22222222-2222-2222-2222-222222222222")
        .expect("static UUID should parse");
    let body = serde_json::from_str::<UserPromptResponse>(
        r#"{"kind":"sampling","id":"22222222-2222-2222-2222-222222222222","response":{"action":"allow_once"}}"#,
    )
    .expect("sampling response should decode");

    assert_eq!(
        body,
        UserPromptResponse::Sampling {
            id,
            response: SamplingDecision {
                action: SamplingAction::AllowOnce,
                selected_model: None,
                selected_agent: None,
            },
        }
    );
}

#[test]
fn http_decoder_rejects_missing_kind() {
    let body = serde_json::from_str::<UserPromptResponse>(
        r#"{"id":"33333333-3333-3333-3333-333333333333","response":{"action":"cancel"}}"#,
    );

    assert!(body.is_err());
}
