use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::tools::ToolContext;
use crate::util::json::{OutputFormat, success};
use regex::Regex;
use rusqlite::params;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const ABSOLUTE_PATH_ERROR: &str = "Absolute paths are not allowed. Use relative paths only.";
const DOT_SEQUENCE_ERROR: &str = "Path traversal components (dot sequences) are not allowed.";
static TAG_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").expect("tag regex"));
static PUNCTUATION_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[:|>\-\[\]{}#&*!@`']").expect("punctuation regex"));
static CONTROL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x00-\x1F\x7F-\x9F]").expect("control regex"));
static WHITESPACE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("whitespace regex"));

// T-12.5: RTM FR-25.2 consolidates shared [[memory.rs]] support helpers so
// split tool modules keep the pre-existing behavior and diagnostics.
pub fn get_help(context: &ToolContext, payload: &Value) -> Result<String> {
    let tool_name = payload
        .get("toolName")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("toolName"))?;

    if let Some(content) = load_help_doc(&context.config.assets_path, tool_name) {
        return Ok(match context.output_format {
            OutputFormat::Json => success(content),
            OutputFormat::Markdown => content,
        });
    }

    let tools_dir = context.config.assets_path.join("usage").join("tools");
    let mut available_tools = fs::read_dir(&tools_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    available_tools.sort();

    let rendered = format!(
        "No help file found for tool: {tool_name}\n\nAvailable tools with help:\n{}",
        available_tools
            .into_iter()
            .map(|tool| format!("  - {tool}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(match context.output_format {
        OutputFormat::Json => success(rendered),
        OutputFormat::Markdown => rendered,
    })
}

pub(crate) fn learn_requested(payload: &Value) -> bool {
    payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn render_learn_help(context: &ToolContext, help: &str) -> String {
    match context.output_format {
        OutputFormat::Json => success(help),
        OutputFormat::Markdown => help.to_string(),
    }
}

pub(crate) fn memory_tool_help(tool_name: &str) -> &'static str {
    match tool_name {
        "WriteMemory" => {
            "WriteMemory creates a new memory note and indexes it into the knowledge graph.\n\nParameters:\n- title (string, required): title for the memory file\n- content (string, required): markdown body that must include at least one [[WikiLink]]\n- folder (string, optional): relative folder under the memory root\n- tags (string[], optional): tags stored in frontmatter\n- learn (bool, optional): when true, returns this help text instead of writing a file\n\nUsage examples:\n- {\"title\":\"Azure Savings Plans\",\"content\":\"Learning about [[Azure Savings Plans]] and [[Commitment Discounts]]\"}\n- {\"title\":\"Team Notes\",\"content\":\"# Team Notes\\n\\nCaptured [[Decisions]]\",\"folder\":\"projects/cloud\",\"tags\":[\"finops\",\"research\"]}"
        }
        "ReadMemory" => {
            "ReadMemory loads a memory note by URI or exact title and optionally returns its checksum.\n\nParameters:\n- identifier (string, required): memory URI or exact title\n- includeChecksum (bool, optional, default true): include checksum for safe edits\n- learn (bool, optional): when true, returns this help text instead of reading a file\n\nUsage examples:\n- {\"identifier\":\"memory://projects/cloud/azure-savings-plans\"}\n- {\"identifier\":\"Azure Savings Plans\",\"includeChecksum\":false}"
        }
        "EditMemory" => {
            "EditMemory updates an existing memory note while preserving WikiLink connectivity.\n\nParameters:\n- identifier (string, required): memory URI or exact title\n- operation (string, required): append | prepend | find_replace | replace_section\n- content (string, required): new content; append/prepend/replace_section content must include at least one [[WikiLink]]\n- checksum (string, optional): optimistic concurrency checksum from ReadMemory\n- findText (string, required for find_replace): literal text to replace\n- expectedCount (integer, optional): expected match count for find_replace\n- sectionName (string, required for replace_section): heading text to replace\n- learn (bool, optional): when true, returns this help text instead of editing a file\n\nUsage examples:\n- {\"identifier\":\"memory://projects/cloud/azure-savings-plans\",\"operation\":\"append\",\"content\":\"\\n\\nAdded note about [[Reservations]].\"}\n- {\"identifier\":\"Azure Savings Plans\",\"operation\":\"find_replace\",\"findText\":\"old text\",\"content\":\"new text\",\"expectedCount\":1}"
        }
        "DeleteMemory" => {
            "DeleteMemory removes an existing memory note after explicit confirmation.\n\nParameters:\n- identifier (string, required): memory URI or exact title\n- confirm (bool, required to delete): must be true to proceed\n- learn (bool, optional): when true, returns this help text instead of deleting a file\n\nUsage examples:\n- {\"identifier\":\"memory://archive/obsolete-note\",\"confirm\":true}\n- {\"identifier\":\"Obsolete Note\",\"confirm\":true}"
        }
        "MoveMemory" => {
            "MoveMemory renames or relocates a memory note while preserving its memory:// destination format.\n\nParameters:\n- source (string, required): source memory URI or exact title\n- destination (string, required): destination URI or relative path/name\n- learn (bool, optional): when true, returns this help text instead of moving a file\n\nUsage examples:\n- {\"source\":\"memory://notes/tmp-note\",\"destination\":\"memory://projects/cloud/final-note\"}\n- {\"source\":\"Temporary Note\",\"destination\":\"projects/cloud/final-note.md\"}"
        }
        _ => "No help available for this memory tool.",
    }
}

pub(crate) fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    let path = assets_path
        .join("usage")
        .join("tools")
        .join(format!("{}.md", tool_name.to_ascii_lowercase()));
    fs::read_to_string(path).ok()
}

