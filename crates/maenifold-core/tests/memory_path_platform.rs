#[path = "memory_path_support/mod.rs"]
mod memory_path_support;

use maenifold_core::markdown;
use memory_path_support::{
    TEST_FOLDER, TEST_MUTEX, isolated_env, read_memory_output, write_memory_output,
};
use std::fs;

// T-0.1: RTM NFR-3 — Split cross-platform path conversion coverage into a focused integration test file.
// T-W1-4: RTM NFR-2 — Cross-platform path conversion parity with the C# oracle.
#[test]
fn path_handling_works_with_unix_style_separators() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let unix_style_folder = format!("{TEST_FOLDER}/unix/style");

    let write_result = write_memory_output(
        &env,
        "Unix Style Path",
        "Testing [[unix paths]] with forward slashes in nested directories.",
        Some(&unix_style_folder),
    );

    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );
    assert!(write_result.contains("memory://memory-path-tests/unix/style/unix-style-path"));

    let read_result = read_memory_output(
        &env,
        "memory://memory-path-tests/unix/style/unix-style-path",
    )
    .expect("read unix path memory");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(
        read_result.contains("Testing [[unix paths]] with forward slashes in nested directories.")
    );

    let expected_dir = env.test_folder_path().join("unix").join("style");
    assert!(
        expected_dir.exists(),
        "directory should exist with correct platform separators"
    );
}

#[test]
fn path_to_uri_produces_consistent_uri_format() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let test_cases = [
        env.test_folder_path().join("simple.md"),
        env.test_folder_path().join("nested").join("file.md"),
        env.test_folder_path()
            .join("deep")
            .join("nesting")
            .join("levels")
            .join("file.md"),
    ];

    for test_path in test_cases {
        if let Some(dir) = test_path.parent() {
            fs::create_dir_all(dir).expect("create test dirs");
        }
        fs::write(&test_path, "# Test File\n\nContent").expect("write fixture");

        let uri = markdown::path_to_uri(&test_path, &env.cfg.memory_path).expect("path to uri");

        assert!(uri.starts_with("memory://"));
        assert!(!uri.contains('\\'), "URI should not contain backslashes");
        assert!(
            !uri.ends_with(".md"),
            "URI should not include .md extension"
        );

        let converted_path =
            markdown::uri_to_path(&uri, &env.cfg.memory_path).expect("uri to path");
        assert!(
            converted_path.exists(),
            "converted path should exist: {:?}",
            converted_path
        );
    }
}

#[test]
fn uri_to_path_handles_platform_specific_separators() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for uri in [
        "memory://memory-path-tests/simple-file",
        "memory://memory-path-tests/nested/subdirectory/file",
        "memory://memory-path-tests/deep/multi/level/nesting/file",
    ] {
        let path = markdown::uri_to_path(uri, &env.cfg.memory_path).expect("uri to path");

        if cfg!(windows) && uri.contains('/') {
            let rendered = path.to_string_lossy();
            assert!(
                rendered.contains('\\') || !uri.contains('/'),
                "windows paths should use backslashes for nested directories: {rendered}"
            );
        }

        assert!(
            path.starts_with(&env.cfg.memory_path),
            "converted path should stay within memory directory"
        );
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("md"));
    }
}

#[test]
fn file_operations_work_with_mixed_path_styles() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let platform_path = env
        .test_folder_path()
        .join("mixed")
        .join("platform")
        .join("file.md");

    if let Some(dir) = platform_path.parent() {
        fs::create_dir_all(dir).expect("create mixed path dirs");
    }

    let content = "# Mixed Path Test\n\nTesting [[cross platform]] compatibility.";
    fs::write(&platform_path, content).expect("write mixed path file");

    let uri = markdown::path_to_uri(&platform_path, &env.cfg.memory_path).expect("path to uri");
    let converted_path = markdown::uri_to_path(&uri, &env.cfg.memory_path).expect("uri to path");

    let original_full = platform_path
        .canonicalize()
        .unwrap_or_else(|_| platform_path.clone());
    let converted_full = converted_path
        .canonicalize()
        .unwrap_or_else(|_| converted_path.clone());
    assert_eq!(
        converted_full, original_full,
        "round-trip conversion should preserve path identity"
    );

    let read_result = read_memory_output(&env, &uri).expect("read mixed path file");
    assert!(
        !read_result.starts_with("ERROR:"),
        "unexpected: {read_result}"
    );
    assert!(read_result.contains("Testing [[cross platform]] compatibility."));
}
