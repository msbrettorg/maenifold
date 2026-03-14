# Test Evaluation: incremental_sync_unification_guards.rs

**Evaluator**: Red Team (🔘 Gray — Skeptical Inquiry)  
**Traceability**: T-5.7 | FR-14.1, FR-14.2 | NFR-2  
**Source under test**: `crates/maenifold-core/src/sync/mod.rs` → `sync_single_file()`  
**Evaluation date**: 2025-07-23

---

## Summary

This file contains **2 tests** covering the two early-exit guards in `sync_single_file()`:

1. **mtime guard** (`modified_at <= last_indexed` → skip entirely, no DB write)
2. **hash guard** (`stored_hash == checksum` → update `last_indexed` only, no concept reprocessing)

The guards are critical for [[incremental sync]] performance — without them, every watcher event triggers a full re-index. The tests use a *tamper-then-assert* pattern: they corrupt the DB index deliberately, re-sync, and verify the guard prevented restoration. This is a sound technique for proving negative behavior.

**Critical context**: A sibling test file, `sync_unification_single_file.rs`, already covers both guards (`process_file_mtime_unchanged_skips_without_restoring_deleted_mentions` and `process_file_mtime_differs_but_hash_same_updates_last_indexed_only`) at the mention-layer. The `guards.rs` tests extend coverage to the **graph-edge layer** (`concept_graph` table) — this is their unique contribution and justifies their existence.

**Both tests contain assertion gaps** that would allow distinct behavioral regressions to pass silently.

---

## Verdicts

| # | Test Name | Load-Bearing | Unique | Correct Assertions | Good Name | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|---------|
| 1 | `incremental_sync_mtime_guard_skips_unchanged_files` | ✅ | ⚠️ partial | ❌ gap | ✅ | **REWRITE** |
| 2 | `incremental_sync_hash_guard_updates_last_indexed_only` | ✅ | ⚠️ partial | ❌ gap | ✅ | **REWRITE** |

---

## Detailed Findings

### Test 1 — `incremental_sync_mtime_guard_skips_unchanged_files`

**RTM**: T-5.7 / FR-14.1  
**What it does**: Syncs a file, pins its filesystem mtime to match the stored `last_indexed`, corrupts both `concept_mentions` and `concept_graph` rows, re-syncs, asserts the guard fired and nothing was restored.

#### What it proves correctly
- Output string contains `"mtime guard"` → matches the exact return value in `sync/mod.rs`: `format!("Skipped {file_uri}: mtime guard")`
- Mentions are not restored after the guard fires
- Graph edges are not restored after the guard fires ← **unique contribution vs. sibling file**

#### Critical assertion gap
The mtime guard path in `sync/mod.rs` is:

```rust
if should_skip_by_mtime(metadata.modified().ok(), last_indexed) {
    tx.commit()?;                           // ← empty transaction, NO UPDATE
    return Ok(format!("Skipped {file_uri}: mtime guard"));
}
```

The `last_indexed` column is **not updated** on this path. Test 2 explicitly asserts `last_indexed` **was** updated by the hash guard (`assert_ne!`). Test 1 has no complementary assertion that `last_indexed` **was not** updated by the mtime guard. If the mtime guard path were accidentally changed to run the same `UPDATE file_content SET last_indexed = ?2` statement that the hash guard runs, Test 1 would still pass.

**Missing assertion**:
```rust
assert_eq!(
    get_file_content_row(&cfg, &memory_uri)
        .expect("row must still exist")
        .last_indexed,
    row_after_create.last_indexed,
    "mtime guard must NOT update last_indexed (distinguish from hash guard path)"
);
```

#### Minor concern: Round-trip precision
The test pins the mtime via:
```
stored_mtime = parse_rfc3339_utc(last_indexed) → set_file_mtime → OS reads back → DateTime<Utc>
```
`should_skip_by_mtime` uses `<=` not `==`, so even if a 1-second precision filesystem truncates sub-second nanos, the comparison `truncated <= full_timestamp` still holds. No flakiness risk in practice. ✅

#### Uniqueness vs. sibling
`sync_unification_single_file.rs::process_file_mtime_unchanged_skips_without_restoring_deleted_mentions` covers mention-layer behavior. This test's unique contribution is `get_edge_source_files` assertion (graph-edge layer). It is load-bearing for that reason alone.

---

### Test 2 — `incremental_sync_hash_guard_updates_last_indexed_only`

**RTM**: T-5.7 / FR-14.2  
**What it does**: Syncs a file, bumps its mtime by +1 minute (to bypass mtime guard), corrupts both `concept_mentions` and `concept_graph`, re-syncs, asserts the hash guard fired and only `last_indexed` changed.

