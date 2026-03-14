use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// T-12.6: RTM NFR-1 — KEEP: verifies content mutation with correct expectedCount
#[test]
fn find_replace_with_correct_expected_count_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Find Replace Test",
        "This is a [[test]] with foo appearing twice: foo and foo.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[bar]]",
            "findText": "foo",
            "expectedCount": 3
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "edit result should contain URI: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[bar]] appearing twice: [[bar]] and [[bar]]"));
    assert!(!read_result.contains("foo"));
}

// T-12.6: RTM NFR-1 — MERGE: parameterized expectedCount mismatch error cases
// Covers: (expected=1, actual=3), (expected=0, actual=1), (expected=2, actual=0)
#[test]
fn find_replace_expectedcount_mismatch_errors() {
    let _guard = TEST_MUTEX.lock();

    let cases: &[(&str, &str, &str, u32, u32)] = &[
        // (title, content, find_text, expected_count, actual_count)
        (
            "Mismatch More Found",
            "This is a [[test]] with foo appearing twice: foo and foo.",
            "foo",
            1,
            3,
        ),
        (
            "Zero Expected One Found",
            "This is a [[test]] with foo in it.",
            "foo",
            0,
            1,
        ),
        (
            "Expected But None Found",
            "This is a [[test]] without the target text.",
            "nonexistent",
            2,
            0,
        ),
    ];

    for (title, content, find_text, expected_count, actual_count) in cases {
        let env = isolated_env();
        let uri = write_memory_file(&env, title, content, Some("edit-memory-tests"));

        let err = memory::edit_memory(
            &env.ctx(),
            &json!({
                "identifier": uri,
                "operation": "find_replace",
                "content": "replacement with [[test]]",
                "findText": find_text,
                "expectedCount": expected_count
            }),
        )
        .expect_err(&format!(
            "expected={expected_count} vs actual={actual_count} should error"
        ));

        let msg = err.to_string();
        assert!(
            msg.contains(&format!(
                "Expected {expected_count} matches but found {actual_count}"
            )),
            "case ({title}): unexpected error: {msg}"
        );
        assert!(
            msg.contains(&format!("Find text: '{find_text}'")),
            "case ({title}): error should mention find text: {msg}"
        );
    }
}

// T-12.6: RTM NFR-1 — KEEP: null expectedCount means "replace all without count validation"
#[test]
fn find_replace_with_null_expected_count_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Count Validation Test",
        "This is a [[test]] with foo appearing multiple times: foo, foo, foo.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "bar",
            "findText": "foo",
            "expectedCount": null
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "edit result should contain URI: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("bar appearing multiple times: bar, bar, bar."));
}

// T-12.6: RTM NFR-1 — REWRITE: renamed from find_replace_with_multiline_pattern_works_correctly
// The needle "paragraph" is a single word, not a multiline pattern. This test verifies
// find_replace correctly replaces a word appearing in distinct document sections.
#[test]
fn find_replace_replaces_word_across_document_sections() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Cross Section Replace Test",
        "This is a [[test]] file.\n\nFirst paragraph with [[content]].\n\nSecond paragraph with [[more-content]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "section",
            "findText": "paragraph",
            "expectedCount": 2
        }),
    );

    assert!(
        edit_result.contains(&uri),
        "edit result should contain URI: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("First section with [[content]]"));
    assert!(read_result.contains("Second section with [[more-content]]"));
}

// T-12.6: RTM NFR-1 — MERGE: parameterized append/prepend success paths
#[test]
fn edit_memory_op_with_concepts_succeeds() {
    let _guard = TEST_MUTEX.lock();

    let cases: &[(&str, &str, &str, &str)] = &[
        // (operation, title, new_content, expected_snippet)
        (
            "append",
            "Append Test",
            "Appended content with [[more-concepts]].",
            "Appended content with [[more-concepts]].",
        ),
        (
            "prepend",
            "Prepend Test",
            "Prepended content with [[new-concepts]].",
            "Prepended content with [[new-concepts]].",
        ),
    ];

    for (operation, title, new_content, expected_snippet) in cases {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            title,
            "Original content with [[test]].",
            Some("edit-memory-tests"),
        );

        let edit_result = edit_memory_output(
            &env,
            json!({
                "identifier": uri,
                "operation": operation,
                "content": new_content
            }),
        );

        assert!(
            edit_result.contains(&uri),
            "case ({operation}): edit result should contain URI: {edit_result}"
        );
        let read_result = read_memory_output(&env, &uri);
        assert!(
            read_result.contains("Original content with [[test]]."),
            "case ({operation}): original content should be preserved"
        );
        assert!(
            read_result.contains(expected_snippet),
            "case ({operation}): new content should be present"
        );
    }
}

// T-12.6: RTM NFR-1 — KEEP: enforces WikiLink-required business rule
#[test]
fn edit_memory_without_concepts_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Concepts Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "append",
            "content": "Content without any concepts."
        }),
    );

    assert!(
        edit_result.starts_with("ERROR:"),
        "unexpected: {edit_result}"
    );
    assert!(edit_result.contains("must contain at least one [[concept]]"));
}
