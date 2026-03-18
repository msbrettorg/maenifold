use crate::config::Config;
use crate::decay::compute_decay_weight_with_created;
use crate::error::Result;
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(crate) fn relation_weighted_score(
    connection: &Connection,
    config: &Config,
    co_occurrence_count: f64,
    source_files: &str,
) -> Result<(f64, f64)> {
    let trimmed = source_files.trim();
    if trimmed.is_empty() {
        return Ok((0.0, 0.0));
    }

    let mut weighted_score = 0.0;
    let mut document_count = 0.0;

    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(map)) => {
            for (uri, weight) in map {
                let file_weight = weight.as_f64().unwrap_or(1.0);
                let decay_weight = load_file_decay_weight(connection, config, &uri)?;
                weighted_score += file_weight * decay_weight;
                document_count += 1.0;
            }
        }
        Ok(Value::Array(entries)) => {
            for entry in entries {
                if let Some(uri) = entry.as_str() {
                    let decay_weight = load_file_decay_weight(connection, config, uri)?;
                    weighted_score += decay_weight;
                    document_count += 1.0;
                }
            }
        }
        _ => {
            for uri in trimmed
                .split(',')
                .map(str::trim)
                .filter(|uri| !uri.is_empty())
            {
                let decay_weight = load_file_decay_weight(connection, config, uri)?;
                weighted_score += decay_weight;
                document_count += 1.0;
            }
        }
    }

    if document_count <= 0.0 {
        return Ok((0.0, 0.0));
    }

    let decay_weight = (weighted_score / document_count).clamp(0.0, 1.0);
    Ok((decay_weight, co_occurrence_count * decay_weight))
}

fn load_file_decay_weight(connection: &Connection, config: &Config, uri: &str) -> Result<f64> {
    let row = connection
        .query_row(
            "SELECT created, last_accessed, status FROM file_content WHERE file_path = ?1",
            [uri],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;

    let Some((created, last_accessed, status)) = row else {
        return Ok(1.0);
    };

    // T-14.1: RTM FR-21 keeps graph weighting consistent with C# fallback
    // semantics when access timestamps are absent.
    Ok(compute_decay_weight_with_created(
        uri,
        last_accessed.as_deref(),
        created.as_deref(),
        config,
        status.as_deref(),
    ))
}

pub(crate) fn emit_graph_tools_diagnostic(method: &str, message: &str, detail: &str) {
    emit_graph_tools_stderr_line(&format!(
        "[DEGRADED][GraphTools][{method}] {message}: {detail}"
    ));
}

#[cfg(unix)]
fn emit_graph_tools_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let mut line = String::with_capacity(message.len() + 1);
    line.push_str(message);
    line.push('\n');
    // SAFETY: fd=2 is the process stderr and the buffer is valid for the call duration.
    unsafe {
        let _ = write(2, line.as_ptr(), line.len());
    }
}

#[cfg(not(unix))]
fn emit_graph_tools_stderr_line(message: &str) {
    eprintln!("{message}");
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

#[allow(dead_code)] // Used by graph visualization — retained for future use
pub(crate) fn format_source_files_for_display(source_files: &str) -> String {
    let trimmed = source_files.trim();
    // T-0.3b: RTM NFR-4
    if trimmed.starts_with('{') {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed) {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            return keys.join(", ");
        }
    }
    source_files.to_string()
}

// T-4.3: RTM FR-23.x
pub(crate) fn format_co_occurrence_weight(weight: f64) -> String {
    if (weight.fract()).abs() < f64::EPSILON {
        format!("{weight:.0}")
    } else {
        format!("{weight:.1}")
    }
}
