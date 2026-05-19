use handlebars::{Handlebars, no_escape};
use serde_json::Value as JsonValue;

use crate::error::{MaenifoldError, Result};

#[allow(deprecated)]
use super::conditional_output::extract_conditional_json_value;

pub(super) fn resolve_dot_path(value: &JsonValue, path: &str) -> Result<JsonValue> {
    let segments = parse_dot_path(path).ok_or_else(|| {
        MaenifoldError::InvalidArgument(format!("Invalid pipeline mapping path: {path}"))
    })?;
    let mut current = value;

    for segment in segments {
        current = match current {
            JsonValue::Object(map) => map.get(segment.as_str()).ok_or_else(|| {
                MaenifoldError::InvalidArgument(format!(
                    "Pipeline mapping path '{path}' is missing segment '{segment}'"
                ))
            })?,
            JsonValue::Array(items) => {
                let index = segment.parse::<usize>().map_err(|_| {
                    MaenifoldError::InvalidArgument(format!(
                        "Pipeline mapping path '{path}' expected an array index, got '{segment}'"
                    ))
                })?;
                items.get(index).ok_or_else(|| {
                    MaenifoldError::InvalidArgument(format!(
                        "Pipeline mapping path '{path}' is missing array index {index}"
                    ))
                })?
            }
            _ => {
                return Err(MaenifoldError::InvalidArgument(format!(
                    "Pipeline mapping path '{path}' cannot traverse '{segment}'"
                )));
            }
        };
    }

    Ok(current.clone())
}

fn parse_dot_path(path: &str) -> Option<Vec<String>> {
    if path.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    let mut chars = path.chars().peekable();
    let mut current = String::new();
    let mut allow_empty_dot = false;

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                    allow_empty_dot = false;
                } else if allow_empty_dot {
                    allow_empty_dot = false;
                } else {
                    return None;
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                let mut index = String::new();
                let mut closed = false;
                for next in chars.by_ref() {
                    if next == ']' {
                        closed = true;
                        break;
                    }
                    index.push(next);
                }
                if !closed || index.is_empty() {
                    return None;
                }
                segments.push(index);
                allow_empty_dot = true;
            }
            ']' => return None,
            other => {
                current.push(other);
                allow_empty_dot = false;
            }
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    Some(segments)
}

pub(super) const CONDITIONAL_NOT_SATISFIED_REASON: &str = "conditional predicate not satisfied";
pub(super) const CONDITIONAL_PARSE_FAILURE_REASON: &str =
    "conditional predicate could not parse JSON from prompt output";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConditionalPredicateResult {
    Satisfied,
    NotSatisfied,
    Unparseable,
}

pub(super) fn conditional_parse_failure_reason(source_id: &str) -> String {
    format!("{CONDITIONAL_PARSE_FAILURE_REASON} (source node: {source_id})")
}

pub(super) fn conditional_output_has_json(
    upstream_artifact: Option<&JsonValue>,
    upstream_output: Option<&str>,
) -> bool {
    if upstream_artifact.is_some() {
        return true;
    }
    #[allow(deprecated)]
    upstream_output
        .and_then(extract_conditional_json_value)
        .is_some()
}

/// Test whether a `Conditional` edge predicate is satisfied for the upstream
/// node's output.
pub(super) fn conditional_predicate_satisfied(
    upstream_artifact: Option<&JsonValue>,
    upstream_output: Option<&str>,
    path: &str,
    equals: &JsonValue,
) -> bool {
    conditional_predicate_result(upstream_artifact, upstream_output, path, equals)
        == ConditionalPredicateResult::Satisfied
}

pub(super) fn conditional_predicate_result(
    upstream_artifact: Option<&JsonValue>,
    upstream_output: Option<&str>,
    path: &str,
    equals: &JsonValue,
) -> ConditionalPredicateResult {
    let parsed;
    let source = if let Some(artifact) = upstream_artifact {
        artifact
    } else {
        let Some(output) = upstream_output else {
            return ConditionalPredicateResult::Unparseable;
        };
        #[allow(deprecated)]
        {
            let Some(value) = extract_conditional_json_value(output) else {
                return ConditionalPredicateResult::Unparseable;
            };
            tracing::debug!(
                "conditional edge falling back to parsing source output as JSON — declare output_schema on the source for stable behavior"
            );
            parsed = value;
        }
        &parsed
    };
    let resolved = if path.starts_with('/') {
        source.pointer(path).cloned()
    } else {
        resolve_dot_path(source, path).ok()
    };
    match resolved {
        Some(value) if &value == equals => ConditionalPredicateResult::Satisfied,
        _ => ConditionalPredicateResult::NotSatisfied,
    }
}

