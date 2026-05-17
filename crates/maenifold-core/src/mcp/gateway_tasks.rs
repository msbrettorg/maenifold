use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::mcp::gateway_catalog::CommandEntry;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{pipeline, wait_for_pipeline_terminal};
use rmcp::model::{CallToolResult, Content};
use serde_json::{Map, Value, json};

const POLL_INTERVAL_MS: u64 = 500;

pub(crate) const DOMAIN_NAME: &str = "maenifold.tasks";
pub(crate) const DOMAIN_DESCRIPTION: &str = "Long-running tool lifecycle: create / poll / block / cancel / list pipeline-backed tasks per the MCP 2025-11-25 tasks spec.";

pub(crate) fn commands() -> Vec<CommandEntry> {
    vec![
        command(
            "TaskCreate",
            "Create and start a pipeline-backed task.",
            create_schema(),
            "orchestration",
        ),
        command(
            "TaskGet",
            "Read current task status without blocking.",
            task_id_schema(),
            "read",
        ),
        command(
            "TaskResult",
            "Wait for a task result.",
            result_schema(),
            "read",
        ),
        command(
            "TaskCancel",
            "Cancel a pipeline-backed task.",
            task_id_schema(),
            "destructive",
        ),
        command(
            "TaskList",
            "List pipeline-backed tasks.",
            list_schema(),
            "read",
        ),
    ]
}

pub(crate) async fn invoke_tasks(
    command: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let display_name = command.clone();
    match call_tasks(&command, parameters).await {
        Ok(value) => Ok(CallToolResult::success(vec![Content::text(render_json(
            value,
        ))])),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
            "Error invoking {display_name}: {err}"
        ))])),
    }
}

async fn call_tasks(command: &str, parameters: Value) -> Result<Value> {
    match command {
        "TaskCreate" => task_create(parameters),
        "TaskGet" => task_get(parameters),
        "TaskResult" => task_result(parameters).await,
        "TaskCancel" => task_cancel(parameters),
        "TaskList" => task_list(parameters),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "unknown tasks Gateway command: {command}"
        ))),
    }
}

fn task_create(parameters: Value) -> Result<Value> {
    let args = object(parameters)?;
    let context = context();
    let mut create_payload = Map::new();
    create_payload.insert(
        "definition".to_string(),
        required_value(&args, "definition")?,
    );
    copy_optional(&args, &mut create_payload, "name");
    let created = pipeline(&context, &Value::Object(create_payload))?;
    let data = envelope_data(&created)?;
    let task_id = required_data_string(&data, "pipelineId")?;
    let mut run_payload = json!({ "sessionId": task_id, "run": true });
    if let Some(prompt) = args.get("prompt") {
        run_payload["prompt"] = prompt.clone();
    }
    pipeline(&context, &run_payload)?;
    Ok(json!({
        "taskId": task_id,
        "pollIntervalMs": POLL_INTERVAL_MS,
        "status": task_status(&pipeline_status(&context, &task_id)?),
        "ttlMs": args.get("ttlMs").cloned().unwrap_or(Value::Null),
    }))
}

fn task_get(parameters: Value) -> Result<Value> {
    let args = object(parameters)?;
    let context = context();
    let task_id = required_string(&args, "taskId")?;
    Ok(json!({
        "taskId": task_id,
        "status": task_status(&pipeline_status(&context, &task_id)?),
        "pollIntervalMs": POLL_INTERVAL_MS,
    }))
}

async fn task_result(parameters: Value) -> Result<Value> {
    let args = object(parameters)?;
    let context = context();
    let task_id = required_string(&args, "taskId")?;
    let timeout_ms = optional_u64(&args, "timeoutMs")?;
    let waited = wait_for_pipeline_terminal(&context, &task_id, timeout_ms).await?;
    let result = envelope_data(&waited)?;
    let status = pipeline_status(&context, &task_id)?;
    Ok(json!({ "taskId": task_id, "status": task_status(&status), "result": result }))
}

fn task_cancel(parameters: Value) -> Result<Value> {
    let args = object(parameters)?;
    let context = context();
    let task_id = required_string(&args, "taskId")?;
    let initial = pipeline_status(&context, &task_id)?;
    if terminal_pipeline_status(&initial) {
        return Ok(json!({
            "taskId": task_id,
            "status": "alreadyTerminal",
            "alreadyTerminal": true,
            "finalStatus": task_status(&initial),
        }));
    }
    pipeline(&context, &json!({ "sessionId": task_id, "cancel": true }))?;
    Ok(json!({ "taskId": task_id, "status": "cancelled" }))
}

