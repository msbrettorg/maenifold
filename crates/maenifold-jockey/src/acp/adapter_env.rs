use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// Cross-crate contract: any failure_reason string that starts with this prefix
/// is recognized by `maenifold-pipeline-driver`'s recovery layer and promoted to
/// `NodeStatus::Stuck` (retriable) instead of remaining `NodeStatus::Abandoned` (terminal).
///
/// The pipeline-driver consumer is `crates/maenifold-pipeline-driver/src/recovery_preflight.rs`.
/// Both sides MUST reference this constant; do not change the value without updating both
/// sides AND the cross-crate test in `adapter_env_tests.rs`.
pub const PREFLIGHT_ENV_VALIDATION_FAILED_PREFIX: &str = "preflight env validation failed";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnvValidationFailure {
    pub var_name: String,
    pub source: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EnvValidationError {
    pub failures: Vec<EnvValidationFailure>,
}

pub(super) fn validate_env(entry: &Value) -> Result<Vec<(String, String)>, EnvValidationError> {
    let mut resolved = Vec::new();
    let mut failures = Vec::new();

    if let Some(commands) = entry.get("envFromCommand").and_then(Value::as_object) {
        for item in commands {
            match validate_command_entry(item) {
                Ok(value) => resolved.push(value),
                Err(failure) => failures.push(failure),
            }
        }
    }
    if let Some(files) = entry.get("envFromFile").and_then(Value::as_object) {
        for item in files {
            match validate_file_entry(item) {
                Ok(Some(value)) => resolved.push(value),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }
    }

    if failures.is_empty() {
        Ok(resolved)
    } else {
        Err(EnvValidationError { failures })
    }
}

fn validate_command_entry(
    (var_name, spec): (&String, &Value),
) -> Result<(String, String), EnvValidationFailure> {
    let (command, args) = command_spec(spec).ok_or_else(|| EnvValidationFailure {
        var_name: var_name.clone(),
        source: spec.to_string(),
        reason: "missing command".to_string(),
    })?;

    let output = match Command::new(&command).args(&args).output() {
        Ok(output) => output,
        Err(error) => {
            return Err(EnvValidationFailure {
                var_name: var_name.clone(),
                source: command,
                reason: format!("failed to run command: {error}"),
            });
        }
    };
    if !output.status.success() {
        return Err(EnvValidationFailure {
            var_name: var_name.clone(),
            source: command,
            reason: format!("command exited with status {:?}", output.status.code()),
        });
    }
    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(error) => {
            return Err(EnvValidationFailure {
                var_name: var_name.clone(),
                source: command,
                reason: format!("stdout was not UTF-8: {error}"),
            });
        }
    };
    let value = trim_trailing_newlines(&stdout);
    if value.is_empty() {
        return Err(EnvValidationFailure {
            var_name: var_name.clone(),
            source: command,
            reason: "stdout was empty".to_string(),
        });
    }
    Ok((var_name.clone(), value.to_string()))
}

fn validate_file_entry(
    (var_name, path_value): (&String, &Value),
) -> Result<Option<(String, String)>, EnvValidationFailure> {
    let Some((path, optional)) = file_spec(path_value) else {
        return Err(EnvValidationFailure {
            var_name: var_name.clone(),
            source: path_value.to_string(),
            reason: "path was not a string".to_string(),
        });
    };
    let path_buf = match expand_home(path) {
        Ok(path_buf) => path_buf,
        Err(error) => {
            return Err(EnvValidationFailure {
                var_name: var_name.clone(),
                source: path.to_string(),
                reason: format!("path could not be resolved: {error}"),
            });
        }
    };
    let contents = match std::fs::read_to_string(&path_buf) {
        Ok(contents) => contents,
        Err(error) => {
            if optional {
                return Ok(None);
            }
            return Err(EnvValidationFailure {
                var_name: var_name.clone(),
                source: path_buf.display().to_string(),
                reason: format!("file not found or unreadable: {error}"),
            });
        }
    };
    let value = trim_trailing_newlines(&contents);
    if value.is_empty() {
        return Err(EnvValidationFailure {
            var_name: var_name.clone(),
            source: path_buf.display().to_string(),
            reason: "file content was empty".to_string(),
        });
    }
    Ok(Some((var_name.clone(), value.to_string())))
}

fn command_spec(spec: &Value) -> Option<(String, Vec<String>)> {
    if let Some(command_line) = spec.as_str() {
        let mut parts = command_line.split_whitespace();
        let command = parts.next()?.to_string();
        return Some((command, parts.map(ToString::to_string).collect()));
    }
    let command = spec.get("command").and_then(Value::as_str)?.to_string();
    let args = spec
        .get("args")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect();
    Some((command, args))
}

fn file_spec(spec: &Value) -> Option<(&str, bool)> {
    if let Some(path) = spec.as_str() {
        return Some((path, false));
    }
    let path = spec.get("path").and_then(Value::as_str)?;
    let optional = spec
        .get("optional")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some((path, optional))
}

fn expand_home(path: &str) -> Result<PathBuf, String> {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|error| format!("HOME not available: {error}"))?;
        return Ok(PathBuf::from(home).join(rest));
    }
    Ok(PathBuf::from(path))
}

fn trim_trailing_newlines(value: &str) -> &str {
    value.trim_end_matches(|ch| ch == '\n' || ch == '\r')
}
