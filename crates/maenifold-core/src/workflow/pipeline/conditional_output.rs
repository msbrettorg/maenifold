use serde_json::Value as JsonValue;

/// Extract legacy conditional-edge JSON from prompt text.
///
/// T-332.0 introduced declared structured outputs, and T-332.9 moved
/// conditional predicates to prefer `NodeState.output_artifact`. This helper
/// remains only for legacy pipelines still persisted with text-only output.
#[deprecated(
    note = "Use NodeState.output_artifact directly. Will be removed when no legacy pipelines remain on disk."
)]
pub(super) fn extract_conditional_json_value(output: &str) -> Option<JsonValue> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<JsonValue>(trimmed)
        && (value.is_object() || value.is_array())
    {
        return Some(value);
    }

    let mut parsed = None;
    for (start, ch) in output.char_indices() {
        if matches!(ch, '{' | '[')
            && let Some(end) = balanced_json_end(output, start)
        {
            let candidate = &output[start..end];
            if let Ok(value) = serde_json::from_str::<JsonValue>(candidate)
                && (value.is_object() || value.is_array())
            {
                parsed = Some(value);
            }
        }
    }

    parsed
}

fn balanced_json_end(text: &str, start: usize) -> Option<usize> {
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if stack.pop() != Some(ch) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(start + offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}
