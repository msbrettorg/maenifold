use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Number, Value, json};
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

// T-0.1: RTM NFR-3 keeps JSON parsing bounded to prevent deep-nesting regressions.
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

// T-0.1: RTM NFR-3 validates nesting depth before deserializing untrusted JSON.
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

// T-16.5: RTM NFR-1 — C# System.Text.Json serializes whole-number doubles as
// integers (e.g. `1` not `1.0`). serde_json preserves the `.0` suffix.
// This post-processor normalizes a JSON value tree so float values that are
// exact integers are emitted without the fractional part, matching C# output.
pub fn normalize_whole_floats(value: &mut Value) {
    match value {
        Value::Number(n) => {
            // Only convert if it's a float that represents an exact integer
            if let Some(f) = n.as_f64() {
                #[allow(clippy::cast_possible_truncation)]
                if f.is_finite() && f == f.floor() && f.abs() < (i64::MAX as f64) {
                    let i = f as i64;
                    if (i as f64) == f {
                        *n = Number::from(i);
                    }
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                normalize_whole_floats(item);
            }
        }
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                normalize_whole_floats(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_whole_floats_converts_integers() {
        let mut val = json!({"score": 1.0, "name": "test", "frac": 0.5});
        normalize_whole_floats(&mut val);
        assert_eq!(val["score"], json!(1));
        assert_eq!(val["frac"], json!(0.5));
        assert_eq!(val["name"], json!("test"));
    }

    #[test]
    fn normalize_whole_floats_handles_nested() {
        let mut val = json!({"results": [{"score": 2.0}, {"score": 0.016}]});
        normalize_whole_floats(&mut val);
        assert_eq!(val["results"][0]["score"], json!(2));
        assert_eq!(val["results"][1]["score"], json!(0.016));
    }

    #[test]
    fn normalize_whole_floats_handles_zero() {
        let mut val = json!({"score": 0.0});
        normalize_whole_floats(&mut val);
        assert_eq!(val["score"], json!(0));
    }
}
