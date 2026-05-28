use super::*;
use crate::acp_config::IntentResolverSettings;
use crate::config::Config;
use crate::endophasia::{ConceptEvent, SignalType};
use crate::markdown::read_markdown;
use crate::mcp::resource_routing::ResourceRoutingRegistry;
use crate::mcp::stdio_pool::StdioPool;
use crate::memory::write_memory;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use rmcp::model::CallToolResult;
use serde_json::{Value, json};
use serial_test::serial;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast::error::TryRecvError;

async fn state() -> GatewayState {
    let pool = Arc::new(
        StdioPool::start(
            Vec::<crate::mcp::gateway_session::GatewayServerConfig>::new(),
            std::sync::Arc::new(crate::mcp::PendingRequestRegistry::new()),
        )
        .await
        .expect("empty pool"),
    );
    let registry = GatewayRegistry::build(Vec::new(), &BTreeMap::new()).expect("registry");
    let resources = Arc::new(ResourceRoutingRegistry::build(pool.as_ref()).expect("resources"));
    GatewayState::new(
        pool,
        resources,
        registry,
        IntentResolverSettings::default(),
        PathBuf::from("."),
    )
}

fn text(result: &CallToolResult) -> String {
    result.content[0]
        .raw
        .as_text()
        .expect("text content")
        .text
        .clone()
}

fn json_text(result: &CallToolResult) -> Value {
    serde_json::from_str(&text(result)).expect("json result")
}

async fn memory_state() -> (TempDir, ConfigResetGuard, Config, GatewayState) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::write(
        config.maenifold_root.join("maenifold.json"),
        r#"{"agents":{},"settings":{}}"#,
    )
    .expect("config");
    let state = state().await;
    (tmp, ConfigResetGuard, config, state)
}

fn memory_context() -> (TempDir, ConfigResetGuard, Config, ToolContext) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::write(
        config.maenifold_root.join("maenifold.json"),
        r#"{"agents":{},"settings":{}}"#,
    )
    .expect("config");
    let context = ToolContext::new(config.clone(), OutputFormat::Json);
    (tmp, ConfigResetGuard, config, context)
}

fn parsed_output(output: &str) -> Value {
    serde_json::from_str(output).expect("json output")
}

fn drain_concept_events(
    receiver: &mut tokio::sync::broadcast::Receiver<ConceptEvent>,
) -> Vec<ConceptEvent> {
    let mut events = Vec::new();
    loop {
        match receiver.try_recv() {
            Ok(event) => events.push(event),
            Err(TryRecvError::Empty | TryRecvError::Closed) => return events,
            Err(TryRecvError::Lagged(_)) => continue,
        }
    }
}

async fn gateway_write_memory(
    state: &GatewayState,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": parameters
        }))
        .await
}

#[tokio::test]
async fn gateway_execute_lists_all_missing_required_native_parameters() {
    let state = state().await;

    let err = state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": {}
        }))
        .await
        .expect_err("missing required parameters should fail");

    let message = err.to_string();
    assert!(
        message.contains("Missing required parameter(s)"),
        "{message}"
    );
    assert!(
        message.contains("maenifold.memory/WriteMemory"),
        "{message}"
    );
    assert!(message.contains("content"), "{message}");
    assert!(message.contains("title"), "{message}");
    assert!(
        message.contains("GetHelp toolName=\"WriteMemory\""),
        "{message}"
    );
    assert!(!message.contains("uri_or_path"), "{message}");
}

#[tokio::test]
async fn gateway_execute_missing_required_message_excludes_non_required_errors() {
    let state = state().await;

    let err = state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": { "title": 7 }
        }))
        .await
        .expect_err("mixed missing/type errors should fail");

    let message = err.to_string();
    assert!(
        message.contains("Missing required parameter(s)"),
        "{message}"
    );
    assert!(message.contains("content"), "{message}");
    assert!(!message.contains("title, content"), "{message}");
    assert!(!message.contains("type"), "{message}");
}

#[tokio::test]
async fn gateway_index_default_catalog_omits_commands_from_domain_entries() {
    let state = state().await;

    let result = state
        .handle_tool_call("GatewayIndex", json!({}), None)
        .await
        .expect("default GatewayIndex should succeed")
        .expect("GatewayIndex should return a result");
    let value = json_text(&result);
    let domains = value["domains"].as_array().expect("domains array");
    let memory_domain = domains
        .iter()
        .find(|domain| domain["name"] == "maenifold.memory")
        .expect("maenifold.memory domain");

    assert!(
        memory_domain.get("commands").is_none(),
        "{memory_domain:#?}"
    );
}

#[tokio::test]
async fn gateway_index_include_commands_embeds_command_summaries_without_schemas() {
    let state = state().await;

    let result = state
        .handle_tool_call("GatewayIndex", json!({ "includeCommands": true }), None)
        .await
        .expect("GatewayIndex includeCommands should succeed")
        .expect("GatewayIndex should return a result");
    let value = json_text(&result);
    let domains = value["domains"].as_array().expect("domains array");
    let memory_domain = domains
        .iter()
        .find(|domain| domain["name"] == "maenifold.memory")
        .expect("maenifold.memory domain");
    let commands = memory_domain["commands"]
        .as_array()
        .expect("commands array");
    let read_memory = commands
        .iter()
        .find(|command| command["name"] == "ReadMemory")
        .expect("ReadMemory command");

    assert!(
        read_memory["description"]
            .as_str()
            .is_some_and(|description| !description.is_empty()),
        "{read_memory:#?}"
    );
    assert!(read_memory.get("schema").is_none(), "{read_memory:#?}");
    assert!(
        commands
            .iter()
            .all(|command| command.get("schema").is_none())
    );
}

