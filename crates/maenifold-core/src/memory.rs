use crate::db::{self, GraphDatabase};
use crate::error::{MaenifoldError, Result};
use crate::json::{OutputFormat, failure, success};
use crate::markdown::{
    Frontmatter, delete_file, file_exists, find_file_as_uri, read_markdown, slugify,
    update_markdown, validate_uri, write_markdown,
};
use crate::tools::ToolContext;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use regex::Regex;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const ABSOLUTE_PATH_ERROR: &str = "Absolute paths are not allowed. Use relative paths only.";
const DOT_SEQUENCE_ERROR: &str = "Path traversal components (dot sequences) are not allowed.";

pub fn write_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("title"))?;
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("content"))?;
    let folder = payload.get("folder").and_then(Value::as_str);
    let tags = payload.get("tags").and_then(Value::as_array).cloned();

    let title = sanitize_user_input(title);
    // T-0.1: RTM NFR-3 - reject overlong write targets before filesystem I/O so CLI JSON callers get a structured error without path leakage.
    if slugify(&title).len() > 255 {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "PATH_TOO_LONG",
                "Memory title is too long to create a safe file path.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Memory title is too long to create a safe file path.".to_string()
            }
        });
    }
    let concepts = crate::markdown::extract_wikilinks(content);
    if concepts.is_empty() {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.
Example: 'Learning about [[Machine Learning]] and [[Data Science]]'
This ensures your note is connected to the knowledge graph and not orphaned.".to_string(),
        });
    }

    // T-0.3c: RTM NFR-4 / RTM-047
    if let Some(folder) = folder
        && let Some(error) = validate_relative_folder(folder)
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure("PATH_VALIDATION_ERROR", error),
            OutputFormat::Markdown => format!("ERROR: Invalid folder path - {error}"),
        });
    }

    let slug = slugify(&title);
    let uri = match folder {
        Some(folder) if !folder.is_empty() => format!("memory://{folder}/{slug}"),
        _ => format!("memory://{slug}"),
    };

    let mut frontmatter: Frontmatter = BTreeMap::new();
    frontmatter.insert("title".to_string(), YamlValue::String(title.clone()));
    frontmatter.insert("permalink".to_string(), YamlValue::String(slug));
    frontmatter.insert("type".to_string(), YamlValue::String("memory".to_string()));
    frontmatter.insert("status".to_string(), YamlValue::String("saved".to_string()));
    if let Some(tags) = tags {
        let tags = tags
            .into_iter()
            .filter_map(|value| {
                value
                    .as_str()
                    .map(|text| YamlValue::String(text.to_string()))
            })
            .collect();
        frontmatter.insert("tags".to_string(), YamlValue::Sequence(tags));
    }

    let full_content = if content.trim_start().starts_with("# ") {
        content.to_string()
    } else {
        format!("# {title}\n\n{content}")
    };

    // T-2.4: RTM FR-11.2 / NFR-11.2.1 blocks writes before mutating memory files.
    if let Err(blocked) = crate::markdown::WikiLinkFilter::new(&context.config.memory_path)
        .try_check(&full_content)?
    {
        return Ok(render_filtered_wikilinks_error(
            context.output_format,
            &blocked,
        ));
    }

    write_markdown(&uri, Some(&frontmatter), &full_content, &context.config)?;
    let (_, _, checksum) = read_markdown(&uri, &context.config)?;
    index_file_content(&uri, &title, &full_content, &checksum, context)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "uri": uri,
            "checksum": checksum,
            "title": title,
            "folder": folder,
        })),
        OutputFormat::Markdown => format!("Created memory FILE: {uri}\nChecksum: {checksum}"),
    })
}

