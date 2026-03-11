use std::fs;

use maenifold_core::memory;
use serde_json::json;

#[path = "memory_tools/support.rs"]
mod support;

use support::{TEST_MUTEX, extract_uri, isolated_env, raw_path_for_uri, read_memory_output};

// T-12.6: RTM NFR-1
#[test]
fn sanitize_user_input_blocks_url_encoded_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%2e%2e%2fmalicious",
            "content": "Safe content with [[security-validation]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
    assert!(result.contains("testmalicious"));

    let read_result = read_memory_output(&env, "memory://memory-tools-tests/testmalicious");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Safe content with [[security-validation]]"));
}

#[test]
fn sanitize_user_input_blocks_double_url_encoded_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%252e%252e%252fmalicious",
            "content": "Safe content with [[input-sanitization]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
}

#[test]
fn sanitize_user_input_blocks_url_encoded_dot_dot() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "test%2e%2e",
            "content": "Safe content with [[path-traversal-prevention]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));
}

#[test]
fn sanitize_user_input_blocks_mixed_encoded_and_plain_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "%2e./test",
            "content": "Safe content with [[url-encoding]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(!result.contains(".."));

    let read_result = read_memory_output(&env, "memory://memory-tools-tests/test");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
}

#[test]
fn sanitize_user_input_preserves_legitimate_percent_signs() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": "Task 50 Complete",
            "content": "Status update with [[task-tracking]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    let uri = extract_uri(&result);
    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Status update with [[task-tracking]]"));
}

#[test]
fn write_memory_truncates_overlong_slug_but_preserves_title_metadata() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "a".repeat(256);

    // T-14.8: RTM FR-14 - overlong slugs should truncate to a safe filesystem
    // name while preserving the original title in frontmatter metadata.
    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": "Safe content with [[long-title-slug]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    let uri = extract_uri(&result);
    let expected_slug = "a".repeat(255);
    assert_eq!(uri, format!("memory://memory-tools-tests/{expected_slug}"));

    let raw = fs::read_to_string(raw_path_for_uri(&env, &uri)).expect("read raw file");
    assert!(
        raw.contains(&format!("title: {}", "a".repeat(256))),
        "unexpected raw:\n{raw}"
    );
    assert!(
        raw.contains(&format!("permalink: {expected_slug}")),
        "unexpected raw:\n{raw}"
    );
}

#[test]
fn write_memory_keeps_normal_slug_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "Normal Title";

    // T-14.8: RTM FR-14 - normal slugs remain unchanged as a regression check.
    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": "Safe content with [[normal-title-slug]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert_eq!(
        extract_uri(&result),
        "memory://memory-tools-tests/normal-title"
    );
}

#[test]
fn write_memory_keeps_exact_255_char_slug_unchanged() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let title = "b".repeat(255);

    // T-14.8: RTM FR-14 - slugs already at the 255-character limit should not
    // be truncated further.
    let result = memory::write_memory(
        &env.ctx(),
        &json!({
            "title": title,
            "content": "Safe content with [[exact-title-slug]].",
            "folder": "memory-tools-tests"
        }),
    )
    .expect("write memory");

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    let expected_slug = "b".repeat(255);
    assert_eq!(
        extract_uri(&result),
        format!("memory://memory-tools-tests/{expected_slug}")
    );
}
