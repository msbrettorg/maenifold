use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// T-12.6: RTM NFR-1
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
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[bar]] appearing twice: [[bar]] and [[bar]]"));
    assert!(!read_result.contains("foo"));
}

#[test]
fn find_replace_with_incorrect_expected_count_throws_argument_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Find Replace Mismatch Test",
        "This is a [[test]] with foo appearing twice: foo and foo.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "foo",
            "expectedCount": 1
        }),
    )
    .expect_err("expected count mismatch should error");

    assert!(err.to_string().contains("Expected 1 matches but found 3"));
    assert!(err.to_string().contains("Find text: 'foo'"));
}

#[test]
fn find_replace_with_expected_count_zero_throws_when_matches_found() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Zero Expected Test",
        "This is a [[test]] with foo in it.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "foo",
            "expectedCount": 0
        }),
    )
    .expect_err("expected count zero should error");

    assert!(err.to_string().contains("Expected 0 matches but found 1"));
}

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
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("bar appearing multiple times: bar, bar, bar."));
}

#[test]
fn find_replace_with_no_matches_throws_when_expecting_matches() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "No Matches Test",
        "This is a [[test]] without the target text.",
        Some("edit-memory-tests"),
    );

    let err = memory::edit_memory(
        &env.ctx(),
        &json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "replacement with [[test]]",
            "findText": "nonexistent",
            "expectedCount": 2
        }),
    )
    .expect_err("no matches should error");

    assert!(err.to_string().contains("Expected 2 matches but found 0"));
    assert!(err.to_string().contains("Find text: 'nonexistent'"));
}

#[test]
fn find_replace_with_multiline_pattern_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiline Replace Test",
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
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("First section with [[content]]"));
    assert!(read_result.contains("Second section with [[more-content]]"));
}

#[test]
fn append_with_concepts_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Append Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "append",
            "content": "Appended content with [[more-concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Original content with [[test]]."));
    assert!(read_result.contains("Appended content with [[more-concepts]]."));
}

#[test]
fn prepend_with_concepts_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Prepend Test",
        "Original content with [[test]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "prepend",
            "content": "Prepended content with [[new-concepts]]."
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("Prepended content with [[new-concepts]]."));
    assert!(read_result.contains("Original content with [[test]]."));
}

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
