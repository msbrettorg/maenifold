use serde::Serialize;

use crate::TaskState;
use crate::frontmatter::{Frontmatter, parse_frontmatter};

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

/// Parse task details from file content, preferring frontmatter `id`, `title`,
/// and `requirement` over heading / body-text equivalents.
pub(super) fn parse_details(
    file: &str,
    content: Option<&str>,
    state: Option<&TaskState>,
) -> TaskDetails {
    let content = content.unwrap_or_default();
    let (fm, body) = parse_frontmatter(content);
    let heading = body
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map(str::trim);
    let fm_ref = fm.as_ref();
    TaskDetails {
        id: fm_str(fm_ref, "id").unwrap_or_else(|| extract_id(file, heading)),
        title: fm_str(fm_ref, "title").unwrap_or_else(|| extract_title(file, heading)),
        attempts: state.map_or(0, |item| item.attempts),
        requirement: fm_str(fm_ref, "requirement").or_else(|| extract_requirement(&body)),
        sections: extract_sections(&body),
    }
}

/// Extract a trimmed, non-empty string value from frontmatter by key.
fn fm_str(fm: Option<&Frontmatter>, key: &str) -> Option<String> {
    fm?.get(key)?
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
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

fn extract_requirement(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        line.strip_prefix("**Requirement**:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn extract_sections(content: &str) -> Vec<SectionOutput> {
    let mut sections = Vec::new();
    let mut current = None::<(String, Vec<String>)>;

    for line in content.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some((heading, body)) = current.take() {
                sections.push(build_section(&heading, &body));
            }
            current = Some((heading.trim().to_string(), Vec::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push(line.to_string());
        }
    }

    if let Some((heading, body)) = current.take() {
        sections.push(build_section(&heading, &body));
    }

    sections
}

fn extract_verdict(body: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let trimmed = line.trim();
        let after = trimmed.strip_prefix("**Verdict**:")?;
        let word = after.trim().split_whitespace().next()?;
        match word {
            "PASS" | "REJECT" | "FAIL" => Some(word.to_string()),
            _ => None,
        }
    })
}

fn normalize_stage_name(heading: &str) -> String {
    heading.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_section(heading: &str, body: &[String]) -> SectionOutput {
    let body_text = trim_body(body);
    let verdict = extract_verdict(&body_text);
    SectionOutput {
        heading: heading.to_string(),
        stage: normalize_stage_name(heading),
        verdict,
        body: body_text,
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
                "# T-68.9 — Gate Kanban Board\n\n\
                 **Requirement**: FR-68\n\n\
                 ## Summary\n\nBoard.\n\n\
                 ## Red Team Audit\n\n**Verdict**: PASS\n\nApproved.\n",
            ),
            None,
        );
        assert_eq!(details.id, "T-68.9");
        assert_eq!(details.title, "Gate Kanban Board");
        assert_eq!(details.requirement.as_deref(), Some("FR-68"));
        assert_eq!(details.sections.len(), 2);
        assert_eq!(details.sections[1].verdict.as_deref(), Some("PASS"));
    }

    #[test]
    fn frontmatter_overrides_stale_heading() {
        let content = "\
            ---\nid: T-99.1\ntitle: Correct Title\nrequirement: FR-99\n---\n\n\
            # T-99.0 — Stale Heading\n\n\
            **Requirement**: FR-OLD\n\n\
            ## Summary\n\nBody.\n";
        let details = parse_details("T-99.1-task.md", Some(content), None);
        assert_eq!(details.id, "T-99.1");
        assert_eq!(details.title, "Correct Title");
        assert_eq!(details.requirement.as_deref(), Some("FR-99"));
    }

    #[test]
    fn falls_back_to_heading_without_frontmatter() {
        let content = "# T-50 — Heading Title\n\n**Requirement**: FR-50\n\n## Summary\n\nBody.\n";
        let details = parse_details("T-50-heading.md", Some(content), None);
        assert_eq!(details.id, "T-50");
        assert_eq!(details.title, "Heading Title");
        assert_eq!(details.requirement.as_deref(), Some("FR-50"));
    }

    #[test]
    fn partial_frontmatter_falls_back_for_missing_fields() {
        let content = "---\nid: T-77\n---\n\n# T-77 — Fallback Title\n\n## Summary\n\nBody.\n";
        let details = parse_details("T-77-task.md", Some(content), None);
        assert_eq!(details.id, "T-77");
        assert_eq!(details.title, "Fallback Title");
        assert!(details.requirement.is_none());
    }

    #[test]
    fn no_content_uses_filename() {
        let details = parse_details("T-42-test.md", None, None);
        assert_eq!(details.id, "T-42");
        assert_eq!(details.title, "T-42-test");
        assert!(details.requirement.is_none());
        assert!(details.sections.is_empty());
    }
}
