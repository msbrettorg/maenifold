use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value};

use crate::cli::{
    AppendArgs, GateContext, HistoryArgs, OutcomeArgs, StatusArgs, SyncArgs, TaskBodyArgs,
};
use crate::error::GateError;
use crate::gate_tool_service::GateToolService;
use crate::status_filter::StatusFilter;
use maenifold_core::McpToolDefinition;

pub(crate) fn to_rmcp_tool(definition: McpToolDefinition) -> Result<Tool, GateError> {
    let schema: Map<String, Value> = match definition.input_schema {
        Value::Object(map) => map,
        _ => {
            return Err(GateError::Cli(
                "tool schema must be a JSON object".to_string(),
            ));
        }
    };
    Ok(Tool::new(
        definition.name,
        definition.description,
        Arc::new(schema),
    ))
}

pub(crate) fn call_gate_tool(
    tool_name: &str,
    arguments: &Value,
    root: Option<&PathBuf>,
) -> Result<String, GateError> {
    let result = match tool_name {
        "status" => GateToolService::status(StatusArgs {
            context: context_from_value(arguments, false, root)?,
            filter: StatusFilter {
                state: optional_string(arguments, "state")?,
                ready_only: optional_bool(arguments, "ready")?.unwrap_or(false),
                include_done: optional_bool(arguments, "includeDone")?.unwrap_or(false),
            },
        })?,
        "append" => GateToolService::append(AppendArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
            agent: required_string(arguments, "agent")?,
            notes: required_string(arguments, "notes")?,
            model: optional_string(arguments, "model")?,
        })?,
        "pass" => GateToolService::pass(OutcomeArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
            agent: required_string(arguments, "agent")?,
            notes: required_string(arguments, "notes")?,
            model: optional_string(arguments, "model")?,
        })?,
        "fail" => GateToolService::fail(OutcomeArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
            agent: required_string(arguments, "agent")?,
            notes: required_string(arguments, "notes")?,
            model: optional_string(arguments, "model")?,
        })?,
        "cancel" => GateToolService::cancel(OutcomeArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
            agent: required_string(arguments, "agent")?,
            notes: required_string(arguments, "notes")?,
            model: optional_string(arguments, "model")?,
        })?,
        "history" => GateToolService::history(HistoryArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
        })?,
        "task_body" => GateToolService::task_body(TaskBodyArgs {
            context: context_from_value(arguments, false, root)?,
            task_id: required_string(arguments, "taskId")?,
        })?,
        "sync" => GateToolService::sync(SyncArgs {
            context: context_from_value(arguments, false, root)?,
            rebuild: optional_bool(arguments, "rebuild")?.unwrap_or(false),
        })?,
        _ => {
            return Err(GateError::Cli(format!(
                "unknown gate MCP tool: {tool_name}"
            )));
        }
    };

    serde_json::to_string_pretty(&result).map_err(GateError::from)
}

