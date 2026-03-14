# Test Evaluation: proximity_weight_cleanup

**File:** `crates/maenifold-core/tests/proximity_weight_cleanup.rs`
**Test count:** 2
**Evaluated:** 2026-06-30
**Traceability:** T-4.3 / RTM FR-23.5 _(see traceability concern below)_
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions.

---

## Summary

The smallest file in the proximity-weight suite. Two integration tests cover a single behavioral axis: **what happens to a graph edge when the file(s) backing it are physically deleted?** Both tests run the full sync pipeline via `invoke_sync()`, assert pre- and post-conditions on the database, and use the shared file-lock serialization workaround.

The tests are load-bearing and cover a scenario not duplicated in `concept_sync_edge_lifecycle.rs` (which tests concept-removal-by-edit via incremental sync, not file-deletion via full sync). However, the file has critical gaps: only 2 of at least 5 meaningful deletion scenarios are covered, one assertion in Test 2 is vacuously true, and magic numeric constants lack documentation. The most significant structural concern is a **traceability collision**: `FR-23.5` does not exist in RTM.md — `FR-23` there refers to Streamable HTTP Transport.

---

## Verdicts

| Test Function | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|:---:|:---:|:---:|:---:|:---:|---|
| `sync_removes_edge_when_last_source_file_is_deleted` | ✅ | ✅ | ⚠️ Partial | ✅ | Low priority | **KEEP** |
| `sync_preserves_remaining_source_file_weights_after_partial_delete` | ✅ | ✅ | ⚠️ Noise present | ✅ | Medium priority | **REWRITE** |

---

## Per-Test Analysis

### `sync_removes_edge_when_last_source_file_is_deleted`

**What it does:** Creates one file contributing a `[[alpha]]`–`[[beta]]` edge, runs full sync to assert precondition, physically deletes the file, runs full sync again, asserts the edge is gone.

**Is it load-bearing?**  
Yes. This is the core regression test for the cleanup path: when `rebuild_affected_graph_from_mentions` finds no `source_files` entries surviving for a concept pair, the edge row must be deleted (via `DELETE FROM concept_graph WHERE concept_a = ?1 AND concept_b = ?2`). No other file in the suite tests this exact trigger — `concept_sync_edge_lifecycle::last_contributor_removed_deletes_edge_row` (T-5.7, FR-14.3) is mechanically different: it uses `sync_one()` + file-content edit, not `invoke_sync()` + `fs::remove_file`. Different code path, different precondition.

**Is it unique?**  
Yes. The distinction between "edit file to remove concepts" and "physically delete the file" is not cosmetic. File deletion removes the entry from `concept_mentions` entirely, whereas editing a file triggers incremental sync. These exercise different branches of the sync pipeline.

**Are the assertions correct?**  
Mostly. Three concerns:
1. **Missing assertion: residual `source_files` state.** After deletion, `!relation_exists` confirms the row is gone, but the test does not verify that the `concepts` table is also pruned (no orphaned `alpha` or `beta` nodes remain). If `prune_orphaned_concepts` regresses silently, this test will still pass.
2. **No assertion on the output of the second sync.** The test asserts `sync.contains("Sync complete:")` for both syncs — this is a signal-of-life check, not a behavioral assertion. It cannot distinguish a clean sync from a sync that encountered errors but still emitted the completion message.
3. **Precondition assertion is correct and appropriately placed.** `assert!(relation_exists(...), "precondition: edge must exist")` is good practice; it prevents the test from producing a false positive if the first sync fails silently.

**Is the name good?**  
Yes. The `verb_noun_when_condition` pattern is clear and matches the behavior under test.

**Verdict: KEEP.** Add an assertion that orphaned concept nodes are pruned, or document explicitly that concept pruning is out of scope for this test.

---

### `sync_preserves_remaining_source_file_weights_after_partial_delete`

**What it does:** Creates two files both contributing the same `[[alpha]]`–`[[beta]]` edge, runs full sync to assert a precondition, deletes one file, runs full sync again, then asserts: (a) the `source_files` column is a JSON object, (b) the deleted file's URI is absent, (c) the kept file's weight is `1.0`, (d) `co_occurrence_count` is `1.0`.

**Is it load-bearing?**  
Yes. This is the only test that verifies the partial-delete case: when a multi-source edge loses one contributor, the remaining contributor's weight must survive intact. If the sync accidentally rebuilds the edge from scratch (e.g., emitting `0.5` instead of `1.0` because section context is lost), only this test would catch it.

**Is it unique?**  
Yes. Test 1 only tests the total-deletion case. No other file tests the partial-deletion case.

**Are the assertions correct?**  
The structure is correct, but the signal-to-noise ratio is poor:

1. **`source_files.is_object()` is vacuously true.** If `relation_source_files()` returns a JSON value that is not an object, the test harness panics at `serde_json::from_str(&raw).expect("source_files should be valid json")` before this assertion is even reached. This assertion tests the support helper's invariant, not production code behavior. It should be removed.

