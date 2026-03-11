// T-15.5: RTM FR-25.2 — validates MoveMemory rejects empty/whitespace
// destinations and that valid moves still succeed (regression check).

#[path = "memory_tools/support.rs"]
mod support;

use support::{TEST_MUTEX, isolated_env, move_memory_output, write_memory_file};

#[test]
fn move_memory_empty_destination_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Empty Dest Test",
        "# Empty Dest Test\n\nContent with [[concepts]].",
        None,
    );

    let result = move_memory_output(&env, &uri, "");

    assert!(
        result.contains("ERROR"),
        "expected error for empty destination, got: {result}"
    );
    assert!(
        result.contains("cannot be empty"),
        "expected 'cannot be empty' message, got: {result}"
    );
}

#[test]
fn move_memory_whitespace_only_destination_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Whitespace Dest Test",
        "# Whitespace Dest Test\n\nContent with [[concepts]].",
        None,
    );

    let result = move_memory_output(&env, &uri, "   ");

    assert!(
        result.contains("ERROR"),
        "expected error for whitespace-only destination, got: {result}"
    );
    assert!(
        result.contains("cannot be empty"),
        "expected 'cannot be empty' message, got: {result}"
    );
}

#[test]
fn move_memory_valid_destination_succeeds() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let uri = write_memory_file(
        &env,
        "Valid Move Test",
        "# Valid Move Test\n\nContent with [[concepts]].",
        None,
    );

    // T-15.5: Regression check — valid simple destination still works after
    // adding empty-destination validation. Uses non-nested path to avoid
    // pre-existing canonicalize bug with nested destination directories.
    let result = move_memory_output(&env, &uri, "Moved Valid Test");

    assert!(
        result.starts_with("Moved memory FILE:"),
        "expected successful move, got: {result}"
    );
    assert!(
        result.contains("memory://moved-valid-test"),
        "expected destination URI in result, got: {result}"
    );
}