pub(crate) fn yaml_value_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Number(value) => Some(value.to_string()),
        YamlValue::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

pub fn sanitize_user_input(input: &str) -> String {
    let mut input = if input.trim().is_empty() {
        "Untitled".to_string()
    } else {
        input.to_string()
    };
    if let Ok(decoded) = urlencoding::decode(&input) {
        input = decoded.into_owned();
    }

    input = TAG_PATTERN.replace_all(&input, "").to_string();

    input = PUNCTUATION_PATTERN.replace_all(&input, "").to_string();

    for ch in ['$', '(', ')', ';', '&', '|', '`', '\\', '<', '>', '"'] {
        input = input.replace(ch, "");
    }

    input = input.replace("..", "").replace(['/', '\\'], "");

    input = CONTROL_PATTERN.replace_all(&input, "").to_string();

    input = WHITESPACE_PATTERN
        .replace_all(&input, " ")
        .trim()
        .to_string();

    if input.is_empty() {
        "Untitled".to_string()
    } else {
        input
    }
}

pub(crate) fn validate_relative_folder(folder: &str) -> Option<String> {
    if folder.is_empty() {
        return None;
    }

    if Path::new(folder).is_absolute() {
        return Some(ABSOLUTE_PATH_ERROR.to_string());
    }

    let decoded = urlencoding::decode(folder)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| folder.to_string());
    let normalized = decoded.replace('\\', "/");
    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        let compact: String = segment.chars().filter(|c| !c.is_whitespace()).collect();
        if !compact.is_empty() && compact.chars().all(|c| c == '.') {
            return Some(DOT_SEQUENCE_ERROR.to_string());
        }
    }

    None
}

pub(crate) fn render_filtered_wikilinks_error(
    output_format: OutputFormat,
    blocked: &[(String, String)],
) -> String {
    match output_format {
        OutputFormat::Json => serde_json::to_string(&json!({
            "success": false,
            "data": Value::Null,
            "error": {
                "code": "FILTERED_WIKILINKS",
                "message": "Content contains WikiLinks that are excluded from the knowledge graph",
                "details": {
                    "blocked": blocked.iter().map(|(concept, reason)| json!({
                        "concept": concept,
                        "reason": reason,
                    })).collect::<Vec<_>>()
                }
            }
        }))
        .expect("filtered wikilinks json serialization"),
        // PARITY-WIKIFILTER: RTM NFR-1 — match C# error message text exactly.
        OutputFormat::Markdown => format!(
            "ERROR: Content contains WikiLinks that are excluded from the knowledge graph:\n{}\n\nRemove these WikiLinks from your content and retry.",
            blocked
                .iter()
                .map(|(concept, reason)| format!("- [[{concept}]]: {reason}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

pub(crate) fn destination_to_uri(destination: &str) -> String {
    if destination.starts_with("memory://") {
        return destination.to_string();
    }

    let normalized = destination
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    let path = PathBuf::from(&normalized);
    let parent = path.parent().and_then(|p| {
        let text = p.to_string_lossy().replace('\\', "/");
        if text.is_empty() || text == "." {
            None
        } else {
            Some(text)
        }
    });
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&normalized);
    let file_stem = file_name.strip_suffix(".md").unwrap_or(file_name);
    let slug = crate::markdown::slugify(file_stem);
    match parent {
        Some(parent) => format!("memory://{parent}/{slug}"),
        None => format!("memory://{slug}"),
    }
}

pub(crate) fn index_file_content(
    uri: &str,
    title: &str,
    content: &str,
    checksum: &str,
    context: &ToolContext,
) -> Result<()> {
    GraphDatabase::initialize_database(&context.config)?;
    let connection = db::open_connection(&context.config.database_path)?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    connection.execute(
        "INSERT INTO file_content (file_path, title, content, summary, last_indexed, status, file_md5, file_size, created, last_accessed) \
         VALUES (?1, ?2, ?3, '', ?4, 'saved', ?5, ?6, ?4, ?4) \
         ON CONFLICT(file_path) DO UPDATE SET title=excluded.title, content=excluded.content, last_indexed=excluded.last_indexed, status=excluded.status, file_md5=excluded.file_md5, file_size=excluded.file_size",
        params![uri, title, content, now, checksum, content.len() as i64],
    )?;
    Ok(())
}

pub(crate) fn update_last_accessed(uri: &str, context: &ToolContext) -> Result<()> {
    let connection = db::open_connection(&context.config.database_path)?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    connection.execute(
        "UPDATE file_content SET last_accessed = ?1 WHERE file_path = ?2",
        params![now, uri],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render_filtered_wikilinks_error;
    use crate::util::json::OutputFormat;
    use serde_json::Value;

    #[test]
    fn filtered_wikilinks_json_error_includes_code_and_details() {
        let rendered = render_filtered_wikilinks_error(
            OutputFormat::Json,
            &[("agent".to_string(), "Too generic".to_string())],
        );
        let parsed: Value = serde_json::from_str(&rendered).expect("json");

        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "FILTERED_WIKILINKS");
        assert_eq!(parsed["error"]["details"]["blocked"][0]["concept"], "agent");
        assert_eq!(
            parsed["error"]["details"]["blocked"][0]["reason"],
            "Too generic"
        );
    }
}