2. **Magic constant `1.0` for weight has no comment.** The value `1.0` is produced by the proximity weight algorithm when both concepts appear in the _same named section_. Without a comment, a reader unfamiliar with the section-weight semantics (weight `1.0` for same-section, `0.5` for cross-section) cannot verify whether `1.0` is the _correct_ expected value or an accidental constant. The file content uses `## Identity` with both `[[alpha]]` and `[[beta]]` in the same section — a comment like `// same-section pair → weight 1.0` would make the intent auditable.

3. **Magic constant `1.0` for `co_occurrence_count` has no comment.** Same problem. After one of two equal-weight files is deleted, the remaining count is the sum of the kept file's proximity weights. A comment is needed.

4. **No assertion that the deleted URI has no ghost entry.** `source_files.get(&remove_uri).is_none()` correctly asserts absence at the key level, but it does not distinguish between "the key was removed" and "the source_files column was entirely rebuilt." This is fine given the precondition setup, but fragile if the sync implementation changes to a different cleanup strategy.

**Is the name good?**  
Yes. Verbose but precise.

**Verdict: REWRITE.** Drop the `is_object()` assertion. Add inline comments explaining the `1.0` values in terms of the section-weight algorithm. Consider adding a cross-section variant (weight `0.5`) as a second scenario to confirm that partial deletion preserves the _actual_ computed weight, not just a fixed constant.

---

## Recommendations

### 1. Fix the traceability collision (Critical)

Both tests are tagged `// T-4.3: RTM FR-23.5`. In the current RTM.md, `FR-23` refers to **Streamable HTTP Transport** (`mcp/http.rs`), not proximity weight cleanup. `FR-23.5` does not appear anywhere in RTM.md. This is either:

- A numbering collision from a previous sprint where the proximity weight requirements consumed FR-23.x before HTTP transport was assigned FR-23, or  
- An RTM staleness issue where the proximity weight requirements were retired without updating the in-code tags.

Either way, the tags are currently misleading. Any automated traceability tooling that links `FR-23.5` test coverage to the HTTP transport feature will report a false positive. The tags should be corrected to reference the actual backing requirement, or the RTM should be updated to define FR-23.5 as a proximity-weight sub-requirement explicitly.

### 2. Expand coverage — at least three scenarios are untested

The current two tests leave the following meaningful deletion behaviors unvalidated:

| Missing Scenario | Risk if Untested |
|---|---|
| File contributing to _multiple_ distinct edges is deleted | Partial cleanup — only some edges are removed |
| Three files back an edge; two are deleted | Count/weight recalculation error when dropping from 3 → 1 |
| File is renamed (effectively: old URI deleted, new URI added) | Edge duplicated under two URIs |
| File in a subdirectory is deleted (path encoding) | URI construction difference breaks lookup |

### 3. Remove or promote the vacuous `is_object()` assertion in Test 2

It currently lives in production test code but tests a test-harness invariant. If the intent is to document that `source_files` must be an object (not a scalar or array), move that contract to a dedicated unit test of the serialization layer, not an integration test that happens to check it in passing.

### 4. Consider unit-testing the cleanup mechanism directly

Both tests are integration tests that exercise the cleanup path only via the full `invoke_sync()` pipeline. If `rebuild_affected_graph_from_mentions` is ever moved, renamed, or conditionally bypassed, these tests will stop exercising the cleanup logic without failing, because they test outcomes, not the mechanism. A unit test against `rebuild_affected_graph_from_mentions` with a pre-populated database would provide faster feedback and more precise attribution on failure.

### 5. Address the file-lock serialization smell (inherited from the suite)

Both tests call `acquire_test_lock()` — a busy-poll file lock on `/tmp/maenifold-rs-proximity-weight-tests.lock`. If a test panics while holding the lock, all subsequent tests in this suite spin for 30 seconds before panicking. This is a fragile approach to test isolation that should be replaced with `Config::isolated()` + `ToolRegistry` scoping.

---

## Deeper Concerns

### The test file header tag is misplaced

```rust
// T-12.0: RTM NFR-1
#[path = "proximity_weight/support.rs"]
mod support;
```

`T-12.0: RTM NFR-1` appears here and on the support module in several test suites. It appears to be a legacy tag for the test infrastructure itself, not the feature under test. It does not describe what `proximity_weight_cleanup.rs` tests. Having an NFR-1 tag on a test file header creates the impression that the test file validates NFR-1 (test coverage / performance), when it actually validates FR-23.5 behavior.

### Low coverage count for the stated scope

Two tests for an entire file-deletion cleanup subsystem is thin. The fact that this file exists as a standalone `proximity_weight_cleanup.rs` (vs being folded into `proximity_weight_sections.rs` or `concept_sync_edge_lifecycle.rs`) implies an intent to house a _cluster_ of deletion scenarios here. The current state looks like a scaffold awaiting additional tests that were never written.

---

## Load-Bearing Score: 2/2

