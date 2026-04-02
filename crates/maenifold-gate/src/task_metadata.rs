use regex::Regex;

use crate::error::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMetadata {
    pub title: String,
    pub requirement: String,
    pub test_cases: Vec<String>,
    pub component: String,
    pub pipeline: String,
    pub pipeline_digest: String,
}

#[derive(Debug, Clone)]
pub struct TaskMetadataParser;

impl TaskMetadataParser {
    pub fn parse(lines: &[&str]) -> Result<TaskMetadata, GateError> {
        let title = parse_title(lines)?.to_string();
        let requirement =
            parse_labeled_value(lines, "Requirement")?.ok_or(GateError::MissingMetadataField("Requirement"))?;
        let test_cases_raw =
            parse_labeled_value(lines, "Test Cases")?.ok_or(GateError::MissingMetadataField("Test Cases"))?;
        let component =
            parse_labeled_value(lines, "Component")?.ok_or(GateError::MissingMetadataField("Component"))?;
        let pipeline =
            parse_labeled_value(lines, "Pipeline")?.ok_or(GateError::MissingMetadataField("Pipeline"))?;
        let pipeline_digest = parse_labeled_value(lines, "Pipeline Digest")?
            .ok_or(GateError::MissingMetadataField("Pipeline Digest"))?;

        if !is_valid_sha256(&pipeline_digest) {
            return Err(GateError::InvalidPipelineDigest(pipeline_digest));
        }

        let test_cases = test_cases_raw
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        Ok(TaskMetadata {
            title,
            requirement,
            test_cases,
            component,
            pipeline,
            pipeline_digest,
        })
    }
}

fn parse_title<'a>(lines: &'a [&str]) -> Result<&'a str, GateError> {
    lines
        .iter()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .ok_or_else(|| GateError::TaskMetadataParse("missing markdown title".to_string()))
}

fn parse_labeled_value(lines: &[&str], label: &str) -> Result<Option<String>, GateError> {
    let pattern = Regex::new(r"^\*\*(?P<label>[^*]+)\*\*:\s*(?P<value>.+?)\s*$")
        .map_err(|error| GateError::TaskMetadataParse(format!("invalid label regex: {error}")))?;

    Ok(lines.iter().find_map(|line| {
        let captures = pattern.captures(line)?;
        let found_label = captures.name("label")?.as_str();
        if found_label == label {
            captures.name("value").map(|value| value.as_str().to_string())
        } else {
            None
        }
    }))
}

fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
