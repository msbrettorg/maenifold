use crate::config::Config;
use crate::error::{MaenifoldError, Result};
use crate::paths::{slugify_title, validate_relative_path};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub type Frontmatter = BTreeMap<String, YamlValue>;

#[derive(Debug, Clone)]
struct CachedWikiLinkFilter {
    modified: SystemTime,
    blocked: BTreeMap<String, String>,
}

// T-2.4: RTM FR-11.1 / NFR-11.1.2 keeps `.wikilink-filter` reads behind an mtime cache.
static WIKILINK_FILTER_CACHE: Lazy<Mutex<BTreeMap<PathBuf, CachedWikiLinkFilter>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

#[derive(Debug, Clone)]
pub struct WikiLinkFilter {
    filter_path: PathBuf,
}

impl WikiLinkFilter {
    pub fn new(memory_path: &Path) -> Self {
        Self {
            filter_path: memory_path.join(".wikilink-filter"),
        }
    }

    pub fn check(&self, content: &str) -> std::result::Result<(), Vec<(String, String)>> {
        self.load_blocked_concepts()
            .map(|blocked| self.check_against(content, &blocked))
            .unwrap_or(Ok(()))
    }

    pub fn try_check(
        &self,
        content: &str,
    ) -> Result<std::result::Result<(), Vec<(String, String)>>> {
        let blocked = self.load_blocked_concepts()?;
        Ok(self.check_against(content, &blocked))
    }

    fn check_against(
        &self,
        content: &str,
        blocked: &BTreeMap<String, String>,
    ) -> std::result::Result<(), Vec<(String, String)>> {
        if blocked.is_empty() {
            return Ok(());
        }

        let blocked_links = extract_wikilinks(content)
            .into_iter()
            .filter_map(|concept| {
                blocked
                    .get(&concept)
                    .map(|reason| (concept.clone(), reason.clone()))
            })
            .collect::<Vec<_>>();

        if blocked_links.is_empty() {
            Ok(())
        } else {
            Err(blocked_links)
        }
    }

    fn load_blocked_concepts(&self) -> Result<BTreeMap<String, String>> {
        // T-2.6: RTM FR-11.1 rejects symlinked `.wikilink-filter` files so policy stays rooted in memory.
        let metadata = match fs::symlink_metadata(&self.filter_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                WIKILINK_FILTER_CACHE.lock().remove(&self.filter_path);
                return Ok(BTreeMap::new());
            }
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            WIKILINK_FILTER_CACHE.lock().remove(&self.filter_path);
            return Ok(BTreeMap::new());
        }
        let modified = metadata.modified()?;

        if let Some(cached) = WIKILINK_FILTER_CACHE.lock().get(&self.filter_path).cloned()
            && cached.modified == modified
        {
            return Ok(cached.blocked);
        }

        // T-2.4: RTM FR-11.1 / NFR-11.1.2 parses the dotfile only when mtime changes.
        let blocked = fs::read_to_string(&self.filter_path)?
            .lines()
            .filter_map(parse_wikilink_filter_line)
            .collect::<BTreeMap<_, _>>();

        WIKILINK_FILTER_CACHE.lock().insert(
            self.filter_path.clone(),
            CachedWikiLinkFilter {
                modified,
                blocked: blocked.clone(),
            },
        );

        Ok(blocked)
    }
}

fn parse_wikilink_filter_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    // T-2.6: RTM FR-11.1 fails closed for malformed filter entries by treating the whole line as the concept.
    let (concept, reason) = trimmed
        .split_once('|')
        .map_or((trimmed, ""), |(concept, reason)| (concept, reason));
    let normalized = normalize_concept(concept);
    if normalized.is_empty() {
        // T-2.6: RTM FR-11.1 skips empty normalized concepts instead of inserting an invalid deny rule.
        eprintln!("Warning: skipping empty concept in .wikilink-filter line: {trimmed}");
        return None;
    }

    Some((normalized, reason.trim().to_string()))
}

