---
id: T-80.86
title: "Test Evaluation: incremental_sync_unification_lifecycle"
status: done
pipeline: tdd
---

# Test Evaluation: incremental_sync_unification_lifecycle

**File:** `crates/maenifold-core/tests/incremental_sync_unification_lifecycle.rs`
**Companion:** `crates/maenifold-core/tests/incremental_sync_unification_guards.rs`
**Support module:** `crates/maenifold-core/tests/incremental_sync_unification/support.rs`
**Test count:** 3
**Evaluated:** 2025-07-18
**Traceability claim:** T-5.7 · RTM FR-14.1 / FR-14.2 / FR-14.3 *(see traceability findings below)*

---

## Summary

This file tests the three DB-state lifecycle transitions of `sync_single_file()`: create (first index of a new file), delete (file removed from disk), and rename (old URI evicted, new URI indexed). It sits in the "unification" layer of the incremental sync test suite, meaning its mandate is to verify coherence of *all affected DB tables* after each transition — not just that output strings are correct. The companion `incremental_sync_unification_guards.rs` tests the orthogonal mtime/hash guard fast-paths; the two files together cover the complete `sync_single_file()` decision tree.

**What this file does well:** Pre-condition assertions before the delete test; uuid-isolated concept names preventing cross-test contamination; explicit per-table verification (file_content, concept_mentions, concept_graph); the support module is well-factored and shared correctly with the guards file.

**The critical finding is Test 1's name.** `incremental_sync_file_changed_creates_expected_db_artifacts` does not test a file *change* — it tests the first indexing of a brand-new file. The word "changed" is actively misleading and implies a mutation lifecycle that isn't exercised here. This is a documentation bug with real maintenance cost: a developer reading the test suite for coverage gaps will falsely believe the update path is covered here.

**Three systemic weaknesses run across all three tests:**

1. **Missing `concept_graph` edge assertion in Tests 1 and 3.** Test 1 verifies `file_content` and `concept_mentions` but never checks that a co-occurrence edge was created in `concept_graph` between `concept_a` and `concept_b`. Test 3 (rename) verifies `concept_mentions` for the new URI but does not verify that `concept_graph` edges are rebuilt referencing the new URI instead of the old one. The delete test (Test 2) *does* check the graph edge, making the omission in Tests 1 and 3 an inconsistency. The whole-table coherence mandate of the unification layer is not fully met.

2. **Output-string coupling.** All three tests assert on substrings of human-readable output from `sync_single_file()` (e.g., `.contains("concept mentions updated")`, `.contains("removed stale index entry")`). These assertions are currently accurate — the implementation strings do contain these substrings — but they couple the tests to a mutable output format. If a maintainer rewrites the output message and updates the `.contains()` check, the test continues to pass while the behavioral contract is silently untestable. The correct approach is to assert on structured state (already done for DB rows) and treat output strings as informational only, or extract output-format constants.

3. **Traceability annotation is non-standard.** Test comments say `// T-5.7: RTM FR-14.3`. In `RTM.md`, FR-14 is not present — the active requirement list uses `RTM-NNN` identifiers. In `CHANGELOG.md`, FR-14 corresponds to the *CI parity gate* (RTM-080), not incremental sync. The "FR-14.x" sub-numbering appears to be a legacy annotation from a sprint that used a different numbering scheme. The annotation provides no usable traceability today and should be updated to reference current `RTM-NNN` identifiers or removed.

**Relationship to `incremental_sync_boundary_mutations.rs`:** That file tests the same three lifecycle operations (`sync_single_file_processes_a_new_file_correctly`, `deleted_files_remove_their_concepts_from_the_graph`, `sync_single_file_updates_concepts_when_content_changes`) from a behavioral perspective — asserting which concept names exist globally. The unification lifecycle tests assert at the DB-column level (md5, file_size, mention_count, edge source_files JSON). The overlap is justified by the depth difference, but it means any test suite reader must understand the layering to avoid thinking coverage is duplicated. This layering is not documented.

