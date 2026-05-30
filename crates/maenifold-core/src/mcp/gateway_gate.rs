use crate::mcp::gateway_registry::CommandEntry;
use maenifold_gate_core::lifecycle::{
    CreateTaskArgs, GateContext, GateLifecycle, StageNoteArgs, StatusArgs, SyncArgs, TaskArgs,
};
use maenifold_gate_core::status_filter::StatusFilter;
use rmcp::model::{CallToolResult, Content};
use serde_json::{Map, Value};
use std::path::PathBuf;

#[path = "gateway_gate_schema.rs"]
mod gateway_gate_schema;
use gateway_gate_schema::{create_schema, note_schema, status_schema, sync_schema, task_schema};

pub(crate) const DOMAIN_NAME: &str = "maenifold.gate";
pub(crate) const DOMAIN_DESCRIPTION: &str =
    "Native Maenifold gate task lifecycle commands backed by markdown task files.";

pub(crate) fn commands() -> Vec<CommandEntry> {
    vec![
        command(
            "gate_Create",
            "Create a gate task markdown file and sync gate state.",
            create_schema(),
            "mutate",
        ),
        command(
            "gate_Status",
            "List gate task status.",
            status_schema(),
            "read",
        ),
        command(
            "gate_Append",
            "Append notes to the active task stage.",
            note_schema(),
            "mutate",
        ),
        command(
            "gate_Pass",
            "Pass the active task stage.",
            note_schema(),
            "mutate",
        ),
        command(
            "gate_Fail",
            "Fail the active task stage.",
            note_schema(),
            "mutate",
        ),
        command(
            "gate_Reopen",
            "Reopen a task that was manually moved from done/ back to todo/.",
            note_schema(),
            "mutate",
        ),
        command(
            "gate_History",
            "Read gate task stage history.",
            task_schema(),
            "read",
        ),
        command(
            "gate_TaskBody",
            "Read a gate task markdown body.",
            task_schema(),
            "read",
        ),
        command(
            "gate_Sync",
            "Sync gate markdown state.",
            sync_schema(),
            "mutate",
        ),
    ]
}

pub(crate) async fn invoke_gate(
    command: String,
    parameters: Value,
) -> std::result::Result<CallToolResult, rmcp::ErrorData> {
    let display_name = command.clone();
    let result = tokio::task::spawn_blocking(move || call_gate(&command, parameters))
        .await
        .map_err(|err| rmcp::ErrorData::internal_error(err.to_string(), None))?;
    match result {
        Ok(value) => Ok(CallToolResult::success(vec![Content::text(render_json(
            value,
        ))])),
        Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
            "Error invoking {display_name}: {err}"
        ))])),
    }
}

fn call_gate(
    command: &str,
    parameters: Value,
) -> Result<Value, maenifold_gate_core::error::GateError> {
    let args = object(parameters)?;
    match command {
        "gate_Create" => GateLifecycle::create(create_args(&args)?),
        "gate_Status" => GateLifecycle::status(StatusArgs {
            context: context(&args)?,
            filter: StatusFilter {
                state: optional_string(&args, "state")?,
                ready_only: optional_bool(&args, "ready")?.unwrap_or(false),
                blocked_only: optional_bool(&args, "blocked")?.unwrap_or(false),
                include_done: optional_bool(&args, "includeDone")?.unwrap_or(false),
            },
            summary: optional_bool(&args, "summary")?.unwrap_or(false),
        }),
        "gate_Append" => GateLifecycle::append(note_args(&args)?),
        "gate_Pass" => GateLifecycle::pass(note_args(&args)?),
        "gate_Fail" => GateLifecycle::fail(note_args(&args)?),
        "gate_Reopen" => GateLifecycle::reopen(note_args(&args)?),
        "gate_History" => GateLifecycle::history(task_args(&args)?),
        "gate_TaskBody" => GateLifecycle::task_body(task_args(&args)?),
        "gate_Sync" => GateLifecycle::sync(SyncArgs {
            context: context(&args)?,
            rebuild: optional_bool(&args, "rebuild")?.unwrap_or(false),
        }),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "unknown gate Gateway command: {command}"
        ))),
    }
}

fn command(name: &str, description: &str, schema: Value, safety_class: &str) -> CommandEntry {
    CommandEntry {
        name: name.to_string(),
        description: description.to_string(),
        schema,
        safety_class: safety_class.to_string(),
        meta: None,
    }
}

fn context(
    args: &Map<String, Value>,
) -> Result<GateContext, maenifold_gate_core::error::GateError> {
    Ok(GateContext {
        root: PathBuf::from(required_string(args, "root")?),
        requested_pipeline_name: optional_string(args, "pipeline")?,
    })
}

fn note_args(
    args: &Map<String, Value>,
) -> Result<StageNoteArgs, maenifold_gate_core::error::GateError> {
    Ok(StageNoteArgs {
        context: context(args)?,
        task_id: required_string(args, "taskId")?,
        agent: required_string(args, "agent")?,
        notes: required_string(args, "notes")?,
        model: optional_string(args, "model")?,
        expected_stage: optional_string(args, "expectedStage")?,
    })
}

fn create_args(
    args: &Map<String, Value>,
) -> Result<CreateTaskArgs, maenifold_gate_core::error::GateError> {
    Ok(CreateTaskArgs {
        context: GateContext {
            root: PathBuf::from(required_string(args, "root")?),
            requested_pipeline_name: Some(required_string(args, "pipeline")?),
        },
        task_id: required_json_string(args, "taskId")?,
        title: required_json_string(args, "title")?,
        body: optional_string(args, "body")?,
        priority: optional_string(args, "priority")?,
        sprint: optional_u64(args, "sprint")?,
        parent: optional_string(args, "parent")?,
    })
}

fn task_args(args: &Map<String, Value>) -> Result<TaskArgs, maenifold_gate_core::error::GateError> {
    Ok(TaskArgs {
        context: context(args)?,
        task_id: required_string(args, "taskId")?,
    })
}

fn object(parameters: Value) -> Result<Map<String, Value>, maenifold_gate_core::error::GateError> {
    match parameters {
        Value::Object(map) => Ok(map),
        _ => Err(maenifold_gate_core::error::GateError::Cli(
            "gate Gateway parameters must be a JSON object".to_string(),
        )),
    }
}

fn required_string(
    args: &Map<String, Value>,
    key: &str,
) -> Result<String, maenifold_gate_core::error::GateError> {
    match args.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.to_string()),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "missing required string parameter '{key}'"
        ))),
    }
}

fn required_json_string(
    args: &Map<String, Value>,
    key: &str,
) -> Result<String, maenifold_gate_core::error::GateError> {
    match args.get(key).and_then(Value::as_str) {
        Some(value) => Ok(value.to_string()),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "missing required string parameter '{key}'"
        ))),
    }
}

fn optional_string(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, maenifold_gate_core::error::GateError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "parameter '{key}' must be a string"
        ))),
    }
}

fn optional_bool(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<bool>, maenifold_gate_core::error::GateError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "parameter '{key}' must be a boolean"
        ))),
    }
}

fn optional_u64(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<u64>, maenifold_gate_core::error::GateError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value.as_u64().map(Some).ok_or_else(|| {
            maenifold_gate_core::error::GateError::Cli(format!(
                "parameter '{key}' must be a non-negative integer"
            ))
        }),
        _ => Err(maenifold_gate_core::error::GateError::Cli(format!(
            "parameter '{key}' must be an integer"
        ))),
    }
}

fn render_json(value: Value) -> String {
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
}