pub fn validate_uri(uri: &str, param_name: &str) -> Result<()> {
    if uri.is_empty() {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI cannot be null or empty."
        )));
    }
    if !uri.starts_with("memory://") {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI must start with 'memory://'. Got: '{uri}'"
        )));
    }
    let relative_part = &uri["memory://".len()..];
    if relative_part.split('/').any(|segment| segment == "..") {
        return Err(MaenifoldError::InvalidArgument(format!(
            "{param_name}: URI contains path traversal (..) which is not allowed: '{uri}'"
        )));
    }
    Ok(())
}

pub fn path_to_uri(path: &Path, base_path: &Path) -> Result<String> {
    let relative = path.strip_prefix(base_path).map_err(|_| {
        MaenifoldError::InvalidArgument(format!("Path is outside memory root: {}", path.display()))
    })?;
    let mut relative = relative.to_string_lossy().replace('\\', "/");
    if relative.ends_with(".md") {
        relative.truncate(relative.len() - 3);
    }
    Ok(format!("memory://{relative}"))
}

pub fn uri_to_path(uri: &str, base_path: &Path) -> Result<PathBuf> {
    validate_uri(uri, "uri")?;
    let relative = validate_relative_path(uri.trim_start_matches("memory://"))?;
    let mut path = base_path.join(&relative);
    if path.extension().is_none() {
        path.set_extension("md");
    }

    let base_full = fs::canonicalize(base_path).unwrap_or_else(|_| base_path.to_path_buf());
    let mut full_path = base_full.join(&relative);
    if full_path.extension().is_none() {
        full_path.set_extension("md");
    }
    if !full_path.starts_with(&base_full) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid URI attempts to escape memory directory: {}",
            uri.trim_start_matches("memory://")
        )));
    }

    Ok(path)
}

pub fn file_exists(uri: &str, config: &Config) -> Result<bool> {
    Ok(uri_to_path(uri, &config.memory_path)?.exists())
}

pub fn delete_file(uri: &str, config: &Config) -> Result<()> {
    let path = uri_to_path(uri, &config.memory_path)?;
    if !path.exists() {
        return Err(MaenifoldError::Message(format!(
            "Memory file not found: {uri}"
        )));
    }
    fs::remove_file(path)?;
    Ok(())
}

pub fn get_memory_files(config: &Config, folder_uri: Option<&str>) -> Result<Vec<String>> {
    let base = if let Some(folder_uri) = folder_uri {
        validate_uri(folder_uri, "folderUri")?;
        let relative = validate_relative_path(folder_uri.trim_start_matches("memory://"))?;
        config.memory_path.join(relative)
    } else {
        config.memory_path.clone()
    };

    let mut files = Vec::new();
    if !base.exists() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(&base)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if is_hidden_path(path, &config.memory_path) {
            continue;
        }
        files.push(path_to_uri(path, &config.memory_path)?);
    }

    files.sort();
    Ok(files)
}

fn is_hidden_path(path: &Path, base_path: &Path) -> bool {
    let relative = path.strip_prefix(base_path).unwrap_or(path);
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        text.starts_with('.') && text.len() > 1
    })
}

pub fn generate_checksum(content: &str) -> String {
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
    let checksum = generate_checksum(&full_text);
    let (frontmatter, content) = parse_frontmatter(&full_text)?;
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
            return Err(MaenifoldError::Message(
                "Checksum mismatch. File has been modified by another process.".to_string(),
            ));
        }
    }

    write_markdown(uri, frontmatter, content, config)
}

pub fn append_h2_section(uri: &str, heading: &str, content: &str, config: &Config) -> Result<()> {
    let (frontmatter, existing_content, _) = read_markdown(uri, config).unwrap_or_default();

    let mut new_content = existing_content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("\n## {heading}\n\n{content}\n"));

    write_markdown(uri, frontmatter.as_ref(), &new_content, config)
}

