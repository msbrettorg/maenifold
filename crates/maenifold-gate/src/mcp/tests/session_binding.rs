use serde_json::json;

use super::super::{GateMcpHandler, KnownRoots, TransportKind, call_gate_tool};
use super::support::{invoke_handler_tool, new_custom_pipeline_root_with_task, single_task_id};

#[test]
fn test_bound_root_set_on_sync() {
    let shared_roots = KnownRoots::default();
    let handler_a = GateMcpHandler::new(TransportKind::Stdio, shared_roots.clone())
        .expect("first handler should build");
    let handler_b = GateMcpHandler::new(TransportKind::Stdio, shared_roots)
        .expect("second handler should build");
    let root_a = new_custom_pipeline_root_with_task("T-117.1");
    let root_b = new_custom_pipeline_root_with_task("T-117.2");

    invoke_handler_tool(
        &handler_a,
        "sync",
        json!({ "root": root_a.path(), "rebuild": true }),
    )
    .expect("sync should bind handler A root");
    invoke_handler_tool(
        &handler_b,
        "sync",
        json!({ "root": root_b.path(), "rebuild": true }),
    )
    .expect("sync should bind handler B root");

    let output =
        invoke_handler_tool(&handler_a, "status", json!({})).expect("status should succeed");

    assert_eq!(single_task_id(&output), "T-117.1");
}

#[test]
fn test_bound_root_used_when_no_explicit_root() {
    let shared_roots = KnownRoots::default();
    let handler_a = GateMcpHandler::new(TransportKind::Stdio, shared_roots.clone())
        .expect("first handler should build");
    let handler_b = GateMcpHandler::new(TransportKind::Stdio, shared_roots)
        .expect("second handler should build");
    let root_a = new_custom_pipeline_root_with_task("T-117.1");
    let root_b = new_custom_pipeline_root_with_task("T-117.2");

    invoke_handler_tool(
        &handler_a,
        "sync",
        json!({ "root": root_a.path(), "rebuild": true }),
    )
    .expect("sync should bind handler A root");
    invoke_handler_tool(
        &handler_b,
        "sync",
        json!({ "root": root_b.path(), "rebuild": true }),
    )
    .expect("sync should bind handler B root");

    let output = invoke_handler_tool(
        &handler_a,
        "pass",
        json!({
            "taskId": "T-117.1",
            "agent": "swe",
            "notes": "advanced without explicit root"
        }),
    )
    .expect("bound root should satisfy pass without explicit root");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");

    assert_eq!(value["state"], "swe");
}

#[test]
fn test_explicit_root_overrides_bound_root() {
    let shared_roots = KnownRoots::default();
    let handler_a = GateMcpHandler::new(TransportKind::Stdio, shared_roots.clone())
        .expect("first handler should build");
    let handler_b = GateMcpHandler::new(TransportKind::Stdio, shared_roots)
        .expect("second handler should build");
    let root_a = new_custom_pipeline_root_with_task("T-117.1");
    let root_b = new_custom_pipeline_root_with_task("T-117.2");

    invoke_handler_tool(
        &handler_a,
        "sync",
        json!({ "root": root_a.path(), "rebuild": true }),
    )
    .expect("sync should bind handler A root");
    invoke_handler_tool(
        &handler_b,
        "sync",
        json!({ "root": root_b.path(), "rebuild": true }),
    )
    .expect("sync should bind handler B root");

    invoke_handler_tool(&handler_a, "status", json!({ "root": root_a.path() }))
        .expect("explicit root call should succeed");
    let output =
        invoke_handler_tool(&handler_b, "status", json!({})).expect("status should succeed");

    assert_eq!(single_task_id(&output), "T-117.2");
}

#[test]
fn test_unbound_session_returns_error() {
    let shared_roots = KnownRoots::default();
    let bound_handler = GateMcpHandler::new(TransportKind::Stdio, shared_roots.clone())
        .expect("bound handler should build");
    let unbound_handler = GateMcpHandler::new(TransportKind::Stdio, shared_roots)
        .expect("unbound handler should build");
    let root = new_custom_pipeline_root_with_task("T-117.1");

    invoke_handler_tool(
        &bound_handler,
        "sync",
        json!({ "root": root.path(), "rebuild": true }),
    )
    .expect("sync should bind root");

    let err = invoke_handler_tool(&unbound_handler, "status", json!({}))
        .expect_err("unbound handler should require explicit root");

    assert!(err.contains("no root bound to this session"));
}

#[test]
fn test_non_sync_call_binds_root() {
    let handler = GateMcpHandler::new(TransportKind::Stdio, KnownRoots::default())
        .expect("handler should build");
    let root = new_custom_pipeline_root_with_task("T-117.1");
    let root_path = root.path().to_path_buf();

    call_gate_tool(
        "sync",
        &json!({ "root": root.path(), "rebuild": true }),
        Some(&root_path),
    )
    .expect("sync should build cache DB");
    invoke_handler_tool(&handler, "status", json!({ "root": root.path() }))
        .expect("status with explicit root should bind session root");
    let output = invoke_handler_tool(&handler, "status", json!({})).expect("status should succeed");

    assert_eq!(single_task_id(&output), "T-117.1");
}

#[test]
fn test_per_session_isolation() {
    let shared_roots = KnownRoots::default();
    let handler_a = GateMcpHandler::new(TransportKind::Stdio, shared_roots.clone())
        .expect("first handler should build");
    let handler_b = GateMcpHandler::new(TransportKind::Stdio, shared_roots)
        .expect("second handler should build");
    let root_a = new_custom_pipeline_root_with_task("T-117.1");
    let root_b = new_custom_pipeline_root_with_task("T-117.2");

    invoke_handler_tool(
        &handler_a,
        "sync",
        json!({ "root": root_a.path(), "rebuild": true }),
    )
    .expect("sync should bind handler A root");
    invoke_handler_tool(
        &handler_b,
        "sync",
        json!({ "root": root_b.path(), "rebuild": true }),
    )
    .expect("sync should bind handler B root");

    let output_a =
        invoke_handler_tool(&handler_a, "status", json!({})).expect("status A should succeed");
    let output_b =
        invoke_handler_tool(&handler_b, "status", json!({})).expect("status B should succeed");

    assert_eq!(single_task_id(&output_a), "T-117.1");
    assert_eq!(single_task_id(&output_b), "T-117.2");
}
