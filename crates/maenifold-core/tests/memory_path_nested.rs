#[allow(clippy::duplicate_mod)]
#[path = "memory_path_support/mod.rs"]
mod memory_path_support;

use maenifold_core::util::paths::slugify_title;
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

    // R4: verify no spurious concepts are extracted (e.g. from path segments).
    let concept_count = extract_result.matches("[[").count();
    assert_eq!(
        concept_count, 2,
        "expected exactly 2 concepts, got {concept_count}: {extract_result}"
    );
}

// T-0.1: RTM NFR-3 — Verify slug generation handles adversarial title characters.
// R1: expanded from slug-safe-only inputs to genuinely adversarial titles (unicode,
// colons, percent signs, dots, parentheses) and uses `slugify_title` to compute
// expected slugs dynamically rather than hardcoding them.
#[test]
fn nested_paths_handle_adversarial_title_characters() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for (title, subfolder) in [
        ("Café Notes", "unicode-test"),
        ("Topic: Subtopic", "colon-test"),
        ("100% Complete", "percent-test"),
        (".Hidden Title", "dot-test"),
        ("File (v2.1)", "parens-test"),
    ] {
        let folder = format!("{TEST_FOLDER}/{subfolder}");
        let content = format!("Testing [[slug generation]] for adversarial title: {title}");
        let expected_slug = slugify_title(title);

        let write_result = write_memory_output(&env, title, &content, Some(&folder));
        assert!(
            write_result.starts_with("Created memory FILE:"),
            "adversarial title '{title}' should succeed: {write_result}"
        );
        assert!(
            write_result.contains(&expected_slug),
            "result should contain dynamically slugified '{expected_slug}' for '{title}': {write_result}"
        );

        let uri = format!("memory://{folder}/{expected_slug}");
        let read_result = read_memory_output(&env, &uri).expect("read adversarial title path");
        assert!(
            !read_result.starts_with("ERROR:"),
            "reading adversarial title '{title}' should succeed: {read_result}"
        );
        assert!(
            read_result.contains(&content),
            "round-trip content mismatch for '{title}': {read_result}"
        );
    }
}

// T-0.1: RTM NFR-3 — Verify no OS path-length limit or depth guard is hit at
// the probe depth. If the system ever imposes a depth limit, a companion test
// should verify rejection above that limit.

/// Empirically validates that no OS path-length limit or implementation depth
/// guard prevents writes at this many nested levels on supported platforms.
const DEEP_NESTING_PROBE_DEPTH: usize = 10;

#[test]
fn deep_nesting_succeeds_at_ten_levels() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let mut deep_folder = TEST_FOLDER.to_string();
    for level in 1..=DEEP_NESTING_PROBE_DEPTH {
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

// T-0.1: RTM NFR-3 — Double-slash folder segments are silently normalised by
// the OS filesystem layer (create_dir_all / write collapse consecutive slashes).
// The test verifies that write_memory succeeds and the content survives a
// round-trip read despite the malformed folder path.
#[test]
fn empty_directory_segments_are_normalised_on_write() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let cases = [
        (
            "Leading Double Slash",
            format!("{TEST_FOLDER}//double-slash"),
            "Testing [[empty segments]] with leading double-slash.",
        ),
        (
            "Mid Path Double Slash",
            format!("{TEST_FOLDER}/normal//double-middle"),
            "Testing [[empty segments]] with mid-path double-slash.",
        ),
        (
            "Trailing Double Slash",
            format!("{TEST_FOLDER}/trailing-slash//"),
            "Testing [[empty segments]] with trailing double-slash.",
        ),
    ];

    for (title, folder, content) in &cases {
        let result = write_memory_output(&env, title, content, Some(folder));

        assert!(
            result.starts_with("Created memory FILE:"),
            "double-slash folder should be normalised and succeed for '{folder}': {result}"
        );

        // Round-trip: read back using the URI that write_memory produced.
        let expected_slug = slugify_title(title);
        let uri = format!("memory://{folder}/{expected_slug}");
        let read_result = read_memory_output(&env, &uri).expect("read double-slash path");
        assert!(
            !read_result.starts_with("ERROR:"),
            "round-trip read should succeed for '{folder}': {read_result}"
        );
        assert!(
            read_result.contains(content),
            "round-trip content mismatch for '{folder}': {read_result}"
        );
    }
}
