use crate::mcp::pending_requests::{
    ElicitationResponse, UserPromptAction, UserPromptResponse,
};
use uuid::Uuid;

#[test]
fn http_decoder_accepts_kind_elicitation() {
    let id =
        Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("static UUID should parse");
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
fn http_decoder_rejects_missing_kind() {
    let body = serde_json::from_str::<UserPromptResponse>(
        r#"{"id":"33333333-3333-3333-3333-333333333333","response":{"action":"cancel"}}"#,
    );

    assert!(body.is_err());
}
