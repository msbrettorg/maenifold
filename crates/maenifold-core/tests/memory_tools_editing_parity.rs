use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// ---------------------------------------------------------------------------
// T-15.1: RTM FR-15 — append/prepend join with "\n\n" separator
// ---------------------------------------------------------------------------

/// Merged from `append_adds_double_newline_separator` and
/// `prepend_adds_double_newline_separator`.  Both operations use the same
/// setup/assertion pattern; only the expected content ordering differs.
#[test]
fn append_prepend_add_double_newline_separator() {
    let _guard = TEST_MUTEX.lock();

    for operation in ["append", "prepend"] {
        let env = isolated_env();
        let title = if operation == "append" {
            "Append Separator Test"
        } else {
            "Prepend Separator Test"
        };
        let uri = write_memory_file(
            &env,
            title,
            "Original content with [[test]].",
            Some("editing-parity"),
        );

        let new_content = if operation == "append" {
            "Appended content with [[concepts]]."
        } else {
            "Prepended content with [[concepts]]."
        };

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": operation,
                "content": new_content,
            }),
        );

        assert!(
            edit_result.starts_with("Updated memory FILE:"),
            "{operation}: unexpected: {edit_result}"
        );

        let read_result = read_memory_output(&env, &uri);

        if operation == "append" {
            // Existing body → \n\n → appended content (no heading between)
            assert!(
                read_result.contains(
                    "Original content with [[test]].\n\nAppended content with [[concepts]]."
                ),
                "append: expected \\n\\n separator between existing and appended content, got:\n{read_result}"
            );
        } else {
            // Prepend places new content before the file (including the
            // # Title heading), so the \n\n separator sits between the
            // prepended block and the heading — not directly adjacent to
            // the original body text.  Verify ordering + separator.
            assert!(
                read_result.contains("Prepended content with [[concepts]]."),
                "prepend: prepended content should be present, got:\n{read_result}"
            );
            assert!(
                read_result.contains("Original content with [[test]]."),
                "prepend: original content should be preserved, got:\n{read_result}"
            );
            let prep_pos = read_result
                .find("Prepended content with [[concepts]].")
                .unwrap();
            let orig_pos = read_result.find("Original content with [[test]].").unwrap();
            assert!(
                prep_pos < orig_pos,
                "prepend: prepended content should appear before original content"
            );
            // The \n\n separator follows the prepended text
            assert!(
                read_result.contains("Prepended content with [[concepts]].\n\n"),
                "prepend: expected \\n\\n after prepended content, got:\n{read_result}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// T-15.2: RTM FR-15 — case-insensitive operation matching with read-back
//         verification (coverage gap fix: rec #1, #3, #4, #5)
// ---------------------------------------------------------------------------

/// Merged from `operation_uppercase_append_resolves`,
/// `operation_titlecase_append_resolves`,
/// `operation_mixed_case_find_replace_resolves`, and
/// `operation_uppercase_replace_section_resolves`.
///
/// Each case variant verifies:
///   1. The edit returns the success prefix ("Updated memory FILE:").
///   2. A read-back confirms the operation was actually applied to content.
///
/// Also fills the coverage gap (rec #4) by testing PREPEND case variant.
#[test]
fn case_insensitive_operation_names_apply_correctly() {
    let _guard = TEST_MUTEX.lock();

    // --- APPEND variants (uppercase + titlecase) ---
    for (label, op_name) in [("uppercase", "APPEND"), ("titlecase", "Append")] {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            &format!("{label} Append CI Test"),
            "Original with [[test]].",
            Some("editing-parity"),
        );

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": op_name,
                "content": "Added with [[concepts]].",
            }),
        );

        assert!(
            edit_result.starts_with("Updated memory FILE:"),
            "{op_name} ({label}) should succeed, got: {edit_result}"
        );
        let read_result = read_memory_output(&env, &uri);
        assert!(
            read_result.contains("Added with [[concepts]]."),
            "{op_name} ({label}): appended content should be present in read-back, got:\n{read_result}"
        );
        assert!(
            read_result.contains("Original with [[test]]."),
            "{op_name} ({label}): original content should be preserved, got:\n{read_result}"
        );
    }

    // --- PREPEND variant (fills coverage gap — rec #4) ---
    {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            "Uppercase PREPEND CI Test",
            "Original with [[test]].",
            Some("editing-parity"),
        );

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": "PREPEND",
                "content": "Prepended with [[concepts]].",
            }),
        );

        assert!(
            edit_result.starts_with("Updated memory FILE:"),
            "PREPEND should succeed, got: {edit_result}"
        );
        let read_result = read_memory_output(&env, &uri);
        assert!(
            read_result.contains("Prepended with [[concepts]]."),
            "PREPEND: prepended content should be present, got:\n{read_result}"
        );
        let prep_pos = read_result.find("Prepended with [[concepts]].").unwrap();
        let orig_pos = read_result.find("Original with [[test]].").unwrap();
        assert!(
            prep_pos < orig_pos,
            "PREPEND: prepended content should appear before original"
        );
    }

    // --- Find_Replace (mixed case with underscore — distinct variant, rec #3) ---
    {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            "Mixed Case Find_Replace CI Test",
            "Text with [[test]] and target word here.",
            Some("editing-parity"),
        );

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": "Find_Replace",
                "content": "[[replacement]]",
                "findText": "target word",
            }),
        );

        assert!(
            edit_result.starts_with("Updated memory FILE:"),
            "Find_Replace (mixed case) should succeed, got: {edit_result}"
        );
        let read_result = read_memory_output(&env, &uri);
        assert!(
            read_result.contains("[[replacement]]"),
            "Find_Replace: replacement content should be present, got:\n{read_result}"
        );
        assert!(
            !read_result.contains("target word"),
            "Find_Replace: original text 'target word' should be replaced, got:\n{read_result}"
        );
    }

    // --- REPLACE_SECTION (uppercase) ---
    {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            "Uppercase REPLACE_SECTION CI Test",
            "Intro with [[test]].\n\n## My Section\n\nOld body content.",
            Some("editing-parity"),
        );

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": "REPLACE_SECTION",
                "content": "New body with [[concepts]].",
                "sectionName": "My Section",
            }),
        );

        assert!(
            edit_result.starts_with("Updated memory FILE:"),
            "REPLACE_SECTION (uppercase) should succeed, got: {edit_result}"
        );
        let read_result = read_memory_output(&env, &uri);
        assert!(
            read_result.contains("New body with [[concepts]]."),
            "REPLACE_SECTION: new content should be present, got:\n{read_result}"
        );
        assert!(
            !read_result.contains("Old body content"),
            "REPLACE_SECTION: old content should be replaced, got:\n{read_result}"
        );
        assert!(
            read_result.contains("## My Section"),
            "REPLACE_SECTION: heading should be preserved, got:\n{read_result}"
        );
    }
}

