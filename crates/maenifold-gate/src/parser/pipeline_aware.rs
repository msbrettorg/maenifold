use std::collections::BTreeSet;
use std::sync::OnceLock;

use anyhow::{Result, bail};

use crate::pipeline::{PipelineDef, load_embedded_pipeline};

pub(crate) fn validate_section_stages(content: &str, pipeline: &PipelineDef) -> Result<()> {
    let valid_stages = pipeline_stage_names(pipeline);
    for section in scan_sections(content) {
        let Some(candidate) = infer_stage_name(&section.heading) else {
            continue;
        };
        if valid_stages.contains(candidate.as_str()) {
            continue;
        }
        bail!(
            "section heading '{}' at line {} maps to unknown stage '{}' for pipeline '{}'; valid stages: {}",
            section.heading,
            section.line,
            candidate,
            pipeline.name,
            valid_stage_list(pipeline)
        );
    }
    Ok(())
}

fn pipeline_stage_names(pipeline: &PipelineDef) -> BTreeSet<&str> {
    pipeline
        .stages
        .iter()
        .map(|stage| stage.name.as_str())
        .collect()
}

fn infer_stage_name(heading: &str) -> Option<String> {
    let heading = heading.trim();
    if heading.is_empty() {
        return None;
    }

    let lower = heading.to_lowercase();
    if known_stage_names().contains(lower.as_str()) {
        return Some(lower);
    }

    let normalized = normalize_stage_name(heading);
    (normalized != lower && known_stage_names().contains(normalized.as_str())).then_some(normalized)
}

fn normalize_stage_name(heading: &str) -> String {
    let lower = heading.to_lowercase();
    lower
        .split_whitespace()
        .next()
        .unwrap_or(&lower)
        .to_string()
}

fn known_stage_names() -> &'static BTreeSet<String> {
    static KNOWN: OnceLock<BTreeSet<String>> = OnceLock::new();
    KNOWN.get_or_init(|| {
        ["sdd", "tdd", "gsd", "lightweight"]
            .into_iter()
            .filter_map(|name| load_embedded_pipeline(name).ok())
            .flat_map(|pipeline| pipeline.stages.into_iter().map(|stage| stage.name))
            .collect()
    })
}

fn valid_stage_list(pipeline: &PipelineDef) -> String {
    pipeline
        .stages
        .iter()
        .map(|stage| stage.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SectionHeading {
    heading: String,
    line: usize,
}

fn scan_sections(content: &str) -> Vec<SectionHeading> {
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            line.strip_prefix("## ").map(|heading| SectionHeading {
                heading: heading.trim().to_string(),
                line: index + 1,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::pipeline::load_embedded_pipeline;

    use super::validate_section_stages;

    #[test]
    fn pipeline_aware_parser_accepts_current_pipeline_stage_headings() {
        let pipeline = load_embedded_pipeline("gsd").unwrap();
        validate_section_stages("## Research Notes\n\nScoped the problem.\n", &pipeline).unwrap();
        validate_section_stages("## Verify\n\n**Verdict**: FAIL\n", &pipeline).unwrap();
    }

    #[test]
    fn pipeline_aware_parser_rejects_unknown_section_stage() {
        let pipeline = load_embedded_pipeline("gsd").unwrap();
        let err = validate_section_stages("## SWE Notes\n\nImplemented it.\n", &pipeline)
            .unwrap_err()
            .to_string();
        assert!(err.contains("section heading 'SWE Notes'"));
        assert!(err.contains("unknown stage 'swe'"));
        assert!(err.contains("pipeline 'gsd'"));
        assert!(err.contains("research, plan, execute, verify"));
    }

    #[test]
    fn pipeline_aware_parser_ignores_non_stage_sections() {
        let pipeline = load_embedded_pipeline("gsd").unwrap();
        validate_section_stages(
            "## Acceptance Criteria\n\n- keep this section\n\n## Summary\n\nContext.\n",
            &pipeline,
        )
        .unwrap();
    }
}
