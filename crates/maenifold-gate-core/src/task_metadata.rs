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
    pub blocked: bool,
    pub blocked_reason: Option<String>,
    pub superseded_by: Option<Vec<String>>,
    pub session_id: Option<String>,
    pub depends_on: Vec<String>,
    pub priority: Option<String>,
    pub sprint: Option<u32>,
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
    #[serde(default)]
    blocked: bool,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    blocked_reason: Option<String>,
    #[serde(default)]
    superseded_by: Option<StringOrVec>,
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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    priority: Option<String>,
    #[serde(default)]
    sprint: Option<u32>,
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
            let priority = validate_priority(frontmatter.priority)?;
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
                blocked: frontmatter.blocked,
                blocked_reason: frontmatter.blocked_reason,
                superseded_by: frontmatter.superseded_by.map(|values| values.0),
                session_id: frontmatter.session_id,
                depends_on,
                priority,
                sprint: frontmatter.sprint,
            });
        }

        Ok(TaskMetadata {
            id: None,
            title: parse_title(lines)?.to_string(),
            pipeline: parse_labeled_value(lines, "Pipeline")?
                .ok_or(GateError::MissingMetadataField("Pipeline"))?,
            state: None,
            blocked: false,
            blocked_reason: None,
            superseded_by: None,
            session_id: None,
            depends_on: Vec::new(),
            priority: None,
            sprint: None,
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

fn validate_priority(priority: Option<String>) -> Result<Option<String>, GateError> {
    match priority.as_deref() {
        None | Some("P0") | Some("P1") | Some("P2") | Some("P3") | Some("P4") => Ok(priority),
        Some(p) => Err(GateError::TaskMetadataParse(format!(
            "invalid priority '{p}': must be one of P0, P1, P2, P3, P4"
        ))),
    }
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
#[path = "task_metadata_tests.rs"]
mod tests;
