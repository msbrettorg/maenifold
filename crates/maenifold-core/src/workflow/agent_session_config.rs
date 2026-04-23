use serde_json::{Map, Value};

use crate::error::{MaenifoldError, Result};

// T-01.15: extracted from agent_session.rs to keep that module under the LOC budget.

pub(super) fn build_role_config_options(
    config_options: Option<&Map<String, Value>>,
    model: Option<&str>,
    thought_level: Option<&str>,
) -> Result<Vec<(String, String)>> {
    let mut options = Vec::new();

    if let Some(map) = config_options {
        for (key, value) in map {
            options.push((key.clone(), config_value_to_string(key, value)?));
        }
    }

    push_option_if_present(&mut options, "model", model);
    push_thought_level_options(&mut options, thought_level);

    Ok(options)
}

fn push_option_if_present(options: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };

    if let Some(existing) = options
        .iter_mut()
        .find(|(existing_key, _)| existing_key == key)
    {
        existing.1 = value.to_string();
    } else {
        options.push((key.to_string(), value.to_string()));
    }
}

fn push_thought_level_options(options: &mut Vec<(String, String)>, thought_level: Option<&str>) {
    let Some(thought_level) = thought_level else {
        return;
    };

    push_option_if_present(options, "thought_level", Some(thought_level));
    push_option_if_present(options, "thoughtLevel", Some(thought_level));
}

fn config_value_to_string(key: &str, value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Null => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must not be null"
        ))),
        other => Err(MaenifoldError::InvalidArgument(format!(
            "configOptions.{key} must be a scalar (string, bool, or number), got {}",
            match other {
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "unknown",
            }
        ))),
    }
}
