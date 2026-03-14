# Test Evaluation: sync_mtime_skip.rs

**Traceability**: T-2.3 · FR-14.1 · FR-14.2 · FR-14.3  
**Evaluator role**: Gray (Skeptical Inquiry)  
**File under review**: `crates/maenifold-core/tests/sync_mtime_skip.rs`  
**Related file**: `crates/maenifold-core/tests/sync_mtime_opt_guards.rs` (T-12.0 / NFR-1)

---

## Summary

This file contains three integration tests for the mtime-based sync optimization path. The
optimization has two tiers: (1) an mtime guard that skips reading the file entirely when the
filesystem timestamp is unchanged, and (2) a hash guard that updates `last_indexed` without
reprocessing when the mtime moved but the MD5 is the same. A third test covers the genuine
content-change path where full reprocessing — including new concept capture — must occur.

All three tests operate at the **`sync_all` level** (via `search::sync()`), which distinguishes
them from the single-file-level tests in `sync_mtime_opt_guards.rs`. They share a common
sabotage pattern: `tamper_db_delete_mentions` removes `concept_mentions` rows, then the test
asserts they are *not* restored on skip, or *are* restored on full reprocess. This is effective.

**Overall posture**: Reasonably well-constructed. The tests are not redundant with each other,
but there is meaningful overlap with `sync_mtime_opt_guards.rs`. The Unix-only test has a latent
fragility. The boilerplate repetition is tolerable but parameterisable.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterisable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix` | ✅ Yes | ⚠️ Partial | ⚠️ Partial | ✅ Yes | ⚠️ Partial | **REWRITE** |
| 2 | `sync_when_mtime_bumped_but_content_same_updates_last_indexed_only_and_does_not_restore_mentions` | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |
| 3 | `sync_when_content_changed_reprocesses_and_restores_mentions_including_new_concept` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partial | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix`

**Verdict: REWRITE**

**What it does**: Baseline-syncs a file, sets the stored mtime back to match the filesystem,
sabotages `concept_mentions`, `chmod(0)`s the file to make it unreadable, then asserts that a
subsequent `sync_all` neither reads the file (which would error) nor restores the deleted
mentions.

**Load-bearing?** Yes. This test provides the only integration-level proof that the mtime guard
at `sync_all` depth genuinely skips the I/O path — not just the processing path. The use of
`chmod(0)` as a read-impossibility oracle is clever: if the file were read, the sync would fail
rather than silently pass, making the skip observable.

**Unique?** Partially. `sync_mtime_opt_guards.rs::process_file_changed_when_mtime_unchanged_skips_before_read_and_does_not_restore_state`
covers the same logical scenario at single-file level and asserts the "mtime guard" log string.
This test exercises the same invariant from the other end of the call stack (`sync_all` vs
`process_file_changed`) and uses a different oracle technique (file unreadability vs log string).
The difference is real but thin.

**Correct assertions?** Partially.

- ✅ `assert_eq!(file_mtime(&file_path), original_mtime, "making file unreadable must not change mtime")` — defensive sanity check embedded in setup; good practice.
- ✅ `last_indexed` unchanged — correct guard for "no processing occurred".
- ✅ Both mention counts remain `None` — correct.
- ⚠️ There is **no assertion that `sync_all` returns without error** — oh wait, there is: `assert!(output.contains("Sync complete:"), ...)`. This is fine.
- ⚠️ Missing: an assertion that `file_size` or `file_md5` is also unchanged. The `last_indexed`
  assertion is the primary signal, but a paranoid gray reading asks: could the sync update `file_md5`
  without triggering the skip? The production code (`should_skip_by_mtime`) skips before reading, so
  no, but the test does not verify this explicitly.

**Fragility**:

1. `chmod(0)` is ineffective when tests run as root (e.g., Docker containers on CI without
   `--no-new-privileges`). On such systems, the test passes even if the mtime guard is broken,
   because root can read mode-0 files. The test should assert this precondition or skip with
   `#[cfg_attr(..., ignore)]` when `nix::unistd::geteuid().is_root()`.
2. The `set_file_mtime_system_time` call after `chmod(0)` is defensive (in case `chmod` itself
   updates mtime on the platform). The preceding `assert_eq!` checks that chmod didn't change
   mtime, then the test proceeds *without* a second `set_file_mtime` — this is actually correct,
   but the order is slightly confusing to read. The assertion acts as a guard, not a corrective.
3. `#[cfg(unix)]` leaves Windows users with zero coverage of the mtime-skip path at this
   integration depth. The Windows path would need a different unreadability oracle (ACL-based or
   via file locking).

