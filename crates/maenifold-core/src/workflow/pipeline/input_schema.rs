use crate::error::{MaenifoldError, Result};
use serde_json::{Map, Value};

pub fn validate_pipeline_inputs(schema: &Value, inputs: &Map<String, Value>) -> Result<()> {
    let validator = jsonschema::validator_for(schema).map_err(|error| {
        MaenifoldError::InvalidArgument(format!("Pipeline input_schema is invalid: {error}"))
    })?;
    let instance = Value::Object(inputs.clone());
    let violations = validator
        .iter_errors(&instance)
        .map(|error| {
            let path = validation_path(&error);
            let keyword = validation_keyword(&error);
            format!("{path}: {keyword}: {error}")
        })
        .collect::<Vec<_>>();

    if violations.is_empty() {
        return Ok(());
    }

    Err(MaenifoldError::InvalidArgument(format!(
        "Pipeline inputs failed input_schema validation: {}",
        violations.join("; ")
    )))
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
