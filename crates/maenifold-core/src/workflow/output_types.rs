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

