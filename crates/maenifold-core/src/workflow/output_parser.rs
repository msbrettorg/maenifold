use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

const GENERIC_EXPECTED_SHAPE: &str = "<output>\n  <field>...</field>\n</output>";

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<\s*(/?)\s*([A-Za-z0-9_-]+)(?:\s[^>]*)?>")
        .expect("output parser tag regex must compile")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOutput {
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ParseError {
    pub message: String,
}

impl ParsedOutput {
    pub fn get(&self, field: &str) -> Option<&str> {
        self.fields
            .get(&field.to_ascii_lowercase())
            .map(String::as_str)
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
    parse_xml_output(text).or_else(|xml_error| {
        json_fallback_parse(text).map_err(|_| xml_error)
    })
}

fn parse_xml_output(text: &str) -> Result<ParsedOutput, ParseError> {
    let (_, output_start_end) = find_tag(text, "output", false, 0).ok_or_else(missing_output)?;
    let (output_end_start, _) = find_tag(text, "output", true, output_start_end)
        .ok_or_else(missing_output)?;

    let inner = &text[output_start_end..output_end_start];
    let mut fields = HashMap::new();
    let mut cursor = 0;

    while let Some((name, _, tag_end, is_closing)) = next_tag(inner, cursor) {
        cursor = tag_end;
        if is_closing {
            continue;
        }

        let value_start = tag_end;
        let Some((close_tag_start, close_tag_end)) = find_tag(inner, &name, true, value_start) else {
            continue;
        };
        fields.insert(name, inner[value_start..close_tag_start].to_string());
        cursor = close_tag_end;
    }

    Ok(ParsedOutput { fields })
}

fn json_fallback_parse(text: &str) -> Result<ParsedOutput, ParseError> {
    let parsed: JsonFallbackOutput = serde_json::from_str(text).map_err(|error| ParseError {
        message: format!("JSON fallback parse failed: {error}"),
    })?;

    let fields = [
        ("response", Some(parsed.response)),
        ("done", Some(parsed.done.to_string())),
        ("conclusion", parsed.conclusion),
        ("needs_more", parsed.needs_more.map(|v| v.to_string())),
        ("revision_of", parsed.revision_of.map(|v| v.to_string())),
        ("thought", parsed.thought),
    ]
    .into_iter()
    .filter_map(|(key, value)| value.map(|v| (key.to_string(), v)))
    .collect();

    Ok(ParsedOutput { fields })
}

fn find_tag(text: &str, name: &str, is_closing: bool, from: usize) -> Option<(usize, usize)> {
    let mut cursor = from;
    while let Some((tag_name, tag_start, tag_end, tag_is_closing)) = next_tag(text, cursor) {
        if tag_is_closing == is_closing && tag_name.eq_ignore_ascii_case(name) {
            return Some((tag_start, tag_end));
        }
        cursor = tag_end;
    }
    None
}

fn next_tag(text: &str, from: usize) -> Option<(String, usize, usize, bool)> {
    let captures = TAG_RE.captures(&text[from..])?;
    let full_match = captures.get(0)?;
    let name = captures.get(2)?;
    Some((
        name.as_str().to_ascii_lowercase(),
        from + full_match.start(),
        from + full_match.end(),
        !captures.get(1)?.as_str().is_empty(),
    ))
}

fn missing_output() -> ParseError {
    ParseError {
        message: format!(
            "No <output> tags found in response. Expected format:\n{GENERIC_EXPECTED_SHAPE}"
        ),
    }
}

#[derive(Debug, Deserialize)]
struct JsonFallbackOutput {
    response: String,
    done: bool,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    needs_more: Option<bool>,
    #[serde(default)]
    revision_of: Option<i64>,
    #[serde(default)]
    thought: Option<String>,
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

    #[test]
    fn output_parser_matches_tags_case_insensitively() {
        let parsed = parse_output(
            "<Output><Response>Hello</ReSpOnSe><DONE>true</Done></OUTPUT>",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("Hello"));
        assert_eq!(parsed.get_bool("done"), Some(true));
    }

    #[test]
    fn output_parser_accepts_whitespace_around_tag_names() {
        let parsed = parse_output(
            " \n <output   >\n  <response   >Hello</response   >\n  <done   >true</done   >\n </output   > \n ",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("Hello"));
        assert_eq!(parsed.get_bool("done"), Some(true));
    }

    #[test]
    fn output_parser_accepts_newlines_inside_tags() {
        let parsed = parse_output(
            "<output\n>\n<response\n>hello</response\n>\n<done\n>false</done\n>\n</output\n>",
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("hello"));
        assert_eq!(parsed.get_bool("done"), Some(false));
    }

    #[test]
    fn output_parser_falls_back_to_json() {
        let parsed = parse_output(
            r#"{"response":"Hello","done":true,"needs_more":false,"revision_of":7,"conclusion":"Done","thought":"[[step]]"}"#,
        )
        .unwrap();

        assert_eq!(parsed.get("response"), Some("Hello"));
        assert_eq!(parsed.get_bool("done"), Some(true));
        assert_eq!(parsed.get_bool("needs_more"), Some(false));
        assert_eq!(parsed.get_u64("revision_of"), Some(7));
        assert_eq!(parsed.get("conclusion"), Some("Done"));
        assert_eq!(parsed.get("thought"), Some("[[step]]"));
    }

    #[test]
    fn output_parser_returns_xml_error_when_json_fallback_is_invalid() {
        let error = parse_output(r#"{"response":"Hello","done":"true"}"#).unwrap_err();

        assert_eq!(
            error,
            ParseError {
                message: "No <output> tags found in response. Expected format:\n<output>\n  <field>...</field>\n</output>".to_string()
            }
        );
    }
}
