use super::paths::{file_exists, uri_to_path};
use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use chrono::{DateTime, Utc};
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::time::SystemTime;

pub type Frontmatter = BTreeMap<String, YamlValue>;

pub fn generate_checksum(content: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    BASE64.encode(hasher.finalize())
}

pub fn read_markdown(uri: &str, config: &Config) -> Result<(Option<Frontmatter>, String, String)> {
    let path = uri_to_path(uri, &config.memory_path)?;
    if !path.exists() {
        return Err(MaenifoldError::Message(format!(
            "Memory file not found: {uri}"
        )));
    }
    let full_text = fs::read_to_string(&path)?;
    // T-15.18: RTM NFR-1 — Strip UTF-8 BOM (U+FEFF) that fs::read_to_string preserves.
    // C# File.ReadAllText strips BOM implicitly via StreamReader; without this,
    // the BOM character corrupts frontmatter delimiter detection ("---").
    let full_text = full_text.strip_prefix('\u{FEFF}').unwrap_or(&full_text);
    let checksum = generate_checksum(full_text);
    // T-14.2: RTM FR-10 — Malformed YAML frontmatter degrades to empty/null frontmatter.
    let (frontmatter, content) = parse_frontmatter(full_text, uri);
    Ok((frontmatter, content, checksum))
}

pub fn write_markdown(
    uri: &str,
    frontmatter: Option<&Frontmatter>,
    content: &str,
    config: &Config,
) -> Result<()> {
    let path = uri_to_path(uri, &config.memory_path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut envelope = frontmatter.cloned().unwrap_or_default();
    if !envelope.contains_key("created") {
        let created = if path.exists() {
            fs::metadata(&path)
                .ok()
                .and_then(|m| m.created().ok().or_else(|| m.modified().ok()))
                .map(system_time_to_iso)
                .unwrap_or_else(now_iso)
        } else {
            now_iso()
        };
        envelope.insert("created".to_string(), YamlValue::String(created));
    }
    envelope.insert("modified".to_string(), YamlValue::String(now_iso()));
    for key in [
        "embedding",
        "embedding_base64",
        "embedding_model",
        "embedding_date",
    ] {
        envelope.remove(key);
    }

    let mut markdown = String::new();
    if !envelope.is_empty() {
        markdown.push_str("---\n");
        markdown.push_str(&render_frontmatter(&envelope)?);
        if !markdown.ends_with('\n') {
            markdown.push('\n');
        }
        markdown.push_str("---\n\n");
    }
    markdown.push_str(content);

    fs::write(path, markdown)?;
    Ok(())
}

pub fn update_markdown(
    uri: &str,
    frontmatter: Option<&Frontmatter>,
    content: &str,
    expected_checksum: Option<&str>,
    config: &Config,
) -> Result<()> {
    if let Some(expected_checksum) = expected_checksum {
        let (_, _, actual_checksum) = read_markdown(uri, config)?;
        if actual_checksum != expected_checksum {
            // PARITY-VAL005-ORDER: RTM NFR-1 — include both checksums in error message
            return Err(MaenifoldError::Message(format!(
                "Checksum mismatch. Expected: {expected_checksum}, Actual: {actual_checksum}"
            )));
        }
    }

    write_markdown(uri, frontmatter, content, config)
}

pub fn append_h2_section(uri: &str, heading: &str, content: &str, config: &Config) -> Result<()> {
    // T-15.17: RTM NFR-1 — Match C# AppendH2Section: only default to empty on NotFound.
    // Permission/corruption errors must propagate, not be swallowed by unwrap_or_default().
    let (frontmatter, existing_content, _) = if file_exists(uri, config)? {
        read_markdown(uri, config)?
    } else {
        (None, String::new(), String::new())
    };

    let mut envelope = frontmatter.unwrap_or_default();
    let section_summary = summarize_section_for_frontmatter(heading, content);
    if existing_content.trim().is_empty() && !envelope.contains_key("first") {
        envelope.insert(
            "first".to_string(),
            YamlValue::String(section_summary.clone()),
        );
    }
    envelope.insert("last".to_string(), YamlValue::String(section_summary));

    let mut new_content = existing_content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("\n## {heading}\n\n{content}\n"));

    write_markdown(uri, Some(&envelope), &new_content, config)
}

fn summarize_section_for_frontmatter(heading: &str, content: &str) -> String {
    format!("{heading}: {content}").chars().take(256).collect()
}

fn parse_frontmatter(full_text: &str, uri: &str) -> (Option<Frontmatter>, String) {
    // T-14.3: RTM FR-10 — Frontmatter delimiters accept LF/CRLF and closing --- at EOF.
    if let Some((yaml_text, content)) = split_frontmatter(full_text) {
        if yaml_text.trim().is_empty() {
            return (None, content);
        }

        match serde_yaml::from_str(yaml_text) {
            Ok(frontmatter) => return (Some(frontmatter), content),
            Err(error) => {
                // T-14.2: RTM FR-10 — Match C# oracle: warn and continue with null frontmatter.
                eprintln!("[YAML WARNING] Failed to parse frontmatter in '{uri}': {error}");
                return (None, content);
            }
        }
    }
    (None, full_text.to_string())
}

