#[path = "recent_activity_pipeline/support.rs"]
mod recent_activity_pipeline_support;

use recent_activity_pipeline_support::*;

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_empty_input_returns_empty_string() {
    assert_eq!(recent_activity_extract_snippet("", 100), "");
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_short_content_returns_full_content() {
    let content = "This is a short sentence.";
    assert_eq!(recent_activity_extract_snippet(content, 200), content);
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_long_content_truncates_with_ellipsis() {
    let long_content = "A".repeat(200);
    let result = recent_activity_extract_snippet(&long_content, 50);
    assert_eq!(result.chars().count(), 50);
    assert!(result.ends_with("..."));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_bullet_only_content_falls_back_to_all_lines() {
    let content = "* Bullet one\n* Bullet two\n* Bullet three";
    let result = recent_activity_extract_snippet(content, 1000);
    assert!(!result.is_empty());
    assert!(result.contains("Bullet"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_mixed_content_skips_bullet_lines() {
    let content = "* Metadata bullet\nActual prose content here.";
    let result = recent_activity_extract_snippet(content, 1000);
    assert!(result.contains("prose content"));
    assert!(!result.contains("Metadata bullet"));
}

// T-2.3: RTM FR-4 / RTM-025
#[test]
fn extract_snippet_uses_default_length_when_minus_one() {
    let long_content = "X".repeat(2000);
    let result = recent_activity_extract_snippet(&long_content, -1);
    assert!(
        result.chars().count() <= 1000,
        "unexpected len: {}",
        result.chars().count()
    );
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_standard_sequential_path_extracts_id() {
    let id =
        recent_activity_extract_session_id("memory://thinking/sequential/2026/02/session-12345.md");
    assert_eq!(id, "session-12345");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_simple_memory_path_extracts_filename() {
    let id = recent_activity_extract_session_id("memory://docs/my-note.md");
    assert_eq!(id, "my-note");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn extract_session_id_chat_path_extracts_filename() {
    let id = recent_activity_extract_session_id("memory://chat/2026/02/chat-session.md");
    assert_eq!(id, "chat-session");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_sequential_path_returns_sequential() {
    let file_type =
        recent_activity_determine_file_type("memory://thinking/sequential/2026/02/session-abc.md");
    assert_eq!(file_type, "sequential");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_workflow_path_returns_workflow() {
    let file_type =
        recent_activity_determine_file_type("memory://thinking/workflow/2026/02/wf-session.md");
    assert_eq!(file_type, "workflow");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_memory_path_returns_memory() {
    let file_type = recent_activity_determine_file_type("memory://docs/my-note.md");
    assert_eq!(file_type, "memory");
}

// T-2.3: RTM FR-4 / RTM-029
#[test]
fn determine_file_type_chat_path_returns_memory() {
    let file_type = recent_activity_determine_file_type("memory://chat/2026/02/assumptions.md");
    assert_eq!(file_type, "memory");
}
