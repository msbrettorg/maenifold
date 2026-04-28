use super::envelope_indicates_failure;

#[test]
fn detects_domain_failure_envelope() {
    let body = r#"{"success":false,"data":null,"error":{"code":"X","message":"Y"}}"#;
    assert!(envelope_indicates_failure(body));
}

#[test]
fn passes_success_envelope_through() {
    let body = r#"{"success":true,"data":{"foo":1},"error":null}"#;
    assert!(!envelope_indicates_failure(body));
}

#[test]
fn ignores_non_envelope_text() {
    assert!(!envelope_indicates_failure("plain markdown"));
    assert!(!envelope_indicates_failure("{}"));
}
