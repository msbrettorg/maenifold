use super::context::{ToolContext, load_help_doc};
use crate::db;
use crate::error::Result;
use crate::util::json::{OutputFormat, success};
use serde_json::Value;
use std::fmt::Write;
use std::path::Path;

pub fn get_config(context: &ToolContext, payload: &Value) -> Result<String> {
    // T-1.5: RTM FR-1 / FR-3 / FR-4 / FR-5 / FR-6 preserve C#-compatible learn mode for system diagnostics.
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "GetConfig")
            .unwrap_or_else(|| "ERROR: Help file not found for GetConfig".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let rendered = render_config_markdown(context);
    Ok(match context.output_format {
        OutputFormat::Json => success(rendered),
        OutputFormat::Markdown => rendered,
    })
}

// T-13.4: RTM FR-7 — Complete MemoryStatus output to match C# parity.
// C# SystemTools.MemoryStatus returns: file count, folder count, total size MB,
// graph database stats (concepts, relations, mentions), and DB file size MB.
pub fn memory_status(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "MemoryStatus")
            .unwrap_or_else(|| "ERROR: Help file not found for MemoryStatus".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let rendered = render_memory_status_markdown(context);
    Ok(match context.output_format {
        OutputFormat::Json => success(rendered),
        OutputFormat::Markdown => rendered,
    })
}

// T-13.4: RTM FR-7 — Render the full markdown status matching C# SystemTools.MemoryStatus.
// T-16.2: RTM FR-7 — Fixed to only count .md files and non-hidden folders (C# parity).
#[allow(clippy::too_many_lines)]
fn render_memory_status_markdown(context: &ToolContext) -> String {
    let mut stats = String::from("# Memory System Status\n\n");

    // T-16.2: RTM FR-7 — File count, folder count, total size from walkdir scan.
    // C# MarkdownIO.GetMemoryFiles() only returns *.md files excluding hidden paths.
    // C# MarkdownIO.GetMemoryDirectories() only returns non-hidden directories.
    let mut file_count: u64 = 0;
    let mut folder_count: u64 = 0;
    let mut total_size: u64 = 0;
    let memory_root = &context.config.memory_path;

    for entry in walkdir::WalkDir::new(memory_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        // T-16.2: Skip hidden entries — any path segment starting with '.' (len > 1)
        // matches C# MarkdownIO.IsHiddenPath which checks all relative segments.
        if is_hidden_path(entry.path(), memory_root) {
            continue;
        }

        if entry.file_type().is_file() {
            // T-16.2: Only count .md files, matching C# GetMemoryFiles("*.md") filter.
            let is_md = entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if is_md {
                file_count += 1;
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        } else if entry.file_type().is_dir() && entry.depth() > 0 {
            // depth() > 0 excludes the root directory itself, matching C#
            // MarkdownIO.GetMemoryDirectories() which lists subdirectories only.
            folder_count += 1;
        }
    }

    let total_size_mb = total_size as f64 / 1024.0 / 1024.0;

    let _ = writeln!(stats, "### Files");
    let _ = writeln!(stats, "- Total files: {file_count}");
    let _ = writeln!(stats, "- Total folders: {folder_count}");
    let _ = writeln!(stats, "- Total size: {total_size_mb:.2} MB");

    // T-13.4: RTM FR-7 — Graph database stats from SQLite queries.
    let db_path = &context.config.database_path;
    if db_path.exists() {
        match db::open_connection(db_path) {
            Ok(conn) => {
                let concept_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM concepts", [], |row| row.get(0))
                    .unwrap_or(0);
                let relation_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM concept_graph", [], |row| row.get(0))
                    .unwrap_or(0);
                let mention_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM concept_mentions", [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0);

                let _ = writeln!(stats, "\n### Graph Database");
                let _ = writeln!(stats, "- Concepts: {concept_count}");
                let _ = writeln!(stats, "- Relations: {relation_count}");
                let _ = writeln!(stats, "- Mentions: {mention_count}");

                // T-13.4: RTM FR-7 — DB file size from filesystem metadata.
                let db_size_mb = std::fs::metadata(db_path)
                    .map(|m| m.len() as f64 / 1024.0 / 1024.0)
                    .unwrap_or(0.0);
                let _ = writeln!(stats, "- Database size: {db_size_mb:.2} MB");
            }
            Err(_) => {
                let _ = writeln!(
                    stats,
                    "\n### Graph Database\n- Not initialized (run sync first)"
                );
            }
        }
    } else {
        let _ = writeln!(
            stats,
            "\n### Graph Database\n- Not initialized (run sync first)"
        );
    }

    stats
}

pub(crate) fn render_config_markdown(context: &ToolContext) -> String {
    let settings = &context.config.settings;
    format!(
        "Maenifold Configuration:\n  Memory Path: {}\n  Database: {}\n  Debounce: {}ms\n  Auto Sync: {}\n  Decay Config:\n    Grace Days (Sequential): {}\n    Grace Days (Workflows): {}\n    Grace Days (Default): {}\n    Half-Life Days: {}\n    Function: {}\n  Community Detection:\n    Louvain Gamma: {}\n    Sibling Min Shared Neighbors: {}\n    Sibling Min Overlap: {}\n    Sibling Max Results: {}\n    Watcher Debounce: {}ms\n    NMI Drift Threshold: {}",
        context.config.memory_path.display(),
        context.config.database_path.display(),
        settings.debounce_ms,
        // T-10.9: RTM FR-15
        if context.config.enable_auto_sync() {
            "True"
        } else {
            "False"
        },
        context.config.decay_grace_days_sequential(),
        context.config.decay_grace_days_workflows(),
        context.config.decay_grace_days_default(),
        context.config.decay_half_life_days(),
        context.config.decay_function(),
        settings.community.louvain_gamma,
        settings.community.min_shared_neighbors,
        settings.community.min_overlap,
        settings.community.max_siblings,
        settings.community.debounce_ms,
        // T-14.6: RTM FR-15
        settings.community.nmi_threshold,
    )
}

// T-16.2: RTM FR-7 — Check if a path has any hidden segment (starts with '.', len > 1).
// Mirrors C# MarkdownIO.IsHiddenPath — checks all relative path segments.
fn is_hidden_path(path: &Path, base_path: &Path) -> bool {
    let relative = path.strip_prefix(base_path).unwrap_or(path);
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}