pub(crate) fn context_from_value(
    arguments: &Value,
    json_output: bool,
    default_root: Option<&PathBuf>,
) -> Result<GateContext, GateError> {
    let raw_root = match optional_path(arguments, "root")? {
        Some(root) => {
            if root.is_relative() {
                return Err(GateError::Cli(format!(
                    "root path '{}' is relative; pass an absolute path (e.g. '/Users/you/project') \
                     because the MCP server cannot resolve paths relative to the client's working directory",
                    root.display()
                )));
            }
            root
        }
        None => default_root.cloned().ok_or_else(|| {
            GateError::Cli(
                "no root bound to this session; pass 'root' argument on any tool call to bind a workspace"
                    .to_string(),
            )
        })?,
    };
    let root = std::fs::canonicalize(&raw_root).map_err(|error| {
        GateError::Cli(format!(
            "root path '{}' is not accessible: {error}; pass an absolute path to a writable project directory via the 'root' argument",
            raw_root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(GateError::Cli(format!(
            "root path '{}' is not a directory; pass an absolute path to a writable project directory via the 'root' argument",
            root.display()
        )));
    }

    Ok(GateContext {
        root,
        requested_pipeline_name: optional_string(arguments, "pipeline")?,
        json_output,
    })
}

pub(crate) fn optional_path(arguments: &Value, key: &str) -> Result<Option<PathBuf>, GateError> {
    Ok(optional_string(arguments, key)?.map(PathBuf::from))
}

pub(crate) fn required_string(arguments: &Value, key: &str) -> Result<String, GateError> {
    optional_string(arguments, key)?
        .ok_or_else(|| GateError::Cli(format!("missing required MCP argument '{key}'")))
}

pub(crate) fn optional_string(arguments: &Value, key: &str) -> Result<Option<String>, GateError> {
    match arguments.get(key) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(GateError::Cli(format!(
            "MCP argument '{key}' must be a string"
        ))),
    }
}

pub(crate) fn optional_bool(arguments: &Value, key: &str) -> Result<Option<bool>, GateError> {
    match arguments.get(key) {
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(GateError::Cli(format!(
            "MCP argument '{key}' must be a boolean"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        call_gate_tool, context_from_value, optional_bool, optional_string, required_string,
    };

    #[test]
    fn context_from_value_rejects_nonexistent_root() {
        let args = json!({"root": "/nonexistent/path/abc123"});
        let result = context_from_value(&args, false, None);
        let err = result.expect_err("nonexistent root should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("/nonexistent/path/abc123"),
            "error should contain the path: {msg}"
        );
        assert!(msg.contains("root"), "error should mention root: {msg}");
    }

    #[test]
    fn context_from_value_rejects_relative_root() {
        let args = json!({"root": "."});
        let result = context_from_value(&args, false, None);
        let err = result.expect_err("relative root should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("relative"),
            "error should mention relative: {msg}"
        );
        assert!(
            msg.contains("absolute"),
            "error should suggest absolute: {msg}"
        );
    }

    #[test]
    fn context_from_value_rejects_relative_subdir_root() {
        let args = json!({"root": "some/subdir"});
        let result = context_from_value(&args, false, None);
        let err = result.expect_err("relative subdir root should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("relative"),
            "error should mention relative: {msg}"
        );
    }

    #[test]
    fn optional_string_returns_some_for_string() {
        let args = json!({"key": "value"});
        assert_eq!(
            optional_string(&args, "key").expect("parse string"),
            Some("value".to_string())
        );
    }

    #[test]
    fn optional_string_returns_none_for_null() {
        let args = json!({"key": null});
        assert_eq!(optional_string(&args, "key").expect("parse null"), None);
    }

    #[test]
    fn optional_string_returns_none_for_missing() {
        let args = json!({});
        assert_eq!(optional_string(&args, "key").expect("parse missing"), None);
    }

    #[test]
    fn optional_string_rejects_non_string() {
        let args = json!({"key": 42});
        let err = optional_string(&args, "key").expect_err("should reject number");
        assert!(err.to_string().contains("must be a string"));
    }

    #[test]
    fn optional_bool_returns_some_for_bool() {
        let args = json!({"key": true});
        assert_eq!(optional_bool(&args, "key").expect("parse bool"), Some(true));
    }

    #[test]
    fn optional_bool_returns_none_for_missing() {
        let args = json!({});
        assert_eq!(optional_bool(&args, "key").expect("parse missing"), None);
    }

    #[test]
    fn optional_bool_rejects_non_bool() {
        let args = json!({"key": "true"});
        let err = optional_bool(&args, "key").expect_err("should reject string");
        assert!(err.to_string().contains("must be a boolean"));
    }

    #[test]
    fn required_string_returns_value() {
        let args = json!({"key": "value"});
        assert_eq!(
            required_string(&args, "key").expect("parse required"),
            "value"
        );
    }

    #[test]
    fn required_string_rejects_missing() {
        let args = json!({});
        let err = required_string(&args, "key").expect_err("should reject missing");
        assert!(err.to_string().contains("missing required"));
    }

    #[test]
    fn call_gate_tool_rejects_unknown_tool() {
        let args = json!({});
        let err =
            call_gate_tool("nonexistent", &args, None).expect_err("should reject unknown tool");
        assert!(err.to_string().contains("unknown gate MCP tool"));
    }
}
