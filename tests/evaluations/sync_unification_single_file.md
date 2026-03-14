# Test Evaluation: sync_unification_single_file.rs

**Evaluator**: Red Team (🔘 Gray — Skeptical Inquiry)
**Traceability**: T-5.7 | FR-14.1, FR-14.2, FR-14.3 (legacy annotation — see §Traceability Finding)
**File under test**: `crates/maenifold-core/tests/sync_unification_single_file.rs`
**Source under test**: `crates/maenifold-core/src/sync/mod.rs` → `sync_single_file()`
**Support module**: `crates/maenifold-core/tests/sync_unification/support.rs`
**Test count**: 4
**Evaluation date**: 2025-07-24

---

## Summary

This file tests `sync_single_file()` at the **single-file mention layer** — the 4 tests map directly to the 4 code paths in the function's decision tree:

1. **New file** → insert `file_content`, `concept_mentions`, trigger graph rebuild
2. **mtime guard fires** → skip entirely (no DB write)
3. **hash guard fires** → update `last_indexed` only, skip concept re-extraction
4. **Content changed** → full reprocess, replace all mentions, rebuild graph

The file is structurally sound. The mutex-guarded isolation, `ConfigOverrideReset` RAII guard, and tamper-then-assert pattern for guards are all correct. The test naming is largely accurate and traceable to distinct behavior branches.

**The central finding is assertion shallowness**: Tests 1 and 4 verify the *right rows* exist but not the *right values* in those rows. Test 2 proves the guard fired but not the invariant that distinguishes it from the hash guard (no DB write vs. one DB write). Test 3 is the strongest test in the file — it uses exact-value assertions and is notably better than its counterpart in `incremental_sync_unification_guards.rs`.

**Systemic issues across all 4 tests:**

1. **Shallow `file_content` row checks** — Tests 1 and 4 call `get_file_content_row(...).is_some()` only. The `FileContentRow` struct already carries `file_md5`, `file_size`, and `last_indexed`; none are asserted by Tests 1 or 4. The `incremental_sync_unification_lifecycle.rs` counterpart (which tests the same new-file path) correctly asserts `file_md5` is non-empty and `file_size > 0`. The omission here is an unjustified gap.

2. **Missing guard-distinguishing invariant in Test 2** — The semantic difference between mtime guard and hash guard is:
   - mtime guard → `tx.commit()` with no preceding `UPDATE` (last_indexed stays frozen)
   - hash guard → `UPDATE file_content SET last_indexed = ?2` before `tx.commit()`
   Test 2 proves the guard fires but never checks that `last_indexed` was not updated. A regression that caused the mtime guard path to also run the `last_indexed` UPDATE would pass Test 2 silently.

3. **Missing concept-removal scenario in Test 4** — Test 4 verifies that adding a new concept during content change is picked up. It never verifies that a concept *removed* from the file's content is also removed from `concept_mentions`. The implementation does `DELETE FROM concept_mentions WHERE source_file = ?1` before re-inserting, so this path exists — but no test in this file exercises it.

4. **Traceability annotation uses a stale scheme** — `// T-5.7: RTM FR-14.x` comments reference `FR-14.x` sub-numbers that do not appear in `RTM.md` or `PRD.md`. The current RTM uses `RTM-NNN` identifiers. This is consistent with the same finding in `incremental_sync_unification_lifecycle.md`. The annotations provide zero usable traceability in the current toolchain.

5. **Non-UUID concept names share a fixed namespace** — All 4 tests use the same concepts (`sync-unify-a`, `sync-unify-b`, `sync-unify-c`) in separate `isolated_config()` DB instances. The mutex serializes them, so there is no cross-test contamination at runtime. However, the fixed names mean the tests cannot safely parallelize if the mutex is ever removed, and a future reader cannot tell whether the concept-name reuse is intentional or accidental. The sibling `incremental_sync_unification_lifecycle.rs` uses `uuid::Uuid::new_v4()` for isolation-by-default.

---

## Verdicts

| # | Test Name | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `process_file_new_file_inserts_content_mentions_and_index_rows` | ✅ | ⚠️ partial | ❌ shallow | ⚠️ | ❌ | **REWRITE** |
| 2 | `process_file_mtime_unchanged_skips_without_restoring_deleted_mentions` | ✅ | ⚠️ partial | ❌ gap | ✅ | ❌ | **REWRITE** |
| 3 | `process_file_mtime_differs_but_hash_same_updates_last_indexed_only` | ✅ | ⚠️ partial | ✅ strong | ✅ | ❌ | **KEEP** |
| 4 | `process_file_content_changed_reprocesses_all_mentions` | ✅ | ✅ | ⚠️ partial | ✅ | ❌ | **REWRITE** |

---

## Detailed Findings

### Test 1 — `process_file_new_file_inserts_content_mentions_and_index_rows`