pub(super) fn pipeline_inputs_scope(inputs: &serde_json::Map<String, JsonValue>) -> JsonValue {
    JsonValue::Object(serde_json::Map::from_iter([(
        "inputs".to_string(),
        JsonValue::Object(inputs.clone()),
    )]))
}

const DISABLED_HANDLEBARS_HELPERS: &[&str] = &[
    "if",
    "unless",
    "each",
    "with",
    "lookup",
    "raw",
    "log",
    "eq",
    "ne",
    "gt",
    "gte",
    "lt",
    "lte",
    "and",
    "or",
    "not",
    "len",
    "lowerCamelCase",
    "upperCamelCase",
    "snakeCase",
    "kebabCase",
    "shoutySnakeCase",
    "shoutyKebabCase",
    "titleCase",
    "trainCase",
];

fn simple_path_substitution_handlebars() -> Handlebars<'static> {
    let mut handlebars = Handlebars::new();
    for helper in DISABLED_HANDLEBARS_HELPERS {
        handlebars.unregister_helper(helper);
    }
    handlebars.register_escape_fn(no_escape);
    handlebars.set_strict_mode(true);
    handlebars
}

/// Interpolation is restricted to Tool node `tool_args`.
/// ACP node prompts (Prompt, Workflow, SequentialThinking) MUST NOT pass through
/// this function. Use the `## Runtime inputs` envelope (T-332.5) instead.
pub(super) fn interpolate_string(template: &str, scope: &JsonValue) -> Result<String> {
    simple_path_substitution_handlebars()
        .render_template(template, scope)
        .map_err(|error| {
            MaenifoldError::InvalidArgument(format!(
                "Pipeline string interpolation failed: {error}"
            ))
        })
}

/// Interpolation is restricted to Tool node `tool_args`.
/// ACP node prompts (Prompt, Workflow, SequentialThinking) MUST NOT pass through
/// this function. Use the `## Runtime inputs` envelope (T-332.5) instead.
pub(super) fn resolve_template_value(value: &JsonValue, scope: &JsonValue) -> Result<JsonValue> {
    match value {
        JsonValue::String(template) => Ok(JsonValue::String(interpolate_string(template, scope)?)),
        JsonValue::Array(items) => Ok(JsonValue::Array(
            items
                .iter()
                .map(|item| resolve_template_value(item, scope))
                .collect::<Result<Vec<_>>>()?,
        )),
        JsonValue::Object(map) if map.len() == 1 => match map.get("$ref") {
            Some(JsonValue::String(path)) => resolve_dot_path(scope, path).map_err(|error| {
                MaenifoldError::InvalidArgument(format!(
                    "Pipeline $ref resolution for '{path}' failed: {error}"
                ))
            }),
            _ => Ok(JsonValue::Object(
                map.iter()
                    .map(|(key, value)| Ok((key.clone(), resolve_template_value(value, scope)?)))
                    .collect::<Result<serde_json::Map<_, _>>>()?,
            )),
        },
        JsonValue::Object(map) => Ok(JsonValue::Object(
            map.iter()
                .map(|(key, value)| Ok((key.clone(), resolve_template_value(value, scope)?)))
                .collect::<Result<serde_json::Map<_, _>>>()?,
        )),
        _ => Ok(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn resolve_template_value_renders_strings_in_strict_mode() {
        let resolved = resolve_template_value(
            &json!({ "message": "hello {{inputs.name}}" }),
            &json!({ "inputs": { "name": "world" } }),
        )
        .expect("template should resolve");

        assert_eq!(resolved, json!({ "message": "hello world" }));
    }

    #[test]
    fn resolve_template_value_resolves_typed_refs() {
        let resolved = resolve_template_value(
            &json!({ "count": { "$ref": "inputs.payload.count" } }),
            &json!({ "inputs": { "payload": { "count": 3 } } }),
        )
        .expect("ref should resolve");

        assert_eq!(resolved, json!({ "count": 3 }));
    }

    #[test]
    fn interpolate_string_rejects_default_helpers() {
        let error = interpolate_string(
            "{{#if inputs.enabled}}enabled{{/if}}",
            &json!({ "inputs": { "enabled": true } }),
        )
        .expect_err("default helpers should be disabled");

        assert!(
            error
                .to_string()
                .contains("Pipeline string interpolation failed")
        );
        assert!(error.to_string().contains("if"));
    }

    #[test]
    fn interpolate_string_preserves_raw_value_substitution() {
        let resolved = interpolate_string(
            "Hello {{inputs.name}}",
            &json!({ "inputs": { "name": "<world>" } }),
        )
        .expect("simple substitution should preserve raw value");

        assert_eq!(resolved, "Hello <world>");
    }
}