// T-0.3e: RTM-049 prerequisite for FR-4 RecentActivity section parsing.
pub fn extract_h2_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some(previous_heading) = current_heading.replace(heading.trim().to_string()) {
                sections.push((
                    previous_heading,
                    current_lines.join("\n").trim().to_string(),
                ));
                current_lines.clear();
            }
        } else if current_heading.is_some() {
            current_lines.push(line);
        }
    }

    if let Some(current_heading) = current_heading {
        sections.push((current_heading, current_lines.join("\n").trim().to_string()));
    }

    sections
}

pub fn slugify(text: &str) -> String {
    slugify_title(text)
}

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut visible = String::with_capacity(content.len());
    let mut code_block_depth = 0usize;

    for event in Parser::new_ext(content, Options::all()) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_block_depth = code_block_depth.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { .. },
                dest_url,
                ..
            }) => {
                if code_block_depth == 0 {
                    let link = normalize_concept(dest_url.as_ref());
                    if !link.is_empty() && seen.insert(link.clone()) {
                        links.push(link);
                    }
                }
            }
            Event::Code(_) => {}
            Event::Text(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                if code_block_depth == 0 {
                    visible.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if code_block_depth == 0 {
                    visible.push('\n');
                }
            }
            _ => {}
        }
    }

    for link in extract_wikilinks_regex_only(&visible) {
        if seen.insert(link.clone()) {
            links.push(link);
        }
    }

    links
}

// T-4.3: RTM FR-23.x
pub fn extract_wikilinks_with_sections(content: &str) -> Vec<(String, usize)> {
    let mut sections = Vec::new();
    let mut current_section = String::new();
    let mut section_index = 0usize;

    for line in content.lines() {
        if line.starts_with("## ") {
            sections.push((section_index, std::mem::take(&mut current_section)));
            section_index += 1;
        }

        current_section.push_str(line);
        current_section.push('\n');
    }
    sections.push((section_index, current_section));

    sections
        .into_iter()
        .flat_map(|(section_index, section_content)| {
            extract_wikilinks(&section_content)
                .into_iter()
                .map(move |concept| (concept, section_index))
        })
        .collect()
}

fn extract_wikilinks_regex_only(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut index = 0;

    while index + 3 < bytes.len() {
        if bytes[index] != b'[' || bytes[index + 1] != b'[' {
            index += 1;
            continue;
        }

        if index > 0 && bytes[index - 1] == b'[' {
            index += 1;
            continue;
        }

        let mut cursor = index + 2;
        let mut closing = None;
        let mut invalid = false;

        while cursor + 1 < bytes.len() {
            match (bytes[cursor], bytes[cursor + 1]) {
                (b']', b']') => {
                    closing = Some(cursor);
                    break;
                }
                (b'[', _) | (b']', _) => {
                    invalid = true;
                    break;
                }
                _ => cursor += 1,
            }
        }

        let Some(end) = closing else {
            index += 2;
            continue;
        };

        if invalid || (end + 2 < bytes.len() && bytes[end + 2] == b']') {
            index = end + 2;
            continue;
        }

        let link = normalize_concept(&content[index + 2..end]);
        if !link.is_empty() && seen.insert(link.clone()) {
            links.push(link);
        }
        index = end + 2;
    }

    links
}

// T-0.3e: RTM-048 prerequisite for FR-2 AnalyzeConceptCorruption and FR-6 RepairConcepts.
pub fn normalize_concept(concept: &str) -> String {
    let mut normalized = String::with_capacity(concept.len());
    let mut previous_was_hyphen = false;

    for ch in concept.trim().chars().flat_map(|c| c.to_lowercase()) {
        let mapped = match ch {
            ' ' | '_' | '/' => '-',
            other => other,
        };

        if mapped == '-' {
            if !previous_was_hyphen {
                normalized.push('-');
                previous_was_hyphen = true;
            }
        } else {
            normalized.push(mapped);
            previous_was_hyphen = false;
        }
    }

    normalized.trim_matches('-').to_string()
}

