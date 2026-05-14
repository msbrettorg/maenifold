use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use serde_json::Value;
use std::collections::HashMap;

use super::read_workflow_definition;

struct InputDeclaration {
    input_type: String,
    required: bool,
    options: Vec<String>,
}

pub(crate) fn validate_workflow_inputs(
    config: &Config,
    workflow_ids: &[String],
    inputs: Option<&HashMap<String, Value>>,
) -> Result<()> {
    let workflow_id = workflow_ids
        .first()
        .ok_or(MaenifoldError::MissingProperty("workflowId"))?;
    let workflow_value = read_workflow_definition(config, workflow_id)?;
    let declarations = workflow_input_declarations(workflow_id, &workflow_value)?;

    // Multi-workflow starts already use the first workflow for session dispatch.
    // Keep input validation scoped to that first workflow to avoid inventing
    // cross-workflow schema composition rules.
    if declarations.is_empty() {
        return Ok(());
    }

    let supplied = inputs;
    if let Some(supplied) = supplied {
        for key in supplied.keys() {
            if !declarations.contains_key(key) {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Unknown input key '{key}' for workflow '{workflow_id}'"
                )));
            }
        }
    }

    for (key, declaration) in &declarations {
        let value = supplied.and_then(|inputs| inputs.get(key));
        if value.is_none() && declaration.required {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Missing required input key '{key}' for workflow '{workflow_id}'"
            )));
        }
        if let Some(value) = value {
            validate_input_value(workflow_id, key, declaration, value)?;
        }
    }

    Ok(())
}

fn workflow_input_declarations(
    workflow_id: &str,
    workflow_value: &Value,
) -> Result<HashMap<String, InputDeclaration>> {
    let Some(inputs) = workflow_value.get("inputs") else {
        return Ok(HashMap::new());
    };

    match inputs {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| {
                input_declaration(workflow_id, key, value)
                    .map(|declaration| (key.clone(), declaration))
            })
            .collect(),
        Value::Array(fields) => {
            let mut declarations = HashMap::new();
            for field in fields {
                let object = field.as_object().ok_or_else(|| {
                    MaenifoldError::InvalidArgument(format!(
                        "Workflow '{workflow_id}' input declaration entries must be objects"
                    ))
                })?;
                let key = object
                    .get("name")
                    .or_else(|| object.get("id"))
                    .or_else(|| object.get("key"))
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        MaenifoldError::InvalidArgument(format!(
                            "Workflow '{workflow_id}' input declaration is missing a name"
                        ))
                    })?;
                declarations.insert(key.to_string(), input_declaration(workflow_id, key, field)?);
            }
            Ok(declarations)
        }
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "Workflow '{workflow_id}' inputs declaration must be an object or array"
        ))),
    }
}

fn input_declaration(workflow_id: &str, key: &str, value: &Value) -> Result<InputDeclaration> {
    let source = match value {
        Value::String(input_type) => {
            return Ok(InputDeclaration {
                input_type: normalize_input_type(input_type),
                required: false,
                options: Vec::new(),
            });
        }
        Value::Object(source) => source,
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Workflow '{workflow_id}' input '{key}' declaration must be a string or object"
            )));
        }
    };

    let options: Vec<String> = source
        .get("options")
        .or_else(|| source.get("enum"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let input_type = source
        .get("type")
        .and_then(Value::as_str)
        .map(normalize_input_type)
        .unwrap_or_else(|| {
            if options.is_empty() {
                "string".to_string()
            } else {
                "select".to_string()
            }
        });

    Ok(InputDeclaration {
        input_type,
        required: source
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        options,
    })
}

fn normalize_input_type(input_type: &str) -> String {
    match input_type.to_ascii_lowercase().as_str() {
        "string" | "number" | "boolean" | "select" => input_type.to_ascii_lowercase(),
        "text" | "textarea" | "multiline" => "string".to_string(),
        "integer" => "number".to_string(),
        "bool" => "boolean".to_string(),
        "enum" => "select".to_string(),
        _ => "string".to_string(),
    }
}

fn validate_input_value(
    workflow_id: &str,
    key: &str,
    declaration: &InputDeclaration,
    value: &Value,
) -> Result<()> {
    let valid = match declaration.input_type.as_str() {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "select" => value.as_str().is_some_and(|text| {
            declaration.options.is_empty()
                || declaration.options.iter().any(|option| option == text)
        }),
        _ => true,
    };
    if valid {
        return Ok(());
    }

    Err(MaenifoldError::InvalidArgument(format!(
        "Invalid input '{key}' for workflow '{workflow_id}': expected {}",
        declaration.input_type
    )))
}
