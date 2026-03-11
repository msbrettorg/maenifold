use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// ---------------------------------------------------------------------------
// T-15.1: RTM FR-15 — append/prepend join with "\n\n" separator
// ---------------------------------------------------------------------------

#[test]
fn append_adds_double_newline_separator() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Append Separator Test",
        "Original content with [[test]].",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "append",
            "content": "Appended content with [[concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    // The two pieces must be separated by a blank line (\n\n)
    assert!(
        read_result.contains("Original content with [[test]].\n\nAppended content with [[concepts]]."),
        "expected \\n\\n separator between existing and appended content, got:\n{read_result}"
    );
}

#[test]
fn prepend_adds_double_newline_separator() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Prepend Separator Test",
        "Original content with [[test]].",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "prepend",
            "content": "Prepended content with [[concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    // Prepended content → \n\n → original content
    assert!(
        read_result.contains("Prepended content with [[concepts]].\n\nOriginal content with [[test]]."),
        "expected \\n\\n separator between prepended and existing content, got:\n{read_result}"
    );
}

// ---------------------------------------------------------------------------
// T-15.2: RTM FR-15 — case-insensitive operation matching
// ---------------------------------------------------------------------------

#[test]
fn operation_uppercase_append_resolves() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Uppercase APPEND Test",
        "Original with [[test]].",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "APPEND",
            "content": "Added with [[concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "APPEND (uppercase) should work, got: {edit_result}"
    );
}

#[test]
fn operation_titlecase_append_resolves() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Titlecase Append Test",
        "Original with [[test]].",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "Append",
            "content": "Added with [[concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "Append (titlecase) should work, got: {edit_result}"
    );
}

#[test]
fn operation_mixed_case_find_replace_resolves() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Mixed Case Find_Replace Test",
        "Text with [[test]] and target word here.",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "Find_Replace",
            "content": "[[replacement]]",
            "findText": "target word"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "Find_Replace (mixed case) should work, got: {edit_result}"
    );
}

#[test]
fn operation_uppercase_replace_section_resolves() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Uppercase REPLACE_SECTION Test",
        "Intro with [[test]].\n\n## My Section\n\nOld body content.",
        Some("editing-parity"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "REPLACE_SECTION",
            "content": "New body with [[concepts]].",
            "sectionName": "My Section"
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "REPLACE_SECTION (uppercase) should work, got: {edit_result}"
    );
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
