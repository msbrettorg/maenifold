---
id: vector_sync_diag_unavailable
status: done
---

# Test Evaluation: `vector_sync_diag_unavailable.rs`

> **Traceability**: T-9.6 | FR-7, FR-8 (module comment) / `fr25_2_ac3`, `fr25_2_ac4` (test names — see [Naming Inconsistency](#naming-inconsistency))
> **Evaluator lens**: 🔘 Gray — skeptical inquiry; demand evidence, question assumptions, surface vacuous assertions.

---

## Summary

`vector_sync_diag_unavailable.rs` contains **4 `#[test]` functions** that collectively verify the
structured diagnostic output emitted by the `Sync` tool and `sync_files` function when
`MAENIFOLD_DISABLE_VECTOR=true` is set. The file is the "unavailable-path" split of the broader
`vector_sync_diag_*` family, complementing `vector_sync_diag_reporting.rs` (which tests the unit
functions `build_vector_failure_report`, `build_vector_unavailable_report`, and
`generate_concept_embeddings` in isolation).

**Disposition in brief**: Tests 1 and 2 are genuinely load-bearing integration tests and should be
kept. Tests 3 and 4 are guardrail tests that produce *no unique positive assertions* — every
positive claim they make is already in tests 1 and 2 — and their only contribution (negative
assertions) should be merged up. Additionally, Test 1 carries one vacuous negative assertion against
a warning string that no longer exists anywhere in the production codebase.

### Naming Inconsistency

The module-level doc comment cites `T-9.6: RTM FR-7, FR-8`. The test names encode `fr25_2_ac3` /
`fr25_2_ac4`. FR-25 in the current RTM is **Azure Deployment Manifests**; FR-7 and FR-8 are
completed requirements archived in `CHANGELOG.md`. The prefix `fr25_2` appears to come from a
previous sprint's numbering scheme (confirmed by `T-QUAL-002.6b: RTM FR-25.2` comments in
`vector_diagnostics.rs`). This makes RTM traceability ambiguous — a reader cannot tell from the
test name alone which live requirement is being validated.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique Coverage? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `fr25_2_ac3_sync_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason` | ✅ Yes | ✅ Yes | ⚠️ Mostly — one vacuous negative assertion | ⚠️ Partially — `fr25_2` prefix is stale | ⚠️ Partially | **REWRITE** |
| 2 | `fr25_2_ac4_sync_files_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partially — `fr25_2` prefix is stale | ⚠️ Partially | **REWRITE** |
| 3 | `fr25_2_ac3_sync_vector_unavailable_does_not_emit_failure_or_verification_sections` | ⚠️ Marginal | ❌ No — positive assertions are a subset of test 1 | ✅ Negative assertions are correct | ❌ No — redundant `ac3` name | ❌ No (same entry point as test 1) | **MERGE → test 1** |
| 4 | `fr25_2_ac4_sync_files_vector_unavailable_does_not_emit_failure_or_verification_sections` | ⚠️ Marginal | ❌ No — positive assertions are a subset of test 2 | ✅ Negative assertions are correct | ❌ No — redundant `ac4` name | ❌ No (same entry point as test 2) | **MERGE → test 2** |

---

## Per-Test Detail

---

### Test 1 — `fr25_2_ac3_sync_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason`

**What it tests**: Invokes the full `Sync` tool via `ToolRegistry::global().try_invoke("Sync", …)`
with `MAENIFOLD_DISABLE_VECTOR=true`. Writes one fixture file containing one WikiLink
(`[[ac3-sync-concept]]`). Asserts the structured diagnostic block is present with correct counters
and reason code.

**Load-bearing?** ✅ Yes. This is the only test exercising the full `Sync` tool code path under the
unavailable condition (as opposed to calling unit functions directly). It validates that
`run_sync()` correctly branches into `build_vector_unavailable_report()` and the counts are
thread-safe end-to-end.

**Unique coverage?** ✅ Yes. The sibling file `vector_sync_diag_reporting.rs` tests
`build_vector_unavailable_report()` in isolation; this test proves the function is actually *called*
from the live `Sync` tool dispatch path. The counter assertion `concepts=1, memory=1, total=2`
specifically validates that `collect_vector_skip_diagnostics_for_paths` is invoked with the correct
file set.

**Correct assertions?** ⚠️ Mostly.

- The six positive `output.contains(…)` assertions are correct and meaningful.
- The assertion on line 59 is **vacuous**:
  ```rust
  assert!(
      !output.contains("- WARNING: sqlite-vec extension unavailable; semantic tooling disabled"),
      "unexpected output: {output}"
  );
  ```
  The string `"sqlite-vec extension unavailable; semantic tooling disabled"` does not appear
  anywhere in `crates/maenifold-core/src/` — it was removed in a prior refactor. This assertion
  **always passes trivially** and provides zero regression protection. It is dead test code.

**Good name?** ⚠️ Partially. The test name is descriptive of behavior but the `fr25_2_ac3` prefix
encodes a stale RTM reference. Consider `sync_tool_vector_disabled_emits_structured_diagnostics_with_skipped_counts_and_reason`.

**Parameterizable?** ⚠️ Partially. Tests 1 and 2 are structurally parallel (same assertion
pattern, different entry point and fixture count). They can't be trivially macro-parameterized
because the entry points have different types (`ToolRegistry` invocation vs. direct `sync_files`
call), but a shared assertion helper could reduce duplication.

**Verdict: REWRITE**
1. Absorb the two negative assertions from test 3 (`!output.contains("- FAILURES:")` and
   `!output.contains("Vector index verification:")`).
2. Remove the vacuous `sqlite-vec extension unavailable` negative assertion.
3. Rename to drop the stale `fr25_2` prefix.

---

### Test 2 — `fr25_2_ac4_sync_files_vector_unavailable_emits_structured_diagnostics_with_skipped_counts_and_reason`

**What it tests**: Invokes `sync_files(&context, &[file_path_a, file_path_b])` directly with
`MAENIFOLD_DISABLE_VECTOR=true`. Two fixture files, one unique WikiLink each. Asserts the structured
diagnostic block with `concepts=2, memory=2, total=4`.

**Load-bearing?** ✅ Yes. `sync_files` is a distinct code path from the full `Sync` tool
(notably: no community detection, no FTS sync, no leader election). This test proves the
`vector_sync_disabled_by_config()` branch inside `sync_files` (lines 263–271 of `sync/mod.rs`) is
exercised and correctly accumulates per-file skip counts.

**Unique coverage?** ✅ Yes. The distinct entry point justifies this as a separate test from test 1.
The counter `concepts=2, memory=2, total=4` (rather than `1,1,2`) also validates that concept
deduplication within `collect_vector_skip_diagnostics_for_paths` works correctly across multiple
files — two files, two *distinct* WikiLinks, so no deduplication occurs here.

**Correct assertions?** ✅ Yes. All six `output.contains(…)` assertions are semantically correct
and anchored to observable behavior.

**Good name?** ⚠️ Partially. Same stale `fr25_2` prefix issue. Consider
`sync_files_vector_disabled_emits_structured_diagnostics_with_skipped_counts_and_reason`.

**Parameterizable?** ⚠️ Partially. As with test 1, a shared assertion helper extracting the
diagnostic block check would reduce duplication. A future enhancement: parameterize the fixture
count and assert `concepts=N, memory=N` to prove the counter scales correctly.

**Verdict: REWRITE**
1. Absorb the two negative assertions from test 4.
2. Rename to drop the stale `fr25_2` prefix.

---

### Test 3 — `fr25_2_ac3_sync_vector_unavailable_does_not_emit_failure_or_verification_sections`

**What it tests**: Invokes `Sync` via `ToolRegistry` (identical setup to test 1). Asserts:
- `output.contains("Vector sync diagnostics:")` ← **already asserted in test 1**
- `output.contains("- STATUS: unavailable")` ← **already asserted in test 1**
- `!output.contains("- FAILURES:")` ← **unique negative assertion**
- `!output.contains("Vector index verification:")` ← **unique negative assertion**

**Load-bearing?** ⚠️ Marginal. The two negative assertions are real regression guards: if
`build_vector_failure_report()` or `build_vector_verification_report()` were accidentally called on
the unavailable code path, this test would catch it. That's a real scenario (e.g., a future
refactor accidentally calls `format_sync_output(…)` without checking `vector_disabled` first). So
the *assertions* have value. But the test itself uses a complete DB init + file write + Sync
invocation — the same expensive setup as test 1 — just to make two negative assertions that could
live in test 1 at zero incremental cost.

**Unique coverage?** ❌ No positive coverage. The only unique contribution is the two negative
assertions. Everything else is already covered by test 1.

**Correct assertions?** ✅ The negative assertions are logically correct. The two positive assertions
are redundant with test 1.

**Good name?** ❌ The name duplicates the `ac3` label already used by test 1. Two tests sharing
the same acceptance criterion label is misleading — it implies they test different ACs but they do
not.

**Parameterizable?** ❌ Same invocation as test 1; cannot be parameterized meaningfully as a
separate test.

**Verdict: MERGE → test 1**
Move `!output.contains("- FAILURES:")` and `!output.contains("Vector index verification:")` into
the rewritten test 1. Delete this test.

---

### Test 4 — `fr25_2_ac4_sync_files_vector_unavailable_does_not_emit_failure_or_verification_sections`

**What it tests**: Invokes `sync_files` (identical setup to test 2, but with only one fixture file
instead of two). Asserts:
- `output.contains("Vector sync diagnostics:")` ← **already asserted in test 2**
- `output.contains("- STATUS: unavailable")` ← **already asserted in test 2**
- `!output.contains("- FAILURES:")` ← **unique negative assertion**
- `!output.contains("Vector index verification:")` ← **unique negative assertion**

Note: `sync_files` does not call `build_vector_verification_report()` at all (it's only called in
the full `run_sync()` path). This means `!output.contains("Vector index verification:")` in this
test is **vacuous for this entry point** — the string cannot appear via `sync_files` regardless of
the vector state.

**Load-bearing?** ⚠️ Marginal. The `!output.contains("- FAILURES:")` guard is marginally useful
(if `build_vector_failure_report()` were accidentally added to `sync_files`). The verification
guard is vacuous. Both could live in test 2.

**Unique coverage?** ❌ No positive coverage beyond what test 2 already has.

**Correct assertions?** ⚠️ Mostly. The `!output.contains("Vector index verification:")` assertion
is vacuous for `sync_files` — the verification function is not on this code path regardless of
condition, making this assertion always-pass with no failure scenario possible.

**Good name?** ❌ Same `ac4` label used by test 2. Sharing the label implies same AC, which it is,
but the duplication is confusing.

**Parameterizable?** ❌ Same invocation as test 2.

**Verdict: MERGE → test 2**
Move `!output.contains("- FAILURES:")` into the rewritten test 2. Drop the vacuous
`!output.contains("Vector index verification:")` assertion (it can't fire for `sync_files`). Delete
this test.

---

## Recommendations

### P0 — Remove Vacuous Assertions (test 1 and test 4)

**Test 1**: Delete the negative assertion against `"sqlite-vec extension unavailable; semantic
tooling disabled"`. This string does not exist in production code. It is a historical artifact from
a prior refactor. It always passes, it never fails, it adds noise.

**Test 4**: Delete `!output.contains("Vector index verification:")`. The `build_vector_verification_report()`
function is only called inside `run_sync()`, not `sync_files()`. The string cannot appear from this
entry point. The assertion is permanently vacuous.

---

### P1 — Merge Tests 3 and 4 into Tests 1 and 2

The two "guardrail" tests (3 and 4) double the test infrastructure overhead (DB init, file writes,
`Sync` / `sync_files` invocations) for two additional `assert!` calls that could live in the
primary tests. Suggested merged test structure:

```rust
// Merged replacement for tests 1 + 3:
#[test]
fn sync_tool_vector_disabled_emits_structured_unavailable_diagnostics() {
    // ... setup ...
    let output = /* ToolRegistry Sync invocation */;

    // Positive: structured diagnostic block is present
    assert!(output.contains("Sync complete:"), …);
    assert!(output.contains("Vector sync diagnostics:"), …);
    assert!(output.contains("- STATUS: unavailable"), …);
    assert!(output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"), …);
    assert!(output.contains("- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"), …);
    assert!(output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"), …);

    // Negative: failure and verification sections must be absent
    assert!(!output.contains("- FAILURES:"), …);
    assert!(!output.contains("Vector index verification:"), …);
}

// Merged replacement for tests 2 + 4:
#[test]
fn sync_files_vector_disabled_emits_structured_unavailable_diagnostics() {
    // ... setup (2 files) ...
    let output = /* sync_files invocation */;

    // Positive: structured diagnostic block is present
    assert!(output.contains("SyncFiles complete:"), …);
    assert!(output.contains("Vector sync diagnostics:"), …);
    assert!(output.contains("- STATUS: unavailable"), …);
    assert!(output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"), …);
    assert!(output.contains("- COUNTERS_SKIPPED: concepts=2, memory=2, total=4"), …);
    assert!(output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"), …);

    // Negative: failure section must be absent (verification is not on this path)
    assert!(!output.contains("- FAILURES:"), …);
}
```

Result: **4 tests → 2 tests**, same meaningful coverage, half the fixture overhead.

---

### P2 — Fix Stale RTM Naming

The `fr25_2_ac3` / `fr25_2_ac4` prefixes refer to a prior sprint's acceptance criterion numbering
that no longer maps to any live RTM item. Rename tests to behavior-descriptive names and update the
module doc comment to reference the correct current traceability tag (e.g., `T-9.6: FR-7, FR-8`
without the stale `fr25_2` alias).

---

### P3 — Extract Shared Assertion Helper

Tests 1 and 2 share an assertion shape differing only in the specific count strings. A helper
reduces drift if the diagnostic format changes:

```rust
fn assert_vector_unavailable_diagnostics(output: &str, prefix: &str, concepts: usize, memory: usize) {
    let total = concepts + memory;
    assert!(output.contains(&format!("{prefix} complete:")), "…");
    assert!(output.contains("Vector sync diagnostics:"), "…");
    assert!(output.contains("- STATUS: unavailable"), "…");
    assert!(output.contains("- COUNTERS_FAILED: concepts=0, memory=0, total=0"), "…");
    assert!(output.contains(
        &format!("- COUNTERS_SKIPPED: concepts={concepts}, memory={memory}, total={total}")
    ), "…");
    assert!(output.contains("- REASON_CODE VECTOR_DISABLED_BY_CONFIG"), "…");
    assert!(!output.contains("- FAILURES:"), "…");
}
```

---

## Load-Bearing Score: 2/4

Two of the four tests carry genuinely unique, load-bearing coverage:

| Test | Load-Bearing? | Rationale |
|------|:---:|-----------|
| Test 1 (`ac3` positive) | ✅ | Primary integration proof for `Sync` tool + unavailable path |
| Test 2 (`ac4` positive) | ✅ | Primary integration proof for `sync_files` + unavailable path |
| Test 3 (`ac3` guardrail) | ❌ | Negative assertions belong in test 1; no unique positive coverage |
| Test 4 (`ac4` guardrail) | ❌ | Negative assertions belong in test 2; one vacuous assertion |

**Score: 2/4.** The file delivers 2 tests worth of unique coverage spread across 4 tests, with 2
vacuous assertions (1 per entry point) adding noise without protection. Post-merge and cleanup the
score would be 2/2.

---
## SWE Remediation
- REWRITE test 1 → `sync_tool_vector_disabled_emits_structured_unavailable_diagnostics`: absorbed negative assertions from test 3 (P1), removed vacuous sqlite-vec assertion (P0), renamed to drop stale `fr25_2` prefix (P2)
- REWRITE test 2 → `sync_files_vector_disabled_emits_structured_unavailable_diagnostics`: absorbed `!FAILURES:` from test 4 (P1), dropped vacuous `Vector index verification:` assertion (P0), renamed (P2)
- MERGE test 3 → test 1: deleted (P1)
- MERGE test 4 → test 2: deleted (P1)
- Extracted `assert_vector_unavailable_diagnostics` shared helper (P3)
- Result: 4 tests → 2 tests, 113 lines deleted, 41 added
- Commit: ee0de75

---
## Red-team Audit

**Traceability**: T-9.6 | FR-7, FR-8

### REWRITE Verdicts

- ✅ REWRITE test 1 → `sync_tool_vector_disabled_emits_structured_unavailable_diagnostics`:
  - Absorbed `!output.contains("- FAILURES:")` from test 3 (via shared helper) — P1 ✅
  - Absorbed `!output.contains("Vector index verification:")` from test 3 (explicit assertion, line 78) — P1 ✅
  - Removed vacuous `sqlite-vec extension unavailable` negative assertion — P0 ✅
  - Renamed to drop stale `fr25_2` prefix — P2 ✅
  - All 6 original positive assertions preserved in helper — ✅
  - Total: 8 assertions (6 positive + 2 negative), matches eval spec

- ✅ REWRITE test 2 → `sync_files_vector_disabled_emits_structured_unavailable_diagnostics`:
  - Absorbed `!output.contains("- FAILURES:")` from test 4 (via shared helper) — P1 ✅
  - Correctly dropped vacuous `!output.contains("Vector index verification:")` (not on `sync_files` code path) — P0 ✅
  - Renamed to drop stale `fr25_2` prefix — P2 ✅
  - All 6 original positive assertions preserved in helper — ✅
  - Total: 7 assertions (6 positive + 1 negative), matches eval spec

### MERGE Verdicts

- ✅ MERGE test 3 → test 1: deleted, both unique negative assertions absorbed into test 1, no unique positive coverage lost (both positive assertions were already in test 1)
- ✅ MERGE test 4 → test 2: deleted, useful `!FAILURES:` assertion absorbed, vacuous `!Vector index verification:` correctly dropped per P0

### P3 — Shared Helper

- ✅ `assert_vector_unavailable_diagnostics(output, prefix, concepts, memory)` extracted with parameterized counters
- Helper contains 6 positive assertions + 1 negative (`!FAILURES:`)
- Test 1 adds `!Vector index verification:` outside the helper (meaningful for `Sync` tool path only)
- Test 2 relies on helper alone (verification assertion not on `sync_files` path)

### Cross-checks

- Only 1 file changed (the test file) — no source code modifications ✅
- Test count: 4 → 2 as specified ✅
- Line delta: 41 added, 113 deleted as claimed ✅
- `cargo test -- vector_sync_diag_unavailable`: 2 passed, 0 failed ✅
- No unrelated changes introduced ✅

- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- vector_sync_diag_unavailable`: 2 passed, 0 failed
- Tests confirmed:
  - `vector_sync_diag_unavailable::sync_tool_vector_disabled_emits_structured_unavailable_diagnostics` — ok
  - `vector_sync_diag_unavailable::sync_files_vector_disabled_emits_structured_unavailable_diagnostics` — ok
- Verdict: **PASS**
