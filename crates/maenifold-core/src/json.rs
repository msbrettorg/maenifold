use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::io;

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

// T-0.1: RTM NFR-3 / SEC-001 keeps JSON parsing bounded to prevent deep-nesting regressions.
pub const SAFE_JSON_MAX_DEPTH: usize = 32;

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

// T-0.1: RTM NFR-3 / SEC-001 validates nesting depth before deserializing untrusted JSON.
pub fn from_str_safe<T>(input: &str) -> serde_json::Result<T>
where
    T: DeserializeOwned,
{
    ensure_max_depth(input, SAFE_JSON_MAX_DEPTH)?;
    serde_json::from_str(input)
}

// T-0.1: RTM FR-25.2 preserves C# PayloadReader degraded diagnostics for string-backed arrays.
pub fn get_string_array(payload: &Value, key: &str) -> Option<Vec<String>> {
    let value = payload.get(key)?;
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect(),
        Value::String(text) => match from_str_safe::<Vec<String>>(text) {
            Ok(values) => Some(values),
            Err(error) => {
                emit_payload_reader_diagnostic(
                    "GetStringArray",
                    &format!("Failed to deserialize string property '{key}' as string[]: {error}"),
                );
                None
            }
        },
        _ => None,
    }
}

fn ensure_max_depth(input: &str, max_depth: usize) -> serde_json::Result<()> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }

            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' | '[' => {
                depth += 1;
                if depth > max_depth {
                    return Err(serde_json::Error::io(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("JSON exceeds max depth of {max_depth}"),
                    )));
                }
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }

    Ok(())
}

fn emit_payload_reader_diagnostic(method: &str, message: &str) {
    emit_stderr_line(&format!("[DEGRADED][PayloadReader][{method}] {message}"));
}

#[cfg(unix)]
fn emit_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let mut line = String::with_capacity(message.len() + 1);
    line.push_str(message);
    line.push('\n');
    // SAFETY: fd=2 is the process stderr and the buffer is valid for the call duration.
    unsafe {
        let _ = write(2, line.as_ptr(), line.len());
    }
}

#[cfg(not(unix))]
fn emit_stderr_line(message: &str) {
    eprintln!("{message}");
}
