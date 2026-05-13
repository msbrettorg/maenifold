use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use crate::workflow::pipeline::{pipeline, wait_for_pipeline_terminal};
use chrono::Utc;
use rmcp::ErrorData;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, CancelTaskParams, CancelTaskResult, Content,
    CreateTaskResult, GetTaskInfoParams, GetTaskPayloadResult, GetTaskResult, GetTaskResultParams,
    ListTasksResult, PaginatedRequestParams, Task, TaskStatus,
};
use serde_json::{Map, Value, json};

const POLL_INTERVAL_MS: u64 = 500;

pub(super) async fn enqueue_task(
    request: CallToolRequestParams,
) -> std::result::Result<CreateTaskResult, ErrorData> {
    if request.name.as_ref() != "Pipeline" {
        return Err(ErrorData::invalid_params(
            "Only Pipeline supports task-augmented tools/call".to_string(),
            None,
        ));
    }
    let args = request.arguments.unwrap_or_default();
    let ttl = task_ttl_ms(request.task.as_ref());
    let context = context();
    let task_id = create_and_run_pipeline(&context, args).map_err(map_error)?;
    let status = pipeline_status(&context, &task_id).map_err(map_error)?;
    Ok(CreateTaskResult::new(task(&task_id, &status, ttl)))
}

pub(super) fn get_task_info(
    request: GetTaskInfoParams,
) -> std::result::Result<GetTaskResult, ErrorData> {
    let context = context();
    let view = view_task(&context, &request.task_id).map_err(map_error)?;
    let status = view_status(&view);
    Ok(GetTaskResult {
        meta: None,
        task: task(&request.task_id, status, None).with_status_message(summary_message(&view)),
    })
}

pub(super) async fn get_task_result(
    request: GetTaskResultParams,
) -> std::result::Result<GetTaskPayloadResult, ErrorData> {
    let context = context();
    let waited = wait_for_pipeline_terminal(&context, &request.task_id, None)
        .await
        .map_err(map_error)?;
    let data = envelope_data(&waited).map_err(map_error)?;
    let status = view_status(&data);
    let result = if status == "completed" {
        CallToolResult::success(vec![Content::text(render_json(&data))])
    } else {
        CallToolResult::error(vec![Content::text(render_json(&data))])
    };
    serde_json::to_value(result)
        .map(GetTaskPayloadResult::new)
        .map_err(|err| ErrorData::internal_error(err.to_string(), None))
}

pub(super) fn cancel_task(
    request: CancelTaskParams,
) -> std::result::Result<CancelTaskResult, ErrorData> {
    let context = context();
    let initial = pipeline_status(&context, &request.task_id).map_err(map_error)?;
    let (status, message) = if terminal_pipeline_status(&initial) {
        (initial, "alreadyTerminal".to_string())
    } else {
        pipeline(
            &context,
            &json!({ "sessionId": request.task_id, "cancel": true }),
        )
        .map_err(map_error)?;
        ("cancelled".to_string(), "cancelled".to_string())
    };
    Ok(CancelTaskResult {
        meta: None,
        task: task(&request.task_id, &status, None).with_status_message(message),
    })
}

pub(super) fn list_tasks(
    request: Option<PaginatedRequestParams>,
) -> std::result::Result<ListTasksResult, ErrorData> {
    let context = context();
    let listed = pipeline(&context, &json!({ "list": true })).map_err(map_error)?;
    let data = envelope_data(&listed).map_err(map_error)?;
    let pipelines = data["pipelines"].as_array().cloned().unwrap_or_default();
    let cursor = request
        .as_ref()
        .and_then(|params| params.cursor.as_ref())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = 50usize;
    let tasks = pipelines
        .iter()
        .skip(cursor)
        .take(limit)
        .map(task_from_pipeline_summary)
        .collect::<Vec<_>>();
    let next_cursor =
        (cursor + tasks.len() < pipelines.len()).then(|| (cursor + tasks.len()).to_string());
    let mut result = ListTasksResult::new(tasks);
    result.next_cursor = next_cursor;
    result.total = Some(pipelines.len() as u64);
    Ok(result)
}

