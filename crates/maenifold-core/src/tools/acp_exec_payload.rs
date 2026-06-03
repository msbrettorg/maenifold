use super::acp_exec_context::optional_context_pairs;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub(super) struct AcpExecRequest {
    pub prompt: String,
    pub context: Vec<(String, String)>,
}

pub(super) fn request_from_payload(payload: &Value, mode: &str) -> Result<AcpExecRequest> {
    let mut context = optional_context_pairs(payload)?;
    match mode {
        "prompt" => {
            context.push(("acp_exec_mode".to_string(), "prompt".to_string()));
            Ok(AcpExecRequest {
                prompt: required_str(payload, "prompt")?.to_string(),
                context,
            })
        }
        "think" => {
            let topic = required_str(payload, "topic")?;
            let total_thoughts = payload
                .get("totalThoughts")
                .and_then(Value::as_u64)
                .unwrap_or(6);
            context.push(("acp_exec_mode".to_string(), "think".to_string()));
            context.push(("total_thoughts".to_string(), total_thoughts.to_string()));
            Ok(AcpExecRequest {
                prompt: format!(
                    "Use sequential thinking for the following topic and return only the final conclusion.\n\nTopic:\n{topic}"
                ),
                context,
            })
        }
        "workflow" => {
            let workflow_id = required_str(payload, "workflowId")?;
            let workflow_prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
            context.push(("acp_exec_mode".to_string(), "workflow".to_string()));
            context.push(("workflow_id".to_string(), workflow_id.to_string()));
            Ok(AcpExecRequest {
                prompt: if workflow_prompt.is_empty() {
                    format!(
                        "Execute workflow '{workflow_id}' and return only the final conclusion."
                    )
                } else {
                    format!(
                        "Execute workflow '{workflow_id}' using the following input and return only the final conclusion.\n\nInput:\n{workflow_prompt}"
                    )
                },
                context,
            })
        }
        other => Err(MaenifoldError::InvalidArgument(format!(
            "unknown mode '{other}'; expected prompt, think, or workflow"
        ))),
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct RequestOptions {
    pub app_session_id: String,
    pub cwd: PathBuf,
    pub auto_approve: bool,
    pub role_config_options: Vec<(String, String)>,
    pub timeout_seconds: Option<u64>,
    pub extra_context: Vec<(String, String)>,
}

pub(super) fn request_options_from(cwd: PathBuf, payload: &Value) -> Result<RequestOptions> {
    let role_config_options = payload_config_options(payload)?;
    let extra_context = payload_extra_context(payload)?;
    let app_session_id = optional_string(payload, "sessionId").unwrap_or_else(fresh_session_id);

    Ok(RequestOptions {
        app_session_id,
        cwd: optional_string(payload, "cwd")
            .map(PathBuf::from)
            .unwrap_or(cwd),
        auto_approve: required_bool(payload, "autoGrantPermissions")?,
        role_config_options,
        timeout_seconds: payload.get("timeoutSeconds").and_then(Value::as_u64),
        extra_context,
    })
}

fn payload_config_options(payload: &Value) -> Result<Vec<(String, String)>> {
    let mut options = Vec::new();

    if let Some(map) = payload.get("configOptions").and_then(Value::as_object) {
        for (key, value) in map {
            options.push((key.clone(), config_value_to_string(key, value)?));
        }
    }

    push_option_if_present(&mut options, "model", optional_string(payload, "model"));
    push_option_if_present(
        &mut options,
        "thoughtLevel",
        optional_string(payload, "thoughtLevel"),
    );

    Ok(options)
}

fn push_option_if_present(options: &mut Vec<(String, String)>, key: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };

    if let Some(existing) = options
        .iter_mut()
        .find(|(existing_key, _)| existing_key == key)
    {
        existing.1 = value;
    } else {
        options.push((key.to_string(), value));
    }
}

fn config_value_to_string(key: &str, value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Null => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must not be null"
        ))),
        other => serde_json::to_string(other).map_err(MaenifoldError::from),
    }
}

fn payload_extra_context(payload: &Value) -> Result<Vec<(String, String)>> {
    let mut context = Vec::new();

    if let Some(allowed_tools) = payload.get("allowedTools").and_then(Value::as_array) {
        let tools = allowed_tools
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !tools.is_empty() {
            context.push(("allowed_tools".to_string(), tools.join(", ")));
        }
    }

    if let Some(max_turns) = payload.get("maxTurns").and_then(Value::as_u64) {
        context.push(("max_turns".to_string(), max_turns.to_string()));
    }

    Ok(context)
}

pub(super) fn fresh_session_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("acp-exec-{millis}")
}

pub(super) fn required_str<'a>(payload: &'a Value, field: &'static str) -> Result<&'a str> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty(field))
}

pub(super) fn mode_or_default(payload: &Value) -> Result<&str> {
    match payload.get("mode") {
        Some(value) => value.as_str().ok_or_else(|| {
            MaenifoldError::InvalidArgument(
                "mode must be a string; expected prompt, think, or workflow".to_string(),
            )
        }),
        None => Ok("prompt"),
    }
}

pub(super) fn mode_for_log(payload: &Value) -> &str {
    match payload.get("mode") {
        Some(Value::String(mode)) => mode.as_str(),
        Some(_) => "<invalid>",
        None => "prompt",
    }
}

pub(super) fn optional_string(payload: &Value, field: &str) -> Option<String> {
    payload.get(field).and_then(Value::as_str).map(String::from)
}

pub(super) fn required_bool(payload: &Value, field: &'static str) -> Result<bool> {
    payload
        .get(field)
        .and_then(Value::as_bool)
        .ok_or(MaenifoldError::MissingProperty(field))
}

#[cfg(test)]
mod tests {
    use super::request_options_from;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn request_options_from_uses_payload_cwd_when_present() {
        let root = PathBuf::from("/repo/root");
        let opts = request_options_from(
            root,
            &json!({
                "autoGrantPermissions": true,
                "sessionId": "sess-123",
                "cwd": "/repo/feature"
            }),
        )
        .expect("request options");

        assert_eq!(opts.app_session_id, "sess-123");
        assert_eq!(opts.cwd, PathBuf::from("/repo/feature"));
    }

    #[test]
    fn request_options_from_defaults_cwd_to_root_when_omitted() {
        let root = PathBuf::from("/repo/root");
        let opts = request_options_from(
            root.clone(),
            &json!({
                "autoGrantPermissions": true
            }),
        )
        .expect("request options");

        assert!(opts.app_session_id.starts_with("acp-exec-"));
        assert_eq!(opts.cwd, root);
    }
}
