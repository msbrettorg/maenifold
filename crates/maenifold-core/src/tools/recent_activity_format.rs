use super::recent_activity::RecentActivityRecord;
use super::recent_activity_render::{
    extract_recent_activity_session_id, format_memory_entry, format_thinking_entry,
};
use crate::markdown::extract_h2_sections;

#[derive(Clone, Copy)]
pub(super) enum RecentActivityType {
    Sequential,
    Workflow,
    Thinking,
    Memory,
}

impl RecentActivityType {
    pub(super) fn as_str(self) -> &'static str {
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

// PARITY-A01-FMT: RTM FR-4 / RTM-020 — C# splits formatting into ProcessThinkingFile
// (sequential/workflow) and ProcessMemoryFile (memory/thinking).  Field order, Status
// presence, snippet conditionals, and "Completion"/"Cancellation" filtering all differ.
fn format_recent_activity_entry(
    row: &RecentActivityRecord,
    file_type: RecentActivityType,
    sections: Vec<(String, String)>,
    include_content: bool,
) -> String {
    match file_type {
        // T-2.1: RTM FR-4 — sequential & workflow go through ProcessThinkingFile path.
        RecentActivityType::Sequential | RecentActivityType::Workflow => {
            format_thinking_entry(row, file_type, sections, include_content)
        }
        // T-2.1: RTM FR-4 — memory & thinking go through ProcessMemoryFile path.
        RecentActivityType::Memory | RecentActivityType::Thinking => {
            format_memory_entry(row, file_type, sections, include_content)
        }
    }
}
