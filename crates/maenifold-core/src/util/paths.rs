use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use chrono::{Datelike, TimeZone, Utc};
use std::fs;
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

// T-15.25: RTM FR-12 - walk up the directory tree until an existing ancestor is found,
// canonicalize it, then re-append the non-existent tail components. This allows nested
// destination paths (e.g. `subfolder/new-doc`) to resolve even when intermediate
// directories have not been created yet, while still detecting symlink escapes.
fn canonicalize_existing_or_parent(path: &Path) -> std::io::Result<PathBuf> {
    // Fast path: the full path already exists.
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }

    // Collect non-existent tail components while walking towards the root.
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path.to_path_buf();

    loop {
        if let Some(name) = current.file_name() {
            tail.push(name.to_os_string());
        }
        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
            }
            _ => {
                // Reached filesystem root without finding an existing ancestor.
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no existing ancestor for path: {}", path.display()),
                ));
            }
        }
        match fs::canonicalize(&current) {
            Ok(canonical_ancestor) => {
                let mut result = canonical_ancestor;
                for component in tail.into_iter().rev() {
                    result = result.join(component);
                }
                return Ok(result);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Keep walking up.
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

// T-14.5: RTM FR-12 - canonicalize joined paths so symlink targets cannot escape the configured root.
pub fn join_validated_relative_path(base_path: &Path, input: &str) -> Result<PathBuf> {
    let relative = validate_relative_path(input)?;
    join_validated_path_buf(base_path, &relative)
}

// T-14.5: RTM FR-12 - shared canonicalization helper for callers that already parsed a relative PathBuf.
pub fn join_validated_path_buf(base_path: &Path, relative: &Path) -> Result<PathBuf> {
    let joined = base_path.join(relative);
    let canonical_base = fs::canonicalize(base_path).unwrap_or_else(|_| base_path.to_path_buf());
    let canonical_joined = canonicalize_existing_or_parent(&joined)?;

    if !canonical_joined.starts_with(&canonical_base) {
        return Err(MaenifoldError::PathTraversal(format!(
            "Resolved path escapes base directory: {}",
            relative.display()
        )));
    }

    Ok(joined)
}

pub fn slugify_title(title: &str) -> String {
    let result = ::slug::slugify(title);
    if result.is_empty() {
        "untitled".to_string()
    } else {
        result
    }
}

pub fn memory_uri_to_path(identifier: &str, config: &Config) -> Result<PathBuf> {
    let Some(rest) = identifier.strip_prefix("memory://") else {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Expected memory:// URI, got {identifier}"
        )));
    };
    // T-14.5: RTM FR-12 - reject memory URIs whose resolved symlink target escapes the memory root.
    let mut full = join_validated_relative_path(&config.memory_path, rest)?;
    // T-15.19: RTM NFR-1 - append .md unless already ends with .md (case-insensitive), matching C# UriToPath.
    if !full
        .as_os_str()
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".md")
    {
        let mut name = full.file_name().unwrap_or_default().to_os_string();
        name.push(".md");
        full.set_file_name(name);
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