Both tests are load-bearing — no test should be removed without a replacement covering the same scenario. However, the file's **coverage breadth** warrants a separate assessment: only 2 of approximately 5 meaningful deletion scenarios are tested. The load-bearing score measures what exists; it does not imply that what exists is sufficient.

---

## SWE Remediation

- **KEEP** `sync_removes_edge_when_last_source_file_is_deleted`: no logic changes; added traceability collision note on FR-23.5 comment
- **REWRITE** `sync_preserves_remaining_source_file_weights_after_partial_delete`:
  - Removed vacuous `is_object()` assertion (tested harness invariant, not production code)
  - Added inline comments documenting: same-section pair → weight 1.0; co_occurrence_count = distinct file count
  - Improved assertion message ("removed uri must disappear from source_files")
- **ADD** `sync_preserves_cross_section_weight_after_partial_delete`: new cross-section variant confirms partial deletion preserves actual computed weight (0.5) rather than a fixed constant (1.0), strengthening confidence that cleanup preserves algorithm-derived weights
- **Traceability collision (Rec #1)**: FR-23.5 does not exist in RTM.md (FR-23 = HTTP Transport). Added NOTE comments on all three test traceability tags flagging the collision for PM RTM reconciliation. SWE cannot resolve without PM updating RTM.md.
- Commit: b91b56d

---

## Red-team Audit

### Per-change verification (commit b91b56d)

- ✅ **KEEP** `sync_removes_edge_when_last_source_file_is_deleted`: Test body unchanged. Only the traceability comment was updated to flag the FR-23.5 collision — no logic, assertions, or behavioral changes introduced. Matches eval KEEP verdict.
- ✅ **REWRITE** `sync_preserves_remaining_source_file_weights_after_partial_delete`:
  - Vacuous `is_object()` assertion removed (eval Rec #3 / REWRITE directive) ✅
  - Inline comment on file content: `"Both concepts in the same named section → proximity weight 1.0"` added to both `write_memory_file` calls (eval REWRITE directive: document `1.0` in terms of section-weight algorithm) ✅
  - Inline comment on weight assertion: `"Same-section pair → proximity weight 1.0 per file."` (eval REWRITE directive) ✅
  - Inline comment on count assertion: `"co_occurrence_count = number of distinct contributing files = 1"` (eval REWRITE directive) ✅
  - Assertion message improved from `"removed uri must disappear"` → `"removed uri must disappear from source_files"` — minor clarity improvement, no semantic risk ✅
  - No assertions lost: all 3 surviving assertions (`get(&remove_uri).is_none()`, `assert_approx_eq` on weight, `assert_approx_eq` on count) preserved from original ✅
- ✅ **ADD** `sync_preserves_cross_section_weight_after_partial_delete`: New test directly implements eval recommendation "Consider adding a cross-section variant (weight 0.5) as a second scenario." The setup is correct: remove-file has both `[[gamma]]`/`[[delta]]` in one section (weight 1.0), keep-file has them in different sections (weight 0.5). After deletion, asserts kept weight is 0.5 — confirming cleanup preserves algorithm-derived weight, not a fixed constant. Uses distinct concepts (`gamma`/`delta`) to avoid cross-contamination with test 1 and 2's `alpha`/`beta`. ✅
- ✅ **Traceability collision (Rec #1)**: NOTE comments added to all three test traceability tags flagging `FR-23.5` collision with HTTP Transport. SWE correctly scoped this as a PM-level RTM reconciliation item — they cannot unilaterally reassign FR numbers. ✅
- ✅ **No unrelated changes**: Commit touches exactly 1 file (`proximity_weight_cleanup.rs`), 59 insertions, 7 deletions. All changes trace to eval directives. ✅
- ✅ **Tests pass**: All 3 tests pass (`cargo test -p maenifold-core proximity_weight_cleanup` — 3 passed, 0 failed, 1.38s). ✅

### Items noted but not grounds for rejection

- The eval's KEEP verdict for test 1 suggested "Add an assertion that orphaned concept nodes are pruned, or document explicitly that concept pruning is out of scope." The SWE did neither. This is acceptable — the eval framed it as a suggestion under a KEEP verdict, not a mandatory REWRITE directive. It remains a valid future improvement.
- The `T-12.0: RTM NFR-1` header tag (line 1) was flagged in the eval's "Deeper Concerns" as misplaced. The SWE did not address it. This is acceptable — the eval's Deeper Concerns section is advisory, not part of the formal verdicts table.
- The `acquire_test_lock()` smell (Rec #5) was not addressed. Also acceptable — it is a suite-wide concern, not scoped to this eval's KEEP/REWRITE verdicts.

### Verdict: **PASS**

All eval verdicts (KEEP × 1, REWRITE × 1) are correctly implemented. The bonus ADD test is well-designed and directly addresses an eval recommendation. No unrelated changes. Tests green.

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- proximity_weight_cleanup`: 3 passed, 0 failed (1.95s)
- Verdict: PASS
