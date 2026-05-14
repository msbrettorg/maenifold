use super::*;

#[test]
fn create_task_with_mcp_action_persists() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("mcp-create", false);
    payload["mcp_action"] = json!("maenifold.gate.gate_Status");
    payload["mcp_parameters"] = json!({"root": "/workspace"});
    let fake_bin = PathBuf::from("/nonexistent/binary");

    create_task_with_bin(&context, &payload, &fake_bin).expect("create mcp schedule");

    let persisted = schedule_store(&context)
        .read("mcp-create")
        .expect("read schedule");
    assert_eq!(
        persisted.mcp_action.as_deref(),
        Some("maenifold.gate.gate_Status")
    );
    assert_eq!(
        persisted.mcp_parameters,
        Some(json!({"root": "/workspace"}))
    );
}

#[test]
fn create_task_invalid_mcp_action_format_rejected() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("mcp-invalid", false);
    payload["mcp_action"] = json!("no-dot-here");
    let fake_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &fake_bin);

    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message)) if message.contains("domain.command")
    ));
}

#[test]
fn update_task_clears_mcp_action_when_null() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("mcp-clear", false);
    payload["mcp_action"] = json!("maenifold.gate.gate_Status");
    payload["mcp_parameters"] = json!({"root": "/workspace"});
    let fake_bin = PathBuf::from("/nonexistent/binary");
    create_task_with_bin(&context, &payload, &fake_bin).expect("create mcp schedule");

    update_task(
        &context,
        &json!({"action": "update", "id": "mcp-clear", "mcp_action": null}),
    )
    .expect("clear mcp action");

    let persisted = schedule_store(&context)
        .read("mcp-clear")
        .expect("read schedule");
    assert_eq!(persisted.mcp_action, None);
    assert_eq!(persisted.mcp_parameters, None);
}

#[test]
fn parameters_must_be_object_when_present() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("mcp-array", false);
    payload["mcp_action"] = json!("maenifold.gate.gate_Status");
    payload["mcp_parameters"] = json!([1, 2, 3]);
    let fake_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &fake_bin);

    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message))
            if message == "mcp_parameters must be a JSON object (or omitted)"
    ));
}

#[test]
fn create_task_rejects_unallowed_mcp_action() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let mut payload = create_payload("mcp-acp", false);
    payload["mcp_action"] = json!("maenifold.acp.AcpExec");
    let fake_bin = PathBuf::from("/nonexistent/binary");

    let result = create_task_with_bin(&context, &payload, &fake_bin);

    assert!(matches!(
        result,
        Err(MaenifoldError::InvalidArgument(message))
            if message.contains("is not on the schedule allowlist")
                && message.contains("desktop Sessions/ACP inspector")
    ));
}

#[test]
fn create_task_rejects_inference_actions() {
    let temp = TempDir::new().expect("temp dir");
    let context = test_context(&temp);
    let fake_bin = PathBuf::from("/nonexistent/binary");

    for action in [
        "maenifold.thinking.SequentialThinking",
        "maenifold.system.GatewayIntent",
        "maenifold.workflow.Workflow",
    ] {
        let mut payload = create_payload(
            &format!("reject-{}", action.rsplit('.').next().expect("command")),
            false,
        );
        payload["mcp_action"] = json!(action);
        let result = create_task_with_bin(&context, &payload, &fake_bin);
        assert!(matches!(
            result,
            Err(MaenifoldError::InvalidArgument(message))
                if message.contains("is not on the schedule allowlist")
        ));
    }
}
