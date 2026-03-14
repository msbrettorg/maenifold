use std::fs;

#[path = "memory_tools/support.rs"]
mod support;

use support::{
    TEST_MUTEX, isolated_env, move_memory_output, raw_path_for_uri, read_memory_output,
    write_memory_file, write_raw_memory,
};

// T-12.6: RTM NFR-1 — MoveMemory with a slash-containing destination creates
// the subdirectory structure and moves content so it is readable at the new URI.
// The source URI must become inaccessible after the move.
#[test]
fn move_memory_path_based_destination_moves_content_to_subdirectory() {
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

    // Verify content is readable at the new URI
    let read_result = read_memory_output(&env, "memory://regression-test/moved-extension-test");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[file-operations]] preserves .md extension."));

    // Verify the source URI is no longer accessible — a move that copies
    // without deleting the source would pass all prior assertions.
    let source_read = read_memory_output(&env, &uri);
    assert!(
        source_read.starts_with("ERROR:"),
        "source URI should be inaccessible after move, got: {source_read}"
    );
}

// T-12.6: RTM NFR-1
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

// T-12.6: RTM NFR-1 — file has no frontmatter delimiters at all (not the same
// as empty frontmatter `---\n---`). ReadMemory should fall back to the filename
// slug for heading and URI, and preserve the body content.
#[test]
fn read_memory_handles_file_with_no_frontmatter() {
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
    assert!(read_result.contains("URI: memory://memory-tools-tests/no-frontmatter-test"));
    assert!(read_result.contains("This is a [[yaml-frontmatter]] without any frontmatter."));
}

// T-12.6: RTM NFR-1 — file has empty frontmatter delimiters (`---\n---`) with
// no YAML keys. ReadMemory should handle this identically to no-frontmatter.
#[test]
fn read_memory_handles_empty_frontmatter_delimiters() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_raw_memory(
        &env,
        "memory-tools-tests/empty-frontmatter-test.md",
        "---\n---\n# Body\n\nContent with [[yaml-frontmatter]] after empty delimiters.\n",
    );

    let read_result = read_memory_output(&env, &uri);

    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("URI: memory://memory-tools-tests/empty-frontmatter-test"));
    assert!(read_result.contains("Content with [[yaml-frontmatter]] after empty delimiters."));
}

// T-12.6: RTM NFR-1
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

// T-12.6: RTM NFR-1
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
