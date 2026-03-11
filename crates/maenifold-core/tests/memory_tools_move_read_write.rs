use std::fs;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, isolated_env, move_memory_output, raw_path_for_uri, read_memory_output,
    write_memory_file, write_raw_memory,
};

// T-12.6: RTM NFR-1
#[test]
fn move_memory_with_path_based_destination_preserves_markdown_extension() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Move Extension Test",
        "# Move Extension Test\n\nTesting [[file-operations]] preserves .md extension.",
        None,
    );

    let move_result = move_memory_output(&env, &uri, "regression-test/Moved Extension Test");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://regression-test/moved-extension-test"));

    let read_result = read_memory_output(&env, "memory://regression-test/moved-extension-test");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[file-operations]] preserves .md extension."));
}

#[test]
fn move_memory_with_simple_destination_preserves_existing_behavior() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Simple Move Test",
        "# Simple Move Test\n\nTesting [[refactoring]] functionality.",
        None,
    );

    let move_result = move_memory_output(&env, &uri, "Simple Move Renamed");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://simple-move-renamed"));

    let read_result = read_memory_output(&env, "memory://simple-move-renamed");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[refactoring]] functionality."));
}

#[test]
fn read_memory_handles_missing_title_in_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_raw_memory(
        &env,
        "memory-tools-tests/no-title-test.md",
        "---\ntype: memory\nstatus: saved\n---\n\n# Test Content\n\nThis is a [[metadata-handling]] without a title in frontmatter.\n",
    );

    let read_result = read_memory_output(&env, &uri);

    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("# no-title-test"));
    assert!(read_result.contains("URI: memory://memory-tools-tests/no-title-test"));
    assert!(
        read_result.contains("This is a [[metadata-handling]] without a title in frontmatter.")
    );
}

#[test]
fn read_memory_handles_empty_frontmatter() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_raw_memory(
        &env,
        "memory-tools-tests/no-frontmatter-test.md",
        "# Test Without Frontmatter\n\nThis is a [[yaml-frontmatter]] without any frontmatter.\n",
    );

    let read_result = read_memory_output(&env, &uri);

    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("# no-frontmatter-test"));
    assert!(read_result.contains("This is a [[yaml-frontmatter]] without any frontmatter."));
}

#[test]
fn write_memory_does_not_write_embedding_fields() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Embedding Removal Test",
        "Content with [[vector-embeddings]] concept to ensure write path works.",
        Some("memory-tools-tests"),
    );
    let raw = fs::read_to_string(raw_path_for_uri(&env, &uri)).expect("read raw file");

    assert!(!raw.contains("embedding_base64"));
    assert!(!raw.contains("embedding_model"));
    assert!(!raw.contains("embedding_date"));
}

#[test]
fn write_memory_with_existing_h1_does_not_create_duplicate_heading() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Duplicate H1 Test",
        "# Existing Heading\n\nContent with [[concept]]",
        Some("memory-tools-tests"),
    );

    let raw = fs::read_to_string(raw_path_for_uri(&env, &uri)).expect("read raw file");
    let h1_count = raw
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("# ") && !trimmed.starts_with("## ")
        })
        .count();

    assert_eq!(h1_count, 1, "unexpected raw content:\n{raw}");
    assert!(raw.contains("# Existing Heading"));

    let read_result = read_memory_output(&env, &uri);
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Content with [[concept]]"));
}
