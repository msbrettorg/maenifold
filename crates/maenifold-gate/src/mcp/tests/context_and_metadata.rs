use rmcp::ServerHandler;
use serde_json::json;

use super::super::{
    GATE_SERVER_INSTRUCTIONS, GateMcpHandler, KnownRoots, TransportKind, call_gate_tool,
    context_from_value,
};
use super::support::new_custom_pipeline_root;

#[test]
fn mcp_task_body_tool_returns_raw_markdown() {
    let root = new_custom_pipeline_root();
    let root_path = root.path().to_path_buf();

    let output = call_gate_tool(
        "task_body",
        &json!({ "root": root.path(), "taskId": "T-9.1" }),
        Some(&root_path),
    )
    .expect("task_body tool should succeed");
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid JSON output");

    assert_eq!(value["taskId"], "T-9.1");
    assert_eq!(
        value["path"],
        std::fs::canonicalize(root.path().join("todo").join("T-9.1.md"))
            .expect("canonicalize task path")
            .to_string_lossy()
            .as_ref()
    );
    assert!(
        value["markdown"]
            .as_str()
            .expect("markdown string")
            .contains("## Acceptance Criteria")
    );
}

#[test]
fn context_requires_explicit_root_when_service_default_is_unset() {
    let err = context_from_value(&json!({}), false, None).expect_err("missing root must fail");
    assert!(err.to_string().contains("no root bound to this session"));
}

#[test]
fn server_instructions_explain_authoritative_gate_workflow() {
    let handler =
        GateMcpHandler::new(TransportKind::Stdio, KnownRoots::default()).expect("handler should build");
    let info = <GateMcpHandler as ServerHandler>::get_info(&handler);
    let instructions = info
        .instructions
        .expect("server instructions should be present");

    assert_eq!(instructions, GATE_SERVER_INSTRUCTIONS);
    assert!(instructions.contains("authoritative project task tracker"));
    assert!(instructions.contains("prefer gate instead of ad-hoc todo lists"));
    assert!(instructions.contains("replace SQL todo-table tracking with gate"));
    assert!(instructions.contains("run sync"));
    assert!(instructions.contains("run status"));
    assert!(instructions.contains("use append"));
    assert!(instructions.contains("pass to move the task forward"));
    assert!(instructions.contains("fail to send it back"));
    assert!(instructions.contains("default SDD workflow"));
    assert!(instructions.contains("stage and role transitions"));
    assert!(instructions.contains("pending -> swe -> red-team -> blue-team -> done"));
    assert!(instructions.contains("establishes and stores"));
    assert!(instructions.contains("reuses that stored root"));
}
