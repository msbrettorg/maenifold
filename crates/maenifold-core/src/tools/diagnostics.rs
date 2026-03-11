use super::context::{load_help_doc, ToolContext};
use crate::db;
use crate::error::Result;
use crate::util::json::{success, OutputFormat};
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
                    .query_row("SELECT COUNT(*) FROM concept_mentions", [], |row| row.get(0))
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
    format!(
        "Maenifold Configuration:\n  Memory Path: {}\n  Database: {}\n  Debounce: {}ms\n  Auto Sync: {}\n  Decay Config:\n    Grace Days (Sequential): {}\n    Grace Days (Workflows): {}\n    Grace Days (Default): {}\n    Half-Life Days: {}\n    Function: {}\n  Community Detection:\n    Louvain Gamma: {}\n    Sibling Min Shared Neighbors: {}\n    Sibling Min Overlap: {}\n    Sibling Max Results: {}\n    Watcher Debounce: {}ms\n    NMI Drift Threshold: {}",
        context.config.memory_path.display(),
        context.config.database_path.display(),
        std::env::var("MAENIFOLD_DEBOUNCE_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(150),
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
        std::env::var("MAENIFOLD_LOUVAIN_GAMMA")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "1".to_string()),
        std::env::var("MAENIFOLD_COMMUNITY_MIN_SHARED")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(3),
        std::env::var("MAENIFOLD_COMMUNITY_MIN_OVERLAP")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.4),
        std::env::var("MAENIFOLD_COMMUNITY_MAX_SIBLINGS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10),
        std::env::var("MAENIFOLD_COMMUNITY_DEBOUNCE_MS")
            .ok()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(2_000),
        // T-14.6: RTM FR-15
        std::env::var("MAENIFOLD_COMMUNITY_NMI_THRESHOLD")
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0.8".to_string()),
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
