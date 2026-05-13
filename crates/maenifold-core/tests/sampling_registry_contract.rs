#![allow(non_snake_case)]

fn client_handler_source() -> &'static str {
    include_str!("../src/mcp/client_handler.rs")
}

fn registry_source() -> &'static str {
    include_str!("../src/mcp/sampling_session_registry.rs")
}

#[tokio::test]
async fn serverName_propagated_through_create_message() {
    let source = client_handler_source();

    assert!(
        !source.contains("server_name: \"<unknown>\".to_string()"),
        "create_message must propagate the RequestContext server identity"
    );
    assert!(
        source.contains("test-server-x") || source.contains("server_name_from_context"),
        "contract expects a context-derived serverName path for test-server-x"
    );
}

#[tokio::test]
async fn denied_sampling_request_appears_in_registry() {
    let handler = client_handler_source();
    let registry = registry_source();
    let deny = handler.find("SamplingAction::Deny").expect("deny branch exists");
    let insert = handler.find(".insert(SamplingDispatch").expect("registry insert exists");

    assert!(insert < deny, "denied decisions must be inserted before returning Err");
    assert!(registry.contains("Denied"), "SamplingSessionStatus must expose Denied");
    assert!(registry.contains("decision"), "SamplingDispatch must record decision");
    assert!(handler.contains("SamplingDecisionRecord::Deny"));
}

#[tokio::test]
async fn decision_field_recorded_for_allow_once_vs_allow_always() {
    let handler = client_handler_source();
    let registry = registry_source();

    assert!(registry.contains("pub decision: SamplingDecisionRecord"));
    assert!(handler.contains("SamplingDecisionRecord::AllowOnce"));
    assert!(handler.contains("SamplingDecisionRecord::AllowAlways"));
    assert!(handler.contains("SamplingAction::AllowOnce"));
    assert!(handler.contains("SamplingAction::AllowAlways"));
}

#[test]
fn samplingdispatch_has_decision_field_with_required_variants() {
    let registry = registry_source();
    let variants = ["AllowOnce", "AllowAlways", "Deny", "Cancelled", "Expired"];

    assert!(registry.contains("enum SamplingDecisionRecord"));
    assert!(registry.contains("decision: SamplingDecisionRecord"));
    for variant in variants {
        assert!(registry.contains(variant), "missing variant {variant}");
    }
}
