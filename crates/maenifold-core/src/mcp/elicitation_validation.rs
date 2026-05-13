use serde_json::Value;

pub fn validate_elicitation_response(schema: &Value, content: &Value) -> Result<(), Vec<String>> {
    // JSONForms uses AJV Draft-07 in the renderer; the daemon validates with
    // jsonschema's default Draft 2020-12 support so the MCP response path is
    // authoritative for schemas the renderer can only show as raw JSON.
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| vec![format!("invalid elicitation schema: {error}")])?;
    let errors = validator
        .iter_errors(content)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::validate_elicitation_response;
    use serde_json::json;

    #[test]
    fn accepts_content_matching_schema() {
        let schema = json!({
            "type": "object",
            "properties": { "reason": { "type": "string" } },
            "required": ["reason"]
        });
        let content = json!({ "reason": "approved" });

        assert!(validate_elicitation_response(&schema, &content).is_ok());
    }

    #[test]
    fn rejects_content_not_matching_schema() {
        let schema = json!({
            "type": "object",
            "properties": { "reason": { "type": "string" } },
            "required": ["reason"]
        });
        let content = json!({ "reason": 5 });

        let errors = validate_elicitation_response(&schema, &content)
            .expect_err("invalid content should report validation errors");
        assert!(!errors.is_empty());
    }
}
