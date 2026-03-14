use super::support::load_help_doc;
use crate::db;
use crate::decay::{
    compute_decay_weight as decay_compute_decay_weight, emit_stderr_line, parse_datetime,
    read_created_date, redact_diagnostic_path,
};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use chrono::{DateTime, Local, Utc};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct DecayMetadata {
    created: String,
    last_accessed: String,
    decay_weight: f64,
}

// T-12.5: RTM FR-25.2 moves listing/decay metadata concerns into a focused
// submodule while preserving the public `memory::list_memories` surface.
pub fn list_memories(context: &ToolContext, payload: &serde_json::Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "ListMemories")
            .unwrap_or_else(|| "ERROR: Help file not found for ListMemories".to_string());
        return Ok(match context.output_format {
            crate::util::json::OutputFormat::Json => crate::util::json::success(help),
            crate::util::json::OutputFormat::Markdown => help,
        });
    }

    let path = payload.get("path").and_then(serde_json::Value::as_str);
    if let Some(path) = path
        && let Some(error) = super::support::validate_relative_folder(path)
    {
        return Err(MaenifoldError::Message(format!("Invalid path - {error}")));
    }

    let target_path = match path {
        Some(relative) if !relative.is_empty() => context.config.memory_path.join(relative),
        _ => context.config.memory_path.clone(),
    };
    if !target_path.is_dir() {
        return Ok(format!("Directory not found: {}", path.unwrap_or_default()));
    }
    // T-QUAL-002.6h: RTM FR-25.6
    let canonical_memory_root = fs::canonicalize(&context.config.memory_path)?;
    let canonical_target_path = fs::canonicalize(&target_path)?;
    if !canonical_target_path.starts_with(&canonical_memory_root) {
        return Err(MaenifoldError::Message("Resolved path escapes memory root".to_string()));
    }

    let last_accessed_cache = load_last_accessed_timestamps(&context.config)?;
    // T-0.3c: RTM NFR-4 / RTM-047
    let mut output = match path {
        Some(path) => format!("# Directory: {path}\n\n"),
        None => "# Directory: /\n\n".to_string(),
    };

    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in fs::read_dir(&target_path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if is_hidden_entry(&path, &context.config.memory_path) {
            continue;
        }
        if file_type.is_dir() {
            directories.push(path);
        } else if file_type.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
        {
            files.push(path);
        }
    }

    directories.sort();
    if !directories.is_empty() {
        output.push_str("### Folders\n");
        for dir in directories {
            let name = dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let file_count = count_top_level_markdown_files(&dir, &context.config.memory_path)?;
            output.push_str(&format!("📁 {name}/ ({file_count} files)\n"));
        }
        output.push('\n');
    }

    files.sort();
    if !files.is_empty() {
        output.push_str("### Files\n");
        for file_path in files {
            let file_uri = crate::markdown::path_to_uri(&file_path, &context.config.memory_path)?;
            let name = format!(
                "{}.md",
                file_uri
                    .trim_start_matches("memory://")
                    .split('/')
                    .next_back()
                    .unwrap_or_default()
            );
            let info = fs::metadata(&file_path)?;
            output.push_str(&format!(
                "📄 {} ({:.1} KB)\n",
                name,
                info.len() as f64 / 1024.0
            ));

            let decay_metadata = get_decay_metadata_for_file(
                &file_uri,
                &file_path,
                &last_accessed_cache,
                &context.config,
            )?;
            output.push_str(&format!("   created: {}\n", decay_metadata.created));
            output.push_str(&format!(
                "   last_accessed: {}\n",
                decay_metadata.last_accessed
            ));
            output.push_str(&format!(
                "   decay_weight: {:.2}\n",
                decay_metadata.decay_weight
            ));
        }
    }

    Ok(output)
}

fn is_hidden_entry(path: &Path, memory_root: &Path) -> bool {
    path.strip_prefix(memory_root)
        .unwrap_or(path)
        .components()
        .any(|component| {
            let text = component.as_os_str().to_string_lossy();
            text.starts_with('.') && text.len() > 1
        })
}

fn count_top_level_markdown_files(dir: &Path, memory_root: &Path) -> Result<usize> {
    Ok(fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| !is_hidden_entry(path, memory_root))
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md")
        })
        .count())
}

fn load_last_accessed_timestamps(
    config: &crate::config::Config,
) -> Result<BTreeMap<String, DateTime<Utc>>> {
    let mut cache = BTreeMap::new();
    if !config.database_path.exists() {
        return Ok(cache);
    }

    let connection = match db::open_connection(&config.database_path) {
        Ok(connection) => connection,
        Err(error) => {
            // T-0.1: RTM FR-25.2 preserves degraded diagnostics when last_accessed cache loading fails.
            emit_system_tools_diagnostic(
                "LoadLastAccessedTimestamps",
                "Failed to open SQLite cache; returning empty timestamp dictionary",
                &config.database_path.to_string_lossy(),
                config,
                &error.to_string(),
            );
            return Ok(cache);
        }
    };
    let mut statement = match connection.prepare(
        "SELECT file_path, last_accessed FROM file_content WHERE last_accessed IS NOT NULL",
    ) {
        Ok(statement) => statement,
        Err(error) => {
            emit_system_tools_diagnostic(
                "LoadLastAccessedTimestamps",
                "Failed to query last_accessed timestamps; returning empty timestamp dictionary",
                &config.database_path.to_string_lossy(),
                config,
                &error.to_string(),
            );
            return Ok(cache);
        }
    };
    let rows = match statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            emit_system_tools_diagnostic(
                "LoadLastAccessedTimestamps",
                "Failed to enumerate last_accessed timestamps; returning empty timestamp dictionary",
                &config.database_path.to_string_lossy(),
                config,
                &error.to_string(),
            );
            return Ok(cache);
        }
    };

    for row in rows {
        let (file_path, timestamp) = match row {
            Ok(row) => row,
            Err(error) => {
                emit_system_tools_diagnostic(
                    "LoadLastAccessedTimestamps",
                    "Failed to read a last_accessed row; returning empty timestamp dictionary",
                    &config.database_path.to_string_lossy(),
                    config,
                    &error.to_string(),
                );
                return Ok(BTreeMap::new());
            }
        };
        if let Some(parsed) = parse_datetime(&timestamp) {
            cache.insert(file_path, parsed);
        }
    }

    Ok(cache)
}

fn get_decay_metadata_for_file(
    file_uri: &str,
    file_path: &Path,
    last_accessed_cache: &BTreeMap<String, DateTime<Utc>>,
    config: &crate::config::Config,
) -> Result<DecayMetadata> {
    let created_date = read_created_date(file_path)?;
    let last_accessed_date = last_accessed_cache.get(file_uri).cloned();
    let effective_last_accessed = last_accessed_date.unwrap_or(created_date);

    Ok(DecayMetadata {
        created: created_date
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string(),
        last_accessed: effective_last_accessed
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string(),
        // T-4.4: RTM FR-4.4
        decay_weight: decay_compute_decay_weight(
            &file_path.to_string_lossy(),
            Some(&effective_last_accessed.to_rfc3339()),
            config,
            None,
        ),
    })
}

fn emit_system_tools_diagnostic(
    method: &str,
    message: &str,
    file_identifier: &str,
    config: &crate::config::Config,
    detail: &str,
) {
    emit_stderr_line(&format!(
        "[DEGRADED][SystemTools][{method}] {message} for '{}': {detail}",
        redact_diagnostic_path(file_identifier, config)
    ));
}