**RTM claim**: T-5.7 / FR-14.3 (stale annotation — see §Traceability Finding)
**Code path exercised**: `sync_single_file()` → no `indexed_state` → `upsert_processed_file()` → full insert

#### What it proves correctly
- A `file_content` row is created after syncing a new file ✅
- `concept_mentions` rows are created for both WikiLink concepts with `mention_count = 1` ✅
- Output contains the synced file URI ✅

#### Assertion gaps

**Gap 1 — `file_content` row values not checked.**
The test calls `get_file_content_row(...).is_some()` but never inspects the returned `FileContentRow`. The following are unverified:

```rust
// Missing — currently these assertions are absent:
let row = get_file_content_row(&cfg, &file_uri).expect("row must exist");
assert!(
    row.file_md5.as_deref().is_some_and(|h| !h.is_empty()),
    "file_md5 must be populated on first index"
);
assert!(
    row.file_size.is_some_and(|s| s > 0),
    "file_size must be populated on first index"
);
assert!(
    row.last_indexed.is_some(),
    "last_indexed must be set on first index"
);
```

The counterpart test in `incremental_sync_unification_lifecycle.rs` (`incremental_sync_file_changed_creates_expected_db_artifacts`) asserts `file_md5` and `file_size` correctly. This file does not. That is not a justifiable gap.

**Gap 2 — Output string check is a prefix substring only.**
`output.contains("Synced memory://tests/sync-unification/new-file")` does not verify the mention count in the output. The actual format is:
```
Synced {file_uri}: {mention_count} concept mentions updated, {file_filtered} filtered, {relation_count} affected relations rebuilt
```
After inserting 2 concepts, the test could additionally assert:
```rust
assert!(
    output.contains("2 concept mentions updated"),
    "output must report 2 mentions for 2 wikilinks: {output}"
);
```
This catches regressions where the reprocessing fires but the mention-count calculation is wrong.

**Gap 3 — No assertion on `concepts` table.**
`upsert_processed_file` inserts rows into both `concept_mentions` (by file) and `concepts` (by concept name). The test verifies `concept_mentions` but not that the `concepts` table was populated. This is minor since `concept_mentions` has a FK-like INSERT dependency on `concepts`, but a future schema change decoupling those would silently break the `concepts` insert with no test catching it.

#### Uniqueness vs. siblings
`incremental_sync_unification_lifecycle.rs::incremental_sync_file_changed_creates_expected_db_artifacts` covers the same new-file path with uuid-isolated concepts, stronger `file_md5`/`file_size` assertions, and graph-edge (`concept_graph`) verification. This test's contribution is narrower — it is the only test in the *unification-single-file* file that exercises the new-file path. That justifies retention but demands the assertion quality be brought to parity.

---

### Test 2 — `process_file_mtime_unchanged_skips_without_restoring_deleted_mentions`

**RTM claim**: T-5.7 / FR-14.1 (stale annotation)
**Code path exercised**: `sync_single_file()` → `indexed_state` exists → `should_skip_by_mtime()` returns `true` → early return

#### What it proves correctly
- After initial sync, `concept_mentions` contains `concept_a` with count 1 ✅
- After tampering (deleting mentions), the tamper is confirmed ✅
- After second sync with unchanged mtime, output contains `"mtime guard"` ✅
- Mentions remain absent (the guard did not restore them) ✅

#### Critical assertion gap — guard does not distinguish itself from hash guard

The mtime guard path in `sync/mod.rs` is:
```rust
if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
    tx.commit()?;                   // ← empty transaction, NO UPDATE at all
    return Ok(format!("Skipped {file_uri}: mtime guard"));
}
```

The hash guard path immediately below it is:
```rust
if stored_hash == Some(checksum.as_str()) {
    tx.execute(
        "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
        rusqlite::params![file_uri, indexed_at],
    )?;
    tx.commit()?;
    return Ok(format!("Skipped {file_uri}: hash guard"));
}
```

The behavioral contract that distinguishes mtime guard from hash guard is:
- **mtime guard** → `last_indexed` is NOT updated (frozen at original value)
- **hash guard** → `last_indexed` IS updated to the new mtime

Test 2 does not assert that `last_indexed` remains unchanged. A regression that erroneously routes the mtime guard through the `last_indexed` UPDATE statement would pass this test silently. The missing assertion is:

```rust
let row_before = get_file_content_row(&cfg, &file_uri).expect("row before tamper");
tamper_delete_mentions(&cfg, &file_uri);
// ... (existing sync + guard check) ...
let row_after = get_file_content_row(&cfg, &file_uri).expect("row after re-sync");
assert_eq!(
    row_after.last_indexed, row_before.last_indexed,
    "mtime guard must NOT update last_indexed — distinguishes it from hash guard path"
);
```

This `last_indexed` frozen-value assertion is the single most important assertion missing from this test.

