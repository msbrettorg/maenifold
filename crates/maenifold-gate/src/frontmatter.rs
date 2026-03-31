use chrono::{DateTime, Utc};
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};
use std::collections::BTreeMap;
use std::path::Path;

pub type Frontmatter = BTreeMap<String, YamlValue>;

/// Split YAML frontmatter from markdown content.
/// Returns (yaml_text, body) if frontmatter delimiters found.
pub fn split_frontmatter(full_text: &str) -> Option<(&str, String)> {
    let yaml_start = if full_text.starts_with("---\n") {
        4
    } else if full_text.starts_with("---\r\n") {
        5
    } else {
        return None;
    };

    let mut cursor = yaml_start;
    while cursor <= full_text.len() {
        let remaining = &full_text[cursor..];
        match remaining.find('\n') {
            Some(line_end) => {
                let line = remaining[..line_end].trim_end_matches('\r');
                if line == "---" {
                    let yaml_text = &full_text[yaml_start..cursor];
                    let content = remaining[line_end + 1..]
                        .trim_start_matches(['\r', '\n'])
                        .to_string();
                    return Some((yaml_text, content));
                }
                cursor += line_end + 1;
            }
            None => {
                if remaining.trim_end_matches('\r') == "---" {
                    let yaml_text = &full_text[yaml_start..cursor];
                    return Some((yaml_text, String::new()));
                }
                return None;
            }
        }
    }

    None
}

/// Parse YAML frontmatter into a Frontmatter map.
/// Gracefully degrades: malformed YAML returns (None, full content).
pub fn parse_frontmatter(full_text: &str) -> (Option<Frontmatter>, String) {
    if let Some((yaml_text, content)) = split_frontmatter(full_text) {
        if yaml_text.trim().is_empty() {
            return (None, content);
        }

        match serde_yaml::from_str(yaml_text) {
            Ok(frontmatter) => return (Some(frontmatter), content),
            Err(error) => {
                eprintln!("[YAML WARNING] Failed to parse frontmatter: {error}");
                return (None, content);
            }
        }
    }
    (None, full_text.to_string())
}

/// Render frontmatter to YAML string with ordered keys.
/// Gate-specific key ordering: id, status, pipeline, attempts first.
pub fn render_frontmatter(frontmatter: &Frontmatter) -> Result<String, serde_yaml::Error> {
    let mut mapping = YamlMapping::new();
    for key in [
        "id",
        "title",
        "status",
        "pipeline",
        "attempts",
        "requirement",
        "created",
        "modified",
    ] {
        if let Some(value) = frontmatter.get(key) {
            mapping.insert(YamlValue::String(key.to_string()), value.clone());
        }
    }

    for (key, value) in frontmatter {
        if mapping.contains_key(YamlValue::String(key.clone())) {
            continue;
        }
        mapping.insert(YamlValue::String(key.clone()), value.clone());
    }

    Ok(serde_yaml::to_string(&mapping)?.trim_end().to_string())
}

/// Format DateTime as ISO 8601 string.
pub fn format_iso(date_time: DateTime<Utc>) -> String {
    format!(
        "{}.{:07}+00:00",
        date_time.format("%Y-%m-%dT%H:%M:%S"),
        date_time.timestamp_subsec_nanos() / 100,
    )
}

/// Current time as ISO 8601 string.
pub fn now_iso() -> String {
    format_iso(Utc::now())
}

/// Read a task file, returning parsed frontmatter and body content.
pub fn read_task_frontmatter(path: &Path) -> anyhow::Result<(Option<Frontmatter>, String)> {
    let content = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let text = String::from_utf8(content)
        .map_err(|e| anyhow::anyhow!("invalid UTF-8 in {}: {e}", path.display()))?;
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    Ok(parse_frontmatter(text))
}

/// Extract attempts count from frontmatter, accepting numeric or string values.
pub fn read_attempts(fm: &Frontmatter) -> u64 {
    fm.get("attempts")
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
        })
        .unwrap_or(0)
}

/// Write a task file with frontmatter + body content.
/// Auto-manages `created` and `modified` timestamps.
pub fn write_task_with_frontmatter(
    path: &Path,
    frontmatter: Option<&Frontmatter>,
    content: &str,
) -> anyhow::Result<()> {
    let mut envelope = frontmatter.cloned().unwrap_or_default();

    if !envelope.contains_key("created") {
        let created = if path.exists() {
            std::fs::metadata(path)
                .ok()
                .and_then(|m| m.created().ok().or_else(|| m.modified().ok()))
                .map(|st| {
                    let dt: DateTime<Utc> = st.into();
                    format_iso(dt)
                })
                .unwrap_or_else(now_iso)
        } else {
            now_iso()
        };
        envelope.insert("created".to_string(), YamlValue::String(created));
    }
    if !envelope.contains_key("modified") {
        envelope.insert("modified".to_string(), YamlValue::String(now_iso()));
    }

    let mut markdown = String::new();
    if !envelope.is_empty() {
        markdown.push_str("---\n");
        let rendered =
            render_frontmatter(&envelope).map_err(|e| anyhow::anyhow!("YAML render error: {e}"))?;
        markdown.push_str(&rendered);
        if !markdown.ends_with('\n') {
            markdown.push('\n');
        }
        markdown.push_str("---\n\n");
    }
    markdown.push_str(content);
    let dir = path.parent().unwrap_or(Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("task");
    let tmp_path = dir.join(format!(".{name}.tmp"));
    std::fs::write(&tmp_path, &markdown)
        .map_err(|e| anyhow::anyhow!("failed to write temp file {}: {e}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::anyhow!(
            "failed to rename {} → {}: {e}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// Read-modify-write: update frontmatter fields.
/// Refuses to overwrite when frontmatter delimiters exist but YAML is malformed.
pub fn update_task_frontmatter<F>(path: &Path, mutator: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut Frontmatter),
{
    let raw = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    let text = String::from_utf8(raw)
        .map_err(|e| anyhow::anyhow!("invalid UTF-8 in {}: {e}", path.display()))?;
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);

    let (mut fm, body) = match split_frontmatter(text) {
        Some((yaml_text, body)) if !yaml_text.trim().is_empty() => {
            let fm: Frontmatter = serde_yaml::from_str(yaml_text).map_err(|e| {
                anyhow::anyhow!(
                    "refusing to update {}: file has frontmatter delimiters \
                     but YAML is malformed: {e}",
                    path.display()
                )
            })?;
            (fm, body)
        }
        Some((_, body)) => (Frontmatter::new(), body),
        None => (Frontmatter::new(), text.to_string()),
    };

    mutator(&mut fm);
    fm.insert("modified".to_string(), YamlValue::String(now_iso()));
    write_task_with_frontmatter(path, Some(&fm), &body)
}

#[cfg(test)]
#[path = "frontmatter_tests.rs"]
mod tests;
