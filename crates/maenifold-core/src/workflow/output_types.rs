use super::output_parser::{ParseError, ParsedOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingOutput {
    pub response: String,
    pub done: bool,
    pub needs_more: bool,
    pub revision_of: Option<u64>,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowOutput {
    pub response: String,
    pub done: bool,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOutput {
    pub response: String,
}

impl ThinkingOutput {
    pub fn from_parsed(parsed: &ParsedOutput) -> Result<Self, ParseError> {
        Ok(Self {
            response: parsed.require("response")?.to_string(),
            done: require_bool(parsed, "done")?,
            needs_more: parsed.get_bool("needs_more").unwrap_or(false),
            revision_of: optional_u64(parsed, "revision_of")?,
            conclusion: optional_string(parsed, "conclusion"),
        })
    }

    pub fn expected_shape() -> &'static str {
        "Respond in XML or JSON. Required: response (string), done (bool). Optional: conclusion, needs_more (bool).\n\
         XML: <output><response>[[concept]] analysis</response><done>false</done></output>\n\
         JSON: {\"response\":\"[[concept]] analysis\",\"done\":false}"
    }
}

impl WorkflowOutput {
    pub fn from_parsed(parsed: &ParsedOutput) -> Result<Self, ParseError> {
        Ok(Self {
            response: parsed.require("response")?.to_string(),
            done: require_bool(parsed, "done")?,
            conclusion: optional_string(parsed, "conclusion"),
        })
    }

    pub fn expected_shape() -> &'static str {
        "<output>\n  <response>...</response>\n  <done>false</done>\n  <conclusion>Required when done is true. Your final summary.</conclusion>\n</output>"
    }
}

impl PromptOutput {
    pub fn from_parsed(parsed: &ParsedOutput) -> Result<Self, ParseError> {
        Ok(Self {
            response: parsed.require("response")?.to_string(),
        })
    }

    pub fn expected_shape() -> &'static str {
        "<output>\n  <response>...</response>\n</output>"
    }
}

fn require_bool(parsed: &ParsedOutput, field: &str) -> Result<bool, ParseError> {
    if let Some(value) = parsed.get_bool(field) {
        return Ok(value);
    }
    if parsed.get(field).is_none() {
        return Err(ParseError {
            message: format!("Missing required field <{field}> in output"),
        });
    }
    Err(ParseError {
        message: format!("Invalid boolean value for <{field}> in output"),
    })
}

fn optional_u64(parsed: &ParsedOutput, field: &str) -> Result<Option<u64>, ParseError> {
    let Some(raw) = parsed.get(field) else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    parsed.get_u64(field).map(Some).ok_or_else(|| ParseError {
        message: format!("Invalid integer value for <{field}> in output"),
    })
}

fn optional_string(parsed: &ParsedOutput, field: &str) -> Option<String> {
    let value = parsed.get(field)?;
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{PromptOutput, ThinkingOutput, WorkflowOutput};
    use crate::workflow::output_parser::ParsedOutput;

    #[test]
    fn output_types_converts_thinking_output() {
        let parsed = parsed_output([
            ("response", "Thought text"),
            ("done", "true"),
            ("needs_more", "false"),
            ("revision_of", "4"),
            ("conclusion", "Finished"),
        ]);

        let output = ThinkingOutput::from_parsed(&parsed).unwrap();

        assert_eq!(output.response, "Thought text");
        assert!(output.done);
        assert!(!output.needs_more);
        assert_eq!(output.revision_of, Some(4));
        assert_eq!(output.conclusion.as_deref(), Some("Finished"));
    }

    #[test]
    fn output_types_converts_thinking_output_optional_empty_fields_to_none() {
        let parsed = parsed_output([
            ("response", "Thought text"),
            ("done", "false"),
            ("needs_more", "true"),
            ("revision_of", ""),
            ("conclusion", ""),
        ]);

        let output = ThinkingOutput::from_parsed(&parsed).unwrap();

        assert_eq!(output.revision_of, None);
        assert_eq!(output.conclusion, None);
    }

    #[test]
    fn output_types_converts_workflow_output() {
        let parsed = parsed_output([
            ("response", "Workflow step"),
            ("done", "true"),
            ("conclusion", "All done"),
        ]);

        let output = WorkflowOutput::from_parsed(&parsed).unwrap();

        assert_eq!(output.response, "Workflow step");
        assert!(output.done);
        assert_eq!(output.conclusion.as_deref(), Some("All done"));
    }

    #[test]
    fn output_types_converts_prompt_output() {
        let parsed = parsed_output([("response", "Prompt reply")]);

        let output = PromptOutput::from_parsed(&parsed).unwrap();

        assert_eq!(output.response, "Prompt reply");
    }

    #[test]
    fn output_types_reports_missing_required_bool_field() {
        let parsed = parsed_output([("response", "Workflow step")]);
        let error = WorkflowOutput::from_parsed(&parsed).unwrap_err();

        assert_eq!(error.message, "Missing required field <done> in output");
    }

    fn parsed_output<const N: usize>(entries: [(&str, &str); N]) -> ParsedOutput {
        ParsedOutput {
            fields: HashMap::from_iter(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.to_string(), value.to_string())),
            ),
        }
    }
}
