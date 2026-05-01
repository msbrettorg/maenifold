use regex::Regex;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::error::GateError;

static LABELED_VALUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*\*(?P<label>[^*]+)\*\*:\s*(?P<value>.+?)\s*$")
        .expect("task metadata label regex")
});
static TASK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._-]+$").expect("task id regex"));
static CANONICAL_TASK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^T-\d+\.\d+(?:\.[1-9]\d*)?$").expect("canonical task id regex"));

const EXPECTED_TASK_ID_FORMAT: &str = "T-{sprint}.{N}.md or T-{sprint}.{N}.{M}.md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMetadata {
    pub id: Option<String>,
    pub title: String,
    pub pipeline: String,
    pub state: Option<String>,
    pub session_id: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LenientTaskMetadata {
    pub id: Option<String>,
    pub title: String,
    pub pipeline: String,
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
    #[serde(default)]
    depends_on: Option<StringOrVec>,
    #[serde(default)]
    depends: Option<StringOrVec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StringOrVec(Vec<String>);

impl<'de> Deserialize<'de> for StringOrVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match serde_yml::Value::deserialize(deserializer)? {
            serde_yml::Value::String(value) => Ok(StringOrVec(vec![value])),
            serde_yml::Value::Sequence(values) => values
                .into_iter()
                .map(|value| match value {
                    serde_yml::Value::String(value) => Ok(value),
                    _ => Err(D::Error::custom("depends_on entries must be strings")),
                })
                .collect::<Result<Vec<_>, _>>()
                .map(StringOrVec),
            _ => Err(D::Error::custom("depends_on must be a string or list")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskMetadataParser;

impl TaskMetadataParser {
    pub fn parse(lines: &[&str]) -> Result<TaskMetadata, GateError> {
        Self::parse_with_optional_path(lines, None)
    }

    pub fn parse_with_path(lines: &[&str], path: &Path) -> Result<TaskMetadata, GateError> {
        Self::parse_with_optional_path(lines, Some(path))
    }

    pub fn parse_lenient(lines: &[&str]) -> Result<LenientTaskMetadata, GateError> {
        if let Some(frontmatter) = parse_frontmatter(lines)? {
            return Ok(LenientTaskMetadata {
                id: validate_lenient_task_id(frontmatter.id)?,
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
            });
        }

        Ok(LenientTaskMetadata {
            id: None,
            title: parse_title(lines)?.to_string(),
            pipeline: parse_labeled_value(lines, "Pipeline")?
                .ok_or(GateError::MissingMetadataField("Pipeline"))?,
        })
    }

    fn parse_with_optional_path(
        lines: &[&str],
        path: Option<&Path>,
    ) -> Result<TaskMetadata, GateError> {
        if let Some(frontmatter) = parse_frontmatter(lines)? {
            let id = validate_task_id(frontmatter.id, path)?;
            let depends_on =
                normalize_dependencies(frontmatter.depends_on, frontmatter.depends, id.as_deref())?;
            return Ok(TaskMetadata {
                id,
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
                depends_on,
            });
        }

        Ok(TaskMetadata {
            id: None,
            title: parse_title(lines)?.to_string(),
            pipeline: parse_labeled_value(lines, "Pipeline")?
                .ok_or(GateError::MissingMetadataField("Pipeline"))?,
            state: None,
            session_id: None,
            depends_on: Vec::new(),
        })
    }

    pub fn parse_task_id(text: &str) -> Result<Option<String>, GateError> {
        Self::parse_task_id_with_optional_path(text, None)
    }

    pub fn parse_task_id_with_path(text: &str, path: &Path) -> Result<Option<String>, GateError> {
        Self::parse_task_id_with_optional_path(text, Some(path))
    }

    fn parse_task_id_with_optional_path(
        text: &str,
        path: Option<&Path>,
    ) -> Result<Option<String>, GateError> {
        let lines: Vec<_> = text.lines().collect();
        let frontmatter = parse_frontmatter(&lines)?;
        validate_task_id(frontmatter.and_then(|frontmatter| frontmatter.id), path)
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
    let value = Option::<serde_yml::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_yml::Value::String(value)) => Some(value),
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
            let frontmatter = serde_yml::from_str::<FrontmatterTask>(&yaml).map_err(|error| {
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

fn validate_task_id(
    task_id: Option<String>,
    path: Option<&Path>,
) -> Result<Option<String>, GateError> {
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
            if !CANONICAL_TASK_ID_RE.is_match(&task_id) {
                if let Some(path) = path {
                    return Err(GateError::MalformedTaskId {
                        path: path.to_path_buf(),
                        expected_format: EXPECTED_TASK_ID_FORMAT,
                        actual_id: task_id,
                    });
                }
                return Err(GateError::TaskMetadataParse(format!(
                    "malformed task id: expected format {EXPECTED_TASK_ID_FORMAT}, actual id '{task_id}'"
                )));
            }
            Ok(task_id)
        })
        .transpose()
}

fn validate_lenient_task_id(task_id: Option<String>) -> Result<Option<String>, GateError> {
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

fn normalize_dependencies(
    depends_on: Option<StringOrVec>,
    depends: Option<StringOrVec>,
    task_id: Option<&str>,
) -> Result<Vec<String>, GateError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in depends_on
        .into_iter()
        .chain(depends)
        .flat_map(|StringOrVec(values)| values)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        if task_id == Some(value.as_str()) {
            return Err(GateError::TaskMetadataParse(
                "task depends on itself".to_string(),
            ));
        }
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{TaskMetadataParser, is_linked_session_id};

    fn parse_yaml(yaml: &str) -> crate::task_metadata::TaskMetadata {
        let lines: Vec<_> = yaml.lines().collect();
        TaskMetadataParser::parse(&lines).expect("parse metadata")
    }

    #[test]
    fn parses_optional_session_id_from_frontmatter() {
        let metadata = parse_yaml(
            "---\nid: T-108.2\ntitle: Linked task\npipeline: sdd\nsessionId: session-1775916753112-57359\n---",
        );
        assert_eq!(
            metadata.session_id.as_deref(),
            Some("session-1775916753112-57359")
        );
        assert!(is_linked_session_id(metadata.session_id.as_deref()));
    }

    #[test]
    fn ignores_non_string_session_id_values() {
        let metadata =
            parse_yaml("---\ntitle: Lenient task\npipeline: sdd\nsessionId:\n  - malformed\n---");
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

        for invalid in ["../../escape", "foo/bar", "foo\\bar"] {
            let yaml = format!("---\nid: {invalid}\ntitle: bad\npipeline: sdd\n---\n# bad\n");
            assert!(TaskMetadataParser::parse_task_id(&yaml).is_err());
        }
    }

    #[test]
    fn parses_depends_on_string_list_alias_and_dedups() {
        let cases = [
            ("depends_on: T-237.11", vec!["T-237.11"]),
            (
                "depends_on: [T-237.11, T-238.24]",
                vec!["T-237.11", "T-238.24"],
            ),
            ("depends: T-237.11", vec!["T-237.11"]),
            (
                "depends_on: [T-1, T-2]\ndepends: [T-2, T-3]",
                vec!["T-1", "T-2", "T-3"],
            ),
            ("depends_on: ['  T-1  ', '', '   ', T-1]", vec!["T-1"]),
        ];

        for (depends_yaml, expected) in cases {
            let yaml = format!("---\nid: T-9.1\ntitle: deps\npipeline: sdd\n{depends_yaml}\n---");
            let metadata = parse_yaml(&yaml);
            assert_eq!(metadata.depends_on, expected);
        }
    }

    #[test]
    fn rejects_self_dependency() {
        let yaml =
            "---\nid: T-1.1\ntitle: self dep\npipeline: sdd\ndepends_on: [T-0.1, T-1.1]\n---";
        let lines: Vec<_> = yaml.lines().collect();
        let error = TaskMetadataParser::parse(&lines).expect_err("self dependency should fail");
        assert!(error.to_string().contains("task depends on itself"));
    }
}
