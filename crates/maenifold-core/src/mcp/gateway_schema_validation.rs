use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;

pub(crate) const MAX_SCHEMA_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayValidationError {
    message: String,
}

impl GatewayValidationError {
    fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for GatewayValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GatewayValidationError {}

#[allow(dead_code)]
pub(crate) fn validate_parameters(
    command_name: &str,
    parameters: &Value,
    schema: &Value,
) -> Result<(), GatewayValidationError> {
    let Some(validator) = compile_validator(schema) else {
        return Ok(());
    };
    validate_with_validator(command_name, parameters, &validator)
}

pub(crate) fn compile_validator(schema: &Value) -> Option<Arc<jsonschema::Validator>> {
    if !is_supported_object_schema(schema) || schema_size(schema)? > MAX_SCHEMA_BYTES {
        return None;
    }
    jsonschema::validator_for(schema).ok().map(Arc::new)
}

pub(crate) fn validate_with_validator(
    command_name: &str,
    parameters: &Value,
    validator: &jsonschema::Validator,
) -> Result<(), GatewayValidationError> {
    if let Some(error) = validator.iter_errors(parameters).next() {
        let path = validation_path(&error);
        let keyword = validation_keyword(&error);
        return Err(GatewayValidationError::new(format!(
            "gateway validation failed for '{command_name}': {path}: {keyword}"
        )));
    }
    Ok(())
}

pub(crate) fn compute_schema_hash(schema: &Value) -> String {
    let mut hasher = Sha256::new();
    write_canonical_json(schema, &mut hasher);
    to_hex(&hasher.finalize())
}

pub(crate) fn schema_size(schema: &Value) -> Option<usize> {
    serde_json::to_vec(schema).ok().map(|bytes| bytes.len())
}

fn is_supported_object_schema(schema: &Value) -> bool {
    schema
        .as_object()
        .and_then(|object| object.get("type"))
        .and_then(Value::as_str)
        == Some("object")
}

fn validation_path(error: &jsonschema::ValidationError<'_>) -> String {
    match &error.kind {
        jsonschema::error::ValidationErrorKind::Required { property } => {
            let base = error.instance_path.to_string();
            let property = property
                .as_str()
                .map(escape_json_pointer_segment)
                .unwrap_or_else(|| "required".to_string());
            if base.is_empty() {
                format!("/{property}")
            } else {
                format!("{base}/{property}")
            }
        }
        _ => error.instance_path.to_string(),
    }
}

fn validation_keyword(error: &jsonschema::ValidationError<'_>) -> String {
    let schema_path = error.schema_path.to_string();
    schema_path
        .rsplit('/')
        .next()
        .filter(|keyword| !keyword.is_empty())
        .map(unescape_json_pointer_segment)
        .unwrap_or_else(|| "schema".to_string())
}

fn escape_json_pointer_segment(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn unescape_json_pointer_segment(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

fn write_canonical_json(value: &Value, out: &mut Sha256) {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            out.update(serde_json::to_vec(value).expect("scalar JSON serialization cannot fail"));
        }
        Value::Array(items) => {
            out.update(b"[");
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.update(b",");
                }
                write_canonical_json(item, out);
            }
            out.update(b"]");
        }
        Value::Object(object) => {
            out.update(b"{");
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.update(b",");
                }
                out.update(serde_json::to_vec(key).expect("key serialization cannot fail"));
                out.update(b":");
                write_canonical_json(value, out);
            }
            out.update(b"}");
        }
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
