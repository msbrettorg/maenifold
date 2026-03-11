#[path = "memory_path_support/mod.rs"]
mod memory_path_support;

use memory_path_support::{
    TEST_FOLDER, TEST_MUTEX, extract_concepts_output, isolated_env, move_memory_output,
    read_memory_output, write_memory_output,
};

// T-0.1: RTM NFR-3 — Split nested-path coverage into a focused integration test file.
// T-W1-4: RTM NFR-2 — Ported MemoryPathTests.cs coverage for nested memory paths and URI/path conversion.
#[test]
fn write_memory_creates_nested_directories_automatically() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let nested_folder = format!("{TEST_FOLDER}/level1/level2/level3");

    let result = write_memory_output(
        &env,
        "Deep Nested Test",
        "Testing [[nested directory]] creation in deep folders.",
        Some(&nested_folder),
    );

    assert!(
        result.starts_with("Created memory FILE:"),
        "unexpected: {result}"
    );
    assert!(result.contains("memory://memory-path-tests/level1/level2/level3/deep-nested-test"));

    let expected_dir = env
        .test_folder_path()
        .join("level1")
        .join("level2")
        .join("level3");
    assert!(
        expected_dir.exists(),
        "nested directories should be created"
    );

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/level1/level2/level3/deep-nested-test",
    )
    .expect("read nested memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[nested directory]] creation in deep folders."));
}

#[test]
fn move_memory_creates_nested_destination_directories() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let write_result = write_memory_output(
        &env,
        "Move Source Test",
        "Testing [[move to nested]] directory functionality.",
        Some(TEST_FOLDER),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let nested_destination = format!("{TEST_FOLDER}/categories/subcategory/moved-file");
    let move_result = move_memory_output(
        &env,
        "memory://memory-path-tests/move-source-test",
        &nested_destination,
    )
    .expect("move memory");

    assert!(
        move_result.starts_with("Moved memory FILE:"),
        "unexpected: {move_result}"
    );
    assert!(move_result.contains("memory://memory-path-tests/categories/subcategory/moved-file"));

    let expected_dir = env
        .test_folder_path()
        .join("categories")
        .join("subcategory");
    assert!(
        expected_dir.exists(),
        "nested destination directories should be created"
    );

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/categories/subcategory/moved-file",
    )
    .expect("read moved memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[move to nested]] directory functionality."));
}

#[test]
fn extract_concepts_from_file_works_with_nested_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let nested_folder = format!("{TEST_FOLDER}/concepts/nested");

    let write_result = write_memory_output(
        &env,
        "Concept Test",
        "This contains [[machine learning]] and [[data science]] concepts for testing.",
        Some(&nested_folder),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let extract_result = extract_concepts_output(
        &env,
        "memory://memory-path-tests/concepts/nested/concept-test",
    )
    .expect("extract concepts");
    assert!(
        !extract_result.starts_with("ERROR:"),
        "unexpected: {extract_result}"
    );
    assert!(extract_result.contains("[[machine-learning]]"));
    assert!(extract_result.contains("[[data-science]]"));
}

#[test]
fn nested_paths_handle_special_characters() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for (title, folder, uri) in [
        (
            "File With Hyphens",
            "memory-path-tests/special-chars",
            "file-with-hyphens",
        ),
        (
            "File With Underscores",
            "memory-path-tests/special_chars",
            "file-with-underscores",
        ),
        (
            "File With Numbers456",
            "memory-path-tests/numbers123",
            "file-with-numbers456",
        ),
        (
            "Final File",
            "memory-path-tests/mixed-chars_123",
            "final-file",
        ),
    ] {
        let content = format!("Testing [[special characters]] in path: {folder}");
        let write_result = write_memory_output(&env, title, &content, Some(folder));

        assert!(
            write_result.starts_with("Created memory FILE:"),
            "special character path should work for {folder}: {write_result}"
        );

        let read_result = read_memory_output(&env, &format!("memory://{folder}/{uri}"))
            .expect("read special character path");
        assert!(
            !read_result.starts_with("ERROR:"),
            "reading special character path should work for {folder}: {read_result}"
        );
        assert!(read_result.contains(&content));
    }
}

#[test]
fn deep_nesting_limits_are_respected() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut deep_folder = TEST_FOLDER.to_string();
    for level in 1..=10 {
        deep_folder.push_str(&format!("/level{level}"));
    }

    let write_result = write_memory_output(
        &env,
        "Deep File",
        "Testing [[very deep]] nesting in directory structures.",
        Some(&deep_folder),
    );

    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    let read_result = read_memory_output(&env, &format!("memory://{deep_folder}/deep-file"))
        .expect("read deep nesting file");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[very deep]] nesting in directory structures."));
}

#[test]
fn empty_directory_names_are_handled_gracefully() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for problematic_folder in [
        "memory-path-tests//double-slash",
        "memory-path-tests/normal//double-middle",
        "memory-path-tests/trailing-slash//",
    ] {
        let result = write_memory_output(
            &env,
            "Empty Directory Test",
            "Testing [[empty segments]] in folder paths for validation.",
            Some(problematic_folder),
        );

        assert!(
            !result.contains("Exception"),
            "empty directory segments should be handled gracefully for {problematic_folder}: {result}"
        );
    }
}
