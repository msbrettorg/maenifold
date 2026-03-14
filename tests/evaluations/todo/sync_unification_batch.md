# Test Evaluation: sync_unification_batch

**File:** `crates/maenifold-core/tests/sync_unification_batch.rs`
**Support module:** `crates/maenifold-core/tests/sync_unification/support.rs`
**Companion file:** `crates/maenifold-core/tests/sync_unification_single_file.rs`
**Test count:** 3
**Evaluated:** 2025-07-18
**Traceability claim:** T-5.7 · RTM FR-14.3 (×3, identical on all tests — see traceability findings)

---

## Summary

This file is meant to be the *batch* counterpart to `sync_unification_single_file.rs`, covering multi-file and edge-case behavior of the `sync()` batch function. In practice, only **one of the three tests** actually exercises batch behavior. The other two use `sync_one` (`sync_single_file`) — the single-file function — placing single-file operations in a file whose name promises batch coverage. This is a structural mismatch between file naming and test content.

**What the file does well:** The `sync_empty_memory_directory_returns_without_error` test is appropriately minimal for its edge case. The support infrastructure (`isolated_config`, `ConfigOverrideReset`, `TEST_MUTEX`) is shared correctly with the single-file companion file and produces genuine DB-level isolation. Pre-condition assertions in the delete test provide good discipline.

**Three systemic concerns run across all tests:**

1. **File name / test content mismatch.** `remove_file_cleans_content_and_mentions` calls `sync_one`, not `sync_all`. It belongs in `sync_unification_single_file.rs`, and a nearly identical (but more thorough) test already exists in `incremental_sync_unification_lifecycle.rs`. The presence of a single-file deletion test in the batch file creates a false impression that the batch suite covers deletion semantics — it does not, via `sync_all`.

2. **Output string fragility is systemic.** All three tests assert on human-readable output substrings (`"removed stale index entry"`, `"Sync complete:"`, `"- 3 FILES processed"`, `"- 0 FILES processed"`). These couple the tests to presentation-layer formatting. Any refactor of the output messages breaks tests even when behavior is correct. This pattern appears in at least 15 other test files across the codebase (`proximity_weight_sections.rs`, `sync_perf_baseline.rs`, `incremental_sync_unification_lifecycle.rs`, etc.) — it is a codebase-wide concern but is concentrated here because output strings are the *only* assertions in Tests 2 and 3 beyond DB-row existence checks.

3. **Stale and non-traceable annotations.** Every test carries `// T-5.7: RTM FR-14.3`. In the current `RTM.md`, `FR-14` is not defined — the document uses `RTM-NNN` identifiers. The `FR-14.x` sub-numbering appears to be a legacy artifact from a sprint that predates the current RTM schema. As documented in the `incremental_sync_unification_lifecycle.md` evaluation, all three tests in that file carry the same stale annotation, and the same comment appears across `sync_unification_single_file.rs` as well. This is a coordinated traceability debt, not an isolated error.

**Relationship to overlapping test files:**

| This test | Closest overlap |
|---|---|
| `remove_file_cleans_content_and_mentions` | `incremental_sync_unification_lifecycle.rs::incremental_sync_file_deleted_cleans_all_artifacts` (more thorough: includes `concept_graph` check) |
| `remove_file_cleans_content_and_mentions` | `incremental_sync_boundary_mutations.rs` delete test (covers global concept existence, not DB rows) |
| `sync_batch_processes_multiple_files` | `sync_perf_baseline.rs` (asserts "Sync complete:" only; this file adds DB-row assertions — more thorough) |
| `sync_empty_memory_directory_returns_without_error` | No direct counterpart — unique |

---

## Verdicts

| Test Function | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|---|---|---|---|---|---|
| `remove_file_cleans_content_and_mentions` | ⚠️ Partial | ❌ Duplicated (weaker version) | ⚠️ Missing `concept_graph` check | ⚠️ Misplaced in batch file | ➖ N/A | **MERGE** |
| `sync_batch_processes_multiple_files` | ✅ Yes | ✅ Yes | ⚠️ Brittle count string | ✅ Yes | ⚠️ Minor | **REWRITE** |
| `sync_empty_memory_directory_returns_without_error` | ✅ Yes | ✅ Yes | ⚠️ Output-only (unavoidable) | ✅ Yes | ➖ N/A | **KEEP** |

