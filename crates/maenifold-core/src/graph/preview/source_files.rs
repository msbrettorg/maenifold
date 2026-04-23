// T-01.16: source-files JSON helpers extracted from graph/preview.rs.
// No behavioural changes.

use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

use crate::error::Result;

use super::section::extract_section_with_concept;

// T-13.6: RTM NFR-1 — parses source_files JSON into a JSON array for BuildContext output
pub(crate) fn parse_source_files_for_json(source_files: &str) -> Value {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(array)) => Value::Array(array),
        Ok(Value::Object(map)) => {
            Value::Array(map.keys().cloned().map(Value::String).collect::<Vec<_>>())
        }
        Ok(other) => other,
        Err(_) => Value::Array(
            source_files
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| Value::String(value.to_string()))
                .collect(),
        ),
    }
}

// T-13.6: RTM NFR-1 — extracts file URI strings from source_files JSON
fn parse_source_file_uris(source_files: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(source_files) {
        Ok(Value::Array(entries)) => entries
            .into_iter()
            .filter_map(|entry| entry.as_str().map(str::to_string))
            .collect(),
        // T-0.3: RTM-047 - Preserve source file URI extraction on older serde_json Map APIs.
        Ok(Value::Object(map)) => map.into_iter().map(|(key, _)| key).collect(),
        Ok(Value::String(uri)) => vec![uri],
        Ok(_) => Vec::new(),
        Err(_) => source_files
            .split(',')
            .map(str::trim)
            .filter(|uri| !uri.is_empty())
            .map(str::to_string)
            .collect(),
    }
}

// T-13.6: RTM NFR-1 — loads content previews for source files keyed by URI
pub(crate) fn load_content_preview_map(
    connection: &Connection,
    focus_concept: &str,
    source_files: &str,
) -> Result<Value> {
    let mut previews = serde_json::Map::new();
    for uri in parse_source_file_uris(source_files) {
        let content = connection
            .query_row(
                "SELECT COALESCE(content, '') FROM file_content WHERE file_path = ?1",
                [uri.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(content) = content else {
            continue;
        };
        let preview = extract_section_with_concept(&content, focus_concept, 500);
        if !preview.is_empty() {
            previews.insert(uri, Value::String(preview));
        }
    }
    Ok(Value::Object(previews))
}
