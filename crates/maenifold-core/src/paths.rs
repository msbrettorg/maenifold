use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use chrono::{Datelike, TimeZone, Utc};
use std::path::{Path, PathBuf};

const ABSOLUTE_PATH_ERROR: &str = "Absolute paths are not allowed. Use relative paths only.";
const DOT_SEQUENCE_ERROR: &str = "Path traversal components (dot sequences) are not allowed.";

pub fn validate_relative_path(input: &str) -> Result<PathBuf> {
    if Path::new(input).is_absolute() {
        return Err(MaenifoldError::Message(ABSOLUTE_PATH_ERROR.to_string()));
    }

    let decoded = urlencoding::decode(input)
        .map(|v| v.into_owned())
        .unwrap_or_else(|_| input.to_string());
    let normalized = decoded.replace('\\', "/");

    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        let compact: String = segment.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact.is_empty() && compact.chars().all(|c| c == '.') {
            return Err(MaenifoldError::Message(DOT_SEQUENCE_ERROR.to_string()));
        }
    }

    Ok(PathBuf::from(normalized.trim_matches('/')))
}

pub fn slugify_title(title: &str) -> String {
    let lower = title.to_lowercase();
    let with_hyphens = lower.replace([' ', '_'], "-");
    let filtered: String = with_hyphens
        .chars()
        .filter(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '-')
        .collect();
    let mut collapsed = String::new();
    let mut previous_dash = false;
    for ch in filtered.chars() {
        if ch == '-' {
            if !previous_dash {
                collapsed.push(ch);
            }
            previous_dash = true;
        } else {
            collapsed.push(ch);
            previous_dash = false;
        }
    }
    let collapsed = collapsed.trim_matches('-').to_string();
    if collapsed.is_empty() {
        "untitled".to_string()
    } else {
        collapsed
    }
}

pub fn memory_uri_to_path(identifier: &str, config: &Config) -> Result<PathBuf> {
    let Some(rest) = identifier.strip_prefix("memory://") else {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Expected memory:// URI, got {identifier}"
        )));
    };
    let relative = validate_relative_path(rest)?;
    let mut full = config.memory_path.join(relative);
    if full.extension().is_none() {
        full.set_extension("md");
    }
    Ok(full)
}

// T-DATE-001: RTM-001 - Session paths must derive their date directories from the session ID timestamp.
pub fn dated_thinking_path(session_type: &str, session_id: &str, config: &Config) -> PathBuf {
    let (year, month, day) = date_parts_from_session_id(session_id);
    config
        .thinking_path
        .join(session_type)
        .join(format!("{year:04}"))
        .join(format!("{month:02}"))
        .join(format!("{day:02}"))
        .join(format!("{session_id}.md"))
}

// T-DATE-001: RTM-002 - Sequential session paths must avoid regressing to epoch-derived directories.
pub fn dated_sequential_path(session_id: &str, config: &Config) -> PathBuf {
    dated_thinking_path("sequential", session_id, config)
}

fn date_parts_from_session_id(session_id: &str) -> (i32, u32, u32) {
    let millis = session_id
        .split('-')
        .nth(1)
        .and_then(|part| part.parse::<i64>().ok());
    let timestamp = millis
        .and_then(|value| Utc.timestamp_millis_opt(value).single())
        .unwrap_or_else(Utc::now);
    (timestamp.year(), timestamp.month(), timestamp.day())
}