fn parse_frontmatter(full_text: &str) -> Result<(Option<Frontmatter>, String)> {
    if let Some((rest, index)) = full_text
        .strip_prefix("---\n")
        .and_then(|rest| rest.find("\n---\n").map(|index| (rest, index)))
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

fn system_time_to_iso(system_time: std::time::SystemTime) -> String {
    let date_time: DateTime<Utc> = system_time.into();
    format_iso(date_time)
}

pub fn resolve_folder_uri(folder_uri: &str, config: &Config) -> Result<PathBuf> {
    validate_uri(folder_uri, "folderUri")?;
    let relative = validate_relative_path(folder_uri.trim_start_matches("memory://"))?;
    let path = config.memory_path.join(&relative);
    let base_full_path =
        fs::canonicalize(&config.memory_path).unwrap_or(config.memory_path.clone());
    let full_path = base_full_path.join(relative);
    if !full_path.starts_with(&base_full_path) {
        return Err(MaenifoldError::InvalidArgument(format!(
            "Invalid folder URI attempts to escape memory directory: {folder_uri}"
        )));
    }
    Ok(path)
}

pub fn find_file_as_uri(identifier: &str, config: &Config) -> Option<String> {
    if identifier.trim().is_empty() {
        return None;
    }

    let normalized = identifier.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty() {
        return None;
    }

    let relative_path = normalized.replace('/', std::path::MAIN_SEPARATOR_STR);
    let direct_path = config.memory_path.join(&relative_path);
    if direct_path.is_file() {
        let uri_part = normalized.strip_suffix(".md").unwrap_or(&normalized);
        return Some(format!("memory://{uri_part}"));
    }

    if Path::new(&relative_path).extension().is_none() {
        let md_path = config.memory_path.join(format!("{relative_path}.md"));
        if md_path.is_file() {
            return Some(format!("memory://{normalized}"));
        }
    }

    let slug = slugify(identifier);
    get_memory_files(config, None)
        .ok()?
        .into_iter()
        .find(|uri| {
            uri.trim_start_matches("memory://")
                .split('/')
                .next_back()
                .map(|name| name.eq_ignore_ascii_case(&slug))
                .unwrap_or(false)
        })
}

pub fn markdown_path_from_relative(relative: &str, config: &Config) -> Result<PathBuf> {
    let relative = validate_relative_path(relative)?;
    let mut path = config.memory_path.join(relative);
    if path.extension().is_none() {
        path.set_extension("md");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::WikiLinkFilter;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    #[test]
    fn wikilink_filter_allows_content_when_filter_file_is_missing() {
        let tmp = TempDir::new().expect("tmp");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Allowed [[Agent]] content");

        assert!(matches!(result, Ok(Ok(()))));
    }

    #[test]
    fn wikilink_filter_blocks_normalized_concepts_with_reasons() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter"),
            "# comment\nagent | Too generic - hub concept\nsystem | Also blocked\n",
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path())
            .try_check("Testing [[Agent]] and [[system]] in one note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![
                ("agent".to_string(), "Too generic - hub concept".to_string()),
                ("system".to_string(), "Also blocked".to_string()),
            ])
        );
    }

    #[test]
    fn wikilink_filter_treats_malformed_lines_as_empty_reason_entries() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(tmp.path().join(".wikilink-filter"), "agent\n").expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![("agent".to_string(), String::new())])
        );
    }

    #[test]
    fn wikilink_filter_skips_empty_normalized_concepts() {
        let tmp = TempDir::new().expect("tmp");
        std::fs::write(
            tmp.path().join(".wikilink-filter"),
            "   | missing concept\nagent | blocked\n",
        )
        .expect("write filter");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert_eq!(
            result.expect("filter read"),
            Err(vec![("agent".to_string(), "blocked".to_string())])
        );
    }

    #[cfg(unix)]
    #[test]
    fn wikilink_filter_ignores_symlinked_filter_files() {
        let tmp = TempDir::new().expect("tmp");
        let external = tmp.path().join("external-filter");
        std::fs::write(&external, "agent | blocked\n").expect("write external filter");
        unix_fs::symlink(&external, tmp.path().join(".wikilink-filter")).expect("create symlink");

        let result = WikiLinkFilter::new(tmp.path()).try_check("Testing [[agent]] in one note");

        assert!(matches!(result, Ok(Ok(()))));
    }
}
