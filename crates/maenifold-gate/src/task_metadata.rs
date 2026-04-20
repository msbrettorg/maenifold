use regex::Regex;
use serde::{Deserialize, Deserializer};
use std::sync::LazyLock;

use crate::error::GateError;

static LABELED_VALUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*\*(?P<label>[^*]+)\*\*:\s*(?P<value>.+?)\s*$")
        .expect("task metadata label regex")
});
static TASK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._-]+$").expect("task id regex"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMetadata {
    pub id: Option<String>,
    pub title: String,
    pub pipeline: String,
    pub state: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct FrontmatterTask {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    id: Option<String>,
    title: Option<String>,
    pipeline: Option<String>,
    state: Option<String>,
    #[serde(
        default,
        rename = "sessionId",
        deserialize_with = "deserialize_optional_string"
    )]
    session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskMetadataParser;

impl TaskMetadataParser {
    pub fn parse(lines: &[&str]) -> Result<TaskMetadata, GateError> {
        if let Some(frontmatter) = parse_frontmatter(lines)? {
            return Ok(TaskMetadata {
                id: validate_task_id(frontmatter.id)?,
                title: frontmatter
                    .title
                    .or_else(|| parse_title(lines).ok().map(str::to_string))
                    .ok_or_else(|| {
                        GateError::TaskMetadataParse(
                            "frontmatter task is missing both title and markdown heading"
                                .to_string(),
                        )
                    })?,
                pipeline: frontmatter
                    .pipeline
                    .ok_or(GateError::MissingMetadataField("pipeline"))?,
                state: frontmatter.state,
                session_id: frontmatter.session_id,
            });
        }

        Ok(TaskMetadata {
            id: None,
            title: parse_title(lines)?.to_string(),
            pipeline: parse_labeled_value(lines, "Pipeline")?
                .ok_or(GateError::MissingMetadataField("Pipeline"))?,
            state: None,
            session_id: None,
        })
    }

    pub fn parse_task_id(text: &str) -> Result<Option<String>, GateError> {
        let lines: Vec<_> = text.lines().collect();
        let frontmatter = parse_frontmatter(&lines)?;
        validate_task_id(frontmatter.and_then(|frontmatter| frontmatter.id))
    }
}

pub fn is_linked_session_id(session_id: Option<&str>) -> bool {
    matches!(
        session_id,
        Some(value) if value.starts_with("session-") || value.starts_with("workflow-")
    )
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_yaml::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_yaml::Value::String(value)) => Some(value),
        _ => None,
    })
}

fn parse_frontmatter(lines: &[&str]) -> Result<Option<FrontmatterTask>, GateError> {
    if lines.first().copied() != Some("---") {
        return Ok(None);
    }

    let mut yaml_lines = Vec::new();
    for line in lines.iter().skip(1) {
        if *line == "---" {
            let yaml = yaml_lines.join("\n");
            let frontmatter = serde_yaml::from_str::<FrontmatterTask>(&yaml).map_err(|error| {
                GateError::TaskMetadataParse(format!("invalid task frontmatter: {error}"))
            })?;
            return Ok(Some(frontmatter));
        }
        yaml_lines.push((*line).to_string());
    }

    Err(GateError::TaskMetadataParse(
        "frontmatter task is missing closing --- delimiter".to_string(),
    ))
}

fn parse_title<'a>(lines: &'a [&str]) -> Result<&'a str, GateError> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .ok_or_else(|| GateError::TaskMetadataParse("missing markdown title".to_string()))
}

fn parse_labeled_value(lines: &[&str], label: &str) -> Result<Option<String>, GateError> {
    Ok(lines.iter().find_map(|line| {
        let captures = LABELED_VALUE_RE.captures(line)?;
        let found_label = captures.name("label")?.as_str();
        if found_label == label {
            captures
                .name("value")
                .map(|value| value.as_str().to_string())
        } else {
            None
        }
    }))
}

fn validate_task_id(task_id: Option<String>) -> Result<Option<String>, GateError> {
    task_id
        .map(|task_id| {
            if task_id.contains("..")
                || task_id.contains('/')
                || task_id.contains('\\')
                || !TASK_ID_RE.is_match(&task_id)
            {
                return Err(GateError::TaskMetadataParse(format!(
                    "invalid task id '{task_id}': must match ^[A-Za-z0-9._-]+$ and must not contain path separators or '..'"
                )));
            }
            Ok(task_id)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{TaskMetadataParser, is_linked_session_id};

    #[test]
    fn parses_optional_session_id_from_frontmatter() {
        let lines = vec![
            "---",
            "id: T-108.2",
            "title: Linked task",
            "pipeline: sdd",
            "sessionId: session-1775916753112-57359",
            "---",
        ];

        let metadata = TaskMetadataParser::parse(&lines).expect("parse metadata");
        assert_eq!(
            metadata.session_id.as_deref(),
            Some("session-1775916753112-57359")
        );
        assert!(is_linked_session_id(metadata.session_id.as_deref()));
    }

    #[test]
    fn ignores_non_string_session_id_values() {
        let lines = vec![
            "---",
            "title: Lenient task",
            "pipeline: sdd",
            "sessionId:",
            "  - malformed",
            "---",
        ];

        let metadata = TaskMetadataParser::parse(&lines).expect("parse metadata");
        assert_eq!(metadata.session_id, None);
        assert!(!is_linked_session_id(metadata.session_id.as_deref()));
    }

    #[test]
    fn parse_task_id_accepts_valid_ids_and_rejects_path_traversal() {
        let valid = "---\nid: T-138.4\ntitle: Valid task\npipeline: sdd\n---\n# Valid task\n";
        assert_eq!(
            TaskMetadataParser::parse_task_id(valid).expect("valid id"),
            Some("T-138.4".to_string())
        );

        let traversal =
            "---\nid: ../../escape\ntitle: Escape task\npipeline: sdd\n---\n# Escape task\n";
        assert!(TaskMetadataParser::parse_task_id(traversal).is_err());

        let slash = "---\nid: foo/bar\ntitle: Slash task\npipeline: sdd\n---\n# Slash task\n";
        assert!(TaskMetadataParser::parse_task_id(slash).is_err());

        let backslash =
            "---\nid: foo\\bar\ntitle: Backslash task\npipeline: sdd\n---\n# Backslash task\n";
        assert!(TaskMetadataParser::parse_task_id(backslash).is_err());
    }
}
