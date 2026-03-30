use serde::Serialize;

use crate::{TaskState, parser};

#[derive(Debug, Default)]
pub(super) struct TaskDetails {
    pub id: String,
    pub title: String,
    pub attempts: u32,
    pub requirement: Option<String>,
    pub sections: Vec<SectionOutput>,
}

#[derive(Debug, Serialize)]
pub(super) struct SectionOutput {
    pub heading: String,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    pub body: String,
}

pub(super) fn parse_details(
    file: &str,
    content: Option<&str>,
    state: Option<&TaskState>,
) -> TaskDetails {
    let content = content.unwrap_or_default();
    let heading = content
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim);
    TaskDetails {
        id: extract_id(file, heading),
        title: extract_title(file, heading),
        attempts: state.map_or(0, |item| item.attempts),
        requirement: extract_requirement(content),
        sections: extract_sections(content),
    }
}

fn extract_id(file: &str, heading: Option<&str>) -> String {
    heading
        .map(clean_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| {
            file.trim_end_matches(".md")
                .split('-')
                .take(2)
                .collect::<Vec<_>>()
                .join("-")
        })
}

fn extract_title(file: &str, heading: Option<&str>) -> String {
    heading
        .map(clean_title)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| file.trim_end_matches(".md").to_string())
}

fn clean_id(raw: &str) -> String {
    raw.split_once(" — ")
        .or_else(|| raw.split_once(" - "))
        .map(|(id, _)| id.trim().to_string())
        .unwrap_or_else(|| raw.trim().to_string())
}

fn clean_title(raw: &str) -> String {
    raw.split_once(" — ")
        .or_else(|| raw.split_once(" - "))
        .map(|(_, title)| title.trim().to_string())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| raw.trim().to_string())
}

fn extract_requirement(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        line.strip_prefix("**Requirement**:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn extract_sections(content: &str) -> Vec<SectionOutput> {
    let verdicts = parser::extract_sections(content).unwrap_or_default();
    let mut sections = Vec::new();
    let mut current = None::<(String, Vec<String>)>;

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some((heading, body)) = current.take() {
                sections.push(build_section(&heading, &body, &verdicts, sections.len()));
            }
            current = Some((heading.trim().to_string(), Vec::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push(line.to_string());
        }
    }

    if let Some((heading, body)) = current.take() {
        sections.push(build_section(&heading, &body, &verdicts, sections.len()));
    }

    sections
}

fn build_section(
    heading: &str,
    body: &[String],
    verdicts: &[parser::ParsedSection],
    index: usize,
) -> SectionOutput {
    SectionOutput {
        heading: heading.to_string(),
        stage: parser::normalize_stage_name(heading),
        verdict: verdicts
            .get(index)
            .filter(|section| section.heading == heading)
            .and_then(|section| section.verdict.map(|verdict| verdict.as_str().to_string())),
        body: trim_body(body),
    }
}

fn trim_body(lines: &[String]) -> String {
    lines
        .join("\n")
        .trim()
        .replace("\n\n\n", "\n\n")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::parse_details;

    #[test]
    fn parses_task_title_and_sections() {
        let details = parse_details(
            "T-68.9-gate-kanban-board.md",
            Some(
                "# T-68.9 — Gate Kanban Board\n\n**Requirement**: FR-68\n\n## Summary\n\nBoard.\n\n## Red Team Audit\n\n**Verdict**: PASS\n\nApproved.\n",
            ),
            None,
        );

        assert_eq!(details.id, "T-68.9");
        assert_eq!(details.title, "Gate Kanban Board");
        assert_eq!(details.requirement.as_deref(), Some("FR-68"));
        assert_eq!(details.sections.len(), 2);
        assert_eq!(details.sections[1].verdict.as_deref(), Some("PASS"));
    }
}