---

## Verdicts

| Test Function | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|---|---|---|---|---|---|
| `incremental_sync_file_changed_creates_expected_db_artifacts` | ✅ Yes | ⚠️ Partial | ⚠️ Missing graph edge | ❌ Misnaming | ✅ Yes | **REWRITE** |
| `incremental_sync_file_deleted_cleans_all_artifacts` | ✅ Yes | ✅ Yes | ✅ Complete | ✅ Yes | ➖ N/A | **KEEP** |
| `incremental_sync_file_renamed_reindexes_under_new_uri` | ✅ Yes | ✅ Yes | ⚠️ Missing graph edge for new URI | ✅ Yes | ➖ N/A | **KEEP** |

---

## Detailed Findings

### Test 1: `incremental_sync_file_changed_creates_expected_db_artifacts`

**What it actually tests:** First-time indexing of a new file. The file has never been synced; `sync_path` is called once; DB artifacts are verified.

**Name problem:** "file_changed" implies an update to a previously-indexed file. The test never syncs the file once, mutates it, then re-syncs — that would be the "changed" lifecycle. This test is the *create* lifecycle. A developer scanning for "does the test suite cover the content-change path?" will incorrectly mark this as covering it. The boundary suite has `sync_single_file_updates_concepts_when_content_changes` which actually tests the change path; this test does not.

**Assertion gap:** After `sync_path` creates the index, the test verifies `file_content` (md5, file_size, row existence) and `concept_mentions` (mention_count for both concepts). It does **not** verify that `concept_graph` has a co-occurrence edge between `normalized_concept(concept_a)` and `normalized_concept(concept_b)`. The sync implementation calls `rebuild_affected_graph_from_mentions` on every full index pass (see `sync/mod.rs` line ~220). If that function had a bug that silently dropped edge creation, this test would not catch it. Test 2 verifies graph edges are *deleted*; Test 1 should symmetrically verify they are *created*.

**Parameterization opportunity:** The setup is identical in structure to Tests 2 and 3. The assertion sequence (output string → file_content row → mention_count × 2 → md5 → file_size) could be extracted as a helper. Not mandatory, but would remove the copy-paste risk.

