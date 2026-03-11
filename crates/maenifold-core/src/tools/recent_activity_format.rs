use super::recent_activity::RecentActivityRecord;
use crate::markdown::extract_h2_sections;
use chrono::{DateTime, Utc};

#[derive(Clone, Copy)]
enum RecentActivityType {
    Sequential,
    Workflow,
    Thinking,
    Memory,
}

impl RecentActivityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Workflow => "workflow",
            Self::Thinking => "thinking",
            Self::Memory => "memory",
        }
    }
}

fn classify_recent_activity_type(file_path: &str) -> RecentActivityType {
    // T-2.1: RTM FR-4 / RTM-029 path classification for RecentActivity.
    if file_path.contains("/sequential/") {
        RecentActivityType::Sequential
    } else if file_path.contains("/workflow/") {
        RecentActivityType::Workflow
    } else if file_path.starts_with("memory://thinking/") {
        RecentActivityType::Thinking
    } else {
        RecentActivityType::Memory
    }
}

// T-2.1: RTM FR-4 / RTM-029 exposes path classification parity for RecentActivity pipeline tests.
pub fn recent_activity_determine_file_type(file_path: &str) -> &'static str {
    classify_recent_activity_type(file_path).as_str()
}

// T-2.3: RTM FR-4 exposes session-id validation used by pure date-detection regression tests.
pub fn is_valid_session_id_format(session_id: &str) -> bool {
    session_id
        .split('-')
        .nth(1)
        .filter(|segment| !segment.is_empty())
        .and_then(|segment| segment.parse::<i64>().ok())
        .is_some()
}

// T-2.1: RTM FR-4 / RTM-029 exposes session-id extraction parity for RecentActivity pipeline tests.
pub fn recent_activity_extract_session_id(file_path: &str) -> String {
    extract_recent_activity_session_id(file_path)
}

// T-2.1: RTM FR-4 / RTM-025 exposes formatter behavior for RecentActivity pipeline tests.
pub fn format_recent_activity_report(
    rows: Vec<RecentActivityRecord>,
    include_content: bool,
) -> String {
    let entries = rows
        .iter()
        .map(|row| {
            let file_type = classify_recent_activity_type(&row.file_path);
            let sections = extract_h2_sections(&row.content);
            format_recent_activity_entry(row, file_type, sections, include_content)
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "No recent activity found".to_string()
    } else {
        format!("# Recent Activity\n\n{}", entries.join("\n\n"))
    }
}

fn format_recent_activity_entry(
    row: &RecentActivityRecord,
    file_type: RecentActivityType,
    sections: Vec<(String, String)>,
    include_content: bool,
) -> String {
    let session_id = extract_recent_activity_session_id(&row.file_path);
    let modified = format_recent_activity_timestamp(&row.last_indexed);
    let (count_label, count_value) = count_recent_activity_sections(file_type, &sections);
    let status = if row.status.trim().is_empty() {
        "unknown"
    } else {
        row.status.trim()
    };

    let mut lines = vec![format!("**{session_id}** ({})", file_type.as_str())];
    lines.push(format!("  Modified: {modified}"));
    lines.push(format!("  {count_label}: {count_value}"));
    lines.push(format!("  Status: {status}"));

    if matches!(file_type, RecentActivityType::Memory) && !row.title.trim().is_empty() {
        lines.push(format!("  Title: {}", row.title.trim()));
    }

    if include_content {
        let (first_heading, first_content) = section_details(sections.first());
        let (last_heading, last_content) = section_details(sections.last());
        lines.push(format!("  First H2: {first_heading}"));
        lines.push(format!("  First H2 Content: {first_content}"));
        lines.push(format!("  Last H2: {last_heading}"));
        lines.push(format!("  Last H2 Content: {last_content}"));
    } else {
        let first = render_recent_activity_snippet(sections.first());
        let last = render_recent_activity_snippet(sections.last());
        lines.push(format!("  First: \"{first}\""));
        lines.push(format!("  Last: \"{last}\""));
    }

    lines.join("\n")
}

fn count_recent_activity_sections(
    file_type: RecentActivityType,
    sections: &[(String, String)],
) -> (&'static str, usize) {
    match file_type {
        RecentActivityType::Sequential | RecentActivityType::Thinking => (
            "Thoughts",
            sections
                .iter()
                .filter(|(heading, _)| heading.starts_with("Thought"))
                .count(),
        ),
        RecentActivityType::Workflow => ("Steps", sections.len()),
        RecentActivityType::Memory => ("Sections", sections.len()),
    }
}

fn section_details(section: Option<&(String, String)>) -> (String, String) {
    match section {
        Some((heading, content)) => (
            heading.trim().to_string(),
            if content.trim().is_empty() {
                String::new()
            } else {
                content.trim().to_string()
            },
        ),
        None => (String::new(), String::new()),
    }
}

// T-2.1: RTM FR-4 / RTM-025 exposes snippet extraction parity for RecentActivity pipeline tests.
pub fn recent_activity_extract_snippet(content: &str, max_length: i32) -> String {
    let max_length = if max_length == -1 {
        std::env::var("MAENIFOLD_SNIPPET_LENGTH")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1000)
    } else {
        usize::try_from(max_length)
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(0)
    };
    if max_length == 0 {
        return String::new();
    }

    let lines = content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let mut content_lines = lines
        .iter()
        .copied()
        .filter(|line| !line.trim_start().starts_with('*'))
        .collect::<Vec<_>>();
    if content_lines.is_empty() {
        content_lines = lines;
    }

    let snippet = content_lines.join(" ").trim().to_string();
    if snippet.chars().count() <= max_length {
        return snippet;
    }
    if max_length <= 3 {
        return ".".repeat(max_length);
    }

    let prefix = snippet.chars().take(max_length - 3).collect::<String>();
    format!("{prefix}...")
}

fn render_recent_activity_snippet(section: Option<&(String, String)>) -> String {
    let source = match section {
        Some((_, content)) if !content.trim().is_empty() => content,
        Some((heading, _)) => heading,
        None => "",
    };
    recent_activity_extract_snippet(source, -1)
}

fn extract_recent_activity_session_id(file_path: &str) -> String {
    file_path
        .trim_start_matches("memory://")
        .split('/')
        .next_back()
        .unwrap_or(file_path)
        .trim_end_matches(".md")
        .to_string()
}

fn format_recent_activity_timestamp(value: &str) -> String {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return timestamp
            .with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M")
            .to_string();
    }
    if let Ok(timestamp) = DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S %Z") {
        return timestamp
            .with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M")
            .to_string();
    }
    if let Some(prefix) = value.get(0..16) {
        return prefix.replace('T', " ");
    }
    value.to_string()
}
