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
// REWRITE: strengthened assertions — pins "all lines included" contract, not just "some bullet"
#[test]
fn extract_snippet_bullet_only_content_falls_back_to_all_lines() {
    let content = "* Bullet one\n* Bullet two\n* Bullet three";
    let result = recent_activity_extract_snippet(content, 1000);
    assert!(
        result.contains("Bullet one"),
        "expected 'Bullet one' in result: {result}"
    );
    assert!(
        result.contains("Bullet two"),
        "expected 'Bullet two' in result: {result}"
    );
    assert!(
        result.contains("Bullet three"),
        "expected 'Bullet three' in result: {result}"
    );
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
// REWRITE: exact equality replaces trivially-true inequality (Rec #2)
#[test]
fn extract_snippet_uses_default_length_when_minus_one() {
    let long_content = "X".repeat(2000);
    let result = recent_activity_extract_snippet(&long_content, -1);
    assert_eq!(
        result.chars().count(),
        1000,
        "expected exactly 1000 chars (997 + '...'), got {}",
        result.chars().count()
    );
}

// T-2.3: RTM FR-4 / RTM-029
// MERGE of extract_session_id_standard_sequential, _simple_memory, _chat (Rec #3)
#[test]
fn extract_session_id_extracts_filename_stem() {
    let cases: &[(&str, &str)] = &[
        (
            "memory://thinking/sequential/2026/02/session-12345.md",
            "session-12345",
        ),
        ("memory://docs/my-note.md", "my-note"),
        ("memory://chat/2026/02/chat-session.md", "chat-session"),
    ];
    for (path, expected) in cases {
        let id = recent_activity_extract_session_id(path);
        assert_eq!(id, *expected, "path={path}");
    }
}

// T-2.3: RTM FR-4 / RTM-029
// MERGE of determine_file_type_sequential, _workflow, _memory, _chat (Rec #4)
// Adds missing Thinking variant coverage (Rec #5)
#[test]
fn determine_file_type_classifies_path_correctly() {
    let cases: &[(&str, &str)] = &[
        (
            "memory://thinking/sequential/2026/02/session.md",
            "sequential",
        ),
        ("memory://thinking/workflow/2026/02/wf.md", "workflow"),
        ("memory://thinking/my-note.md", "thinking"),
        ("memory://docs/my-note.md", "memory"),
        ("memory://chat/2026/02/assumptions.md", "memory"),
    ];
    for (path, expected) in cases {
        let file_type = recent_activity_determine_file_type(path);
        assert_eq!(file_type, *expected, "path={path}");
    }
}
