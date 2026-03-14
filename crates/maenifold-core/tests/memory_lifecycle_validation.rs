// T-15.5: RTM NFR-1 — validates MoveMemory rejects empty/whitespace
// destinations via the Err path in lifecycle.rs (legacy FR-25.2, renumbered).
// Valid-destination regression covered by memory_tools_move_read_write.rs.

#[path = "memory_tools/support.rs"]
mod support;

use maenifold_core::memory;
use serde_json::json;
use support::{TEST_MUTEX, isolated_env, write_memory_file};

/// T-15.5: RTM NFR-1 — move_memory() returns Err for empty and whitespace-only
/// destinations. Covers both "" and "   " in a single parameterized loop.
/// Previous tests panicked because the `move_memory_output` helper called
/// `.expect()` on an `Err` variant; this test calls the function directly and
/// inspects the `Err` with `.expect_err()`.
#[test]
fn move_memory_rejects_empty_and_whitespace_destinations() {
    let _guard = TEST_MUTEX.lock();
    for (label, dest) in [("empty", ""), ("whitespace-only", "   ")] {
        let env = isolated_env();
        let uri = write_memory_file(
            &env,
            "Dest Validation Test",
            "# Test\n\nContent with [[concepts]].",
            None,
        );

        let err = memory::move_memory(
            &env.ctx(),
            &json!({ "source": uri, "destination": dest }),
        )
        .expect_err(&format!("expected error for {label} destination"));

        assert!(
            err.to_string().contains("cannot be empty"),
            "{label} destination: expected 'cannot be empty', got: {err}"
        );
    }
}
