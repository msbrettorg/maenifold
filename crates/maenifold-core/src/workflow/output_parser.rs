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
    parse_xml_output(text).or_else(|xml_error| json_fallback_parse(text).map_err(|_| xml_error))
}

fn parse_xml_output(text: &str) -> Result<ParsedOutput, ParseError> {
    let (_, output_start_end) = find_tag(text, "output", false, 0).ok_or_else(missing_output)?;
    let (output_end_start, _) =
        find_tag(text, "output", true, output_start_end).ok_or_else(missing_output)?;

    let inner = &text[output_start_end..output_end_start];
    let mut fields = HashMap::new();
    let mut cursor = 0;

    while let Some((name, _, tag_end, is_closing)) = next_tag(inner, cursor) {
        cursor = tag_end;
        if is_closing {
            continue;
        }

        let value_start = tag_end;
        let Some((close_tag_start, close_tag_end)) = find_tag(inner, &name, true, value_start)
        else {
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

