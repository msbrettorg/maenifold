use super::handle_read_resource;
use super::schedule_consistency_helpers::*;
use crate::mcp::resources::map_resource_error;
use crate::mcp::{MaenifoldClient, MaenifoldHandler, PendingRequestRegistry, TransportKind};
use crate::tools::schedule as schedule_tool;
use rmcp::ServiceExt;
use rmcp::model::{ErrorCode, ReadResourceRequestParams};
use serde_json::json;
use serial_test::serial;
use std::fs;
use std::sync::Arc;

#[test]
#[serial]
fn schedule_tool_and_resources_match_after_mutation_matrix() {
    let (_temp, context, _guard) = test_context();
    let id = "matrix-task";

    let create = call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Initial matrix task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Run consistency matrix",
            "enabled": false
        }),
    );
    assert_eq!(create["action"], "create");
    assert_eq!(
        assert_all_surfaces_agree(&context, id),
        SemanticTask {
            id: id.to_string(),
            description: "Initial matrix task".to_string(),
            cron_expression: "0 9 * * 1-5".to_string(),
            agent: "swe".to_string(),
            prompt: "Run consistency matrix".to_string(),
            enabled: false,
        }
    );

    // Tool update currently synchronizes the OS scheduler even for disabled tasks.
    // Mutating through ScheduleStore keeps this matrix side-effect free while the
    // assertions still compare the public tool get/list and resource surfaces.
    update_fields_with_store(&context, id);
    let updated = assert_all_surfaces_agree(&context, id);
    assert_eq!(updated.description, "Updated matrix task");
    assert_eq!(updated.cron_expression, "30 10 * * 1-5");
    assert_eq!(updated.agent, "blue-team");
    assert_eq!(updated.prompt, "Verify schedule consistency");

    set_enabled_with_store(&context, id, true);
    assert!(assert_all_surfaces_agree(&context, id).enabled);
    set_enabled_with_store(&context, id, false);
    assert!(!assert_all_surfaces_agree(&context, id).enabled);

    let run_now = call_schedule(&context, json!({"action": "run_now", "id": id}));
    assert_eq!(run_now["action"], "run_now");
    assert_eq!(run_now["id"], id);
    let tool_after_run = call_schedule(&context, json!({"action": "get", "id": id}));
    let resource_after_run = resource_json(&context, &format!("schedule://job/{id}"));
    let (tool_last_run, resource_last_run) =
        (&tool_after_run["last_run"], &resource_after_run["last_run"]);
    assert!(!tool_last_run.is_null() && !tool_last_run.as_str().is_some_and(str::is_empty));
    assert!(!resource_last_run.is_null() && !resource_last_run.as_str().is_some_and(str::is_empty));
    assert!(!assert_all_surfaces_agree(&context, id).enabled);

    let schedule_store = store(&context);
    schedule_store
        .delete(id)
        .expect("delete without OS scheduler");
    let tool_get_error = schedule_tool(&context, &json!({"action": "get", "id": id}))
        .expect_err("tool get after delete fails");
    assert!(tool_get_error.to_string().contains("scheduled task"));
    let resource_error = map_resource_error(
        handle_read_resource(&format!("schedule://job/{id}")).expect_err("resource is missing"),
    );
    assert_eq!(resource_error.code, ErrorCode::RESOURCE_NOT_FOUND);
}

#[test]
#[serial]
fn missing_schedule_resources_are_not_found_without_filesystem_paths() {
    let (temp, _context, _guard) = test_context();

    for uri in ["schedule://job/missing-id", "schedule://logs/missing-id"] {
        let error = handle_read_resource(uri).expect_err("missing schedule resource fails");
        let mapped = map_resource_error(error);
        assert_eq!(mapped.code, ErrorCode::RESOURCE_NOT_FOUND);
        let message = mapped.message.to_string();
        assert!(message.contains("scheduled-task/missing-id"));
        assert!(!message.contains(&temp.path().display().to_string()));
        assert!(!message.contains('\\'));
    }
}