pub fn read_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let include_checksum = payload
        .get("includeChecksum")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        find_file_as_uri(identifier, &context.config).ok_or_else(|| {
            MaenifoldError::Message(format!("Memory file not found: {identifier}"))
        })?
    };

    if !file_exists(&uri, &context.config)? {
        return Ok(match context.output_format {
            OutputFormat::Json => {
                failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
            }
            OutputFormat::Markdown => format!("ERROR: Memory file not found: {identifier}"),
        });
    }

    let (frontmatter, content, checksum) = read_markdown(&uri, &context.config)?;
    update_last_accessed(&uri, context).ok();

    let title = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("title"))
        .and_then(yaml_value_to_string)
        .unwrap_or_else(|| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .unwrap_or_default()
                .to_string()
        });
    let location = uri.trim_start_matches("memory://").to_string();
    let created = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("created"))
        .and_then(yaml_value_to_string);
    let modified = frontmatter
        .as_ref()
        .and_then(|frontmatter| frontmatter.get("modified"))
        .and_then(yaml_value_to_string);

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({
            "uri": uri,
            "title": title,
            "location": location,
            "created": created,
            "modified": modified,
            "checksum": if include_checksum { Value::String(checksum) } else { Value::Null },
            "content": content,
        })),
        OutputFormat::Markdown => {
            let mut output = format!(
                "# {title}\nURI: {uri}\nLocation: {location}\nCreated: {}\nModified: {}\n",
                created.unwrap_or_default(),
                modified.unwrap_or_default(),
            );
            if include_checksum {
                output.push_str(&format!("Checksum: {checksum}\n"));
            }
            output.push_str("\n---\n\n");
            output.push_str(&content);
            output
        }
    })
}

pub fn edit_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let operation = payload
        .get("operation")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("operation"))?;
    let content = payload
        .get("content")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("content"))?;
    let checksum = payload.get("checksum").and_then(Value::as_str);

    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        find_file_as_uri(identifier, &context.config).ok_or_else(|| {
            MaenifoldError::Message(format!("Memory file not found: {identifier}"))
        })?
    };

    let (mut frontmatter, existing_content, _) = read_markdown(&uri, &context.config)?;
    if matches!(operation, "append" | "prepend" | "replace_section")
        && crate::markdown::extract_wikilinks(content).is_empty()
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Edited content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Edited content must contain at least one [[concept]] in double brackets to connect to the knowledge graph."
                    .to_string()
            }
        });
    }
    let updated_content = match operation {
        "append" => format!("{existing_content}{content}"),
        "prepend" => format!("{content}{existing_content}"),
        "find_replace" => {
            let find_text = payload
                .get("findText")
                .and_then(Value::as_str)
                .ok_or(MaenifoldError::MissingProperty("findText"))?;
            let expected_count = payload.get("expectedCount").and_then(Value::as_i64);
            if let Some(expected_count) = expected_count {
                let actual_count = count_non_overlapping_occurrences(&existing_content, find_text);
                if actual_count != expected_count as usize {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "Expected {expected_count} matches but found {actual_count}. Find text: '{find_text}'"
                    )));
                }
            }
            existing_content.replace(find_text, content)
        }
        "replace_section" => {
            let section_name = payload
                .get("sectionName")
                .and_then(Value::as_str)
                .ok_or(MaenifoldError::MissingProperty("sectionName"))?;
            replace_section(&existing_content, section_name, content)?
        }
        _ => {
            return Err(MaenifoldError::InvalidArgument(format!(
                "Unsupported operation: {operation}"
            )));
        }
    };

    if contains_malformed_wikilinks(&updated_content)
        || crate::markdown::extract_wikilinks(&updated_content).is_empty()
    {
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "VALIDATION_ERROR",
                "Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.",
            ),
            OutputFormat::Markdown => {
                "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph."
                    .to_string()
            }
        });
    }

    // T-2.4: RTM FR-11.2 / NFR-11.2.2 / NFR-11.2.3 reports blocked [[WikiLinks]] in both output modes.
    if let Err(blocked) = crate::markdown::WikiLinkFilter::new(&context.config.memory_path)
        .try_check(&updated_content)?
    {
        return Ok(render_filtered_wikilinks_error(
            context.output_format,
            &blocked,
        ));
    }

    let frontmatter = frontmatter.get_or_insert_with(BTreeMap::new);
    frontmatter
        .entry("type".to_string())
        .or_insert_with(|| YamlValue::String("memory".to_string()));
    frontmatter
        .entry("status".to_string())
        .or_insert_with(|| YamlValue::String("saved".to_string()));

    update_markdown(
        &uri,
        Some(frontmatter),
        &updated_content,
        checksum,
        &context.config,
    )?;
    let (_, _, new_checksum) = read_markdown(&uri, &context.config)?;

    Ok(format!(
        "Updated memory FILE: {uri}\nNew checksum: {new_checksum}"
    ))
}

