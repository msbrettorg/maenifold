---
id: incremental_sync_boundary_mutations
status: done
---

# Test Evaluation: `incremental_sync_boundary_mutations`

**File**: `crates/maenifold-core/tests/incremental_sync_boundary_mutations.rs`
**Evaluator role**: 🔘 Gray — Skeptical Inquiry
**Traceability**: T-5.7 (as tagged in file) | NFR-2 (test coverage gate) | NFR-1 (C# oracle parity)
**Date**: 2025-07-01

---

## Summary

This file contains **3 tests** that exercise `sync_single_file()` at the integration boundary:
new-file insertion, content-mutation concept replacement, and deletion cleanup. The tests
operate at a higher level than the `sync_unification_single_file.rs` family — they reason about
**whole-graph state** (`concept_names`, `concept_sources`) and **cross-file isolation**
(`last_indexed` of untouched peers) rather than per-row DB counts.

The tests are meaningful and not redundant, but the file carries **one critical traceability
defect**, **one missing assertion** (Test 2), and **one fragile output-string assertion**
(Test 1). The support module also exposes `fetch_file_md5` which is never used by any test
in this file (dead export).

The cross-file isolation assertions in Tests 1 and 2 are the strongest differentiators from
the `sync_unification_single_file.rs` family and represent load-bearing signal not found
elsewhere.

### Critical Traceability Defect

All three tests are tagged `// T-5.7: RTM FR-5.7`. **FR-5.7 does not exist in the RTM.**
FR-5 in the CHANGELOG covers `RunFullBenchmark tool (RTM-031–037)` — unrelated to
incremental sync. Every comparable test file (`sync_unification_single_file.rs`,
`incremental_sync_unification_guards.rs`, `concept_sync_edge_lifecycle.rs`, etc.) correctly
cites `T-5.7: RTM FR-14.1 / FR-14.2 / FR-14.3`. The boundary-mutations tests should
reference `RTM FR-14.3` (content-change reprocessing path). This is a copy-paste traceability
error that silently breaks RTM coverage accounting.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique Signal? | Assertions Correct? | Name Descriptive? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `sync_single_file_processes_a_new_file_correctly` | ✅ Yes | ✅ Yes (cross-file isolation) | ⚠️ Partial (brittle string count) | ✅ Yes | Low value | **KEEP** |
| 2 | `sync_single_file_updates_concepts_when_content_changes` | ✅ Yes | ✅ Yes (cross-file isolation) | ⚠️ Partial (missing source assertion) | ✅ Yes | Low value | **REWRITE** |
| 3 | `deleted_files_remove_their_concepts_from_the_graph` | ✅ Yes | ✅ Yes (shared-concept preservation) | ⚠️ Partial (missing survivor source check) | ✅ Yes | Low value | **KEEP** |

---

## Test-by-Test Analysis

---

### Test 1 — `sync_single_file_processes_a_new_file_correctly`

**Scenario**: Create a baseline file, run full sync, then create a second file and run
`sync_single_file` on it. Verify the new file is indexed, the graph includes all expected
concepts, and the baseline file's `last_indexed` timestamp was not touched.

**Is it load-bearing?**  
Yes. The cross-file isolation assertion on line 53–57 is the defining value:

```rust
assert_eq!(
    fetch_last_indexed(&cfg, &baseline_uri),
    Some(baseline_last_indexed),
    "unchanged baseline file should not be reprocessed"
);
```

No test in `sync_unification_single_file.rs` or `incremental_sync_unification_lifecycle.rs`
verifies that `sync_single_file` truly does not touch peer files. This is a contractual
guarantee of the "single-file" scope — if the implementation ever accidentally reprocesses
siblings, this is the only test that will catch it.

**Is the assertion set correct and sufficient?**  
Mostly. Four assertions are made:

1. Output contains `"Synced memory://notes/new-file"` — verifies routing and URI construction. ✅
2. Output contains `"2 concept mentions updated"` — **fragile hardcoded count**. If the
   output format changes or concept deduplication changes this number, the test breaks for
   the wrong reason. Should assert `>= 1` or use `concept_names`/`file_concepts` instead. ⚠️
3. `concept_names` equals `{fresh, seed, shared}` — correct, verifies whole-graph state. ✅
4. `baseline_uri`'s `last_indexed` unchanged — load-bearing cross-file isolation check. ✅

**Missing**: No assertion that `concept_sources("fresh")` points to `notes/new-file`. The
concept is in `concept_names`, but the graph entry could theoretically be orphaned (no
`concept_mentions` row). Low likelihood, but the hole exists.

**Parameterizable?**  
Low benefit. The scenario requires a two-file baseline + single-file sync structure that does
not generalize cleanly to a data-driven table without losing the clarity of the isolation
assertion.

**Verdict**: **KEEP**. Fix the brittle string count assertion. Add `concept_sources` check.

---

### Test 2 — `sync_single_file_updates_concepts_when_content_changes`

**Scenario**: Create two files (`target.md` with `[[alpha]]`, `untouched.md` with `[[stable]]`),
run full sync, then rewrite `target.md` to replace `[[alpha]]` with `[[beta]]` (after
back-dating `last_indexed` to force mtime guard passage), run `sync_single_file`, and verify:
concept graph updated, old concept removed, untouched file's `last_indexed` unchanged.

**Is it load-bearing?**  
Yes — but partially overlaps with `sync_unification_single_file::process_file_content_changed_reprocesses_all_mentions`.
The unique signals are:

1. **`alpha` fully purged from `concept_names`** (not just from `target.md`'s mentions):
   `concept_names` = `{beta, shared, stable}` — alpha gone. ✅
2. **Cross-file isolation**: `untouched_last_indexed` unchanged. ✅

The per-file mention check (`file_concepts` returns `{beta, shared}`) overlaps with the
unification test, but is still worth keeping as a consistency anchor.

**Is the assertion set correct and sufficient?**  
No — a meaningful assertion is missing. After replacing `alpha` with `beta`, the test does
not verify:

```rust
// MISSING: verify beta's source is actually the target file
assert_eq!(
    concept_sources(&cfg, "beta"),
    BTreeSet::from(["memory://notes/target".to_string()]),
    "beta should be sourced only from the target file"
);
```

Without this check, a bug that inserts `beta` into `concepts` but omits the
`concept_mentions` row would pass all existing assertions (since `concept_names` pulls from
`concepts`, not `concept_mentions`). The `file_concepts` check partially covers this, but
`concept_sources` would catch it from the opposite direction and is consistent with how
Tests 1 and 3 use it.

**`set_last_indexed` hack**: Using `set_last_indexed(&cfg, &target_uri, "2000-01-01T00:00:00+00:00")`
to force mtime guard passage is correct, but the comment `// T-5.7: RTM FR-5.7` doesn't
explain the intent. The setup is non-obvious — `write_memory_fixture` updates the file
mtime, and `set_last_indexed` pushes the DB record into the past, so the guard sees
`file_mtime > last_indexed` and proceeds. This pattern deserves a comment.

**Parameterizable?**  
The concept-swap pattern (`alpha → beta`, add/remove) could theoretically be parameterized
across multiple concept set changes, but the cross-file isolation check anchors the test to a
two-file structure. Not worth the complexity.

**Verdict**: **REWRITE**. Add `concept_sources("beta")` assertion. Add comment explaining
the `set_last_indexed` mtime guard setup. Fix traceability tag.

---

### Test 3 — `deleted_files_remove_their_concepts_from_the_graph`

**Scenario**: Create two files (`deleted.md` with `[[alpha]] [[shared]]`, `survivor.md` with
`[[shared]] [[stable]]`), run full sync, physically delete `deleted.md`, run `sync_single_file`
on the now-absent path, and verify: stale index entry cleared, unique concept `alpha` removed,
shared concept `shared` preserved, and the deleted file has no remaining mentions.

**Is it load-bearing?**  
Yes. This is the primary test for the shared-concept preservation boundary: deleting a file
that owns *some* of a concept's mentions should not destroy the concept if other files still
reference it. The assertion `concept_names == {shared, stable}` — note `shared` survives —
is the key load-bearing signal. This is not verified at this level by `process_file_deleted_after_create_removes_file_content_and_mentions_from_db` in `incremental_sync_unification_lifecycle.rs`, which only checks that the deleted file's own rows are gone.

**Is the assertion set correct and sufficient?**  
Three out of four assertions are correct. The fourth is missing:

```rust
// MISSING: verify shared concept's mention in survivor.md still exists
assert_eq!(
    concept_sources(&cfg, "shared"),
    BTreeSet::from(["memory://notes/survivor".to_string()]),
    "shared concept's mention in survivor file must survive deletion of peer"
);
```

Without this, the test does not distinguish between:
- (A) `shared` surviving in `concept_names` because the graph correctly kept the survivor's mention, vs.
- (B) `shared` surviving in `concept_names` as an orphaned concept with zero `concept_mentions` rows.

Case (B) is a data consistency bug that the current assertion set cannot detect.

**Output string assertion** (`"removed stale index entry"`) is more stable than Test 1's
count-bearing string, as it is a status message rather than a numeric value.

**Parameterizable?**  
No meaningful benefit. The shared/unique concept split is the specific scenario being tested;
parameterizing concept names adds noise without coverage gain.

**Verdict**: **KEEP**. Add `concept_sources("shared")` assertion for the survivor-file check.

---

## Recommendations

Listed in priority order:

1. **Fix traceability tag** (all 3 tests) — Change `// T-5.7: RTM FR-5.7` to
   `// T-5.7: RTM FR-14.3`. FR-5.7 does not exist; FR-14.3 (content-change reprocessing,
   RTM-080) is the correct anchor. Broken tags silently drop tests from RTM coverage reports.

2. **Rewrite Test 2** — Add `concept_sources("beta")` assertion and add a clarifying comment
   above `set_last_indexed` explaining that it back-dates the DB record to force the mtime
   guard to classify the rewritten file as changed.

3. **Strengthen Test 3** — Add `concept_sources("shared")` assertion confirming the shared
   concept's source reference in `survivor.md` is intact after the deletion. This is the
   difference between verifying "the concept name exists" and verifying "the concept's data
   integrity survived."

4. **Weaken Test 1's output-string count assertion** — Replace:
   ```rust
   assert!(output.contains("2 concept mentions updated"), ...)
   ```
   with a structural assertion:
   ```rust
   assert_eq!(
       file_concepts(&cfg, &file_uri("notes/new-file.md")),
       BTreeSet::from(["fresh".to_string(), "shared".to_string()])
   );
   ```
   Output format is an implementation detail; the graph state is the contract.

5. **Add `concept_sources` check in Test 1** — Verify `concept_sources("fresh")` returns
   `{memory://notes/new-file}` to close the orphaned-concept hole.

6. **Clean up dead support export** — `fetch_file_md5` in `support.rs` is exported but
   unused by any test in this file. Either use it (it would strengthen Test 2 — verify
   `file_md5` changed after rewrite) or scope it with `#[cfg(test)]` internal visibility.

7. **Add comment for `ConfigOverrideReset` + `baseline_sync()` interaction** — The fact that
   `baseline_sync()` uses `ToolRegistry::global()` which depends on `Config::current()`,
   which is redirected by `isolated_config()` via `Config::override_root()`, and that
   `ConfigOverrideReset` is needed to clean up for the *next* test rather than the current
   one, is non-obvious. A one-line comment saves future maintainers 10 minutes.

---

## Load-Bearing Score: 3/3

All three tests carry unique, non-redundant signal. However, all three have assertion gaps
that leave specific failure modes undetected. The tests are structurally sound and correctly
scoped; the issues are in assertion completeness, not in scenario selection.

| Test | Load-Bearing Signal | Assertion Completeness | Overall |
|------|--------------------|-----------------------|---------|
| `sync_single_file_processes_a_new_file_correctly` | ✅ Cross-file isolation | ⚠️ Missing `concept_sources`, brittle count | 4/5 |
| `sync_single_file_updates_concepts_when_content_changes` | ✅ Cross-file isolation + concept purge | ⚠️ Missing `concept_sources("beta")` | 3/5 |
| `deleted_files_remove_their_concepts_from_the_graph` | ✅ Shared-concept preservation | ⚠️ Missing survivor `concept_sources` | 3/5 |

**Score: 3 load-bearing / 3 total. No tests are safe to remove.**

---
## SWE Remediation
- KEEP test 1 `sync_single_file_processes_a_new_file_correctly`: fixed traceability FR-5.7→FR-14.3, replaced brittle `"2 concept mentions updated"` output string with structural `file_concepts` assertion, added `concept_sources("fresh")` to close orphaned-concept hole, added ConfigOverrideReset comment (P0, P1×2, P3)
- REWRITE test 2 `sync_single_file_updates_concepts_when_content_changes`: fixed traceability FR-5.7→FR-14.3, added `concept_sources("beta")` assertion verifying source linkage, added clarifying comment for `set_last_indexed` mtime guard setup (P0, P1)
- KEEP test 3 `deleted_files_remove_their_concepts_from_the_graph`: fixed traceability FR-5.7→FR-14.3, added `concept_sources("shared")` survivor assertion distinguishing live mention from orphaned concept row (P0, P1)
- SKIP P2 `fetch_file_md5`: not dead — used by `incremental_sync_boundary_guards.rs` via shared support module
- All 3 tests pass: `cargo test --test integration_tests -p maenifold-core -- incremental_sync_boundary_mutations`
- Commit: 80a6fec

---
## Red-team Audit

**Traceability**: T-5.7 | NFR-2 | FR-14.3 (RTM-080)

### P0 — Traceability Fix (all 3 tests)
- ✅ Test 1 `sync_single_file_processes_a_new_file_correctly`: `FR-5.7` → `FR-14.3` on line 13
- ✅ Test 2 `sync_single_file_updates_concepts_when_content_changes`: `FR-5.7` → `FR-14.3` on line 68
- ✅ Test 3 `deleted_files_remove_their_concepts_from_the_graph`: `FR-5.7` → `FR-14.3` on line 132

### P1 — KEEP Test 1: brittle string → structural assertion + concept_sources
- ✅ Removed: `assert!(output.contains("2 concept mentions updated"), ...)`
- ✅ Added: `file_concepts(&cfg, &file_uri("notes/new-file.md"))` == `{fresh, shared}` — matches eval Recommendation 4 exactly
- ✅ Added: `concept_sources(&cfg, "fresh")` == `{memory://notes/new-file}` — closes orphaned-concept hole per Recommendation 5
- ✅ Cross-file isolation assertion (`baseline_last_indexed` unchanged) preserved untouched

### P1 — REWRITE Test 2: concept_sources("beta") + mtime guard comment
- ✅ Added: `concept_sources(&cfg, "beta")` == `{memory://notes/target}` — matches eval Recommendation 2 verbatim
- ✅ Added: two-line comment above `set_last_indexed` explaining back-dating intent for mtime guard
- ✅ All original assertions preserved: `file_concepts`, `concept_sources("alpha").is_empty()`, `concept_names`, cross-file isolation

### P1 — KEEP Test 3: concept_sources("shared") survivor assertion
- ✅ Added: `concept_sources(&cfg, "shared")` == `{memory://notes/survivor}` — matches eval Recommendation 3 verbatim
- ✅ Message text matches eval spec: `"shared concept's mention in survivor file must survive deletion of peer"`
- ✅ Distinguishes live-mention survival (A) from orphaned-concept row (B) as eval required

### P2 — SKIP fetch_file_md5 dead export
- ✅ Correct skip. `fetch_file_md5` is alive: imported and used by `incremental_sync_boundary_guards.rs` lines 6, 72, 94 via the shared support module. Eval scoped analysis to `_mutations.rs` only; SWE correctly widened scope to shared consumers.

### P3 — ConfigOverrideReset comment
- ✅ Added in Test 1 line 17: `// ConfigOverrideReset restores Config::current() for the next test on drop.` — sufficient, pattern is self-documenting in Tests 2/3 after first explanation.

### No Unrelated Changes
- ✅ Diff is 1 file, +26 −6 lines. Every change maps to an eval recommendation. No code additions, removals, or refactors outside scope.

### Tests Pass
- ✅ `cargo test --test integration_tests -p maenifold-core -- incremental_sync_boundary_mutations`: 3 passed, 0 failed (verified at audit time, 0.09s)

### Verdict: PASS

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_boundary_mutations`: 3 passed, 0 failed
- Verdict: PASS
