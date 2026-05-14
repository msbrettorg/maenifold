use crate::error::{MaenifoldError, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::LazyLock;

static INPUT_TEMPLATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{\s*inputs?\.([A-Za-z0-9_-]+)\s*\}\}")
        .expect("static workflow input template regex is valid")
});

pub(crate) fn expand_inputs_template(
    text: &str,
    inputs: &HashMap<String, Value>,
) -> Result<String> {
    let mut rendered = String::with_capacity(text.len());
    let mut last_end = 0;

    for captures in INPUT_TEMPLATE_RE.captures_iter(text) {
        let matched = captures
            .get(0)
            .expect("regex captures always include the full match");
        let key = captures
            .get(1)
            .expect("workflow input template regex captures the key")
            .as_str();
        let value = inputs.get(key).ok_or_else(|| {
            MaenifoldError::Message(format!("Workflow step references unknown input '{key}'"))
        })?;
        rendered.push_str(&text[last_end..matched.start()]);
        rendered.push_str(&input_value_to_string(value)?);
        last_end = matched.end();
    }

    if last_end == 0 {
        return Ok(text.to_string());
    }
    rendered.push_str(&text[last_end..]);
    Ok(rendered)
}

fn input_value_to_string(value: &Value) -> Result<String> {
    Ok(match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value)?,
    })
}