pub fn delete_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    let identifier = payload
        .get("identifier")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("identifier"))?;
    let confirm = payload
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !confirm {
        // T-1.6: RTM NFR-1 - confirmation failures stay in the JSON envelope.
        return Ok(match context.output_format {
            OutputFormat::Json => failure(
                "CONFIRMATION_REQUIRED",
                "Must set confirm=true to delete a memory file",
            ),
            OutputFormat::Markdown => {
                "ERROR: Must set confirm=true to delete a memory file".to_string()
            }
        });
    }

    let uri = if identifier.starts_with("memory://") {
        identifier.to_string()
    } else {
        match find_file_as_uri(identifier, &context.config) {
            Some(uri) => uri,
            None => {
                return Ok(match context.output_format {
                    OutputFormat::Json => {
                        failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
                    }
                    OutputFormat::Markdown => format!("ERROR: Memory file not found: {identifier}"),
                });
            }
        }
    };

    if !file_exists(&uri, &context.config)? {
        return Ok(match context.output_format {
            OutputFormat::Json => {
                failure("NOT_FOUND", format!("Memory file not found: {identifier}"))
            }
            OutputFormat::Markdown => format!("ERROR: Memory file not found: {identifier}"),
        });
    }

    delete_file(&uri, &context.config)?;

    Ok(match context.output_format {
        OutputFormat::Json => success(json!({ "uri": uri, "deleted": true })),
        OutputFormat::Markdown => format!("Deleted memory FILE: {uri}"),
    })
}

pub fn move_memory(context: &ToolContext, payload: &Value) -> Result<String> {
    let source = payload
        .get("source")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("source"))?;
    let destination = payload
        .get("destination")
        .and_then(Value::as_str)
        .ok_or(MaenifoldError::MissingProperty("destination"))?;

    let source_uri = if source.starts_with("memory://") {
        source.to_string()
    } else {
        find_file_as_uri(source, &context.config)
            .ok_or_else(|| MaenifoldError::Message(format!("Memory file not found: {source}")))?
    };
    let source_path = crate::markdown::uri_to_path(&source_uri, &context.config.memory_path)?;

    let destination_uri = destination_to_uri(destination);
    validate_uri(&destination_uri, "destination")?;
    let destination_path =
        crate::markdown::uri_to_path(&destination_uri, &context.config.memory_path)?;
    if destination_path.exists() {
        return Ok(format!(
            "ERROR: Destination file already exists: {destination_uri}"
        ));
    }
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(source_path, destination_path)?;

    Ok(format!(
        "Moved memory FILE: {source_uri} → {destination_uri}"
    ))
}

pub fn list_memories(context: &ToolContext, payload: &Value) -> Result<String> {
    let learn = payload
        .get("learn")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if learn {
        let help = load_help_doc(&context.config.assets_path, "ListMemories")
            .unwrap_or_else(|| "ERROR: Help file not found for ListMemories".to_string());
        return Ok(match context.output_format {
            OutputFormat::Json => success(help),
            OutputFormat::Markdown => help,
        });
    }

    let path = payload.get("path").and_then(Value::as_str);
    if let Some(path) = path
        && let Some(error) = validate_relative_folder(path)
    {
        return Ok(format!("ERROR: Invalid path - {error}"));
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
        return Ok("ERROR: Resolved path escapes memory root".to_string());
    }

    let last_accessed_cache = load_last_accessed_timestamps(&context.config)?;
    // T-0.3c: RTM NFR-4 / RTM-047
    let mut output = match path {
        Some(path) => format!(
            "# Directory: {path}

"
        ),
        None => "# Directory: /

"
        .to_string(),
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
        "No help file found for tool: {tool_name}

Available tools with help:
{}",
        available_tools
            .into_iter()
            .map(|tool| format!("  - {tool}"))
            .collect::<Vec<_>>()
            .join(
                "
"
            )
    );
    Ok(match context.output_format {
        OutputFormat::Json => success(rendered),
        OutputFormat::Markdown => rendered,
    })
}

#[derive(Debug)]
struct DecayMetadata {
    created: String,
    last_accessed: String,
    decay_weight: f64,
}

