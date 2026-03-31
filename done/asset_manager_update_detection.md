---
id: asset_manager_update_detection
status: done
pipeline: tdd
---

# Test Evaluation: asset_manager_update_detection

**Traceability:** T-12.7 | RTM: NFR-1 (C# Oracle Parity), NFR-2 (Test Coverage)
**File:** `crates/maenifold-core/tests/asset_manager_update_detection.rs`
**Tests evaluated:** 5
**Evaluator color:** 🔘 Gray — Skeptical Inquiry
**Competing files examined:** `update_assets_boundary.rs` (6 tests, ToolRegistry layer), `asset_manager_errors.rs` (4 tests), `asset_manager_initialize.rs` (2 tests)

---

## Summary

This file was split from the legacy `asset_manager_tests.rs` under T-12.7 to isolate _update-detection_ concerns: dry-run classification (added/updated/unchanged), apply-mode content fidelity, and no-change short-circuit. The split is structurally sound. The call layer is distinct: all five tests invoke `update_assets()` directly via `invoke_update_assets()`, whereas the companion `update_assets_boundary.rs` goes through `ToolRegistry::global().try_invoke()`. That layer difference is legitimate grounds for coexistence.

**However, skeptical inquiry surfaces three concrete problems:**

1. **Tests 1 and 3 test an identical scenario** (new file in source → dry-run) and together cover the same assertions a single well-named test would cover. They exist as two tests because the assertions were split during a refactor that forgot to consolidate.

2. **Test 5's `PackagedAssetGuard` is vestigial.** The "no changes detected" path is proved by the combination of `isolated_config()` + `initialize_assets()` alone — a synthetic file adds no signal. Its presence implies an untested premise ("the no-changes path requires a specific file") that the code does not share.

3. **Tests 1, 2, and 3 each carry assertion gaps** relative to what the production path actually emits. Test 1 omits the `Added files: 1` count check. Test 2 uses a weak `||` disjunction that would pass if the wrong category (Updated instead of Added) were reported. Test 4 omits the `Updated files: 1` count assertion that its section-header assertion demands as a logical companion.

All 5 tests currently pass. None are broken. Two are redundant with each other. One carries unnecessary infrastructure. Three carry incomplete assertion sets. The load-bearing ratio is 4/5 when the full test suite is considered.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `update_assets_dry_run_reports_new_files_without_copying` | ✅ Partial | ⚠️ Overlaps T3 | ⚠️ Missing `Added files: 1` count check | ✅ Yes | 🔘 With T3 | **MERGE → T3** |
| 2 | `update_assets_apply_copies_new_files_to_target` | ✅ Yes | ✅ Yes | ⚠️ Weak OR disjunction allows wrong category | ✅ Yes | 🔘 No | **REWRITE** |
| 3 | `update_assets_new_file_in_source_appears_in_added_files` | ✅ Yes | ✅ Cross-platform section header | ⚠️ Missing "not copied" guard and count check | ⚠️ Name omits section-header intent | 🔘 No | **REWRITE** (absorb T1) |
| 4 | `update_assets_changed_file_appears_in_updated_files` | ✅ Yes | ✅ Cross-platform section header | ⚠️ Missing `Updated files: 1` count; exercises size diff not SHA-only diff | ✅ Yes | 🔘 No | **REWRITE** |
| 5 | `update_assets_identical_file_is_not_reported_as_added_or_updated` | ❌ No | ❌ Fully covered by `update_assets_boundary.rs` | ⚠️ Vestigial `PackagedAssetGuard` adds noise, not signal | ✅ Yes | 🔘 No | **REWRITE** (simplify infrastructure) |

---

## Per-Test Analysis

### Test 1 — `update_assets_dry_run_reports_new_files_without_copying`

**What it does:** Creates a new packaged asset, runs dry-run, asserts file name in output and file NOT copied.

**What it proves:** (a) New file surfaces in output text. (b) Dry-run gate is respected.

**Problems:**
- Assertion (a) is a raw substring match on `file_name` — it would pass even if the file appeared under `### Updated Files` instead of `### Added Files`.
- Missing: `assert!(output.contains("Added files: 1"), ...)` — the count line.
- Test 3 tests the same setup (new file, dry-run) and adds the `### Added Files` section assertion but drops the "not copied" guard. Together they form a complete test; apart, each is incomplete.

**Verdict: MERGE into Test 3.** Absorb the "not copied" `assert!(!exists)` into the rewritten Test 3. The two-file overhead and the cognitive tax of matching pairs of near-duplicate tests outweigh any isolation benefit.

---

### Test 2 — `update_assets_apply_copies_new_files_to_target`

**What it does:** Creates a new packaged asset, runs apply (not dry-run), asserts count summary and byte-exact content in copied file.

**What it proves:** Apply mode actually writes the file with correct content — the strongest assertion in this file.

**Problems:**
- `output.contains("Added files: 1") || output.contains("Updated files: 1")` — this OR would pass silently if the implementation miscategorized the file as Updated. It is defensive against a known flakiness source (test isolation race) rather than asserting the correct behavior. A new file that was never present in the target should always be _Added_, not _Updated_. The OR masks a potential classification bug.
- The same OR appears in `update_assets_boundary.rs::update_assets_non_dry_run_copies_added_files_into_runtime_assets`. If the OR is justified, document why explicitly. If it's a cargo-cult copy, tighten it.

**Verdict: REWRITE** — change to `output.contains("Added files: 1")` (strict), add a comment if a real edge case exists. Keep the content equality assertion — it is the unique load-bearing value in this test.

---

### Test 3 — `update_assets_new_file_in_source_appears_in_added_files`

**What it does:** Creates a new packaged asset, runs dry-run, asserts `### Added Files` section heading and file name present.

**What it proves:** The markdown section header for added files is emitted. Uniquely cross-platform: `asset_manager_errors.rs::update_assets_summary_formats_counts_sections_and_errors` (which also checks `### Added Files`) is `#[cfg(unix)]`, making this the only non-unix-gated section-header test for the added-files path.

**Problems:**
- No "not copied" assertion (the dry-run guard is in Test 1).
- No count line assertion (`Added files: 1`).
- Name says "appears_in_added_files" but the substance is the _section heading_ `### Added Files`, not just any list membership. The name understates the precision of the assertion.

**Verdict: REWRITE** — Rename to `update_assets_dry_run_emits_added_files_section_for_new_source_file`. Absorb T1's "not copied" assertion and the `Added files: 1` count check. This yields a complete, single test covering the new-file dry-run scenario at the direct-fn layer.

---

### Test 4 — `update_assets_changed_file_appears_in_updated_files`

**What it does:** Installs a source file, initializes, then overwrites the runtime asset with a different version. Runs dry-run, asserts `### Updated Files` section and file name.

**What it proves:** The `### Updated Files` section heading is emitted when a file differs. Uniquely cross-platform (same reasoning as Test 3 — the errors summary test is `#[cfg(unix)]`).

**Problems:**
- Setup uses `{"id":"updated","version":2}` (source) vs `{"id":"updated","version":1}` (runtime) — these have **different byte lengths**. The production code checks size first (`source_metadata.len() != target_metadata.len()`), then SHA-256. This test exercises the _size-diff branch_, not the SHA-256-only branch. The SHA-256 path (same size, different content) is tested only in `update_assets_boundary.rs::update_assets_uses_sha256_not_file_size_for_equal_length_content`. That split is acceptable if intentional, but should be explicit.
- Missing: `assert!(output.contains("Updated files: 1"), ...)` — the section heading without a count assertion leaves the count line untested at the direct-fn layer.

**Verdict: REWRITE** — Add the `Updated files: 1` count assertion. Add a comment noting this test intentionally exercises the size-diff branch (SHA-only branch is in boundary tests).

---

### Test 5 — `update_assets_identical_file_is_not_reported_as_added_or_updated`

**What it does:** Creates a synthetic packaged asset, runs `initialize_assets` (which copies it), runs dry-run, asserts `No changes detected`.

**What it proves:** The "no changes" short-circuit fires correctly.

**Problems:**
- This exact scenario is already covered at the ToolRegistry layer by `update_assets_boundary.rs::update_assets_reports_no_changes_when_source_and_target_match`. The direct-fn layer coverage is the only unique aspect.
- The `PackagedAssetGuard` is superfluous. `initialize_assets` copies ALL packaged assets to the isolated temp target — the resulting state is "source == target" for every file regardless of whether a synthetic file was added. The test would produce identical results without the guard. This implies either: (a) the guard was copy-pasted from another test and never questioned, or (b) the author believed the synthetic file was necessary without verifying why.
- The test name claims "not reported as added or updated" but asserts only the positive form ("No changes detected"). A stronger test would also assert `!output.contains("Added files:")` and `!output.contains("Updated files:")` to verify neither category is emitted.

**Verdict: REWRITE** — Remove the `PackagedAssetGuard` boilerplate (or add a comment proving why it is needed). Add the negative assertions for "Added files:" and "Updated files:".

---

## Recommendations

Priority order (highest impact first):

1. **[HIGH] Merge T1 into T3 → single rewritten test `update_assets_dry_run_emits_added_files_section_for_new_source_file`.**
   Combine: `### Added Files` section check + file name check + `Added files: 1` count + `!exists()` dry-run guard. Eliminate the cognitive overhead of matching two half-complete tests.

2. **[HIGH] Rewrite T4 to add `Updated files: 1` count assertion.**
   The section heading alone does not prove the count line is rendered correctly. One line, zero risk.

3. **[MEDIUM] Rewrite T2 to use strict `output.contains("Added files: 1")` (drop OR).**
   A new file that has never existed in the target is definitionally _Added_. If there is a real race condition that causes classification ambiguity, document it; otherwise the OR masks a classification bug.

4. **[MEDIUM] Rewrite T5 to remove the vestigial `PackagedAssetGuard`.**
   `initialize_assets` on an isolated config produces a clean source==target state without help from a synthetic guard. Add negative assertions for "Added files:" and "Updated files:" to give the test real teeth.

5. **[LOW] Consider whether the direct-fn layer (this file) and the ToolRegistry layer (`update_assets_boundary.rs`) should each cover all four scenarios** (new file dry-run, new file apply, changed file dry-run, no-change dry-run). Currently the matrix is uneven: apply+changed is only fully covered at the boundary layer. This isn't a defect, but it is an undocumented asymmetry.

6. **[LOW] Add a comment to T4** explicitly noting that it tests the size-diff branch and that the SHA-256-only (equal-size, different-content) branch lives in `update_assets_boundary.rs`. Without this comment, a reader might add a duplicate SHA-256 test here.

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing Verdict | Reasoning |
|------|---------------------|-----------|
| T1 | ✅ Partial (2/3 assertions needed) | "Not copied" dry-run guard is also in boundary test; file name check is weaker than T3's section-header check; survives because T3 lacks the guard |
| T2 | ✅ Yes | Content-equality assertion on apply is unique at this call layer |
| T3 | ✅ Yes | Cross-platform `### Added Files` section header — the only non-`#[cfg(unix)]` test for this heading |
| T4 | ✅ Yes | Cross-platform `### Updated Files` section header — same reasoning as T3 |
| T5 | ❌ No | Fully subsumed by `update_assets_boundary.rs::update_assets_reports_no_changes_when_source_and_target_match`; vestigial guard adds zero signal |

**T1 + T3 should be merged**: together they are load-bearing; apart, each is a partial test masquerading as a complete one. Post-merge, the effective score becomes **3 distinct load-bearing tests** covering: dry-run new-file (merged), apply content fidelity (T2), dry-run changed-file section heading (T4).