// ---------------------------------------------------------------------------
// T-15.3: RTM FR-15 — partial heading match for replace_section
// ---------------------------------------------------------------------------

#[test]
fn replace_section_partial_heading_match() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Partial Heading Match Test",
        "Intro with [[test]].\n\n## Design Decisions\n\nOld design content.\n\n## Implementation\n\nImpl content.",
        Some("editing-parity"),
    );

    // Searching for "Design" should match "## Design Decisions"
    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "New design notes with [[architecture]].",
            "sectionName": "Design"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "partial heading match should succeed, got: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    // Old heading preserved
    assert!(
        read_result.contains("## Design Decisions"),
        "original heading should be preserved"
    );
    // New content present
    assert!(read_result.contains("New design notes with [[architecture]]"));
    // Old content replaced
    assert!(!read_result.contains("Old design content"));
    // Other sections untouched
    assert!(read_result.contains("## Implementation"));
    assert!(read_result.contains("Impl content."));
}

#[test]
fn replace_section_exact_match_still_works() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Exact Heading Match Test",
        "Intro with [[test]].\n\n## Design Decisions\n\nOld content.",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "Updated with [[concepts]].",
            "sectionName": "Design Decisions"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "exact heading match should still work, got: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("## Design Decisions"));
    assert!(read_result.contains("Updated with [[concepts]]"));
}

// ---------------------------------------------------------------------------
// T-15.4: RTM FR-15 — replace_section heading preservation when new content
//         starts with a heading
// ---------------------------------------------------------------------------

#[test]
fn replace_section_with_heading_in_new_content_preserves_old_heading() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Heading Preservation Test",
        "Intro with [[test]].\n\n## Original Section Title\n\nOld body content.\n\n## Next Section\n\nNext content.",
        Some("editing-parity"),
    );

    // Provide new content that starts with its own heading — the OLD heading
    // should be preserved and the new heading stripped.
    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "replace_section",
            "content": "## Replaced Heading\nNew body with [[concepts]].",
            "sectionName": "Original Section Title"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    // Old heading must be preserved
    assert!(
        read_result.contains("## Original Section Title"),
        "old heading should be preserved, got:\n{read_result}"
    );
    // New body must be present
    assert!(
        read_result.contains("New body with [[concepts]]"),
        "new body content should be present"
    );
    // The replacement heading should NOT appear as a separate heading
    assert!(
        !read_result.contains("## Replaced Heading"),
        "the heading from new content should have been stripped"
    );
    // Other sections untouched
    assert!(read_result.contains("## Next Section"));
    assert!(read_result.contains("Next content."));
}
