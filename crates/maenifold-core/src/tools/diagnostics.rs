use super::context::{ToolContext, load_help_doc, render};
use crate::acp_config::AgentsConfig;
use crate::db;
use crate::error::Result;
use crate::util::json::{OutputFormat, success};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

// T-172.0: Unified GetConfig — expose full maenifold.json through MCP.
// Returns agents, mcpServers, settings, and resolved paths in a single response.
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

    // T-172.0: Load full maenifold.json to expose agents, MCP servers, and settings.
    let agents_config = AgentsConfig::load(&context.config.maenifold_root)?;
    let response = build_config_value(context, &agents_config);

    Ok(render(context.output_format, response))
}

// T-172.0: Build structured JSON with all config sections from maenifold.json.
fn build_config_value(context: &ToolContext, agents_config: &AgentsConfig) -> Value {
    // Paths — resolved at runtime, not raw from maenifold.json.
    let paths = json!({
        "maenifoldRoot": context.config.maenifold_root.display().to_string(),
        "memoryPath": context.config.memory_path.display().to_string(),
        "assetsPath": context.config.assets_path.display().to_string(),
        "databasePath": context.config.database_path.display().to_string(),
        "thinkingPath": context.config.thinking_path.display().to_string(),
        "sequentialPath": context.config.sequential_path.display().to_string(),
    });

    // Agents — sorted by name for deterministic output.
    // T-172.0 red-team: break command into binary + args and redact sensitive
    // env values.
    let mut agent_names: Vec<&String> = agents_config.agents.keys().collect();
    agent_names.sort();
    let agents: Value = agent_names
        .into_iter()
        .map(|name| {
            let entry = &agents_config.agents[name];
            let mut obj = serde_json::to_value(entry).unwrap_or(json!({}));
            if let Some(map) = obj.as_object_mut() {
                map.insert("name".to_string(), json!(name));

                // Break "command arg1 arg2" into separate command + args fields.
                let (binary, args) = entry.command_parts();
                map.insert("command".to_string(), json!(binary));
                map.insert(
                    "args".to_string(),
                    Value::Array(args.into_iter().map(|a| json!(a)).collect()),
                );

                // Redact sensitive env values.
                if let Some(env_val) = map.get_mut("env") {
                    redact_env(env_val);
                }

                // Redact env in nested mcpServers entries.
                if let Some(Value::Object(servers)) = map.get_mut("mcpServers") {
                    for server in servers.values_mut() {
                        if let Some(env_val) = server.as_object_mut().and_then(|m| m.get_mut("env"))
                        {
                            redact_env(env_val);
                        }
                    }
                }
            }
            obj
        })
        .collect::<Vec<_>>()
        .into();

    // MCP servers — serialized from config, with sensitive env values redacted.
    let mut mcp_servers = serde_json::to_value(&agents_config.mcp_servers).unwrap_or(json!([]));
    if let Value::Object(servers) = &mut mcp_servers {
        for server in servers.values_mut() {
            if let Some(env_val) = server.as_object_mut().and_then(|m| m.get_mut("env")) {
                redact_env(env_val);
            }
        }
    }

    // Settings — the effective settings with defaults applied.
    let settings =
        serde_json::to_value(agents_config.settings.as_ref().cloned().unwrap_or_default())
            .unwrap_or(json!({}));

    json!({
        "paths": paths,
        "agents": agents,
        "mcpServers": mcp_servers,
        "settings": settings,
    })
}

/// T-172.0 red-team remediation: Redact env map values whose keys match
/// sensitive patterns (case-insensitive): key, token, secret, password,
/// credential, auth. All other values pass through unchanged.
fn redact_env(env_val: &mut Value) {
    if let Value::Object(map) = env_val {
        for (key, value) in map.iter_mut() {
            if is_sensitive_env_key(key) {
                *value = json!("***");
            }
        }
    }
}

/// Returns true when an env key name (case-insensitive) contains any of the
/// sensitive substrings that typically guard secrets.
fn is_sensitive_env_key(key: &str) -> bool {
    const SENSITIVE_PATTERNS: &[&str] =
        &["key", "token", "secret", "password", "credential", "auth"];
    let lower = key.to_ascii_lowercase();
    SENSITIVE_PATTERNS.iter().any(|pat| lower.contains(pat))
}

/// Redact sensitive env values in a `HashMap<String, String>` and return the
/// redacted copy. Standalone helper kept available for unit tests.
#[allow(dead_code)]
fn redact_env_map(env: &HashMap<String, String>) -> HashMap<String, String> {
    env.iter()
        .map(|(k, v)| {
            if is_sensitive_env_key(k) {
                (k.clone(), "***".to_string())
            } else {
                (k.clone(), v.clone())
            }
        })
        .collect()
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

// T-172.0: render_config_markdown removed — GetConfig now returns structured JSON
// via build_config_value covering agents, mcpServers, settings, and paths.

// T-16.2: RTM FR-7 — Check if a path has any hidden segment (starts with '.', len > 1).
// Mirrors C# MarkdownIO.IsHiddenPath — checks all relative path segments.
fn is_hidden_path(path: &Path, base_path: &Path) -> bool {
    let relative = path.strip_prefix(base_path).unwrap_or(path);
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}