#### Tamper pattern is correct
The tamper-then-assert approach is the right technique for proving negative guard behavior. Using `tamper_delete_mentions` and then verifying the tamper succeeded before re-syncing prevents the test from silently passing due to the tamper itself failing.

#### Uniqueness vs. siblings
`incremental_sync_unification_guards.rs::incremental_sync_mtime_guard_skips_unchanged_files` tests the same guard but adds `concept_graph` edge assertions. This test's unique contribution is mention-layer isolation — it keeps the scope narrow and relies on the guards file for edge-layer coverage. The scoped approach is defensible; the missing `last_indexed` invariant is not.

---

### Test 3 — `process_file_mtime_differs_but_hash_same_updates_last_indexed_only`

**RTM claim**: T-5.7 / FR-14.2 (stale annotation)
**Code path exercised**: mtime check fails → `read_markdown()` → hash matches stored → `UPDATE last_indexed` only → early return

#### What it proves correctly
- Output contains `"hash guard"` ✅
- `last_indexed` is updated to the exact new filesystem mtime (`file_mtime_rfc3339(&path)`) ✅ — **exact-value assertion**
- `file_md5` is unchanged ✅
- `file_size` is unchanged ✅
- Mentions remain deleted (guard did not reprocess) ✅

This is the strongest test in the file. The exact-value `last_indexed` check:
```rust
assert_eq!(
    after.last_indexed,
    Some(file_mtime_rfc3339(&path)),
    "last_indexed must update to the new filesystem mtime"
);
```
is notably better than the counterpart assertion in `incremental_sync_unification_guards.rs` which uses `assert_ne!` (only proves "something changed"). The existing evaluation of that file (`incremental_sync_unification_guards.md`) explicitly recommends adopting this pattern from this test.

#### Minor gap — output string does not verify URI
`output.contains("hash guard")` matches the substring but does not verify which file's guard fired. In a concurrent test run (if the mutex were ever removed), a cross-test hash-guard output could produce a false positive. The safer check:
```rust
assert!(
    output.contains(&format!("Skipped {file_uri}: hash guard")),
    "hash guard output must name the correct file: {output}"
);
```

#### Verdict: KEEP
No significant regressions possible through the current assertion set. The one gap (URI in output string) is low severity. This test sets the standard for hash-guard testing in this codebase.

---

### Test 4 — `process_file_content_changed_reprocesses_all_mentions`

**RTM claim**: T-5.7 / FR-14.3 (stale annotation)
**Code path exercised**: mtime check fails → hash differs → `upsert_processed_file()` → full reprocess → graph rebuild

#### What it proves correctly
- Output contains `"concept mentions updated"` ✅
- `file_md5` changes when content changes ✅
- All 3 concepts (A, B, C) have `mention_count = 1` after reprocessing ✅

#### Assertion gaps

**Gap 1 — `last_indexed` not asserted post-reprocess.**
After full reprocessing, `upsert_processed_file` sets `last_indexed` to the file's current mtime (`indexed_at`). This is never checked. The full-reprocess path is the only path that goes through `upsert_processed_file` which also resets `last_indexed`; asserting the updated value closes a small but distinct behavioral contract:
```rust
assert_eq!(
    after.last_indexed.as_deref(),
    Some(file_mtime_rfc3339(&path).as_str()),
    "full reprocess must update last_indexed to the new filesystem mtime"
);
```

**Gap 2 — `file_size` not asserted.**
Test 3 asserts `file_size` is unchanged (as a guard-correctness check). Test 4 rewrites the file with additional content (adding `[[sync-unify-c]]`), making the file physically larger. The test does not check that `file_size` was updated. The before/after row is already fetched via `get_file_content_row` — the assertion is essentially free:
```rust
assert!(
    after.file_size > before.file_size,
    "reprocessed file_size must increase after content was added"
);
```

**Gap 3 — No concept-removal coverage.**
Test 4 only verifies concept *addition* (C is new). The reprocessing path runs:
```rust
// In upsert_processed_file:
connection.execute("DELETE FROM concept_mentions WHERE source_file = ?1", ...)?;
// then re-inserts only concepts found in the new content
```
A failure in the delete step (or a regression that skips it) would result in *stale* mentions accumulating. No test in this file verifies that removing a WikiLink from content actually removes its `concept_mentions` row. A companion test:

```
fn process_file_content_removes_mention_when_concept_deleted_from_content()
```

would exercise the removal path and close this gap. It belongs in this file since it covers the same full-reprocess code path, just the cleanup-of-stale-concepts angle.

**Gap 4 — Concept B assertion adds no unique value.**
After the rewrite, concept A and concept B both appear in the new content. Both are asserted as `Some(1)`. This is correct but symmetric — if upsert_processed_file accidentally skipped concept B specifically, the assertion would catch it. However, the count for B is expected to remain 1 across both syncs, so this is not truly testing *reprocessing* of B — just that B still exists. A more interesting assertion would be that B's row was actually deleted and re-inserted (i.e., verifying idempotency of the upsert), but that's not directly observable from `mention_count`.

