// T-2.3: RTM FR-4 — Ported C# pure validation/extraction regressions for session ID handling.

use maenifold_core::tools::{is_valid_session_id_format, recent_activity_extract_session_id};

// T-2.3: RTM FR-4 — Valid sequential session IDs keep a numeric timestamp segment.
#[test]
fn is_valid_session_id_format_valid_sequential_session_id_returns_true() {
    let session_id = "session-1770910549160-79869";

    let result = is_valid_session_id_format(session_id);

    assert!(
        result,
        "a standard sequential session ID with numeric timestamp should be valid"
    );
}

// T-2.3: RTM FR-4 — Workflow session IDs may omit the random suffix.
#[test]
fn is_valid_session_id_format_valid_workflow_session_id_returns_true() {
    let session_id = "workflow-1770910549160";

    let result = is_valid_session_id_format(session_id);

    assert!(
        result,
        "a workflow session ID with numeric timestamp and no suffix should be valid"
    );
}

// T-2.3: RTM FR-4 — Non-numeric timestamp segments must be rejected.
#[test]
fn is_valid_session_id_format_non_numeric_timestamp_returns_false() {
    let session_id = "session-abc-12345";

    let result = is_valid_session_id_format(session_id);

    assert!(
        !result,
        "a session ID with non-numeric timestamp segment should be invalid"
    );
}

// T-2.3: RTM FR-4 — Missing separators must be rejected.
#[test]
fn is_valid_session_id_format_no_dash_returns_false() {
    let session_id = "noseparator";

    let result = is_valid_session_id_format(session_id);

    assert!(
        !result,
        "a session ID with no dash separator should be invalid"
    );
}

// T-2.3: RTM FR-4 — Empty timestamp segments after a trailing dash must fail.
#[test]
fn is_valid_session_id_format_trailing_dash_only_returns_false() {
    let session_id = "session-";

    let result = is_valid_session_id_format(session_id);

    assert!(
        !result,
        "a session ID with only a trailing dash should be invalid"
    );
}

// T-2.3: RTM FR-4 — Prefix text is irrelevant when the timestamp segment is numeric.
#[test]
fn is_valid_session_id_format_custom_prefix_returns_true() {
    let session_id = "custom-1234567890";

    let result = is_valid_session_id_format(session_id);

    assert!(
        result,
        "any prefix with a numeric timestamp segment should be valid"
    );
}

// T-2.3: RTM FR-4 — Zero remains a valid parsed timestamp value.
#[test]
fn is_valid_session_id_format_zero_timestamp_returns_true() {
    let session_id = "session-0-12345";

    let result = is_valid_session_id_format(session_id);

    assert!(
        result,
        "zero is a valid i64 value and should pass timestamp validation"
    );
}

// T-2.3: RTM FR-4 — Sequential paths with dated directories must extract the filename session ID.
#[test]
fn extract_session_id_sequential_path_with_date_dirs_returns_session_id() {
    let file_path = "memory://thinking/sequential/2026/02/12/session-1770910549160-79869.md";

    let result = recent_activity_extract_session_id(file_path);

    assert_eq!(
        result, "session-1770910549160-79869",
        "should extract session ID from the last path segment, stripping .md extension"
    );
}

// T-2.3: RTM FR-4 — Workflow paths with dated directories must extract the filename session ID.
#[test]
fn extract_session_id_workflow_path_with_date_dirs_returns_session_id() {
    let file_path = "memory://thinking/workflow/2026/02/12/workflow-1770910549160.md";

    let result = recent_activity_extract_session_id(file_path);

    assert_eq!(
        result, "workflow-1770910549160",
        "should extract workflow session ID from the last path segment"
    );
}

// T-2.3: RTM FR-4 — Extraction should not depend on the memory:// prefix.
#[test]
fn extract_session_id_path_without_memory_prefix_returns_session_id() {
    let file_path = "sequential/2026/02/12/session-123-456.md";

    let result = recent_activity_extract_session_id(file_path);

    assert_eq!(
        result, "session-123-456",
        "should handle paths without memory:// prefix"
    );
}

// T-2.3: RTM FR-4 — Regression check: never return a year directory instead of the filename.
#[test]
fn extract_session_id_path_with_date_dirs_does_not_return_year_as_session_id() {
    let file_path = "memory://thinking/sequential/2026/02/12/session-123-456.md";

    let result = recent_activity_extract_session_id(file_path);

    assert_ne!(
        result, "2026",
        "regression: must return the filename-based session ID, not the year directory segment"
    );
    assert_eq!(
        result, "session-123-456",
        "should return the session ID from the last path segment"
    );
}