**Parameterisable?** Partially. The scenario is distinct enough (unreadable file + mtime skip)
that it does not naturally collapse into a parameterised table alongside tests 2 and 3 — the
setup diverges significantly at the chmod step.

**Recommended rewrite**: Replace `chmod(0)` oracle with a `#[cfg(unix)]` block that checks
`!nix::unistd::geteuid().is_root()` before proceeding, and supplement with an assertion that
`file_md5` is also unchanged. Add a non-unix variant that infers skip from `last_indexed`
remaining stable without needing unreadability.

---

### Test 2 — `sync_when_mtime_bumped_but_content_same_updates_last_indexed_only_and_does_not_restore_mentions`

**Verdict: KEEP**

**What it does**: Baseline-syncs a file, sabotages mentions, artificially advances the filesystem
mtime by +120 seconds while leaving file content unchanged, then asserts that `sync_all` updates
`last_indexed` to the new mtime and preserves `file_md5`, but does *not* restore the deleted
mentions (hash guard fires).

**Load-bearing?** Yes. This is the only integration-depth test of the hash guard path specifically
at `sync_all` level. The combination of assertions — `last_indexed` updated, `file_md5` preserved,
mentions *not* restored — is precise and sufficient to verify the hash-guard contract.

**Unique?** Partially. `sync_mtime_opt_guards.rs::process_file_changed_when_mtime_differs_but_hash_same_updates_last_indexed_only`
covers the same contract at single-file level. Both are valuable; the `sync_all` depth can reveal
integration-layer bugs (transaction scoping, edge pruning during full sync) that single-file tests
miss.

**Correct assertions?** Yes.

- ✅ `after.last_indexed == Some(actual_new_mtime)` — pinned to the *filesystem* mtime, not a wall-clock capture. This is important: it tests that `indexed_timestamp_for_metadata` is used correctly.
- ✅ `after.file_md5 == before.file_md5` — confirms the hash guard didn't clobber existing state.
- ✅ Both mention counts remain `None` — confirms no reprocess occurred.

**Naming**: Accurate. Somewhat long (97 chars) but consistent with project conventions and grep-searchable.

**Parameterisable?** Yes. The core setup (baseline sync, tamper, mtime-bump, sync again, assert DB row) is structurally identical to test 3, differing only in whether file content changes. This pair would benefit from a shared fixture helper or table-driven approach, though Rust's test framework makes parameterisation verbose. Low priority given test count is only three.

**Minor concern**: `mtime_plus(&file_path, 120)` picks 120 seconds somewhat arbitrarily. The test
would be more self-documenting with a named constant (`const MTIME_BUMP_SECS: u64 = 120`). Not a
correctness issue.

---

### Test 3 — `sync_when_content_changed_reprocesses_and_restores_mentions_including_new_concept`

**Verdict: KEEP**

**What it does**: Baseline-syncs a file with two concepts, sabotages mentions, rewrites the file
to add a third concept, advances the mtime, then asserts full reprocessing occurred: all three
concept mentions are present, `last_indexed` is updated, and `file_md5` has changed.

**Load-bearing?** Yes, and this is the strongest of the three tests. The phrase "including new
concept" is doing real work: it verifies that `upsert_processed_file` correctly handles concept
*additions* during reprocess, not just restoration of prior state. An implementation that
naively re-inserted the old `file_md5` and skipped `extract_wikilinks` on hash mismatch would
break this test.

**Unique?** Yes. The new-concept addition (`concept_c`) is not present in `sync_mtime_opt_guards.rs`
at the level of `sync_all`. This test provides the only integration-depth proof that the full
reprocess path correctly captures incremental additions.

**Correct assertions?** Yes.

- ✅ `assert_ne!(after.file_md5, before.file_md5)` — confirms the hash changed (content was updated). The `assert_ne!` form is correct here; an `assert_eq!` to a known MD5 would be brittle.
- ✅ `after.last_indexed == Some(new_mtime)` — pinned to filesystem mtime post-rewrite.
- ✅ All three concepts return `Some(1)` — concept_a and concept_b are *restored*, concept_c is *new*.
- ✅ Failure messages are descriptive.

**Naming**: Good. The "including new concept" suffix is meaningful — it distinguishes this from a
test that only checked restoration of pre-existing mentions.

**Minor concern**: After `std::fs::write(...)` the test calls `set_file_mtime_system_time(&file_path, mtime_plus(&file_path, 120))`. `std::fs::write` already updates the filesystem mtime automatically on all platforms. The explicit mtime-set is redundant *unless* the test is trying to ensure the mtime advance is large enough to clear sub-second resolution ambiguity. This should be documented with a comment explaining the intent.

**Parameterisable?** Partially. The setup diverges at the fs::write step; collapsing this with test 2 into a parameterised table would reduce clarity without meaningfully reducing lines.

