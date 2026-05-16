use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;

use crate::error::GateError;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct FrontmatterTask {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub(super) id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) pipeline: Option<String>,
    pub(super) state: Option<String>,
    #[serde(default)]
    pub(super) blocked: bool,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub(super) blocked_reason: Option<String>,
    #[serde(default)]
    pub(super) superseded_by: Option<StringOrVec>,
    #[serde(
        default,
        rename = "sessionId",
        deserialize_with = "deserialize_optional_string"
    )]
    pub(super) session_id: Option<String>,
    #[serde(default)]
    pub(super) depends_on: Option<StringOrVec>,
    #[serde(default)]
    pub(super) depends: Option<StringOrVec>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub(super) priority: Option<String>,
    #[serde(default)]
    pub(super) sprint: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StringOrVec(pub(super) Vec<String>);

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

pub(super) fn deserialize_optional_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_yml::Value>::deserialize(deserializer)?;
    Ok(match value {
        Some(serde_yml::Value::String(value)) => Some(value),
        _ => None,
    })
}

pub(super) fn parse_frontmatter(lines: &[&str]) -> Result<Option<FrontmatterTask>, GateError> {
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

pub(super) fn validate_priority(priority: Option<String>) -> Result<Option<String>, GateError> {
    match priority.as_deref() {
        None | Some("P0") | Some("P1") | Some("P2") | Some("P3") | Some("P4") => Ok(priority),
        Some(p) => Err(GateError::TaskMetadataParse(format!(
            "invalid priority '{p}': must be one of P0, P1, P2, P3, P4"
        ))),
    }
}

pub(super) fn normalize_dependencies(
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