---

## Detailed Findings

### Test 1: `remove_file_cleans_content_and_mentions`

**What it actually tests:** A file containing two WikiLinks is synced via `sync_one`, deleted from disk, and re-synced via `sync_one`. Assertions verify `file_content` row is gone and both `concept_mentions` rows are gone.

**Misplacement:** This test calls `sync_one` = `sync_single_file`, not `sync_all`. It is a single-file operation in a file named `*_batch.rs`. A reader auditing batch sync coverage will find it here and not realize it tests the single-file path. The logical home is `sync_unification_single_file.rs`, which already has 4 tests for `sync_one` pathways.

**Duplication concern:** `incremental_sync_unification_lifecycle.rs::incremental_sync_file_deleted_cleans_all_artifacts` tests the same operation with *more* assertions (pre-condition checks for both `file_content` and `concept_mentions`, plus a `concept_graph` edge verification via `get_edge_source_files`). This test is a strict subset of that one — same behavior, fewer assertions. Two tests covering the same path at the same layer (unification) with different assertion depth is a maintenance liability: any regression in deletion will cause both to fail, but this one provides less diagnostic information about *where* the regression is.

**Missing assertion:** No `concept_graph` check. After indexing a file with two WikiLinks, a co-occurrence edge between the two concepts is created. After deletion, that edge (or the deleted file's contribution to it) should be removed. The lifecycle file's delete test verifies this explicitly. This test does not.

**Output assertion:** `output.contains("removed stale index entry")` is fragile. If the implementation changes this message to "stale entry removed" or "evicted from index", the assertion fails even though behavior is correct. There is no constant or enum for this string in the production code — it is a bare string literal in `sync/mod.rs`.

**Pre-condition discipline:** ✅ Good. The test asserts `file_content` and one `concept_mention` exist *before* deletion. This guards against a silent failure in the first sync causing the deletion assertions to be vacuously true.

**Verdict: MERGE.** Move this test into `sync_unification_single_file.rs` (where it belongs given it uses `sync_one`) and augment it to add the `concept_graph` edge check to reach parity with the lifecycle suite. Once merged, consider whether this is still necessary alongside `incremental_sync_unification_lifecycle.rs::incremental_sync_file_deleted_cleans_all_artifacts`. If the lifecycle test already covers it, this test can be REMOVED rather than merged.

---

### Test 2: `sync_batch_processes_multiple_files`

**What it actually tests:** Three files each containing a single distinct WikiLink are created. `sync_all` is called. Output is checked; then all three `file_content` rows and all three `concept_mentions` counts are verified.

**This is the only test in the file that genuinely tests batch behavior.** It is load-bearing: no other test suite asserts DB-row creation at this level of detail after a `sync_all` call. `sync_perf_baseline.rs` calls `sync_all` but only checks `output.contains("Sync complete:")` without DB introspection.

**Hardcoded string `"- 3 FILES processed"`:** This is the most brittle assertion in the file. It couples the test to:
  - The exact phrase "FILES processed" (case-sensitive uppercase)
  - The exact prefix format `"- 3 "`
  - The count "3" matching the specific setup of three files

If the output format changes to `"3 files processed"`, `"processed 3 files"`, or even `"Processed 3 FILES"`, this assertion fails. Notably, `sync_perf_baseline.rs` already handles the case-sensitivity concern: `output.contains("files processed") || output.contains("FILES processed")`. This test should adopt a similarly tolerant check. Asserting *that some count appears* (e.g., `output.contains("3")`) combined with asserting the DB rows exist is more robust.

**No cross-contamination assertion:** Each file contains only one concept, so there can be no cross-file concept bleed in `concept_mentions`. However, the test does not verify that `file_uri1` has *no* mention of `concept_b` or `concept_c`. This is an implicit design choice (unique concepts prevent overlap), but making it explicit would increase confidence. Optionally, checking `get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_b)) == None` would make the isolation property explicit.

**`_path1`, `_path2`, `_path3` bindings:** The path variables are prefixed with `_` (intentionally unused). This is correct — `TempDir` cleanup handles file deletion; the test only needs the URIs for DB queries. Not a bug.

**Parameterizability:** The 3-file setup is not especially meaningful — 2 or 10 files would test the same thing. If `rstest` or a similar parameterization framework is in use elsewhere, this could become `#[rstest] #[case(1)] #[case(3)] #[case(10)]`. However, this is a minor improvement, not a blocker.

**Verdict: REWRITE.** Fix the brittle `"- 3 FILES processed"` assertion:
  - Separate the "count is correct" concern from the "format is correct" concern.
  - Assert DB row existence (already done well) as the primary contract.
  - For the output string, use a less format-coupled check: `output.contains("3") && output.contains("files processed" || "FILES processed")`, or extract a shared constant.

---

### Test 3: `sync_empty_memory_directory_returns_without_error`

**What it actually tests:** `sync_all` is called on a freshly created, empty temp directory. No files exist. Output is verified to contain "Sync complete:" and "- 0 FILES processed".

**This is the sole guard for the empty-directory edge case.** It protects against panics, `unwrap()` failures, or error returns from `sync_all` when the memory directory has no `.md` files. This is a real boundary condition — the function iterates a directory and must handle the zero-file case gracefully. Without this test, a regression that causes `sync_all` to return `Err(...)` on an empty directory would be undetected until a user reports it at first run.

**Output-only assertions are unavoidable here.** There is no DB state to verify (nothing was written). The only observable outcome is the function's return value and the output string. Output string assertions are the correct mechanism for this test. The fragility concern is noted but is not actionable without a structured output type — a systemic change, not a per-test fix.

**Same `"- 0 FILES processed"` string concern** as Test 2: case-sensitive, format-coupled. But for the empty case, the count `"0"` appearing in the output plus `"Sync complete:"` is a reasonable proxy. No stronger assertion is available.

**Verdict: KEEP.** This test is minimal, correct for its purpose, and unique in the test suite. The string literal fragility is noted but not fixable at the test level without production code changes.

---

## Cross-Cutting Findings

### Finding A: Static Concept Names Without UUID Isolation

All three tests use fixed concept names: `sync-unify-a`, `sync-unify-b`, `sync-unify-c`. The `incremental_sync_unification_lifecycle.rs` tests use UUID-prefixed concept names (`incr-{uuid}`) to prevent cross-test contamination in the unlikely event tests run in the same database. Here, tests are serialized via `TEST_MUTEX` and each uses its own `isolated_config()` (a fresh `TempDir` and fresh DB), so the fixed names cannot collide in practice. However, the isolation model relies entirely on the mutex preventing concurrent execution — if the mutex were ever removed (e.g., after a future refactor to true per-test DB isolation), these tests would be at risk of concept-name collisions. Document the dependency explicitly in the support module.

### Finding B: `remove_file_cleans_content_and_mentions` Tests `sync_one`, Not `sync_all`

The word "batch" in the file name implies all tests use the batch-sync path. Test 1 does not. This creates a false audit trail: when reviewing coverage of `sync_all` (the batch path), Test 1 appears relevant but is actually testing `sync_single_file` (the single-file path). The file should either be renamed to reflect its mixed content (`sync_unification_delete_and_batch.rs`) or Test 1 should be migrated to `sync_unification_single_file.rs`.

### Finding C: Traceability Annotation `RTM FR-14.3` Is Stale on All Three Tests

`// T-5.7: RTM FR-14.3` appears on every test in this file, every test in `sync_unification_single_file.rs`, and (with `FR-14.1`/`FR-14.2`/`FR-14.3`) in `incremental_sync_unification_lifecycle.rs`. In `RTM.md`, no `FR-14` definition exists — the document uses `RTM-NNN` identifiers. This is a coordinated traceability debt that spans at least three test files. The annotation should be replaced with the current RTM-NNN identifier or a pointer to the CHANGELOG entry that archived these items.

### Finding D: `concept_graph` Is Never Verified in This File

Neither `remove_file_cleans_content_and_mentions` (where the edge should be *deleted*) nor `sync_batch_processes_multiple_files` (where no edge should exist, since each file has only one concept) verify `concept_graph` state. The support module does not expose a `get_edge` or `get_edge_source_files` helper (those exist in the unification lifecycle support module). The batch file's support module would need to be extended. For Test 1, adding a graph check is a correctness gap. For Test 2, verifying *no* graph edges exist (since no file has ≥2 concepts in the same file) would be a completeness improvement.

---

## Recommendations

1. **MERGE Test 1** (`remove_file_cleans_content_and_mentions`) into `sync_unification_single_file.rs`. Before merging, add the missing `concept_graph` edge check (expose `get_edge_source_files` from the unification lifecycle support, or add it to the shared `sync_unification/support.rs`). Verify whether the lifecycle file's delete test (`incremental_sync_file_deleted_cleans_all_artifacts`) makes this test redundant at the DB-coherence layer; if so, prefer REMOVE over MERGE to reduce duplication.

2. **REWRITE Test 2** (`sync_batch_processes_multiple_files`):
   - Replace `output.contains("- 3 FILES processed")` with a more resilient check that handles case variation: `(output.contains("files processed") || output.contains("FILES processed")) && output.contains("3")`.
   - Optionally add negative assertion for cross-file concept isolation: `assert_eq!(get_mention_count(&cfg, &file_uri1, &normalize_concept(concept_b)), None, "file1 must not reference file2's concept")`.
   - Add `get_edge_source_files` assertion confirming no graph edges exist (single-concept-per-file means no co-occurrence).

3. **KEEP Test 3** (`sync_empty_memory_directory_returns_without_error`) as-is. The only viable improvement is to extract the output-format strings into named constants if/when that refactor is done codebase-wide.

4. **Fix the traceability annotation** (×3 occurrences). Replace `// T-5.7: RTM FR-14.3` with the applicable current `RTM-NNN` identifier, or add a note: `// RTM: archived — see CHANGELOG.md`. Coordinate with the same fix needed in `sync_unification_single_file.rs` and `incremental_sync_unification_lifecycle.rs`.

5. **Rename or re-document the file.** Either rename `sync_unification_batch.rs` to `sync_unification_batch_and_delete.rs` (to reflect its mixed single-file + batch content), or, after migrating Test 1, verify the remaining two tests genuinely represent "batch" semantics and add a module-level comment explaining the file's mandate.

6. **Add `get_edge_source_files` to the shared support module** (`sync_unification/support.rs`). Currently the support only exposes `get_file_content_row` and `get_mention_count`. Graph-level assertions require a third helper. This would enable both this file and `sync_unification_single_file.rs` to verify `concept_graph` state without importing the lifecycle-specific support module.

7. **Document the mutex rationale** in `support.rs`. Add a comment: `// guards Config::override_root() global state — not DB or filesystem contention`. Prevents future removal during an isolation refactor.

---

## Load-Bearing Score: 2/3

| Test | Load-Bearing | Reasoning |
|---|---|---|
| `remove_file_cleans_content_and_mentions` | ⚠️ Partial (0.5) | Duplicates the lifecycle file's delete test but with fewer assertions. Removing it leaves the lifecycle test as sole guard for deletion DB-coherence — acceptable but slightly reduced redundancy. If the lifecycle test is ever removed, this test's absence would be felt. |
| `sync_batch_processes_multiple_files` | ✅ Full | Only test that exercises `sync_all` with DB-row introspection. Removing it leaves batch sync coverage to output-string-only tests (`sync_perf_baseline.rs`), which provide no row-level guarantees. |
| `sync_empty_memory_directory_returns_without_error` | ✅ Full | Sole guard for the empty-directory edge case. Removing it leaves zero-file `sync_all` unguarded. |

**Effective load-bearing per DB table (after accounting for Test 1's `sync_one` usage):**

| Table | Test 1 (delete via sync_one) | Test 2 (batch via sync_all) | Test 3 (empty) |
|---|---|---|---|
| `file_content` | ✅ Deletion verified | ✅ Creation verified | ➖ N/A |
| `concept_mentions` | ✅ Deletion verified | ✅ Creation verified | ➖ N/A |
| `concept_graph` | ❌ Not checked | ❌ Not checked | ➖ N/A |

The `concept_graph` table is entirely unverified in this file. Any regression in graph-edge creation or deletion during batch sync would not be caught here. The lifecycle suite partially compensates, but only for `sync_single_file` operations — `sync_all`-specific graph behavior is unguarded.

> **Summary verdict:** 2 of 3 tests are genuinely load-bearing. 1 is a weaker duplicate of a better test elsewhere. 0 of 3 tests guard `concept_graph` correctness. Effective load-bearing against [[maenifold-rs]] [[test-quality]] standards: **2/3** for DB completeness, **0/3** for graph coherence.