fn load_help_doc(assets_path: &Path, tool_name: &str) -> Option<String> {
    let path = assets_path
        .join("usage")
        .join("tools")
        .join(format!("{}.md", tool_name.to_ascii_lowercase()));
    fs::read_to_string(path).ok()
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

    let connection = db::open_connection(&config.database_path)?;
    let mut statement = connection.prepare(
        "SELECT file_path, last_accessed FROM file_content WHERE last_accessed IS NOT NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (file_path, timestamp) = row?;
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
        decay_weight: compute_decay_weight(
            &file_path.to_string_lossy(),
            Some(&effective_last_accessed.to_rfc3339()),
            config,
            None,
        ),
    })
}

fn read_created_date(file_path: &Path) -> Result<DateTime<Utc>> {
    match fs::read_to_string(file_path) {
        Ok(full_text) => {
            let (frontmatter, _) = parse_frontmatter_map(&full_text)?;
            if let Some(parsed) = frontmatter
                .and_then(|frontmatter| frontmatter.get("created").and_then(yaml_value_to_string))
                .and_then(|created| parse_datetime(&created))
            {
                return Ok(parsed);
            }
            Ok(system_time_to_datetime(file_path)?)
        }
        Err(_) => Ok(system_time_to_datetime(file_path)?),
    }
}

fn parse_frontmatter_map(full_text: &str) -> Result<(Option<Frontmatter>, String)> {
    if let Some(rest) = full_text.strip_prefix("---\n")
        && let Some(index) = rest.find("\n---\n")
    {
        let yaml_text = &rest[..index];
        let content = rest[index + 5..]
            .trim_start_matches(['\r', '\n'])
            .to_string();
        let frontmatter: Frontmatter = serde_yaml::from_str(yaml_text)?;
        return Ok((Some(frontmatter), content));
    }
    Ok((None, full_text.to_string()))
}

fn system_time_to_datetime(file_path: &Path) -> Result<DateTime<Utc>> {
    let metadata = fs::metadata(file_path)?;
    let system_time = metadata
        .created()
        .or_else(|_| metadata.modified())
        .map_err(MaenifoldError::from)?;
    Ok(DateTime::<Utc>::from(system_time))
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
        })
}

pub fn compute_decay_weight(
    file_path: &str,
    last_accessed: Option<&str>,
    config: &crate::config::Config,
    status: Option<&str>,
) -> f64 {
    // T-4.4: RTM FR-4.4
    let mut grace_period_days = grace_period_for_path(file_path, config);
    let mut half_life_days = config.decay_half_life_days();
    let normalized_path = normalize_decay_path(file_path, config);
    // T-7.8: RTM FR-7.8
    if (normalized_path.contains("/assumptions/") || normalized_path.contains("/chat/"))
        && let Some(status) = status
    {
        match status.trim().to_ascii_lowercase().as_str() {
            "validated" => return 1.0,
            "invalidated" => {
                grace_period_days = 7;
                half_life_days = 14;
            }
            "active" | "refined" => {
                grace_period_days = 14;
                half_life_days = 30;
            }
            _ => {}
        }
    }
    let decay_function = config.decay_function();

    let Some(reference_date) = last_accessed.and_then(parse_datetime) else {
        return 1.0;
    };
    let age_days = (Utc::now() - reference_date).num_days();
    if decay_function.eq_ignore_ascii_case("power-law") {
        if age_days <= grace_period_days {
            return 1.0;
        }
        let effective_age = (age_days - grace_period_days).max(1) as f64;
        return (1.0 * effective_age.powf(-0.5)).clamp(0.0, 1.0);
    }

    if age_days < 0 {
        return 1.0;
    }
    if age_days <= grace_period_days {
        return 1.0;
    }
    if half_life_days <= 0 {
        return 0.0;
    }
    let exponent = (age_days - grace_period_days) as f64 / half_life_days as f64;
    (0.5f64.powf(exponent)).clamp(0.0, 1.0)
}

// T-QUAL-002.6a: RTM FR-25.2
pub fn get_decay_weight_for_file(file_identifier: &str, config: &crate::config::Config) -> f64 {
    let connection = GraphDatabase::initialize_database(config)
        .and_then(|_| db::open_connection(&config.database_path));

    match connection {
        Ok(connection) => {
            get_decay_weight_for_file_with_connection(file_identifier, &connection, config)
        }
        Err(_) => get_decay_weight_from_disk(file_identifier, config),
    }
}

