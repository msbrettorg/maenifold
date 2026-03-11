#[path = "memory_path_support/mod.rs"]
mod memory_path_support;

use maenifold_core::markdown;
use memory_path_support::{
    TEST_FOLDER, TEST_MUTEX, isolated_env, move_memory_output, read_memory_output,
    write_memory_output,
};

// T-0.1: RTM NFR-3 — Split path-security coverage into a focused integration test file.
// T-W1-4: RTM NFR-2 — Path traversal rejection parity with the C# oracle.
#[test]
fn write_memory_rejects_path_traversal_attempts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_folder in [
        "../outside-memory",
        "../../etc",
        "folder/../../../escape",
        "valid/../../invalid",
        "memory-path-tests/../escape-test-folder",
    ] {
        let result = write_memory_output(
            &env,
            "Test Title",
            "Content with [[required concept]] for testing.",
            Some(malicious_folder),
        );

        assert!(
            result.starts_with("ERROR:"),
            "path traversal should be rejected for folder {malicious_folder}: {result}"
        );
    }
}

#[test]
fn read_memory_rejects_path_traversal_attempts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside-memory",
        "memory://../../etc/hosts",
        "memory://folder/../../../system-file",
    ] {
        match read_memory_output(&env, malicious_uri) {
            Ok(result) => assert!(
                result.starts_with("ERROR:"),
                "path traversal read should be rejected for {malicious_uri}: {result}"
            ),
            Err(err) => assert!(
                err.to_string().contains("Path traversal")
                    || err.to_string().contains("path traversal")
                    || err.to_string().contains("Invalid URI")
                    || err.to_string().contains("escape"),
                "unexpected error for {malicious_uri}: {err}"
            ),
        }
    }
}

#[test]
fn move_memory_rejects_path_traversal_in_destination() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let write_result = write_memory_output(
        &env,
        "Valid Source",
        "Content to move with [[required concept]] for validation.",
        Some(TEST_FOLDER),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "unexpected: {write_result}"
    );

    for malicious_destination in [
        "../outside-memory",
        "../../escape-folder/file",
        "memory-path-tests/../escape-test-folder/file",
    ] {
        match move_memory_output(
            &env,
            "memory://memory-path-tests/valid-source",
            malicious_destination,
        ) {
            Ok(result) => assert!(
                result.starts_with("ERROR:")
                    || result.contains("Invalid")
                    || result.contains("escape"),
                "path traversal move should be rejected for {malicious_destination}: {result}"
            ),
            Err(err) => assert!(
                err.to_string().contains("Path traversal")
                    || err.to_string().contains("Invalid")
                    || err.to_string().contains("escape"),
                "unexpected error for {malicious_destination}: {err}"
            ),
        }
    }
}

#[test]
fn uri_to_path_directly_rejects_path_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside",
        "memory://../../system",
        "memory://folder/../../../escape",
    ] {
        let err = markdown::uri_to_path(malicious_uri, &env.cfg.memory_path)
            .expect_err("uri_to_path should reject traversal");
        assert!(
            err.to_string().contains("Path traversal")
                || err.to_string().contains("path traversal")
                || err.to_string().contains("Invalid URI")
                || err.to_string().contains("escape"),
            "unexpected error for {malicious_uri}: {err}"
        );
    }
}
