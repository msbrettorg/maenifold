#[test]
fn execute_node_request_legacy_wrapper_is_absent() {
    let dispatch_source = include_str!("executor_nodes_dispatch.rs");
    let nodes_source = include_str!("executor_nodes.rs");
    let orchestration_source = include_str!("executor_orchestration.rs");

    assert!(
        dispatch_source.contains("fn execute_node_request_with_pipeline_session("),
        "pipeline dispatch must keep the explicit pipeline-session entrypoint"
    );
    assert!(
        !dispatch_source.contains(concat!("fn ", "execute_node_request", "(")),
        "executor_nodes_dispatch.rs must not reintroduce the legacy wrapper"
    );
    assert!(
        !nodes_source.contains(concat!("fn ", "execute_node_request", "(")),
        "executor_nodes.rs must not re-export the legacy wrapper"
    );
    assert!(
        !orchestration_source.contains(concat!("_legacy_", "execute_node_request")),
        "executor_orchestration.rs must not keep dead bindings that defeat reference audits"
    );
}