// T-QUAL-002.6a: RTM FR-25.2
fn get_decay_weight_for_file_with_connection(
    file_identifier: &str,
    connection: &Connection,
    config: &crate::config::Config,
) -> f64 {
    let row = connection
        .query_row(
            "SELECT created, last_accessed, status FROM file_content WHERE file_path = ?1",
            params![file_identifier],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional();

    match row {
        Ok(Some((created, last_accessed, status))) => {
            if let Some(last_accessed) = last_accessed.as_deref().and_then(parse_datetime) {
                return compute_decay_weight(
                    file_identifier,
                    Some(&last_accessed.to_rfc3339()),
                    config,
                    status.as_deref(),
                );
            }

            if let Some(created_value) = created.as_deref() {
                if let Some(created) = parse_datetime(created_value) {
                    return compute_decay_weight(
                        file_identifier,
                        Some(&created.to_rfc3339()),
                        config,
                        status.as_deref(),
                    );
                }

                emit_degraded_decay_diagnostic(
                    "GetDecayWeightForFile(reuse-conn)",
                    "Malformed file_content.created timestamp; using current UTC as degraded fallback",
                    file_identifier,
                    config,
                );
                let now = Utc::now().to_rfc3339();
                return compute_decay_weight(
                    file_identifier,
                    Some(&now),
                    config,
                    status.as_deref(),
                );
            }

            let now = Utc::now().to_rfc3339();
            compute_decay_weight(file_identifier, Some(&now), config, status.as_deref())
        }
        Ok(None) | Err(_) => get_decay_weight_from_disk(file_identifier, config),
    }
}

// T-QUAL-002.6a: RTM FR-25.2
fn get_decay_weight_from_disk(file_identifier: &str, config: &crate::config::Config) -> f64 {
    let path = if file_identifier.starts_with("memory://") {
        crate::markdown::uri_to_path(file_identifier, &config.memory_path)
    } else {
        Ok(PathBuf::from(file_identifier))
    };

    match path.and_then(|path| {
        read_created_date(&path).map(|created| {
            (
                path,
                compute_decay_weight(file_identifier, Some(&created.to_rfc3339()), config, None),
            )
        })
    }) {
        Ok((_, decay_weight)) => decay_weight,
        Err(_) => {
            emit_degraded_decay_diagnostic(
                "GetDecayWeightFromDisk",
                "Falling back to neutral decay weight",
                file_identifier,
                config,
            );
            1.0
        }
    }
}

fn emit_degraded_decay_diagnostic(
    method: &str,
    message: &str,
    file_identifier: &str,
    config: &crate::config::Config,
) {
    emit_stderr_line(&format!(
        "[DEGRADED][MemorySearchTools.Fusion][{method}] {message} for '{}'",
        redact_diagnostic_path(file_identifier, config)
    ));
}

fn redact_diagnostic_path(file_identifier: &str, config: &crate::config::Config) -> String {
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
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_identifier.to_string());
    }

    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
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

fn grace_period_for_path(file_path: &str, config: &crate::config::Config) -> i64 {
    let normalized = normalize_decay_path(file_path, config);
    if normalized.contains("/thinking/sequential/") {
        return config.decay_grace_days_sequential();
    }
    if normalized.contains("/thinking/workflow/") || normalized.contains("/thinking/workflows/") {
        return config.decay_grace_days_workflows();
    }
    config.decay_grace_days_default()
}

fn normalize_decay_path(file_path: &str, config: &crate::config::Config) -> String {
    let normalized = file_path.replace('\\', "/").to_ascii_lowercase();
    let memory_root = config
        .memory_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    if normalized.starts_with("memory://") {
        return normalized.replacen("memory://", &format!("{memory_root}/"), 1);
    }

    normalized
}

