# Test Evaluation: memory_lifecycle_validation

**File:** `crates/maenifold-core/tests/memory_lifecycle_validation.rs`
**Test count:** 3
**Traceability:** T-15.5 / RTM FR-25.2 (legacy sub-item — see Traceability note below)
**Evaluated:** 2025-07-16

---

## Summary

This three-test file guards the `MoveMemory` empty/whitespace destination validation introduced in
sprint T-15.5. The file's premise is sound — this validation path needs regression coverage — but
two of the three tests are **currently failing** due to a fundamental surface mismatch: the
`move_memory_output` test helper calls `.expect("move memory")` unconditionally, which **panics**
when `move_memory()` returns `Err(...)`. The implementation correctly returns `Err(MaenifoldError::Message(...))`
for invalid input, but the helper was written for the happy path only. Neither broken test ever
reaches its `assert!` statements.

Running the suite confirms: `1 passed; 2 failed`. Tests 1 and 2 fail with:

```
thread '...' panicked at crates/maenifold-core/tests/memory_tools/support.rs:141:6:
move memory: Message("Destination parameter is required and cannot be empty.")
```

The third test (valid destination) passes but is structurally redundant with
`move_memory_with_simple_destination_preserves_existing_behavior` in
`memory_tools_move_read_write.rs`, which additionally verifies a read-back. Its only
unique contribution is a brief regression comment — confirmable with a one-line annotation
on the existing test rather than a standalone test.

**Additionally:** the traceability tag `T-15.5: RTM FR-25.2` conflicts with the current RTM,
where FR-25 tracks Azure Deployment Manifests (RTM-137–140). No sub-item FR-25.2 exists in
the active RTM. This is a stale label from a prior numbering scheme and should be updated.

---

## Verdicts

| Test Function | Load-bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `move_memory_empty_destination_returns_error` | Yes — if it worked | Yes (empty `""`) | **No — panics before asserts run** | Yes | Yes (with test 2) | **REWRITE** |
| `move_memory_whitespace_only_destination_returns_error` | Yes — if it worked | Yes (`"   "`) | **No — panics before asserts run** | Yes | Yes (with test 1) | **REWRITE** → also **MERGE** with test 1 |
| `move_memory_valid_destination_succeeds` | Marginal | No — duplicates `memory_tools_move_read_write.rs` | Yes (passes correctly) | Yes | N/A | **REMOVE** (or absorb into sibling file) |

---

## Detailed Findings

### Finding 1 — Tests 1 & 2: Wrong call surface causes panic before any assertion

**Severity: Critical (tests are broken and provide zero coverage)**

`move_memory_output` (support.rs line 133–141) is:

```rust
pub fn move_memory_output(env: &TestEnv, source: &str, destination: &str) -> String {
    memory::move_memory(
        &env.ctx(),
        &json!({ "source": source, "destination": destination }),
    )
    .expect("move memory")   // ← panics if Err
}
```

The production function at `lifecycle.rs:89–92` returns:

```rust
if destination.trim().is_empty() {
    return Err(MaenifoldError::Message(
        "Destination parameter is required and cannot be empty.".to_string(),
    ));
}
```

`Err(...)` + `.expect()` = **panic**. The test thread dies at the helper; the `assert!(result.contains("ERROR"))` line is never reached. The test "passes" the assertion by never getting to it — it fails the harness instead.

This is structurally different from other error returns in the same function, which use
`Ok(format!("ERROR: ..."))` for lower-severity checks (e.g., duplicate destination).
The empty-destination guard uses the `Err` path as it should — it's an input validation
failure, not a domain logic error — but the test helper was written for the `Ok(...)` path only.

**Required fix:** Call `memory::move_memory()` directly and inspect the `Err` variant, or add a
`move_memory_result()` helper that returns `Result<String>`. Do not change the implementation.

**Correct rewrite (both tests merged into one parameterized set):**

```rust
use maenifold_core::memory;
use serde_json::json;

#[test]
fn move_memory_rejects_empty_and_whitespace_destinations() {
    let _guard = TEST_MUTEX.lock();
    for (label, dest) in [("empty", ""), ("whitespace-only", "   ")] {
        let env = isolated_env();
        let uri = write_memory_file(&env, "Dest Validation Test",
            "# Test\n\nContent with [[concepts]].", None);

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
```

