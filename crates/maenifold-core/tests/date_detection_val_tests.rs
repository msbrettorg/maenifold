// T-2.3: RTM FR-4 — Ported C# pure validation/extraction regressions for session ID handling.

use maenifold_core::tools::{is_valid_session_id_format, recent_activity_extract_session_id};

// T-2.3: RTM FR-4 — Any prefix with a numeric second segment is valid; the function is prefix-agnostic.
#[test]
fn is_valid_session_id_format_valid_ids_with_various_prefixes_return_true() {
    let cases: &[(&str, &str)] = &[
        ("session-1770910549160-79869", "standard sequential session ID"),
        ("workflow-1770910549160", "workflow session ID without random suffix"),
        ("custom-1234567890", "arbitrary prefix with numeric timestamp"),
    ];

    for &(session_id, label) in cases {
        assert!(
            is_valid_session_id_format(session_id),
            "{label}: '{session_id}' should be valid"
        );
    }
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

// T-2.3: RTM FR-4 — Date-nested paths must extract the filename session ID regardless of session type.
#[test]
fn extract_session_id_dated_paths_return_filename_session_id() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "memory://thinking/sequential/2026/02/12/session-1770910549160-79869.md",
            "session-1770910549160-79869",
            "sequential path with date directories",
        ),
        (
            "memory://thinking/workflow/2026/02/12/workflow-1770910549160.md",
            "workflow-1770910549160",
            "workflow path with date directories",
        ),
    ];

    for &(file_path, expected, label) in cases {
        let result = recent_activity_extract_session_id(file_path);
        assert_eq!(
            result, expected,
            "{label}: should extract session ID from the last path segment"
        );
    }
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

// T-2.3: RTM FR-4 — Regression: previously returned year directory "2026" instead of filename.
#[test]
fn extract_session_id_path_with_date_dirs_does_not_return_year_as_session_id() {
    let file_path = "memory://thinking/sequential/2026/02/12/session-123-456.md";

    let result = recent_activity_extract_session_id(file_path);

    // Regression: a prior implementation split on "/" incorrectly and returned the year
    // directory "2026" instead of the filename-based session ID.
    assert_eq!(
        result, "session-123-456",
        "should return the session ID from the last path segment, not a directory component"
    );
}

// T-2.3: RTM FR-4 — Flat filename with no path separators should return the filename itself.
#[test]
fn extract_session_id_flat_filename_without_slashes_returns_filename() {
    let file_path = "session-999-42.md";

    let result = recent_activity_extract_session_id(file_path);

    assert_eq!(
        result, "session-999-42",
        "a flat filename with no '/' separators should return the trimmed filename as session ID"
    );
}
