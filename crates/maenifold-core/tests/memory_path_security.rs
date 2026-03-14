#[path = "memory_path_support/mod.rs"]
mod memory_path_support;

use maenifold_core::markdown;
use memory_path_support::{
    TEST_FOLDER, TEST_MUTEX, isolated_env, move_memory_output, read_memory_output,
    write_memory_output,
};

// T-0.1: RTM NFR-3 — Split path-security coverage into a focused integration test file.
// T-W1-4: RTM NFR-2 — Path traversal rejection parity with the C# oracle.

/// Test 1 (REWRITE): Verify write_memory rejects all dot-sequence traversal vectors.
/// Strengthened assertion pins to the specific traversal error message from
/// validate_relative_path ("dot sequences" / "Path traversal") instead of the
/// overly-broad `starts_with("ERROR:")` that would pass on any unrelated error.
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

        // T-0.1: RTM NFR-2 — Pin to traversal-specific error from validate_relative_path
        assert!(
            result.contains("dot sequences") || result.contains("Path traversal"),
            "path traversal should be rejected with traversal error for {malicious_folder}: {result}"
        );
    }
}

/// Test 2 (REWRITE): Verify read_memory rejects URI-based traversal vectors.
/// Removed tautological `contains("escape")` and overly-broad `contains("Invalid URI")`
/// fallbacks. Normalizes Ok/Err to a single message string and checks for the specific
/// "path traversal" signal from validate_uri in markdown/paths.rs.
#[test]
fn read_memory_rejects_path_traversal_attempts() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside-memory",
        "memory://../../etc/hosts",
        "memory://folder/../../../system-file",
    ] {
        // T-W1-4: RTM NFR-2 — Normalize Ok("ERROR: ...")/Err(...) into a single message
        let error_msg = read_memory_output(&env, malicious_uri)
            .map_or_else(|e| e.to_string(), |s| s);

        assert!(
            error_msg.to_lowercase().contains("path traversal"),
            "traversal URI should be rejected with traversal message for {malicious_uri}: {error_msg}"
        );
    }
}

/// Test 3 (REWRITE): Verify move_memory rejects traversal in destination path.
/// Fixed blue-team rejection: production returns lowercase "path traversal" but old
/// assertion checked for "Path traversal" (uppercase P). Removed tautological
/// `contains("escape")` that only passed because the test vector contained that
/// substring, and `contains("Invalid")` that would mask non-traversal errors.
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
        "setup: write_memory should succeed for valid path: {write_result}"
    );

    for malicious_destination in [
        "../outside-memory",
        "../../escape-folder/file",
        "memory-path-tests/../escape-test-folder/file",
    ] {
        // T-W1-4: RTM NFR-2, FR-29 (RTM-183) — Normalize Ok/Err and pin to traversal message
        let error_msg = move_memory_output(
            &env,
            "memory://memory-path-tests/valid-source",
            malicious_destination,
        )
        .map_or_else(|e| e.to_string(), |s| s);

        assert!(
            error_msg.to_lowercase().contains("path traversal"),
            "path traversal in destination should be rejected for '{malicious_destination}': {error_msg}"
        );
    }
}

/// Test 4 (REWRITE): Verify markdown::uri_to_path rejects dotdot segment traversal.
/// Renamed from `uri_to_path_directly_rejects_path_traversal` to distinguish from
/// util-level `memory_uri_to_path` tests in memory_boundary_paths.rs.
/// Removed tautological `contains("escape")` and overly-broad `contains("Invalid URI")`.
#[test]
fn markdown_uri_to_path_rejects_dotdot_segment_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside",
        "memory://../../system",
        "memory://folder/../../../escape",
    ] {
        let err = markdown::uri_to_path(malicious_uri, &env.cfg.memory_path)
            .expect_err("uri_to_path should reject dotdot traversal");
        // T-0.1: RTM NFR-2 — Pin to traversal-specific error from validate_uri
        assert!(
            err.to_string().to_lowercase().contains("path traversal"),
            "expected traversal rejection for {malicious_uri}, got: {err}"
        );
    }
}

/// Test 5 (NEW): Positive control — verify valid paths are NOT rejected.
/// Without this, an over-restrictive validator (e.g. always returning Err) would
/// leave all negative tests green while breaking real operations.
#[test]
fn memory_security_tools_accept_valid_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // T-0.1: RTM NFR-2 — Positive control: valid write succeeds
    let result = write_memory_output(
        &env,
        "Valid Path Test",
        "Content with [[concept]] for positive control.",
        Some(TEST_FOLDER),
    );
    assert!(
        result.starts_with("Created memory FILE:"),
        "valid write should succeed: {result}"
    );

    // T-0.1: RTM NFR-2 — Positive control: valid read succeeds
    let write_uri = format!("memory://{TEST_FOLDER}/valid-path-test");
    let read_result = read_memory_output(&env, &write_uri).expect("valid read should succeed");
    assert!(
        !read_result.starts_with("ERROR:"),
        "valid read should not return error: {read_result}"
    );
}
