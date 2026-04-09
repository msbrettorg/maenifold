use crate::markdown::Frontmatter;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::Path;

pub(crate) fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> String {
    frontmatter
        .get(key)
        .and_then(YamlValue::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn workflow_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://thinking/workflow/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn pipeline_session_uri(session_id: &str) -> String {
    let (year, month, day) = date_parts_from_session_id(session_id);
    format!("memory://pipelines/{year:04}/{month:02}/{day:02}/{session_id}")
}

pub(crate) fn display_utc_now() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

pub(crate) fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = extract_session_timestamp_millis(session_id);
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}

pub(crate) fn extract_session_timestamp_millis(session_id: &str) -> Option<i64> {
    // T-14.10: RTM FR-16 - accept suffix-free session IDs and scan from the right
    // so hyphenated pipeline/workflow prefixes do not break timestamp extraction.
    if session_id
        .split('-')
        .any(|segment| segment.trim().is_empty())
    {
        return None;
    }

    for segment in session_id.rsplit('-') {
        if segment.len() == 13
            && segment.bytes().all(|byte| byte.is_ascii_digit())
            && let Ok(millis) = segment.parse::<i64>()
        {
            if Utc.timestamp_millis_opt(millis).single().is_some() {
                return Some(millis);
            }
        }
    }
    None
}

pub(crate) fn now_iso8601_roundtrip() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

pub(crate) fn frontmatter_timestamp_or_mtime(
    frontmatter: &Frontmatter,
    key: &str,
    path: &Path,
) -> String {
    let value = frontmatter_string(frontmatter, key);
    if !value.is_empty() {
        return value;
    }

    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(|modified| {
            let modified: DateTime<Utc> = modified.into();
            modified.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
        })
        .unwrap_or_default()
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    let path = assets_path
        .join("usage")
        .join("tools")
        .join(format!("{}.md", tool_name.to_ascii_lowercase()));
    fs::read_to_string(path).ok()
}