fn split_frontmatter(full_text: &str) -> Option<(&str, String)> {
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

fn render_frontmatter(frontmatter: &Frontmatter) -> Result<String> {
    let mut mapping = YamlMapping::new();
    for key in [
        "title",
        "permalink",
        "type",
        "status",
        "confidence",
        "context",
        "validation_plan",
        "workflowId",
        "queue",
        "currentWorkflow",
        "currentStep",
        "phase",
        "activeSubmachineType",
        "activeSubmachineSessionId",
        "analysisType",
        "parent",
        "related",
        "first",
        "last",
        "tags",
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

fn format_iso(date_time: DateTime<Utc>) -> String {
    format!(
        "{}.{:07}+00:00",
        date_time.format("%Y-%m-%dT%H:%M:%S"),
        date_time.timestamp_subsec_nanos() / 100,
    )
}

fn now_iso() -> String {
    format_iso(Utc::now())
}

fn system_time_to_iso(system_time: SystemTime) -> String {
    let date_time: DateTime<Utc> = system_time.into();
    format_iso(date_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use parking_lot::Mutex;
    use tempfile::TempDir;

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct ConfigResetGuard;

    impl Drop for ConfigResetGuard {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    fn test_config() -> (TempDir, ConfigResetGuard, Config) {
        let tmp = TempDir::new().expect("tmp");
        Config::override_root(tmp.path());
        Config::set_database_path(tmp.path().join("memory.db"));
        let config = Config::current();
        config.ensure_directories().expect("ensure directories");
        (tmp, ConfigResetGuard, config)
    }

    fn frontmatter_string(frontmatter: &Frontmatter, key: &str) -> Option<String> {
        frontmatter
            .get(key)
            .and_then(YamlValue::as_str)
            .map(str::to_string)
    }

    #[test]
    fn append_h2_section_sets_first_last_on_initial_write() {
        let _guard = TEST_MUTEX.lock();
        let (_tmp, _reset, config) = test_config();
        let uri = "memory://append-first-last-initial";

        append_h2_section(
            uri,
            "Thought 1",
            "Opening [[concept]] with enough detail to preserve verbatim markup.",
            &config,
        )
        .expect("append section");

        let (frontmatter, _, _) = read_markdown(uri, &config).expect("read markdown");
        let frontmatter = frontmatter.expect("frontmatter");
        let expected =
            "Thought 1: Opening [[concept]] with enough detail to preserve verbatim markup.";

        assert_eq!(
            frontmatter_string(&frontmatter, "first").as_deref(),
            Some(expected)
        );
        assert_eq!(
            frontmatter_string(&frontmatter, "last").as_deref(),
            Some(expected)
        );
    }

    #[test]
    fn append_h2_section_first_last_preserve_first_and_update_last() {
        let _guard = TEST_MUTEX.lock();
        let (_tmp, _reset, config) = test_config();
        let uri = "memory://append-first-last-update";
        let mut frontmatter = Frontmatter::new();
        frontmatter.insert(
            "first".to_string(),
            YamlValue::String("Original: keep me".to_string()),
        );

        write_markdown(uri, Some(&frontmatter), "Existing body", &config).expect("write markdown");

        let long_content = format!("Most recent [[state]] {}", "x".repeat(300));
        append_h2_section(uri, "Thought 2", &long_content, &config).expect("append section");

        let (frontmatter, _, _) = read_markdown(uri, &config).expect("read markdown");
        let frontmatter = frontmatter.expect("frontmatter");
        let expected_last: String = format!("Thought 2: {long_content}")
            .chars()
            .take(256)
            .collect();

        assert_eq!(
            frontmatter_string(&frontmatter, "first").as_deref(),
            Some("Original: keep me")
        );
        assert_eq!(
            frontmatter_string(&frontmatter, "last").as_deref(),
            Some(expected_last.as_str())
        );
        assert_eq!(expected_last.chars().count(), 256);
        assert!(
            expected_last.contains("[[state]]"),
            "wikilinks must be preserved in last"
        );
    }

    #[test]
    fn first_last_frontmatter_render_after_related_before_tags() {
        let mut frontmatter = Frontmatter::new();
        frontmatter.insert("tags".to_string(), YamlValue::Sequence(vec![]));
        frontmatter.insert(
            "last".to_string(),
            YamlValue::String("Last value".to_string()),
        );
        frontmatter.insert(
            "related".to_string(),
            YamlValue::Sequence(vec![YamlValue::String("memory://other".to_string())]),
        );
        frontmatter.insert(
            "first".to_string(),
            YamlValue::String("First value".to_string()),
        );

        let rendered = render_frontmatter(&frontmatter).expect("render frontmatter");
        let related_index = rendered.find("related:").expect("related");
        let first_index = rendered.find("first:").expect("first");
        let last_index = rendered.find("last:").expect("last");
        let tags_index = rendered.find("tags:").expect("tags");

        assert!(
            related_index < first_index,
            "related should render before first"
        );
        assert!(first_index < last_index, "first should render before last");
        assert!(last_index < tags_index, "last should render before tags");
    }
}