---

## Recommendations

### Priority 1 — Rewrite Test 2: add the missing `last_indexed` frozen-value assertion

This is the highest-priority gap. Without it, the most important behavioral distinction between mtime guard and hash guard paths is unverified. The fix requires saving a `FileContentRow` snapshot before the tamper, then comparing `last_indexed` after re-sync:

```rust
let row_before_tamper = get_file_content_row(&cfg, &file_uri).expect("row before tamper");
tamper_delete_mentions(&cfg, &file_uri);
// ... (existing assertions on tamper effectiveness) ...
let output = sync_one(&cfg, &path);
// ... (existing output + mention assertions) ...
let row_after = get_file_content_row(&cfg, &file_uri).expect("row after guard");
assert_eq!(
    row_after.last_indexed, row_before_tamper.last_indexed,
    "mtime guard must not update last_indexed (empty transaction path)"
);
```

### Priority 2 — Rewrite Test 1: add `file_md5`, `file_size`, `last_indexed` assertions

The new-file path sets all three `FileContentRow` columns. The test already fetches the row (via `is_some()`) — it just discards the value. The fix is to store and inspect the row:

```rust
let row = get_file_content_row(&cfg, &file_uri).expect("file_content row must exist");
assert!(row.file_md5.as_deref().is_some_and(|h| !h.is_empty()), "file_md5 must be set");
assert!(row.file_size.is_some_and(|s| s > 0), "file_size must be positive");
assert!(row.last_indexed.is_some(), "last_indexed must be set");
```

Optionally add `output.contains("2 concept mentions updated")` to tighten the output assertion.

### Priority 3 — Rewrite Test 4: add `last_indexed` and `file_size` assertions

The full-reprocess path updates both. The fix mirrors the gaps from Test 1. Additionally, consider adding a companion test for concept removal (a concept present in v1 of the file, absent in v2, should have its `concept_mentions` row deleted).

### Priority 4 — Upgrade output assertion in Test 3 to include URI

Low severity, but cheap to fix:
```rust
assert!(output.contains(&format!("Skipped {file_uri}: hash guard")));
```

### Priority 5 — Fix traceability annotations (systemic, all 4 tests)

The `// T-5.7: RTM FR-14.x` comments reference `FR-14.x` which does not exist in `RTM.md` or `PRD.md`. The active RTM uses `RTM-NNN` identifiers. Either:
- Map to current RTM identifiers (e.g., the incremental sync entries near RTM-143, RTM-178), or
- Remove the annotations entirely rather than leave stale references

This is the same systemic issue identified in `incremental_sync_unification_lifecycle.md`.

### Priority 6 — Consider UUID-isolated concept names

The four tests share the concept names `sync-unify-a`, `sync-unify-b`, `sync-unify-c` across separate DB instances serialized by `TEST_MUTEX`. The mutex ensures correctness today, but the fixed names create a hidden coupling to serialized execution. `incremental_sync_unification_lifecycle.rs` uses `uuid::Uuid::new_v4()` per test. Adopting the same pattern here would allow safe future parallelization and makes the isolation intent explicit in the test code.

---

## Positive Observations

- **Test 3 is a reference implementation.** Its exact-value `last_indexed` assertion is more precise than the sibling `incremental_sync_unification_guards.rs` test for the same code path. It is explicitly cited in that file's evaluation as the pattern to adopt.
- **Tamper pattern is methodologically correct.** Using `tamper_delete_mentions` followed by a precondition assertion (`None`) before the second sync is the right way to prove guard behavior — it makes the test self-documenting and avoids silent false passes if the tamper call itself fails.
- **`ConfigOverrideReset` RAII guard is consistently used.** All 4 tests use it, preventing config state leakage between test runs regardless of panic behavior.
- **Mtime manipulation is precise.** `set_file_mtime_utc` with `parse_rfc3339_utc` + `chrono::Duration::minutes(N)` correctly advances the file mtime ahead of the stored `last_indexed`, ensuring the mtime guard check (`modified_at <= last_indexed`) falls on the expected side.
- **Precondition assertions are present.** Tests 2 and 4 assert the expected state before the key operation, preventing silent false passes from state leakage.

---

## Load-Bearing Score: 4/4

All 4 tests are load-bearing. Each covers a distinct code path in `sync_single_file()` that is not fully covered by any other single test file. The verdicts are KEEP (1) and REWRITE (3) — no test should be removed. The REWRITE verdicts reflect assertion shallowness, not structural invalidity. Removing any of the 4 would leave a complete behavioral branch of `sync_single_file()` unverified at the mention layer.
