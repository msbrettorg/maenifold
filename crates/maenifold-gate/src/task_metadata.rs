use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::error::GateError;

static LABELED_VALUE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\*\*(?P<label>[^*]+)\*\*:\s*(?P<value>.+?)\s*$")
        .expect("task metadata label regex")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMetadata {
    pub id: Option<String>,
    pub title: String,
    pub pipeline: String,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct FrontmatterTask {
    id: Option<String>,
    title: Option<String>,
    pipeline: Option<String>,
    state: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskMetadataParser;

impl TaskMetadataParser {
    pub fn parse(lines: &[&str]) -> Result<TaskMetadata, GateError> {
        if let Some(frontmatter) = parse_frontmatter(lines)? {
            return Ok(TaskMetadata {
                id: frontmatter.id,
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
                state: frontmatter.state.or(frontmatter.status),
            });
        }

        Ok(TaskMetadata {
            id: None,
            title: parse_title(lines)?.to_string(),
            pipeline: parse_labeled_value(lines, "Pipeline")?
                .ok_or(GateError::MissingMetadataField("Pipeline"))?,
            state: None,
        })
    }

    pub fn parse_task_id(text: &str) -> Option<String> {
        let lines: Vec<_> = text.lines().collect();
        parse_frontmatter(&lines)
            .ok()
            .flatten()
            .and_then(|frontmatter| frontmatter.id)
    }
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
