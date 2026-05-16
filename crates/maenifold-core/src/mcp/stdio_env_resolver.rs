use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::acp_config::EnvFromCommand;

/// Resolves env_from_file and env_from_command into the env map for a stdio MCP server.
/// Precedence is env_from_file, then env_from_command, then literal env.
pub(crate) fn resolve_mcp_server_env(
    env: &HashMap<String, String>,
    env_from_file: &HashMap<String, String>,
    env_from_command: &HashMap<String, EnvFromCommand>,
) -> Result<HashMap<String, String>, String> {
    let mut resolved = HashMap::new();

    for (var_name, path) in env_from_file {
        let path_buf = expand_home(path)
            .map_err(|error| format!("{var_name}: path could not be resolved: {error}"))?;
        let contents = std::fs::read_to_string(&path_buf).map_err(|error| {
            format!(
                "{var_name}: file not found or unreadable at {}: {error}",
                path_buf.display()
            )
        })?;
        let value = trim_trailing_newlines(&contents);
        if value.is_empty() {
            return Err(format!(
                "{var_name}: file content was empty at {}",
                path_buf.display()
            ));
        }
        resolved.insert(var_name.clone(), value.to_string());
    }

    for (var_name, spec) in env_from_command {
        let output = Command::new(&spec.command)
            .args(&spec.args)
            .output()
            .map_err(|error| format!("{var_name}: failed to run command: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "{var_name}: command exited with status {:?}",
                output.status.code()
            ));
        }
        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("{var_name}: stdout was not UTF-8: {error}"))?;
        let value = trim_trailing_newlines(&stdout);
        if value.is_empty() {
            return Err(format!("{var_name}: stdout was empty"));
        }
        resolved.insert(var_name.clone(), value.to_string());
    }

    for (var_name, value) in env {
        resolved.insert(var_name.clone(), value.clone());
    }

    Ok(resolved)
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
