use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, edit_memory_output, isolated_env, read_memory_output, write_memory_file,
};

// T-12.6: RTM NFR-1
#[test]
fn find_replace_replacing_text_inside_wikilink_creates_nested_brackets() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "WikiLink Nested Brackets Test",
        "This file has a [[Machine Learning]] concept.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Deep Learning]]",
            "findText": "Learning",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("ERROR:"),
        "unexpected: {edit_result}"
    );
    assert!(edit_result.contains("must contain at least one [[concept]]"));
}

#[test]
fn find_replace_replacing_entire_wikilink_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Replace Entire WikiLink Test",
        "This file references [[Machine Learning]] and [[Deep Learning]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Artificial Intelligence]]",
            "findText": "[[Machine Learning]]",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[Artificial Intelligence]]"));
    assert!(!read_result.contains("[[Machine Learning]]"));
    assert!(read_result.contains("[[Deep Learning]]"));
}

#[test]
fn find_replace_multiple_wikilinks_with_same_inner_text_replaces_all() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiple WikiLinks Same Text Test",
        "We study [[Neural Networks]] and [[Convolutional Neural Networks]].\nAlso see [[Recurrent Neural Networks]] and [[Neural Networks Architecture]].",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "NN",
            "findText": "Neural Networks",
            "expectedCount": 4
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[NN]]"));
    assert!(read_result.contains("[[Convolutional NN]]"));
    assert!(read_result.contains("[[Recurrent NN]]"));
    assert!(read_result.contains("[[NN Architecture]]"));
    assert!(!read_result.contains("Neural Networks"));
}

#[test]
fn find_replace_replacing_with_multiple_wikilinks_works_correctly() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Multiple WikiLinks Replace Test",
        "This topic covers [[AI]] fundamentals.",
        Some("edit-memory-tests"),
    );

    let edit_result = edit_memory_output(
        &env,
        json!({
            "identifier": uri,
            "operation": "find_replace",
            "content": "[[Machine Learning]], [[Deep Learning]], and [[Neural Networks]]",
            "findText": "fundamentals",
            "expectedCount": 1
        }),
    );

    assert!(
        edit_result.starts_with("Updated memory FILE:"),
        "unexpected: {edit_result}"
    );
    let read_result = read_memory_output(&env, &uri);
    assert!(read_result.contains("[[Machine Learning]]"));
    assert!(read_result.contains("[[Deep Learning]]"));
    assert!(read_result.contains("[[Neural Networks]]"));
}