fn create_and_run_pipeline(context: &ToolContext, args: Map<String, Value>) -> Result<String> {
    let mut create_payload = Map::new();
    create_payload.insert(
        "definition".to_string(),
        args.get("definition")
            .cloned()
            .ok_or(MaenifoldError::MissingProperty("definition"))?,
    );
    copy_optional(&args, &mut create_payload, "name");
    copy_optional(&args, &mut create_payload, "inputs");
    copy_optional(&args, &mut create_payload, "maxConcurrency");
    let created = pipeline(context, &Value::Object(create_payload))?;
    let data = envelope_data(&created)?;
    let task_id = data
        .get("pipelineId")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("pipelineId"))?
        .to_string();
    let mut run_payload = json!({ "sessionId": task_id, "run": true });
    if let Some(prompt) = args.get("prompt") {
        run_payload["prompt"] = prompt.clone();
    }
    pipeline(context, &run_payload)?;
    Ok(task_id)
}

fn task_from_pipeline_summary(value: &Value) -> Task {
    let task_id = value["pipelineId"].as_str().unwrap_or("").to_string();
    let status = value["status"].as_str().unwrap_or("pending");
    task(&task_id, status, None).with_status_message(summary_message(value))
}

fn task(task_id: &str, pipeline_status: &str, ttl: Option<u64>) -> Task {
    let now = Utc::now().to_rfc3339();
    let mut task = Task::new(
        task_id.to_string(),
        task_status(pipeline_status),
        now.clone(),
        now,
    )
    .with_poll_interval(POLL_INTERVAL_MS);
    if let Some(ttl) = ttl {
        task = task.with_ttl(ttl);
    }
    task
}

fn view_task(context: &ToolContext, task_id: &str) -> Result<Value> {
    let viewed = pipeline(
        context,
        &json!({ "sessionId": task_id, "view": true, "summary": true }),
    )?;
    envelope_data(&viewed)
}

fn pipeline_status(context: &ToolContext, task_id: &str) -> Result<String> {
    Ok(view_status(&view_task(context, task_id)?).to_string())
}

fn view_status(view: &Value) -> &str {
    view.get("status")
        .and_then(Value::as_str)
        .unwrap_or("pending")
}

fn envelope_data(output: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(output)?;
    Ok(value.get("data").cloned().unwrap_or(Value::Null))
}

fn summary_message(view: &Value) -> String {
    let pipeline_id = view
        .get("pipelineId")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!("pipeline {pipeline_id} is {}", view_status(view))
}

fn task_status(status: &str) -> TaskStatus {
    match status {
        "completed" => TaskStatus::Completed,
        "failed" | "stuck" => TaskStatus::Failed,
        "cancelled" | "abandoned" => TaskStatus::Cancelled,
        _ => TaskStatus::Working,
    }
}

fn terminal_pipeline_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "abandoned" | "stuck"
    )
}

fn task_ttl_ms(task: Option<&Map<String, Value>>) -> Option<u64> {
    task.and_then(|task| task.get("ttlMs").or_else(|| task.get("ttl")))
        .and_then(Value::as_u64)
}

fn context() -> ToolContext {
    ToolContext::new(Config::current(), OutputFormat::Json)
}

fn copy_optional(from: &Map<String, Value>, to: &mut Map<String, Value>, key: &str) {
    if let Some(value) = from.get(key) {
        to.insert(key.to_string(), value.clone());
    }
}

fn render_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn map_error(err: MaenifoldError) -> ErrorData {
    if super::tool_utils::is_validation_error(&err) {
        ErrorData::invalid_params(err.to_string(), None)
    } else {
        ErrorData::internal_error(err.to_string(), None)
    }
}
