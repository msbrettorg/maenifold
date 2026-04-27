use crate::error::JSONRPC_INVALID_PARAMS;
use crate::mcp::gateway_batch_types::{BatchItem, BatchItemResult, BatchResult, BatchState};
use crate::mcp::gateway_catalog::CommandTarget;
use crate::mcp::gateway_dispatch::validate_dispatch_parameters;
use crate::mcp::gateway_handler::GatewayState;
use crate::mcp::gateway_schema_validation::validate_parameters;
use crate::mcp::schema::{
    gateway_tools::{BatchItemParams, GatewayBatchParams},
    mcp_tool_from_registry_name,
};
use rmcp::model::{CallToolResult, Content, ErrorCode};
use serde_json::{Map, Value, json};
const MAX_BATCH_ITEMS: usize = 100;

pub(crate) async fn handle_gateway_batch_arguments(
    state: &GatewayState,
    batch_state: &BatchState,
    arguments: Value,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let params: GatewayBatchParams = serde_json::from_value(arguments)
        .map_err(|err| invalid_params(format!("Invalid GatewayBatch arguments: {err}")))?;
    handle_gateway_batch(state, batch_state, params).await
}

pub(crate) async fn handle_gateway_batch(
    state: &GatewayState,
    batch_state: &BatchState,
    params: GatewayBatchParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    match params.operation.as_str() {
        "submit" => submit_batch(state, batch_state, params).await,
        "status" => batch_status(batch_state, params),
        other => Err(invalid_params(format!(
            "GatewayBatch operation must be 'submit' or 'status', got '{other}'."
        ))),
    }
}

async fn submit_batch(
    state: &GatewayState,
    batch_state: &BatchState,
    params: GatewayBatchParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let continue_on_error = params
        .continue_on_error
        .ok_or_else(|| invalid_params("GatewayBatch submit requires continueOnError."))?;
    let items = params
        .items
        .ok_or_else(|| invalid_params("GatewayBatch submit requires items."))?
        .into_iter()
        .map(batch_item)
        .collect::<Vec<_>>();
    if items.len() > MAX_BATCH_ITEMS {
        return Err(invalid_params(format!(
            "GatewayBatch submit accepts at most {MAX_BATCH_ITEMS} items."
        )));
    }

    for item in &items {
        validate_item(state, item)?;
    }

    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut results = Vec::with_capacity(items.len());
    let mut failed = false;
    for item in items {
        if failed && !continue_on_error {
            results.push(skipped_item(item));
            continue;
        }
        match execute_item(state, &item).await {
            Ok(result) => {
                let success = !result.is_error.unwrap_or(false);
                failed |= !success;
                results.push(result_item(item, success, tool_result_value(&result), None));
            }
            Err(error) => {
                failed = true;
                results.push(result_item(
                    item,
                    false,
                    Value::Null,
                    Some(error_value(error)),
                ));
            }
        }
    }

    let result = BatchResult {
        batch_id,
        status: if failed { "failed" } else { "completed" }.to_string(),
        items: results,
    };
    batch_state.insert(result.clone());
    Ok(render_result(result))
}

fn batch_status(
    batch_state: &BatchState,
    params: GatewayBatchParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let batch_id = params
        .batch_id
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| invalid_params("GatewayBatch status requires batchId."))?;
    batch_state
        .get(&batch_id)
        .map(render_result)
        .ok_or_else(|| batch_not_found(&batch_id))
}

fn batch_item(item: BatchItemParams) -> BatchItem {
    BatchItem {
        domain: item.domain,
        command: item.command,
        parameters: item.parameters,
    }
}

fn validate_item(state: &GatewayState, item: &BatchItem) -> Result<(), rmcp::ErrorData> {
    if item.parameters.contains_key("intent") {
        return Err(invalid_params(format!(
            "GatewayBatch item '{}.{}' must not include natural-language intent.",
            item.domain, item.command
        )));
    }
    match state
        .catalog
        .resolve(&item.domain, &item.command)
        .map_err(|err| invalid_params(err.to_string()))?
    {
        CommandTarget::Native { tool_name } => validate_native(&tool_name, &item.parameters),
        CommandTarget::Gate { .. } => Ok(()),
        CommandTarget::Upstream {
            server_name,
            tool_name,
        } => {
            let server = state.consolidated_server(&server_name)?;
            validate_dispatch_parameters(
                server,
                &tool_name,
                &Value::Object(item.parameters.clone()),
            )
            .map_err(|err| invalid_params(err.to_string()))
        }
    }
}

fn validate_native(
    tool_name: &str,
    parameters: &Map<String, Value>,
) -> Result<(), rmcp::ErrorData> {
    let definition =
        mcp_tool_from_registry_name(tool_name).map_err(|err| invalid_params(err.to_string()))?;
    validate_parameters(
        tool_name,
        &Value::Object(parameters.clone()),
        &definition.input_schema,
    )
    .map_err(|err| invalid_params(err.to_string()))
}

async fn execute_item(
    state: &GatewayState,
    item: &BatchItem,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let mut args = Map::new();
    args.insert("domain".to_string(), Value::String(item.domain.clone()));
    args.insert("command".to_string(), Value::String(item.command.clone()));
    args.insert(
        "parameters".to_string(),
        Value::Object(item.parameters.clone()),
    );
    state.handle_gateway_execute(Value::Object(args)).await
}

fn result_item(
    item: BatchItem,
    success: bool,
    result: Value,
    error: Option<Value>,
) -> BatchItemResult {
    BatchItemResult {
        domain: item.domain,
        command: item.command,
        status: if success { "completed" } else { "failed" }.to_string(),
        success,
        result: success.then_some(result),
        error,
    }
}

fn skipped_item(item: BatchItem) -> BatchItemResult {
    BatchItemResult {
        domain: item.domain,
        command: item.command,
        status: "skipped".to_string(),
        success: false,
        result: None,
        error: Some(json!({ "message": "Skipped after a prior batch item failed." })),
    }
}

fn render_result(result: BatchResult) -> CallToolResult {
    let value = json!(result);
    CallToolResult::success(vec![Content::text(
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
    )])
}

fn tool_result_value(result: &CallToolResult) -> Value {
    serde_json::to_value(result)
        .unwrap_or_else(|err| json!({ "serializationError": err.to_string() }))
}

fn error_value(error: rmcp::ErrorData) -> Value {
    json!({ "message": error.to_string() })
}

fn invalid_params(message: impl Into<String>) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(message.into(), None)
}

fn batch_not_found(batch_id: &str) -> rmcp::ErrorData {
    rmcp::ErrorData::new(
        ErrorCode(JSONRPC_INVALID_PARAMS as i32),
        format!("GatewayBatch batchId '{batch_id}' was not found."),
        Some(json!({ "kind": "batchNotFound", "batchId": batch_id })),
    )
}

#[cfg(test)]
#[path = "gateway_batch_tests.rs"]
mod gateway_batch_tests;
