use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload<'a> {
    pub code: &'a str,
    pub message: String,
}

pub fn success<T: Serialize>(data: T) -> String {
    serde_json::to_string(&json!({
        "success": true,
        "data": data,
        "error": Value::Null,
    }))
    .expect("json serialization")
}

pub fn failure(code: &str, message: impl Into<String>) -> String {
    serde_json::to_string(&json!({
        "success": false,
        "data": Value::Null,
        "error": {
            "code": code,
            "message": message.into(),
        }
    }))
    .expect("json serialization")
}