#[test]
#[serial]
fn schedule_log_tool_and_resource_share_bounds() {
    let (_temp, context, _guard) = test_context();
    let id = "log-bounds";
    call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Log bounds task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Generate logs",
            "enabled": false
        }),
    );
    write_large_logs(
        &context,
        &store(&context).read(id).expect("read scheduled task"),
    );

    let tool_logs = call_schedule(&context, json!({"action": "logs", "id": id, "limit": 10}));
    let resource_logs = resource_json(&context, &format!("schedule://logs/{id}"));
    assert_eq!(tool_logs, resource_logs);
    assert_logs_within_bounds(&tool_logs);
}

#[test]
#[serial]
fn schedule_logs_resource_matches_tool_after_unrelated_config_override() {
    let (_temp, context, _guard) = test_context();
    let id = "log-bounds";
    call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Log bounds task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Generate logs",
            "enabled": false
        }),
    );
    write_large_logs(
        &context,
        &store(&context).read(id).expect("read scheduled task"),
    );

    let tool_logs = call_schedule(&context, json!({"action": "logs", "id": id, "limit": 10}));

    let (_other_temp, _other_context, _other_guard) = test_context();

    let resource_logs = resource_json(&context, &format!("schedule://logs/{id}"));
    assert_eq!(tool_logs, resource_logs);
}

#[tokio::test]
#[serial]
async fn handler_schedule_resource_uses_constructor_config_after_unrelated_override()
-> anyhow::Result<()> {
    let (_temp, context, _guard) = test_context();
    let id = "handler-log-bounds";
    call_schedule(
        &context,
        json!({
            "action": "create",
            "id": id,
            "description": "Handler log bounds task",
            "schedule": "every weekday at 9am",
            "agent": "swe",
            "prompt": "Generate logs",
            "enabled": false
        }),
    );
    write_large_logs(
        &context,
        &store(&context).read(id).expect("read scheduled task"),
    );
    let tool_logs = call_schedule(&context, json!({"action": "logs", "id": id, "limit": 10}));

    let (_other_temp, _other_context, _other_guard) = test_context();

    let resource_logs = handler_resource_json(&context, &format!("schedule://logs/{id}")).await?;
    assert_eq!(tool_logs, resource_logs);
    Ok(())
}

#[test]
fn read_recent_logs_merges_runrecord_sidecar_fields() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_dir = temp.path().join("logs");
    fs::create_dir_all(&log_dir).expect("create logs");
    fs::write(log_dir.join("20260514T120000Z.log"), "stdout").expect("write log");
    fs::write(
        log_dir.join("20260514T120000Z.json"),
        serde_json::to_string(&json!({
            "status": "success",
            "duration_seconds": 2,
            "mcp_action": "maenifold.gate.gate_Status",
            "mcp_parameters": {"root": "/workspace"},
            "mcp_result": null
        }))
        .expect("serialize sidecar"),
    )
    .expect("write sidecar");

    let logs = super::read_recent_logs(&log_dir, 10).expect("read logs");

    assert_eq!(logs[0]["fileName"], "20260514T120000Z.log");
    assert_eq!(logs[0]["status"], "success");
    assert_eq!(logs[0]["duration_seconds"], 2);
    assert_eq!(logs[0]["mcp_action"], "maenifold.gate.gate_Status");
    assert_eq!(logs[0]["mcp_result"], serde_json::Value::Null);
}

async fn handler_resource_json(
    context: &crate::tools::ToolContext,
    uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let (server_transport, client_transport) = tokio::io::duplex(2 * 1024 * 1024);
    let handler = MaenifoldHandler::new_with_config(TransportKind::Stdio, context.config.clone())?;
    let server_task = tokio::spawn(async move {
        let server = handler.serve(server_transport).await?;
        server.waiting().await?;
        anyhow::Ok(())
    });

    let mut client = MaenifoldClient::new(Arc::new(PendingRequestRegistry::new()))
        .serve(client_transport)
        .await?;
    let result = client
        .peer()
        .read_resource(ReadResourceRequestParams::new(uri))
        .await?;
    client.close().await?;
    server_task.await??;
    Ok(resource_result_json(result))
}
