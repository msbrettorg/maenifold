use super::recent_activity::RecentActivityRecord;
use super::recent_activity_format::RecentActivityType;
use chrono::{DateTime, Utc};

/// C# `ProcessThinkingFile`: Modified → count → Status → conditional First/Last snippets.
/// Filters out "Completion" and "Cancellation" sections from meaningful sections.
/// Sequential uses "First"/"Last" labels; workflow uses "First"/"Current" labels.
// PARITY-A01-FMT: RTM FR-4 / RTM-020
#[allow(clippy::too_many_lines)]
pub(super) fn format_thinking_entry(
    row: &RecentActivityRecord,
    file_type: RecentActivityType,
    sections: Vec<(String, String)>,
    include_content: bool,
) -> String {
    let session_id = extract_recent_activity_session_id(&row.file_path);
    let modified = format_recent_activity_timestamp(&row.last_indexed);
    let status = if row.status.trim().is_empty() {
        "unknown"
    } else {
        row.status.trim()
    };

    // C# filters out Completion/Cancellation sections for snippet display.
    let meaningful: Vec<&(String, String)> = sections
        .iter()
        .filter(|(heading, _)| heading != "Completion" && heading != "Cancellation")
        .collect();

    // Determine type label and count based on whether sequential or workflow.
    // C# logic: if type=="sequential" OR any heading starts with "Thought" → sequential path.
    let is_sequential = matches!(file_type, RecentActivityType::Sequential)
        || sections.iter().any(|(h, _)| h.starts_with("Thought"));

    let (count_label, count_value, last_label) = if is_sequential {
        let thought_count = sections
            .iter()
            .filter(|(heading, _)| heading.starts_with("Thought"))
            .count();
        ("Thoughts", thought_count, "Last")
    } else {
        let step_count = sections
            .iter()
            .filter(|(heading, _)| heading.contains("Step") || heading.contains("Response"))
            .count();
        ("Steps", step_count, "Current")
    };

    let type_label = if is_sequential {
        "sequential"
    } else {
        "workflow"
    };

    let mut lines = vec![format!("**{session_id}** ({type_label})")];
    lines.push(format!("  Modified: {modified}"));
    lines.push(format!("  {count_label}: {count_value}"));
    lines.push(format!("  Status: {status}"));

    // C# only emits First if meaningfulSections.Count > 0
    if !meaningful.is_empty()
        || row
            .first
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        if include_content {
            let (first_heading, first_content) =
                summary_section_details(row.first.as_deref(), meaningful.first().copied());
            lines.push(format!("  First H2: {first_heading}"));
            lines.push(format!("  First H2 Content: {first_content}"));
        } else {
            let first_snippet = render_recent_activity_summary_or_snippet(
                row.first.as_deref(),
                meaningful.first().copied(),
            );
            lines.push(format!("  First: \"{first_snippet}\""));
        }
    }

    // C# only emits Last/Current if meaningfulSections.Count > 1 AND last != first
    let has_distinct_summary_last = row
        .last
        .as_deref()
        .zip(row.first.as_deref())
        .map(|(last, first)| last.trim() != first.trim())
        .unwrap_or(false);
    if meaningful.len() > 1 || has_distinct_summary_last {
        let last = meaningful.last().copied();
        if include_content {
            let (last_heading, last_content) = summary_section_details(row.last.as_deref(), last);
            lines.push(format!("  Last H2: {last_heading}"));
            lines.push(format!("  Last H2 Content: {last_content}"));
        } else {
            let last_snippet = render_recent_activity_summary_or_snippet(row.last.as_deref(), last);
            let first_snippet = render_recent_activity_summary_or_snippet(
                row.first.as_deref(),
                meaningful.first().copied(),
            );
            if last_snippet != first_snippet {
                lines.push(format!("  {last_label}: \"{last_snippet}\""));
            }
        }
    }

    lines.join("\n")
}

/// C# `ProcessMemoryFile`: Modified → Title → Sections → conditional First/Last snippets.
/// No Status line.  Title emitted before Sections.
// PARITY-A01-FMT: RTM FR-4 / RTM-020
pub(super) fn format_memory_entry(
    row: &RecentActivityRecord,
    file_type: RecentActivityType,
    sections: Vec<(String, String)>,
    include_content: bool,
) -> String {
    let session_id = extract_recent_activity_session_id(&row.file_path);
    let modified = format_recent_activity_timestamp(&row.last_indexed);

    let mut lines = vec![format!("**{session_id}** ({})", file_type.as_str())];
    lines.push(format!("  Modified: {modified}"));
    // PARITY-A01-FMT: C# emits Title before Sections, and omits Status for memory files.
    lines.push(format!("  Title: {}", row.title.trim()));
    lines.push(format!("  Sections: {}", sections.len()));

    // C# only emits First if sections.Count > 0
    if !sections.is_empty()
        || row
            .first
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        if include_content {
            let (first_heading, first_content) =
                summary_section_details(row.first.as_deref(), sections.first());
            lines.push(format!("  First H2: {first_heading}"));
            lines.push(format!("  First H2 Content: {first_content}"));
        } else {
            let first_snippet =
                render_recent_activity_summary_or_snippet(row.first.as_deref(), sections.first());
            lines.push(format!("  First: \"{first_snippet}\""));
        }
    }

    // C# only emits Last if sections.Count > 1 AND last != first
    let has_distinct_summary_last = row
        .last
        .as_deref()
        .zip(row.first.as_deref())
        .map(|(last, first)| last.trim() != first.trim())
        .unwrap_or(false);
    if sections.len() > 1 || has_distinct_summary_last {
        if include_content {
            let (last_heading, last_content) =
                summary_section_details(row.last.as_deref(), sections.last());
            lines.push(format!("  Last H2: {last_heading}"));
            lines.push(format!("  Last H2 Content: {last_content}"));
        } else {
            let last_snippet =
                render_recent_activity_summary_or_snippet(row.last.as_deref(), sections.last());
            let first_snippet =
                render_recent_activity_summary_or_snippet(row.first.as_deref(), sections.first());
            if last_snippet != first_snippet {
                lines.push(format!("  Last: \"{last_snippet}\""));
            }
        }
    }

    lines.join("\n")
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

fn summary_section_details(
    summary: Option<&str>,
    fallback: Option<&(String, String)>,
) -> (String, String) {
    if let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) {
        let heading = summary
            .split_once(':')
            .map(|(heading, _)| heading.trim().to_string())
            .unwrap_or_default();
        return (heading, summary.to_string());
    }
    section_details(fallback)
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

fn render_recent_activity_summary_or_snippet(
    summary: Option<&str>,
    fallback: Option<&(String, String)>,
) -> String {
    if let Some(summary) = summary.map(str::trim).filter(|value| !value.is_empty()) {
        return recent_activity_extract_snippet(summary, -1);
    }
    render_recent_activity_snippet(fallback)
}

pub(super) fn extract_recent_activity_session_id(file_path: &str) -> String {
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