---

### Finding 2 — Test 3: Passes but is redundant and weaker than an existing test

**Severity: Low (test passes, but does not pull its own weight)**

`move_memory_valid_destination_succeeds` asserts only:
1. The move output starts with `"Moved memory FILE:"`.
2. The output contains the expected destination URI.

`memory_tools_move_read_write.rs::move_memory_with_simple_destination_preserves_existing_behavior`
asserts the same two things **plus** reads back the file and verifies its content. This file is
the authoritative regression guard for the simple-destination happy path.

The in-file comment reads:

> *"Regression check — valid simple destination still works after adding empty-destination
> validation. Uses non-nested path to avoid pre-existing canonicalize bug with nested destination
> directories."*

Two concerns:
1. If the sibling test in `memory_tools_move_read_write.rs` already covers the simple-destination
   path, this test adds no net regression value. It does not exercise the new validation code —
   it just goes past it.
2. The comment mentions a **pre-existing canonicalize bug** with nested destination directories.
   This bug is being avoided, not tested. That bug either deserves its own test or a linked
   issue. Silently routing around it in a regression guard is a coverage gap in disguise.

---

### Finding 3 — Stale traceability label

**Severity: Informational**

The file header reads `// T-15.5: RTM FR-25.2`. In the current RTM, FR-25 refers to Azure
Deployment Manifests (RTM-137–140). There is no FR-25.2 sub-item. The PRD notes that
"completed requirements FR-1 through FR-21" are in CHANGELOG.md, implying a renumbering occurred.
The `FR-25.2` label survives in `lifecycle.rs` as well — both need updating to reflect the
current RTM item that this validation actually satisfies (likely an NFR or a named tool contract,
not an Azure deployment requirement).

---

## Positive Observations

- **Names are correct**: all three function names clearly describe intent and follow the existing
  naming conventions in this codebase.
- **Isolation is correct**: each test creates its own `isolated_env()` and holds `TEST_MUTEX`.
- **The implementation under test is correct**: `lifecycle.rs` returns `Err(...)` for
  empty/whitespace destination — the right error type, the right message. The tests are broken;
  the production code is not.
- **Test 3 correctly avoids contaminating state**: the "non-nested path" choice is deliberate and
  prevents a known bug from masking the assertion. The bug awareness itself is good; it just
  needs to be surfaced rather than silently avoided.

---

## Recommendations

1. **Rewrite tests 1 & 2 into a single merged parameterized test** that calls `memory::move_memory()`
   directly and inspects the `Err` return. Use `.expect_err()`. Cover both `""` and `"   "` inputs
   in one loop. This gives clean, correct, non-redundant coverage of the validation guard.

2. **Remove test 3** (`move_memory_valid_destination_succeeds`) and add a one-line comment to
   `move_memory_with_simple_destination_preserves_existing_behavior` in
   `memory_tools_move_read_write.rs` noting it also serves as the T-15.5 regression guard.
   Alternatively, promote it to a full regression note in that file rather than maintaining a
   separate test.

3. **File or surface the canonicalize bug** referenced in the test 3 comment. Either add a
   `move_memory_nested_destination_*` test that demonstrates the failure and is marked
   `#[should_fail]` / `#[ignore]` until fixed, or open a tracked issue. Do not leave known
   bugs as invisible detours in test comments.

4. **Update traceability tags** in `memory_lifecycle_validation.rs` and `lifecycle.rs`. Replace
   `RTM FR-25.2` with the current RTM item that governs MoveMemory input validation. If no
   current RTM item exists, add one or annotate against the relevant NFR.

5. **Add a `move_memory_result()` helper to `support.rs`** returning `Result<String>` for use in
   future error-path tests. The current `move_memory_output()` helper is structurally unsuitable
   for any test that expects an `Err` variant, creating a pitfall for future contributors.

---

## Load-Bearing Score: 1/3

Only test 3 is currently passing and contributes regression value — albeit weaker value than an
already-existing test in a different file. Tests 1 and 2 are broken: they panic before any
assertions execute and provide zero coverage of the behavior they claim to guard. The validation
logic in `lifecycle.rs` is correct and load-bearing; these tests are not currently guarding it.