#### What it proves correctly
- Output string contains `"hash guard"` ✅
- Mentions are not restored ✅
- Graph edges are not restored ✅ ← **unique contribution vs. sibling file**
- `file_md5` hash is not changed ✅

#### Critical assertion gap — weak `last_indexed` check
The test uses:
```rust
assert_ne!(
    after.last_indexed, before.last_indexed,
    "when mtime differs but hash is unchanged, sync_single_file must update last_indexed"
);
```

`assert_ne!` only proves the value *changed* — not *to what*. The hash guard path in `sync/mod.rs` runs:
```rust
tx.execute(
    "UPDATE file_content SET last_indexed = ?2 WHERE file_path = ?1",
    rusqlite::params![file_uri, indexed_at],  // indexed_at = file's current mtime as RFC3339
)?;
```

The correct value of `last_indexed` post-update is the filesystem mtime of the file at the moment of the second sync. `assert_ne!` would pass even if `last_indexed` were set to `current_iso_timestamp()` (wall clock) or any other arbitrary non-equal value. The sibling test in `sync_unification_single_file.rs` correctly uses:
```rust
assert_eq!(
    after.last_indexed,
    Some(file_mtime_rfc3339(&path)),   // ← exact expected value
    "last_indexed must update to the new filesystem mtime"
);
```

**Missing assertion**: replace `assert_ne!` with:
```rust
assert_eq!(
    after.last_indexed.as_deref(),
    Some(file_mtime_rfc3339(&file_path).as_str()),
    "hash guard must update last_indexed to the current filesystem mtime, not any other value"
);
```
where `file_mtime_rfc3339` is already available in the sibling support module. Note: the `incremental_sync_unification/support.rs` used by this file does not expose this helper — it would need to be added (or `set_file_timestamp` followed by `indexed_timestamp_for_metadata` used inline).

#### Secondary gap — no `file_size` check
The sibling test asserts `after.file_size == before.file_size`. The `FileContentRow` struct in `incremental_sync_unification/support.rs` already includes `file_size: Option<i64>`. This assertion is free to add and closes a small gap where the hash guard path could accidentally alter the size column.

**Missing assertion**:
```rust
assert_eq!(
    after.file_size, before.file_size,
    "hash guard must not alter file_size when content is identical"
);
```

#### Uniqueness vs. sibling
The unique contribution is the graph-edge tamper + assertion not present in the sibling. Load-bearing for that reason.

---

## Recommendations

### Priority 1 — Rewrite Test 1: add the missing `last_indexed` invariant

The behavioral contract that distinguishes the **mtime guard** from the **hash guard** is:
- mtime guard: no DB write at all (empty `tx.commit()`)
- hash guard: `last_indexed` updated only

Test 1 asserts the hash guard does NOT fire but does not assert the mtime guard path's DB-write invariant. This asymmetry means a regression that causes the mtime guard to also update `last_indexed` (behaving like the hash guard) would go undetected.

Add:
```rust
let row_after_recheck = get_file_content_row(&cfg, &memory_uri).expect("row must persist");
assert_eq!(
    row_after_recheck.last_indexed, row_after_create.last_indexed,
    "mtime guard must NOT update last_indexed (no DB write on this path)"
);
```

### Priority 2 — Rewrite Test 2: upgrade `assert_ne!` to exact-value check

Replace the weak `assert_ne!(after.last_indexed, before.last_indexed)` with an exact equality check against the file's new mtime. Also add the `file_size` assertion from the sibling. This converts a "something changed" guarantee into a "the right thing happened" guarantee.

To support exact-value comparison, add `file_mtime_rfc3339(path: &Path) -> String` to `tests/incremental_sync_unification/support.rs`:
```rust
pub fn file_mtime_rfc3339(path: &Path) -> String {
    let metadata = fs::metadata(path).expect("read metadata");
    DateTime::<Utc>::from(metadata.modified().expect("mtime")).to_rfc3339()
}
```

### Priority 3 — Consider consolidating support modules

`tests/incremental_sync_unification/support.rs` and `tests/sync_unification/support.rs` both implement mtime manipulation and file content row queries. The `sync_unification` support has `file_mtime_rfc3339` and `file_size` tracking that `incremental_sync_unification` lacks. Rather than fix both in isolation, evaluate whether these support modules should be unified (either via a shared module or by extending `incremental_sync_unification/support.rs` to match parity with the richer sibling).

---

## Load-Bearing Score: 2/2

Both tests are load-bearing. Each provides graph-edge coverage (`concept_graph` table assertions) that exists nowhere else in the test suite. Removing either would leave a behavioral contract completely unverified at the edge layer. The verdict is REWRITE, not REMOVE — the tests must stay but must be strengthened to close the assertion gaps documented above.
