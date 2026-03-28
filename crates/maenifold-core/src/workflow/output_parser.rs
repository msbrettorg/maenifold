use std::collections::HashMap;
use std::fmt;

const GENERIC_EXPECTED_SHAPE: &str = "<output>\n  <field>...</field>\n</output>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOutput {
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

impl ParsedOutput {
    pub fn get(&self, field: &str) -> Option<&str> {
        self.fields.get(field).map(String::as_str)
    }

    pub fn get_bool(&self, field: &str) -> Option<bool> {
        match self.get(field)?.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    pub fn get_u64(&self, field: &str) -> Option<u64> {
        self.get(field)?.trim().parse().ok()
    }

    pub fn require(&self, field: &str) -> Result<&str, ParseError> {
        self.get(field).ok_or_else(|| ParseError {
            message: format!("Missing required field <{field}> in output"),
        })
    }
}

pub fn parse_output(text: &str) -> Result<ParsedOutput, ParseError> {
    let start = text.find("<output>").ok_or_else(|| ParseError {
        message: format!(
            "No <output> tags found in response. Expected format:\n{GENERIC_EXPECTED_SHAPE}"
        ),
    })?;
    let end = text.rfind("</output>").ok_or_else(|| ParseError {
        message: format!(
            "No <output> tags found in response. Expected format:\n{GENERIC_EXPECTED_SHAPE}"
        ),
    })?;
    if end < start {
        return Err(ParseError {
            message: format!(
                "No <output> tags found in response. Expected format:\n{GENERIC_EXPECTED_SHAPE}"
            ),
        });
    }

    let inner = &text[start + "<output>".len()..end];
    let mut fields = HashMap::new();
    let mut cursor = inner;

    while let Some(offset) = cursor.find('<') {
        cursor = &cursor[offset..];
        if cursor.starts_with("</") {
            break;
        }

        let Some(tag_end) = cursor.find('>') else {
            break;
        };
        let tag = &cursor[1..tag_end];
        if !is_valid_tag_name(tag) {
            cursor = &cursor[tag_end + 1..];
            continue;
        }

        let close_tag = format!("</{tag}>");
        let value_start = tag_end + 1;
        let Some(close_offset) = cursor[value_start..].find(&close_tag) else {
            break;
        };
        let value_end = value_start + close_offset;
        fields.insert(tag.to_string(), cursor[value_start..value_end].to_string());
        cursor = &cursor[value_end + close_tag.len()..];
    }

    Ok(ParsedOutput { fields })
}

fn is_valid_tag_name(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

#[cfg(test)]
mod tests {
    use super::{ParseError, parse_output};

    #[test]
    fn output_parser_parses_valid_output_with_all_fields() {
        let parsed = parse_output(
            "<output><response>Hello</response><done>true</done><needs_more>false</needs_more><revision_of>7</revision_of><conclusion>Done</conclusion></output>",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("Hello"));
        assert_eq!(parsed.get_bool("done"), Some(true));
        assert_eq!(parsed.get_bool("needs_more"), Some(false));
        assert_eq!(parsed.get_u64("revision_of"), Some(7));
        assert_eq!(parsed.require("conclusion").unwrap(), "Done");
    }

    #[test]
    fn output_parser_parses_markdown_and_code_in_response() {
        let parsed = parse_output(
            "<output><response>## Plan\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n</response><done>false</done></output>",
        )
        .unwrap();

        let response = parsed.get("response").unwrap();
        assert!(response.contains("## Plan"));
        assert!(response.contains("```rust"));
        assert!(response.contains("println!(\"hi\")"));
    }

    #[test]
    fn output_parser_parses_wikilinks_in_content() {
        let parsed = parse_output(
            "<output><response>Implemented [[workflow]] [[structured-output]] parser</response></output>",
        )
        .unwrap();

        assert_eq!(
            parsed.get("response"),
            Some("Implemented [[workflow]] [[structured-output]] parser")
        );
    }

    #[test]
    fn output_parser_returns_error_when_output_tags_are_missing() {
        let error = parse_output("No structured block here").unwrap_err();

        assert_eq!(
            error,
            ParseError {
                message: "No <output> tags found in response. Expected format:\n<output>\n  <field>...</field>\n</output>".to_string()
            }
        );
    }

    #[test]
    fn output_parser_require_reports_missing_required_field() {
        let parsed = parse_output("<output><response>Hello</response></output>").unwrap();
        let error = parsed.require("done").unwrap_err();

        assert_eq!(error.message, "Missing required field <done> in output");
    }

    #[test]
    fn output_parser_preserves_empty_fields() {
        let parsed = parse_output(
            "<output><response></response><done></done><conclusion></conclusion></output>",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some(""));
        assert_eq!(parsed.get("done"), Some(""));
        assert_eq!(parsed.get("conclusion"), Some(""));
    }

    #[test]
    fn output_parser_ignores_content_outside_output_tags() {
        let parsed = parse_output(
            "Thinking out loud\n<output><response>final</response><done>true</done></output>\nMore notes",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("final"));
        assert_eq!(parsed.get_bool("done"), Some(true));
    }

    #[test]
    fn output_parser_preserves_nested_xml_like_content_inside_fields() {
        let parsed = parse_output(
            "<output><response>```html\n<div class=\"card\">[[workflow]]</div>\n```\n</response><done>false</done></output>",
        )
        .unwrap();

        let response = parsed.get("response").unwrap();
        assert!(response.contains("<div class=\"card\">[[workflow]]</div>"));
        assert_eq!(parsed.get_bool("done"), Some(false));
    }
}