fn task_list(parameters: Value) -> Result<Value> {
    let args = object(parameters)?;
    let context = context();
    let offset = optional_cursor(&args)?;
    let limit = optional_u64(&args, "limit")?.unwrap_or(50) as usize;
    let listed = pipeline(&context, &json!({ "list": true }))?;
    let data = envelope_data(&listed)?;
    let pipelines = data["pipelines"].as_array().cloned().unwrap_or_default();
    let tasks = pipelines
        .iter()
        .skip(offset)
        .take(limit)
        .map(task_list_entry)
        .collect::<Vec<_>>();
    let next = (offset + tasks.len() < pipelines.len()).then(|| (offset + tasks.len()).to_string());
    Ok(json!({ "tasks": tasks, "nextCursor": next }))
}

fn task_list_entry(value: &Value) -> Value {
    let task_id = value["pipelineId"].as_str().unwrap_or("").to_string();
    let status = value["status"].as_str().unwrap_or("pending");
    json!({ "taskId": task_id, "status": task_status(status), "pipelineStatus": status })
}

fn pipeline_status(context: &ToolContext, task_id: &str) -> Result<String> {
    let viewed = pipeline(context, &json!({ "sessionId": task_id, "view": true }))?;
    let data = envelope_data(&viewed)?;
    Ok(data["status"].as_str().unwrap_or("pending").to_string())
}

fn context() -> ToolContext {
    ToolContext::new(Config::current(), OutputFormat::Json)
}

fn envelope_data(output: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(output)?;
    Ok(value.get("data").cloned().unwrap_or(Value::Null))
}

fn object(parameters: Value) -> Result<Map<String, Value>> {
    parameters.as_object().cloned().ok_or_else(|| {
        MaenifoldError::InvalidArgument(
            "tasks Gateway parameters must be a JSON object".to_string(),
        )
    })
}

fn required_value(args: &Map<String, Value>, key: &'static str) -> Result<Value> {
    args.get(key)
        .cloned()
        .ok_or(MaenifoldError::MissingProperty(key))
}

fn required_string(args: &Map<String, Value>, key: &'static str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(MaenifoldError::MissingProperty(key))
}

fn required_data_string(data: &Value, key: &'static str) -> Result<String> {
    data.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(MaenifoldError::MissingProperty(key))
}

fn optional_u64(args: &Map<String, Value>, key: &str) -> Result<Option<u64>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| MaenifoldError::InvalidArgument(format!("{key} must be unsigned"))),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "{key} must be unsigned"
        ))),
    }
}

fn optional_cursor(args: &Map<String, Value>) -> Result<usize> {
    match args.get("cursor") {
        None | Some(Value::Null) => Ok(0),
        Some(Value::String(value)) => value.parse::<usize>().map_err(|_| {
            MaenifoldError::InvalidArgument("cursor must be a non-negative offset".to_string())
        }),
        _ => Err(MaenifoldError::InvalidArgument(
            "cursor must be a string".to_string(),
        )),
    }
}

fn copy_optional(from: &Map<String, Value>, to: &mut Map<String, Value>, key: &str) {
    if let Some(value) = from.get(key) {
        to.insert(key.to_string(), value.clone());
    }
}

fn task_status(status: &str) -> &'static str {
    match status {
        "completed" => "completed",
        "failed" | "stuck" => "failed",
        "cancelled" | "abandoned" => "cancelled",
        _ => "working",
    }
}

fn terminal_pipeline_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "abandoned" | "stuck"
    )
}

fn command(name: &str, description: &str, schema: Value, safety_class: &str) -> CommandEntry {
    CommandEntry {
        name: name.to_string(),
        description: description.to_string(),
        schema,
        safety_class: safety_class.to_string(),
    }
}

fn create_schema() -> Value {
    json!({"type":"object","required":["definition"],"properties":{"definition":{"type":"object"},"prompt":{"type":"string"},"name":{"type":"string"},"ttlMs":{"type":"integer","minimum":0}},"additionalProperties":false})
}

fn task_id_schema() -> Value {
    json!({"type":"object","required":["taskId"],"properties":{"taskId":{"type":"string","minLength":1}},"additionalProperties":false})
}

fn result_schema() -> Value {
    json!({"type":"object","required":["taskId"],"properties":{"taskId":{"type":"string","minLength":1},"timeoutMs":{"type":"integer","minimum":0}},"additionalProperties":false})
}

fn list_schema() -> Value {
    json!({"type":"object","properties":{"cursor":{"type":"string"},"limit":{"type":"integer","minimum":1}},"additionalProperties":false})
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}