---

## Overlap Analysis

| Scenario | `sync_mtime_skip.rs` (T-2.3) | `sync_mtime_opt_guards.rs` (T-12.0) |
|----------|-------------------------------|---------------------------------------|
| mtime unchanged → skip | Test 1 via `sync_all` + chmod oracle | `process_file_changed_...mtime_unchanged...` via `sync_one` + log string oracle |
| mtime bumped, hash same → last_indexed only | Test 2 via `sync_all` | `process_file_changed_...hash_same...` via `sync_one` |
| content changed → full reprocess | Test 3 via `sync_all` | Likely present (file was cut off in inspection) |

The overlap is real but justified by integration depth difference. `sync_all` exercises the
transaction boundary, edge pruning, and full-scan loop that `sync_one` does not. The tests are
complementary, not duplicates. However, if a future test count audit demands culling, Tests 1 and
2 in this file are weaker candidates for removal than Test 3.

---

## Recommendations

1. **[Test 1 — High Priority]** Guard against root-user CI environments by checking
   `!nix::unistd::geteuid().is_root()` before the chmod oracle. Add a comment explaining that the
   chmod technique is the read-impossibility oracle, not a file-system permissions test. Consider
   promoting this to a non-Unix variant that asserts skip via `last_indexed` stability alone.

2. **[Test 1 — Medium Priority]** Add an assertion that `file_md5` is also unchanged after the
   mtime-skip to close the implicit gap: the current assertions confirm skip via `last_indexed` and
   mentions, but not via the hash field.

3. **[Test 3 — Low Priority]** Document the explicit `set_file_mtime_system_time` call after
   `std::fs::write` with a comment explaining whether it compensates for sub-second resolution or is
   simply belt-and-suspenders. If the latter, it could be removed to simplify.

4. **[All tests — Low Priority]** Extract the repeated baseline setup (mutex, reset, env, two
   concepts, create file, sync, assert "Sync complete:") into a named `setup_baseline_with_two_concepts`
   helper in `support.rs`. This reduces per-test boilerplate from ~15 lines to ~3 and makes
   divergence between tests more legible.

5. **[All tests — Informational]** The `mtime_plus(path, 120)` magic number appears in tests 2 and 3.
   Move to a shared constant in `support.rs` with a name like `MTIME_ADVANCE_SECS`.

6. **[Traceability — Informational]** Tests reference T-2.3 / FR-14.1-14.3 in comments, but T-2.3
   does not appear as a live TODO item. Confirm these reference the completed FR-14 (which PRD notes
   is in CHANGELOG.md for FR-1 through FR-21). If completed, the comment references are correctly
   historical and no action is needed.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. None should be removed without a replacement. Test 1 should be
rewritten to address the root-user fragility. Tests 2 and 3 are fit for purpose as written.

The file as a whole defends FR-14's three-tier optimization contract (skip-on-unchanged-mtime,
update-only-on-hash-match, full-reprocess-on-content-change) at integration depth. It is a
necessary complement to the unit-level coverage in `sync_mtime_opt_guards.rs`.

---

## SWE Remediation

- REWRITE test 1 (`sync_when_mtime_unchanged_skips_without_read_and_does_not_restore_mentions_on_unix`):
  - Added runtime root-user guard: after `chmod(0)`, attempts `std::fs::read` — if the file is still readable (root/Docker), the test returns early with an explanatory `eprintln!` instead of giving a false pass. This is more robust than checking `geteuid()` because it validates the actual precondition (file unreadability) regardless of cause (root, capabilities, SELinux, etc.). (P0)
  - Added `file_md5` unchanged assertion to close the implicit gap — now verifies skip via `last_indexed`, `file_md5`, and mentions. (P1)
  - Added oracle strategy comment explaining `chmod(0)` is a read-impossibility oracle, not a permissions test. (P0)
  - Restructured initial capture to use `before: FileContentRow` for `file_md5` comparison, and `after` row for grouped assertions.
- KEEP test 2 (`sync_when_mtime_bumped_but_content_same_updates_last_indexed_only_and_does_not_restore_mentions`): no changes
- KEEP test 3 (`sync_when_content_changed_reprocesses_and_restores_mentions_including_new_concept`):
  - Added comment documenting the explicit `set_file_mtime_system_time` after `fs::write` — explains it ensures mtime delta exceeds filesystem timestamp resolution (e.g. HFS+ 1-second granularity). (P2)
- SKIPPED: P3+ baseline helper extraction (non-trivial refactor, 3 tests only), `MTIME_ADVANCE_SECS` constant (informational), traceability audit (informational, no action needed)
- Commit: 6ee49fb
