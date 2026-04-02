use crate::error::GateError;

#[derive(Debug, Clone)]
pub struct JsonRenderer;

impl JsonRenderer {
    pub fn render_status(value: serde_json::Value) -> Result<String, GateError> {
        Ok(serde_json::to_string_pretty(&value)?)
    }
}