#[tokio::test]
async fn gateway_execute_reports_overwrite_type_error_before_write_memory_business_validation() {
    let state = state().await;

    let outcome = state
        .handle_gateway_execute(json!({
            "domain": "maenifold.memory",
            "command": "WriteMemory",
            "parameters": {
                "title": "x",
                "content": "x",
                "overwrite": "banana"
            }
        }))
        .await;

    let message = match outcome {
        Err(err) => err.to_string(),
        Ok(result) => result.content[0]
            .raw
            .as_text()
            .expect("text content")
            .text
            .clone(),
    };
    let lower = message.to_ascii_lowercase();

    assert!(lower.contains("overwrite"), "{message}");
    assert!(
        lower.contains("bool") || lower.contains("boolean"),
        "{message}"
    );
    assert!(
        !message.contains("Content must contain at least one"),
        "{message}"
    );
}

#[tokio::test]
#[serial]
async fn gateway_execute_write_memory_default_overwrite_rejects_existing_memory() {
    let (_tmp, _guard, config, state) = memory_state().await;
    let title = "overwrite default";
    let uri = "memory://overwrite-default";

    let created = gateway_write_memory(
        &state,
        json!({
            "title": title,
            "content": "Original [[Concept]]"
        }),
    )
    .await
    .expect("initial write should succeed");
    assert!(!created.is_error.unwrap_or(false), "{}", text(&created));

    let replaced = gateway_write_memory(
        &state,
        json!({
            "title": title,
            "content": "Replacement [[Concept]]"
        }),
    )
    .await
    .expect("second write should return tool result");

    assert!(replaced.is_error.unwrap_or(false), "{}", text(&replaced));

    let (_, content, _) = read_markdown(uri, &config).expect("read original memory");
    assert_eq!(content, "# overwrite default\n\nOriginal [[Concept]]");
}

#[test]
#[serial]
fn write_memory_overwrite_false_rejects_existing_memory() {
    let (_tmp, _guard, config, context) = memory_context();
    let title = "overwrite false";
    let uri = "memory://overwrite-false";
    let created_payload = json!({
        "title": title,
        "content": "Original [[Concept]]"
    });

    let created = write_memory(&context, &created_payload).expect("initial write should succeed");
    assert_eq!(parsed_output(&created)["success"], true, "{created}");
    let replaced_payload = json!({
        "title": title,
        "content": "Replacement [[Concept]]",
        "overwrite": false
    });

    let replaced = write_memory(&context, &replaced_payload)
        .expect("second write should return json error payload");

    assert_eq!(parsed_output(&replaced)["success"], false, "{replaced}");

    let (_, content, _) = read_markdown(uri, &config).expect("read original memory");
    assert_eq!(content, "# overwrite false\n\nOriginal [[Concept]]");
}

#[test]
#[serial]
fn write_memory_overwrite_true_replaces_existing_memory() {
    let (_tmp, _guard, config, context) = memory_context();
    let title = "overwrite true";
    let uri = "memory://overwrite-true";
    let created_payload = json!({
        "title": title,
        "content": "Original [[Concept]]"
    });

    let created = write_memory(&context, &created_payload).expect("initial write should succeed");
    assert_eq!(parsed_output(&created)["success"], true, "{created}");
    let replaced_payload = json!({
        "title": title,
        "content": "Replacement [[Concept]]",
        "overwrite": true
    });

    let replaced = write_memory(&context, &replaced_payload)
        .expect("overwrite=true should return success payload");

    assert_eq!(parsed_output(&replaced)["success"], true, "{replaced}");

    let (_, content, _) = read_markdown(uri, &config).expect("read replaced memory");
    assert_eq!(content, "# overwrite true\n\nReplacement [[Concept]]");
}

#[test]
#[serial]
fn write_memory_rejected_duplicate_overwrite_false_emits_no_writing_event() {
    let (_tmp, _guard, _config, context) = memory_context();
    let mut receiver = crate::endophasia::channel::subscribe();
    let original_payload = json!({
        "title": "overwrite false endophasia",
        "content": "Original [[OriginalConcept]]"
    });

    let created = write_memory(&context, &original_payload).expect("initial write should succeed");
    assert_eq!(parsed_output(&created)["success"], true, "{created}");
    let initial_events = drain_concept_events(&mut receiver);
    assert!(
        initial_events
            .iter()
            .any(|event| matches!(event.signal, SignalType::Writing)
                && event.concept == "originalconcept"),
        "{initial_events:?}"
    );

    let duplicate_payload = json!({
        "title": "overwrite false endophasia",
        "content": "Rejected [[RejectedConcept]]",
        "overwrite": false
    });
    let rejected = write_memory(&context, &duplicate_payload)
        .expect("duplicate write should return error payload");
    assert_eq!(parsed_output(&rejected)["success"], false, "{rejected}");

    let duplicate_events = drain_concept_events(&mut receiver);
    assert!(duplicate_events.is_empty(), "{duplicate_events:?}");
}

#[test]
#[serial]
fn write_memory_overwrite_null_is_type_error() {
    let (_tmp, _guard, config, context) = memory_context();
    let payload = json!({
        "title": "overwrite null",
        "content": "Null overwrite [[Concept]]",
        "overwrite": null
    });

    let output =
        write_memory(&context, &payload).expect("write_memory should return a JSON payload");
    let parsed = parsed_output(&output);

    assert_eq!(parsed["success"], false, "{output}");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("Boolean"),
        "{output}"
    );
    assert!(
        read_markdown("memory://overwrite-null", &config).is_err(),
        "{output}"
    );
}