**Output assertion strength:** `output.contains("concept mentions updated")` is weaker than `output.contains("2 concept mentions updated")`. The implementation produces a count; asserting the count is 2 (matching the file's two WikiLinks) makes the test more specific and catches any over-counting or under-counting regression. The boundary suite (`sync_single_file_processes_a_new_file_correctly`) already asserts the exact count: `output.contains("2 concept mentions updated")`. The unification test is *less* specific than its boundary counterpart on this point — the inverse of what should be true.

**Verdict: REWRITE.** Rename to `incremental_sync_new_file_creates_expected_db_artifacts`. Add `concept_graph` edge assertion using `get_edge_source_files`. Tighten the output assertion to include the count.

---

### Test 2: `incremental_sync_file_deleted_cleans_all_artifacts`

**What it actually tests:** File is indexed, then deleted from disk, then sync is called. All three DB tables (`file_content`, `concept_mentions`, `concept_graph`) are verified to be clean.

**Pre-condition discipline:** The test asserts that artifacts *exist before deletion* before asserting they are gone after. This is the correct pattern — without it, a bug where the first sync fails silently would make the post-deletion assertions vacuously true. This is done correctly here and is absent from some peer tests in the suite.

**Graph edge check:** `get_edge_source_files` returns an `Option<BTreeMap<String, f64>>`. The assertion `is_none_or(|files| !files.contains_key(&memory_uri))` is correct: the edge row may still exist (if other files reference both concepts) but the deleted file's URI must not appear as a source. This is precise and avoids over-asserting.

**Relationship to boundary suite:** `deleted_files_remove_their_concepts_from_the_graph` in `incremental_sync_boundary_mutations.rs` tests the same operation but asserts on `concept_names()` (which concepts exist globally) and `concept_sources()` (per-concept source set), not on the internal JSON of `source_files`. The two tests are complementary, not redundant.

**Verdict: KEEP.** This is the strongest test in the file. Pre-condition assertions, all three DB tables checked, graph edge check uses correct semantics.

---

### Test 3: `incremental_sync_file_renamed_reindexes_under_new_uri`

**What it actually tests:** Rename modeled as delete-old + create-new (the calling convention for `sync_single_file`). Old URI is removed from all tables; new URI is created in `file_content` and `concept_mentions`.

**Unique coverage:** No other test file exercises the rename protocol. This is the sole guard for the rename contract. Load-bearing.

**Missing graph edge verification for new URI:** The test verifies that `concept_mentions` rows exist for the new URI (both concepts). It does **not** verify that a `concept_graph` edge exists between the two concepts referencing the new URI as the source file. After rename, `rebuild_affected_graph_from_mentions` should reconstruct the edge with the new URI in its `source_files` JSON. If this was broken, the graph would have orphaned co-occurrence edges or a missing edge after rename, and this test would not catch it.

The test *does* verify (via the delete step's output assertion and the absence of the old URI in `file_content`/`concept_mentions`) that the old state is cleaned up. But the creation side of the graph is unverified.

**Explicit name prefix on Test 1 vs implicit here:** Test 1 passes `Some(&format!("incr-{unique}"))` to `create_test_file`. Tests 2 and 3 pass `None`, generating a `note-{uuid}` prefix. This is not a bug (all use uuid isolation), but the inconsistency is unexplained. Either the explicit prefix in Test 1 was cargo-culted from an earlier version or there was a reason that was lost.

**Verdict: KEEP.** Unique and load-bearing. Add `get_edge_source_files` assertion for the new URI after re-index, mirroring the edge cleanup check already present in Test 2.

---

## Cross-Cutting Findings

### Finding A: The TEST_MUTEX Is Explained but Undocumented

`TEST_MUTEX.lock()` appears in every test. The mutex is necessary because `Config::override_root()` and `Config::reset_overrides()` mutate a process-global config object — despite each test creating its own `TempDir`. The mutex prevents config-clobbering between concurrent test threads, not database contention (each test has its own DB path). This is non-obvious: a future maintainer might remove the mutex believing the `TempDir` isolation makes it redundant. Add a comment to `support.rs` explaining that the mutex guards `Config::override_root()` global state, not filesystem or DB state.

### Finding B: Traceability Annotation `RTM FR-14.x` Is Stale

`FR-14` in `CHANGELOG.md` corresponds to the CI parity gate (RTM-080). `RTM.md` does not define `FR-14` at all. The incremental sync feature is described in the PRD under behavior specifications with `mtime comparison`, `hash check`, `selective re-index` but the corresponding RTM entries that were delivered appear to be in the RTM-001–RTM-114 range now archived to CHANGELOG. The test annotations should reference a current, traceable identifier — or the comment should note the archived status with a pointer to CHANGELOG.

### Finding C: Support Module Has Dead-Looking Functions That Are Actually Used Elsewhere

`tamper_delete_mentions_and_edges_for_file`, `parse_rfc3339_utc`, and `set_file_timestamp` are defined in this file's `support.rs` but not called by any of the three lifecycle tests. They are called by `incremental_sync_unification_guards.rs`, which shares the module. This is correct architecture. However, a static analysis tool or `#[allow(dead_code)]` audit of this support module within the context of only the lifecycle file would flag these as unused. A module-level comment clarifying that the support module is shared with the guards file would prevent confusion.

---

## Recommendations

1. **REWRITE Test 1** — rename it to `incremental_sync_new_file_creates_expected_db_artifacts` and add:
   - `get_edge_source_files(&cfg, &normalized_concept(&concept_a), &normalized_concept(&concept_b))` — assert the edge exists and contains `memory_uri` as a source.
   - Tighten the output assertion to `output.contains("2 concept mentions updated")` to match the specificity of the boundary suite's equivalent test.

2. **Augment Test 3** — after the `create_output` assertion block, add `get_edge_source_files` for `concept_a`/`concept_b` using the `new_memory_uri`. The edge should exist after re-sync and should reference `new_memory_uri`. Confirm `memory_uri` (old) does not appear in the edge's source_files either.

3. **Fix the traceability annotation.** Replace `// T-5.7: RTM FR-14.3` (×3 occurrences) with the actual archived RTM range or a pointer to the CHANGELOG entry. If a current open RTM item covers incremental sync correctness testing, reference it. If all relevant RTM items are closed, annotate with `// FR archived: see CHANGELOG.md § Sprint N`.

4. **Document the mutex rationale** in `support.rs`. One line: `// guards Config::override_root() process-global state; not DB contention`. This prevents future removal.

5. **Add a module-level doc comment** to the test file explaining the layer distinction: unification tests verify multi-table DB coherence; boundary tests verify behavior contracts. Without this, the suite looks like it duplicates `incremental_sync_boundary_mutations.rs`.

6. **Clarify the name_prefix inconsistency.** Test 1 passes an explicit `name_prefix`; Tests 2 and 3 don't. If this was intentional (e.g., to test a specific URI format), document why. If it was accidental, standardize — either always pass a prefix or never.

7. **Extract output-string substrings as named constants.** Define `SYNC_OUTPUT_INDEXED = "concept mentions updated"` and `SYNC_OUTPUT_DELETED = "removed stale index entry"` in the support module. This makes the coupling explicit and provides a single update point if the implementation output format changes.

---

## Load-Bearing Score: 3/3

All three tests guard distinct lifecycle transitions in `sync_single_file()`. Removing any one of them would leave that transition unguarded at the DB-coherence level — the boundary suite guards behavior at a shallower level and would not catch, for example, a bug where `file_size` is not populated on creation, or where `concept_mentions` rows survive a delete but `file_content` is correctly removed. However, two of the three tests (Tests 1 and 3) have incomplete assertion coverage of `concept_graph`, which means their effective load-bearing value against graph coherence regressions is **0** for that specific table. The 3/3 score reflects that the tests exist and exercise the right paths; the annotation reflects that Tests 1 and 3 have exploitable gaps.

> **Effective load-bearing per table:**
> | Table | Test 1 (Create) | Test 2 (Delete) | Test 3 (Rename) |
> |---|---|---|---|
> | `file_content` | ✅ | ✅ | ✅ |
> | `concept_mentions` | ✅ | ✅ | ✅ |
> | `concept_graph` | ❌ Missing | ✅ | ❌ Missing |

---

## SWE Remediation

- REWRITE `incremental_sync_file_changed_creates_expected_db_artifacts` → `incremental_sync_new_file_creates_expected_db_artifacts`: renamed to reflect actual create lifecycle (not change); added `concept_graph` edge assertion via `get_edge_source_files`; tightened output assertion to `"2 concept mentions updated"`; standardized `name_prefix` to `None` (matching Tests 2 and 3)
- KEEP `incremental_sync_file_deleted_cleans_all_artifacts`: untouched (strongest test)
- KEEP+AUGMENT `incremental_sync_file_renamed_reindexes_under_new_uri`: added `concept_graph` edge assertions verifying new URI present and old URI absent in `source_files` after rename
- Fixed stale `RTM FR-14.x` traceability annotations → `FR archived — see CHANGELOG.md § FR-14 (RTM-080)` across lifecycle, guards, and support files
- Documented `TEST_MUTEX` rationale in `support.rs` (guards `Config::override_root()` process-global state)
- Added module-level doc comment to lifecycle file explaining unification vs boundary layer distinction
- Commit: 38fd7aa

---

## Red-team Audit

### REWRITE: `incremental_sync_new_file_creates_expected_db_artifacts` (was `incremental_sync_file_changed_creates_expected_db_artifacts`)

- ✅ **Rename**: Function name changed from `file_changed` → `new_file`, matching eval Recommendation 1. Correctly reflects the create lifecycle, not an update.
- ✅ **`concept_graph` edge assertion**: `get_edge_source_files()` call at lines 73-82 verifies edge exists and `memory_uri` is present in `source_files`. Closes the gap identified in Detailed Findings § Test 1 and the load-bearing table's `concept_graph` ❌ for Test 1.
- ✅ **Output assertion tightened**: `output.contains("2 concept mentions updated")` at line 34 — matches the eval's recommendation to assert the count, achieving parity with the boundary suite's specificity.
- ✅ **`name_prefix` standardized**: `create_test_file(&cfg, &content, None)` at line 30 — matches Tests 2 and 3, resolving Recommendation 6.

### KEEP: `incremental_sync_file_deleted_cleans_all_artifacts`

- ✅ **Untouched** except traceability annotation update. All original assertions preserved. This was already the strongest test; no changes required by the eval.

### KEEP+AUGMENT: `incremental_sync_file_renamed_reindexes_under_new_uri`

- ✅ **`concept_graph` edge for new URI**: Lines 205-214 assert edge exists and `new_memory_uri` is in `source_files`. Matches eval Recommendation 2.
- ✅ **Old URI absent from edge**: Lines 215-219 assert `memory_uri` (old) is NOT in `source_files`. This goes beyond the minimum eval spec (which said "confirm old does not appear") — the SWE added a separate assertion for old-URI absence rather than combining it into one check. Correct and thorough.

### Cross-Cutting Items

- ✅ **Traceability annotations fixed**: All 3 lifecycle test comments + 2 guards test comments + support.rs header now use `FR archived — see CHANGELOG.md § FR-14 (RTM-080)` instead of stale `RTM FR-14.x`. Covers eval Recommendation 3 and Finding B.
- ✅ **TEST_MUTEX rationale documented**: Lines 19-22 of `support.rs` explain Config::override_root() global state guard. Covers eval Recommendation 4 and Finding A.
- ✅ **Module-level doc comment**: Lines 1-7 of lifecycle file explain unification vs boundary layer distinction. Covers eval Recommendation 5.
- ✅ **Shared module documented**: Support.rs header now states "Shared by both lifecycle and guards test files." Addresses Finding C.
- ℹ️ **Rec 7 (output-string constants) not implemented**: The eval suggested extracting `SYNC_OUTPUT_INDEXED`/`SYNC_OUTPUT_DELETED` as named constants. This was a P2 nice-to-have, not a verdict-required change. Not blocking.

### Test Execution

- ✅ All 3 lifecycle tests pass: `incremental_sync_new_file_creates_expected_db_artifacts`, `incremental_sync_file_deleted_cleans_all_artifacts`, `incremental_sync_file_renamed_reindexes_under_new_uri`
- ✅ Both companion guards tests pass: `incremental_sync_mtime_guard_skips_unchanged_files`, `incremental_sync_hash_guard_updates_last_indexed_only`
- ✅ No regressions in broader incremental sync suite (25/25 pass)

### No Unrelated Changes

- Commit 38fd7aa touches exactly 3 files, all test files in the unification suite. No source code changes. No unrelated test modifications. Scope is clean.

### Effective Load-Bearing After Remediation

| Table | Test 1 (Create) | Test 2 (Delete) | Test 3 (Rename) |
|---|---|---|---|
| `file_content` | ✅ | ✅ | ✅ |
| `concept_mentions` | ✅ | ✅ | ✅ |
| `concept_graph` | ✅ Fixed | ✅ | ✅ Fixed |

- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_unification_lifecycle`: 3 passed, 0 failed
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_unification_guards`: 2 passed, 0 failed
- All 5 tests (3 lifecycle + 2 companion guards) pass cleanly
- Verdict: **PASS**
