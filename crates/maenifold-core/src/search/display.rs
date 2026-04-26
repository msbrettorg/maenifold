use crate::error::{MaenifoldError, Result};
use crate::markdown::{Frontmatter, path_to_uri, read_markdown};
use crate::search::text::{build_snippet, slugify_query, tokenize_query};
use crate::tools::ToolContext;
use serde_yml::Value as YamlValue;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    fs::read_to_string(
        assets_path
            .join("usage")
            .join("tools")
            .join(format!("{}.md", tool_name.to_ascii_lowercase())),
    )
    .ok()
}

// T-12.7: RTM NFR-1 preserves the public [[search]] display helper surface after the directory-module split.
pub fn get_file_display_info(
    file_identifier: &str,
    query: &str,
    config: &crate::config::Config,
) -> (String, String) {
    let fallback_title = fallback_display_title(file_identifier);
    let query_terms = tokenize_query(query);
    let query_slug = slugify_query(query);

    let loaded = if file_identifier.starts_with("memory://") {
        read_markdown(file_identifier, config)
            .map(|(frontmatter, content, _)| (frontmatter, content))
    } else if let Ok(uri) = path_to_uri(Path::new(file_identifier), &config.memory_path) {
        read_markdown(&uri, config).map(|(frontmatter, content, _)| (frontmatter, content))
    } else {
        fs::read_to_string(file_identifier)
            .map(|content| (None, content))
            .map_err(MaenifoldError::from)
    };

    match loaded {
        Ok((frontmatter, content)) => (
            frontmatter
                .as_ref()
                .and_then(frontmatter_title)
                .unwrap_or_else(|| fallback_title.clone()),
            build_snippet(&content, &query_terms, &query_slug),
        ),
        Err(_) => {
            emit_stderr_line(&format!(
                "[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo] Falling back to filename/snippet for '{}'",
                redact_display_info_path(file_identifier, config)
            ));
            (fallback_title, "[DEGRADED] Snippet unavailable".to_string())
        }
    }
}

pub(crate) fn file_matches_all_tags(
    context: &ToolContext,
    file_uri: &str,
    required_tags: &[String],
) -> Result<bool> {
    let Ok((frontmatter, _, _)) = read_markdown(file_uri, &context.config) else {
        return Ok(false);
    };
    let file_tags = frontmatter
        .as_ref()
        .map(frontmatter_tags)
        .unwrap_or_default();
    Ok(required_tags.iter().all(|tag| file_tags.contains(tag)))
}

fn frontmatter_title(frontmatter: &Frontmatter) -> Option<String> {
    frontmatter.get("title").and_then(|value| match value {
        YamlValue::String(title) if !title.trim().is_empty() => Some(title.clone()),
        _ => None,
    })
}

fn frontmatter_tags(frontmatter: &Frontmatter) -> BTreeSet<String> {
    match frontmatter.get("tags") {
        Some(YamlValue::Sequence(values)) => values
            .iter()
            .filter_map(YamlValue::as_str)
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(|tag| tag.to_ascii_lowercase())
            .collect(),
        Some(YamlValue::String(value)) if !value.trim().is_empty() => {
            BTreeSet::from([value.trim().to_ascii_lowercase()])
        }
        _ => BTreeSet::new(),
    }
}

fn fallback_display_title(file_identifier: &str) -> String {
    if let Some(relative) = file_identifier.strip_prefix("memory://") {
        return relative
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(relative)
            .to_string();
    }

    let path = Path::new(file_identifier);
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|segment| segment.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_identifier.to_string())
}

fn redact_display_info_path(file_identifier: &str, config: &crate::config::Config) -> String {
    if let Some(relative) = file_identifier.strip_prefix("memory://") {
        return relative
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty())
            .unwrap_or(relative)
            .to_string();
    }

    let path = Path::new(file_identifier);
    if path.is_absolute() && !path.starts_with(&config.memory_path) {
        return path
            .file_name()
            .map(|segment| segment.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_identifier.to_string());
    }

    path.file_name()
        .or_else(|| path.file_stem())
        .map(|segment| segment.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_identifier.to_string())
}

#[cfg(unix)]
fn emit_stderr_line(message: &str) {
    unsafe extern "C" {
        fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    }

    let line = format!("{message}\n");
    // SAFETY: the pointer remains valid for the duration of the syscall and fd=2 is stderr.
    let _ = unsafe { write(2, line.as_ptr(), line.len()) };
}

#[cfg(not(unix))]
fn emit_stderr_line(message: &str) {
    eprintln!("{message}");
}
