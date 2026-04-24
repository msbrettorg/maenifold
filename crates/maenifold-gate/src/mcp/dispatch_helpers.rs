use std::path::PathBuf;

use serde_json::Value;

use crate::cli::GateContext;
use crate::error::GateError;

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
