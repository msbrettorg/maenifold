# Test Evaluation: `incremental_sync_tools_watcher.rs`

**Traceability**: T-12.7 · RTM NFR-1  
**Evaluator**: Red Team (Gray — Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/incremental_sync_tools_watcher.rs`  
**Tests evaluated**: 3

---

## Summary

This file is a focused lifecycle test suite for the file-watcher component, split from a
larger incremental-sync test module during sprint T-12.7. It contains exactly three tests:
one locking the watcher error-format contract, one exercising the out-of-memory-root path
guard, and one walking the full start/stop lifecycle state machine.

The file is lean and purposeful, but it carries several quality deficits that deserve
remediation: unnecessary global mutex usage on a pure function, an under-asserted boundary
test that does not verify database state, and a compound lifecycle test whose failure modes
are hard to isolate. Additionally, none of the three tests actually compare output against
the C# oracle despite the file being tagged NFR-1 (C# oracle parity). This is not a
correctness flaw — the functions are `#[doc(hidden)]` test seams, not tool handlers — but it
raises the question of whether the NFR-1 tag is being applied too broadly as a catch-all.

---

## Test Verdicts

| # | Test Name | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|-----------|:------------:|:------:|:-----------------:|:---------:|:---------------:|---------|
| 1 | `watcher_errors_are_formatted_with_expected_prefix` | Partial | Yes | Partial | Yes | Yes | **REWRITE** |
| 2 | `process_file_created_with_path_outside_memory_path_does_not_throw` | Yes | Yes | No | No | Partial | **REWRITE** |
| 3 | `start_watcher_then_stop_watcher_lifecycle_returns_expected_messages` | Yes | Yes | Partial | Yes | No | **KEEP** |

---

## Finding 1 — `watcher_errors_are_formatted_with_expected_prefix`

### What it tests

Calls the public-for-test helper `format_watcher_error_message("Test watcher error")` and
asserts the result equals `"[FileWatcher] Error: Test watcher error"`.

### Critique

**Unnecessary mutex acquisition.** `format_watcher_error_message` is a pure, stateless
function — `format!("[FileWatcher] Error: {error}")` — with no access to global state, no
side effects, and no thread-shared resources. Taking `TEST_MUTEX` here provides zero
protection and adds contention against the lifecycle test (T3) which does legitimately need
the mutex.

**Shallow contract, not exercised at boundaries.** The test only passes one nominal input.
The source comment explicitly states the purpose of this function is to keep the watcher
error text stable for parity-style assertions. Yet the test never probes boundary inputs:
empty string, a string containing `Error:` already, or a very long message. If the format
string were changed to `"[Watcher] Error: {error}"`, this test catches it — but only
because of the exact-match assert, not because the test exercises the contract thoroughly.

**NFR-1 tag is loose.** The file header claims `RTM NFR-1` (C# oracle parity), but this
function has no C# oracle counterpart to compare against. The tag here means "don't regress
the string," which is a weaker, non-parity claim. This is misleading traceability.

**Verdict: REWRITE**

Remove the `TEST_MUTEX` acquisition. Add two or three parameterized cases (empty input,
input with embedded error prefix, long input). Clarify that this is a format-contract lock
test, not an NFR-1 parity test.

```rust
// Proposed replacement
#[test]
fn watcher_error_message_format_contract() {
    assert_eq!(
        format_watcher_error_message("disk full"),
        "[FileWatcher] Error: disk full"
    );
    assert_eq!(
        format_watcher_error_message(""),
        "[FileWatcher] Error: "
    );
    assert_eq!(
        format_watcher_error_message("Error: nested"),
        "[FileWatcher] Error: Error: nested"
    );
}
```

---

## Finding 2 — `process_file_created_with_path_outside_memory_path_does_not_throw`

### What it tests

Writes a `.md` file outside the configured `memory_path`, calls `process_path_for_test`,
and asserts `result.is_ok()`.

### Critique

**Assertion gap: does not verify the file was not indexed.** The source implementation of
`process_path_for_test` is:

```rust
pub fn process_path_for_test(context: &ToolContext, path: &Path) -> Result<()> {
    if path_to_memory_uri(&context.config, path)?.is_none() {
        return Ok(());   // ← early return, no indexing
    }
    crate::sync::sync_single_file(context, path)?;
    Ok(())
}
```

`result.is_ok()` passes even if the guard clause were accidentally removed and `Ok(())` were
returned after a failed write. The test asserts survivability but not correctness. The
critical contract — that the out-of-scope file was **not inserted into `file_content`** —
goes untested. A regression that silently indexed out-of-memory-root files would not be
caught here.

**Non-idiomatic test name.** `does_not_throw` is a Java/JUnit convention. Rust's idiomatic
phrasing is `returns_ok` or `is_silently_ignored`. This is cosmetic but signals C# muscle
memory bleeding into the naming.

**Missing sibling cases.** The current test covers one scenario: a path on a different temp
filesystem. It does not cover:

- A path that is a *sibling* of `memory_path` (same parent directory, not a child)
- A path using `../` to escape the memory root

**Verdict: REWRITE**

Strengthen the assertion to verify database state is unchanged:

```rust
#[test]
fn process_path_outside_memory_root_is_silently_ignored() {
    let _guard = TEST_MUTEX.lock();
    let _reset = ConfigOverrideReset;
    let (_tmp, cfg) = isolated_config();
    let ctx = context(&cfg);

    let outside_dir = tempfile::TempDir::new().expect("outside tmp");
    let outside_path = outside_dir.path().join("outside-memory.md");
    std::fs::write(&outside_path, "# Outside\n\n[[some-concept]]\n").expect("write");

    let result = process_path_for_test(&ctx, &outside_path);

    assert!(result.is_ok(), "should return Ok without error: {result:?}");

    // Verify the file was NOT indexed
    let conn = db_connection(&cfg);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM file_content", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 0, "out-of-memory-root file must not be indexed");
}
```

---

## Finding 3 — `start_watcher_then_stop_watcher_lifecycle_returns_expected_messages`

### What it tests

Exercises the complete start/stop lifecycle in sequence:

1. Pre-condition teardown (`stop_watcher` to ensure clean state)
2. Start watcher → assert success message contains "Started watching" and memory path
3. Start watcher again → assert "Watcher already running" (idempotency)
4. Assert `watcher_is_running()` is `true`
5. Stop watcher → assert "Stopped incremental sync watcher"
6. Stop watcher again → assert "Watcher not running"
7. Assert `watcher_is_running()` is `false`

### Critique

**Compound test structure hinders isolation.** This test covers four distinct sub-behaviors.
If the idempotency check (step 3) panics, the test aborts before verifying stop behavior.
Splitting into `watcher_start_succeeds`, `watcher_start_is_idempotent`,
`watcher_stop_succeeds`, `watcher_stop_is_idempotent` would let failures be pinpointed
immediately. The cost of setup overhead is low since `isolated_config()` is fast.

**String coupling is brittle but tolerable.** The exact message strings are asserted. While
this creates coupling to human-readable text, the source comment explicitly says "keep watcher
error text stable for parity-style assertions," and the stop function is a simple
two-branch return. This is an acceptable tradeoff. The `watcher_is_running()` state
assertions are the more meaningful checks and correctly validate state, not just messages.

**`watcher_is_running()` not asserted after initial start.** The test calls
`watcher_is_running()` after the idempotency check, but never immediately after the first
successful start. A bug where the watcher starts but fails to register in `WATCHER` would
not be caught until the idempotency check (which would return `Ok("Started watching...")`
instead of `"Watcher already running"`). Adding `assert!(watcher_is_running())` after the
first start closes this gap.

**Missing: the start message includes debounce.** The format string is:
`"Started watching {path} for incremental sync (debounce: {debounce_ms}ms)"`.
The test asserts `contains("Started watching")` and `contains(path)`, but does not assert
the debounce value from the payload (`{"debounceMs": 250}`). This is acceptable given the
test's focus is lifecycle, not message content, but worth noting.

**Verdict: KEEP** — with the following targeted improvements:

1. Add `assert!(watcher_is_running(), "watcher should be running after start");` after the
   first start call (before the idempotency check).
2. Consider extracting to four focused tests if this file grows; the current size makes the
   compound form defensible.

---

## Recommendations

### Priority 1 — Correctness Gap (Test 2)
The missing database-state assertion in
`process_file_created_with_path_outside_memory_path_does_not_throw` is the only finding
that represents a genuine regression risk. A future refactor removing the early-return guard
from `process_path_for_test` would not be caught by the current assertion. Rewrite this
test to assert zero `file_content` rows post-call.

### Priority 2 — Unnecessary Lock Contention (Test 1)
Remove `TEST_MUTEX` from the error-format test. It is a pure-function call with no shared
state. The mutex exists for tests that touch `WATCHER` (the global `Lazy<Mutex<Option<WatcherState>>>`).
Holding the mutex unnecessarily forces T1 and T3 to serialize even when they could run
concurrently under `cargo test`.

### Priority 3 — Naming Convention (Test 2)
Rename `...does_not_throw` → `...is_silently_ignored`. Adopt Rust idioms throughout.

### Priority 4 — State Assertion Gap (Test 3)
Add `assert!(watcher_is_running())` immediately after the first `start_watcher` call to
close the gap where a failed-but-ok-returning start would not be detected until step 3.

### Priority 5 — Parameterize the Format Contract (Test 1)
Expand to three input cases: nominal, empty, and an input that already contains the prefix.
This converts a single brittle equals-check into a meaningful contract specification.

### Not Recommended
- **Merging tests**: the three tests cover distinct concerns (format contract, boundary
  guard, lifecycle). Merging would reduce isolation without benefit.
- **Removing the format contract test**: the source marks `format_watcher_error_message` as
  a parity seam explicitly; keeping a contract-lock test is correct, it just needs the mutex
  removed and the inputs broadened.

---

## Load-Bearing Score: 2/3

| Test | Load-Bearing? | Rationale |
|------|:---:|---|
| `watcher_errors_are_formatted_with_expected_prefix` | **Partial** | Locks the error-format contract used in parity assertions, but the tested function is a one-liner with no RTM requirement mapping to its exact format. A rename of the prefix is caught; a behavioral regression is not. |
| `process_file_created_with_path_outside_memory_path_does_not_throw` | **Yes** | Covers an important boundary guard: out-of-memory-root paths must be silently ignored. However, the assertion only verifies survivability, not the absence of a side-effect, so its load-bearing value is partially undermined. |
| `start_watcher_then_stop_watcher_lifecycle_returns_expected_messages` | **Yes** | This is the most valuable test in the file. It is the only test exercising the global `WATCHER` state machine's idempotency contract in both directions. Without it, start/stop lifecycle regressions would be invisible. |

**2 out of 3 tests are genuinely load-bearing.** The format-contract test earns a partial —
it locks a contract that is explicitly marked stable in source, but the contract has no RTM
trace and the test infrastructure cost (mutex, setup) exceeds the risk it mitigates.

---

*Report generated under [[test-quality]] evaluation framework for [[maenifold-rs]]. References: T-12.7, RTM NFR-1, RTM FR-17.5 (T-COV-001.3).*

---
## SWE Remediation

- REWRITE `watcher_errors_are_formatted_with_expected_prefix` → `watcher_error_message_format_contract`: removed unnecessary TEST_MUTEX lock on pure function (P2); parameterized across 3 inputs — nominal, empty, nested-prefix (P5)
- REWRITE `process_file_created_with_path_outside_memory_path_does_not_throw` → `process_path_outside_memory_root_is_silently_ignored`: added database-state assertion verifying zero `file_content` rows after processing out-of-scope path (P1); renamed to Rust-idiomatic convention (P3)
- KEEP `start_watcher_then_stop_watcher_lifecycle_returns_expected_messages`: added `assert!(watcher_is_running())` immediately after first `start_watcher` call to close state-assertion gap (P4)
- All 3 tests pass: `cargo test --test integration_tests -p maenifold-core -- incremental_sync_tools_watcher`
- Commit: 55f0a46

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_tools_watcher`: 3 passed, 0 failed
- Verdict: PASS
