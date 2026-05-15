use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

pub(super) fn env_from_command(entry: &Value) -> Vec<(String, String)> {
    entry
        .get("envFromCommand")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(resolve_command_entry)
        .collect()
}

pub(super) fn env_from_file(entry: &Value) -> Vec<(String, String)> {
    entry
        .get("envFromFile")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(resolve_file_entry)
        .collect()
}

fn resolve_command_entry((var_name, spec): (&String, &Value)) -> Option<(String, String)> {
    let Some(command) = spec.get("command").and_then(Value::as_str) else {
        tracing::warn!(var_name = %var_name, "envFromCommand entry missing command");
        return None;
    };
    let args = spec
        .get("args")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);

    let output = match Command::new(command).args(args).output() {
        Ok(output) => output,
        Err(error) => {
            tracing::warn!(
                var_name = %var_name,
                command = %command,
                error = %error,
                "envFromCommand failed to run"
            );
            return None;
        }
    };
    if !output.status.success() {
        tracing::warn!(
            var_name = %var_name,
            command = %command,
            status = ?output.status.code(),
            "envFromCommand exited unsuccessfully"
        );
        return None;
    }
    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(error) => {
            tracing::warn!(
                var_name = %var_name,
                command = %command,
                error = %error,
                "envFromCommand stdout was not UTF-8"
            );
            return None;
        }
    };
    let value = trim_trailing_newlines(&stdout);
    if value.is_empty() {
        tracing::warn!(
            var_name = %var_name,
            command = %command,
            "envFromCommand stdout was empty"
        );
        return None;
    }
    Some((var_name.clone(), value.to_string()))
}

fn resolve_file_entry((var_name, path_value): (&String, &Value)) -> Option<(String, String)> {
    let Some(path) = path_value.as_str() else {
        tracing::warn!(var_name = %var_name, "envFromFile entry path was not a string");
        return None;
    };
    let path_buf = match expand_home(path) {
        Ok(path_buf) => path_buf,
        Err(error) => {
            tracing::warn!(
                var_name = %var_name,
                path = %path,
                error = %error,
                "envFromFile path could not be resolved"
            );
            return None;
        }
    };
    let contents = match std::fs::read_to_string(&path_buf) {
        Ok(contents) => contents,
        Err(error) => {
            tracing::warn!(
                var_name = %var_name,
                path = %path_buf.display(),
                error = %error,
                "envFromFile could not be read"
            );
            return None;
        }
    };
    let value = trim_trailing_newlines(&contents);
    if value.is_empty() {
        tracing::warn!(
            var_name = %var_name,
            path = %path_buf.display(),
            "envFromFile content was empty"
        );
        return None;
    }
    Some((var_name.clone(), value.to_string()))
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