fn replace_section(
    existing_content: &str,
    section_name: &str,
    new_content: &str,
) -> Result<String> {
    let mut line_start = 0usize;
    let mut section_start = None;
    let mut heading_end = None;
    let mut section_end = existing_content.len();

    for line in existing_content.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let heading_text = trimmed.trim_end().trim_start_matches('#').trim();
            if section_start.is_some() {
                section_end = line_start;
                break;
            }
            if heading_text == section_name {
                section_start = Some(line_start);
                heading_end = Some(line_start + line.len());
            }
        }
        line_start += line.len();
    }

    let section_start = section_start.ok_or_else(|| {
        MaenifoldError::Message(format!(
            "Section not found: '{section_name}'. Cannot replace a section that doesn't exist."
        ))
    })?;
    let heading_end = heading_end.expect("heading end set with section start");

    let replacement = if starts_with_markdown_heading(new_content) {
        new_content.trim_end().to_string()
    } else {
        format!(
            "{}{}",
            &existing_content[section_start..heading_end],
            new_content.trim_end()
        )
    };

    Ok(format!(
        "{}{}{}",
        &existing_content[..section_start],
        replacement,
        &existing_content[section_end..]
    ))
}

fn count_non_overlapping_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    haystack.match_indices(needle).count()
}

fn starts_with_markdown_heading(content: &str) -> bool {
    content
        .trim_start()
        .lines()
        .next()
        .map(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#')
                && trimmed
                    .chars()
                    .nth(1)
                    .map(char::is_whitespace)
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn contains_malformed_wikilinks(content: &str) -> bool {
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index + 1 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    if cursor + 2 < bytes.len() && bytes[cursor + 2] == b']' {
                        return true;
                    }
                    index = cursor + 2;
                    break;
                }
                (b'[', _) | (b']', _) => return true,
                _ => cursor += 1,
            }
        }

        if cursor + 1 >= bytes.len() {
            return true;
        }
    }

    false
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

    let tag_pattern = Regex::new(r"<[^>]*>").expect("tag regex");
    input = tag_pattern.replace_all(&input, "").to_string();

    let punctuation_pattern = Regex::new(r"[:|>\-\[\]{}#&*!@`']").expect("punctuation regex");
    input = punctuation_pattern.replace_all(&input, "").to_string();

    for ch in ['$', '(', ')', ';', '&', '|', '`', '\\', '<', '>', '"'] {
        input = input.replace(ch, "");
    }

    input = input.replace("..", "").replace(['/', '\\'], "");

    let control_pattern = Regex::new(r"[\x00-\x1F\x7F-\x9F]").expect("control regex");
    input = control_pattern.replace_all(&input, "").to_string();

    let whitespace_pattern = Regex::new(r"\s+").expect("whitespace regex");
    input = whitespace_pattern
        .replace_all(&input, " ")
        .trim()
        .to_string();

    if input.is_empty() {
        "Untitled".to_string()
    } else {
        input
    }
}

fn validate_relative_folder(folder: &str) -> Option<String> {
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

fn render_filtered_wikilinks_error(
    output_format: OutputFormat,
    blocked: &[(String, String)],
) -> String {
    match output_format {
        OutputFormat::Json => serde_json::to_string(&json!({
            "success": false,
            "data": Value::Null,
            "error": {
                "code": "FILTERED_WIKILINKS",
                "message": "Content contains filtered WikiLinks",
                "details": {
                    "blocked": blocked.iter().map(|(concept, reason)| json!({
                        "concept": concept,
                        "reason": reason,
                    })).collect::<Vec<_>>()
                }
            }
        }))
        .expect("filtered wikilinks json serialization"),
        OutputFormat::Markdown => format!(
            "ERROR: Content contains filtered WikiLinks that are blocked:\n{}\n\nRemove these WikiLinks or update .wikilink-filter to allow them.",
            blocked
                .iter()
                .map(|(concept, reason)| format!("- [[{concept}]]: {reason}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

fn yaml_value_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(value) => Some(value.clone()),
        YamlValue::Number(value) => Some(value.to_string()),
        YamlValue::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn destination_to_uri(destination: &str) -> String {
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
    let slug = slugify(file_stem);
    match parent {
        Some(parent) => format!("memory://{parent}/{slug}"),
        None => format!("memory://{slug}"),
    }
}

fn index_file_content(
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

fn update_last_accessed(uri: &str, context: &ToolContext) -> Result<()> {
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
    use crate::json::OutputFormat;
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
