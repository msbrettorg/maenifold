# Test Evaluation: `sync_mtime_opt_guards.rs`

> **Color**: 🔘 Gray — Skeptical Inquiry
> **Traceability**: T-12.0 · RTM NFR-1
> **Related features**: [[incremental sync]] · [[mtime guard]] · [[hash guard]] · [[test quality]] · [[sync optimization]]
> **Source under test**: `crates/maenifold-core/src/sync/mod.rs` → `sync_single_file()`
> **Sibling test files (overlap risk)**: `sync_unification_single_file.rs` (T-5.7) · `incremental_sync_unification_guards.rs` (T-5.7) · `incremental_sync_boundary_guards.rs` (T-5.7)

---

## Summary

`sync_mtime_opt_guards.rs` contains exactly **3 `#[test]` functions** covering all three decision branches of `sync_single_file()`'s two-tier optimization gate: the *mtime guard* (skip entirely before read), the *hash guard* (update timestamp only, skip reprocessing), and the *full reprocessing path* (mtime and hash both changed).

The file's **strongest contribution** is the `UnreadableFileGuard` technique in test 1, which makes the file physically unreadable to prove the mtime guard fires before the filesystem read — a behavioral guarantee that no other test file in the codebase verifies at this level of fidelity.

The file has **three systemic problems**:

1. **String-coupling to output format**: All three tests use `output.contains(...)` to verify which guard fired. The sentinel strings (`"mtime guard"`, `"hash guard"`, `"concept mentions updated"`) are not constants shared with production code. Any message refactor breaks all three tests without a logic change.

2. **Significant overlap with `sync_unification_single_file.rs`**: Tests 2 and 3 largely duplicate `process_file_mtime_differs_but_hash_same_updates_last_indexed_only` and `process_file_content_changed_reprocesses_all_mentions` in that file. Without explicit differentiation, one of the two families becomes redundant maintenance burden.

3. **Test 3 is missing its most important assertions**: Tests 1 and 2 both verify `get_edge_source_files()` after the guard fires. Test 3 — the full reprocessing path — does not verify that `concept_graph` edges are correctly rebuilt. This leaves the graph-reprocessing leg of the optimisation partially untested.

Additional defects are noted per-test below.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `process_file_changed_when_mtime_unchanged_skips_before_read_and_does_not_restore_state` | ✅ Yes | ✅ Yes | ⚠️ Partial (3 gaps) | ✅ Accurate | ⬜ Low value | **REWRITE** |
| 2 | `process_file_changed_when_mtime_differs_but_hash_same_updates_last_indexed_only` | ✅ Yes | ⚠️ Partial | ⚠️ Partial (2 gaps) | ✅ Accurate | ⬜ Low value | **REWRITE** |
| 3 | `process_file_changed_when_hash_differs_reprocesses_and_updates_mentions` | ✅ Yes | ⚠️ Partial | ❌ Critical gap | ✅ Accurate | ⬜ Low value | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `process_file_changed_when_mtime_unchanged_skips_before_read_and_does_not_restore_state`

**What it claims to prove**: When the stored `last_indexed` matches the filesystem mtime, the sync short-circuits *before* opening the file, and does not restore DB state that was lost after indexing.

**Technique**: Uses `acquire_unreadable_file_guard` (chmod `0o200` on Unix, exclusive share lock on Windows) as a canary — if `sync_single_file` attempted a read it would return an error, but the test expects success. The tamper-then-assert pattern verifies the guard truly skips (deleted mentions remain deleted).

#### Defect 1 — Redundant `set_file_mtime_utc` call

```rust
// Step A: align filesystem mtime to what was stored in the DB
set_file_mtime_utc(&path, stored_mtime);

// … tamper …

// Step B: re-read the mtime that was JUST SET in Step A
let original_mtime = parse_rfc3339_utc(&file_mtime_rfc3339(&path));
{
    let _unreadable = acquire_unreadable_file_guard(&path);
    // Step C: set it again to the same value — dead code
    set_file_mtime_utc(&path, original_mtime);
    // …
}
```

`original_mtime` is `stored_mtime` because no intervening write has occurred. Step C is a no-op. The test still passes, but the reader must work through the indirection to reach this conclusion. Either Step C should be removed, or a comment should explain why it is retained (e.g., "guard against future OS-level mtime coarsening").

#### Defect 2 — String-coupled guard detection

```rust
assert!(output.contains("mtime guard"), "…{output}");
```

The production sentinel is `format!("Skipped {file_uri}: mtime guard")` (line 171 of `sync/mod.rs`). It is not a shared constant. If the message changes to "mtime-guard", "mtimeguard", or anything else, the assertion becomes a false-positive vacuous check — it would silently pass on a build where the guard fires with a different string. **Recommendation**: extract a `pub const MTIME_GUARD_SKIP_MARKER: &str = "mtime guard"` in production code and import it in the test.

#### Defect 3 — `file_content` row immutability not verified

```rust
assert!(
    get_file_content_row(&cfg, &file_uri).is_some(),
    "initial sync should keep the file_content row intact"
);
```

This asserts the row *exists* but does not assert the row was *not modified*. The mtime guard fires before any DB write occurs (`tx.commit()` is called but no UPDATE is executed). If a regression caused the mtime guard to accidentally refresh `last_indexed` — equivalent to the hash guard's behavior — this test would not catch it. The row's `last_indexed`, `file_md5`, and `file_size` should be captured before the guard run and compared after.

#### Defect 4 — Platform-asymmetric guard semantics (informational)

On **Unix**, `UnreadableFileGuard` removes read permission (`0o200`); `sync_single_file` would receive `io::ErrorKind::PermissionDenied`.
On **Windows**, it holds an exclusive share lock; the error would be `ERROR_SHARING_VIOLATION`.

The test only asserts the *output string* (not the error kind). If `sync_single_file` were to silently swallow a permission error and return an "mtime guard" skip-path string regardless, both platforms would pass. The test does not distinguish "mtime guard fired correctly" from "file read errored but was misclassified as a guard hit." This is acceptable given the integration scope, but worth documenting.

---

### Test 2 — `process_file_changed_when_mtime_differs_but_hash_same_updates_last_indexed_only`

**What it claims to prove**: When mtime advances but file content is identical (hash unchanged), the sync updates `last_indexed` only — no mention/edge reprocessing occurs.

#### Defect 1 — TOCTOU in expected `last_indexed` value

```rust
let output = sync_one(&cfg, &path);
assert!(
    output.contains("hash guard"),
    "mtime-only changes should skip reprocessing on identical content: {output}"
);

let after = get_file_content_row(&cfg, &file_uri).expect("row after");
assert_eq!(
    after.last_indexed,
    Some(file_mtime_rfc3339(&path)),    // ← reads filesystem AFTER sync
    "hash guard must refresh last_indexed to the filesystem mtime"
);
```

`file_mtime_rfc3339(&path)` is called after `sync_one` returns. The test relies on the filesystem mtime being stable between `sync_one()` completing and the `metadata()` call. In practice this is stable, but the **correct design** is to compare against the known `new_mtime` value that was explicitly set before the sync:

```rust
// new_mtime is already a DateTime<Utc> in scope
assert_eq!(
    after.last_indexed.as_deref(),
    Some(new_mtime.to_rfc3339().as_str()),
    "hash guard must record the filesystem mtime"
);
```

Comparing against the value that was set (not re-read) removes the implicit filesystem dependency and documents exactly what the test expects.

#### Defect 2 — String-coupled guard detection (same as test 1)

```rust
assert!(output.contains("hash guard"), "…{output}");
```

Same fragility as test 1. The production sentinel is `"Skipped {file_uri}: hash guard"`.

#### Overlap assessment

This test is substantively similar to `sync_unification_single_file.rs::process_file_mtime_differs_but_hash_same_updates_last_indexed_only`. The unique contribution here is the `file_size` and `file_md5` preservation assertions — that file's test does not appear to check those columns. This differentiation should be made explicit in a comment or the test should be consolidated with its sibling and the unique assertions retained.

---

### Test 3 — `process_file_changed_when_hash_differs_reprocesses_and_updates_mentions`

**What it claims to prove**: When both mtime and hash change, the sync performs full reprocessing — concept mentions are removed for concepts no longer present, added for new ones, and preserved for surviving ones.

#### Defect 1 — Critical: no graph-edge assertions

Tests 1 and 2 both invoke `get_edge_source_files()` to verify graph edges are untouched by the guards. Test 3 — the full reprocessing path — does not assert anything about `concept_graph`. This means the following regressions would pass undetected:

- `rebuild_affected_graph_from_mentions` fails to add the new `concept_b ↔ concept_common` edge
- The stale `concept_a ↔ concept_common` edge is not removed after reprocessing

Correct reprocessing requires:
```rust
// Old edge removed: concept_a × concept_common (concept_a no longer present)
assert_eq!(
    get_edge_source_files(&cfg, &normalized(concept_a), &normalized(concept_common)),
    None,
    "reprocessing must remove the edge for a concept no longer mentioned"
);

// New edge created: concept_b × concept_common
assert!(
    get_edge_source_files(&cfg, &normalized(concept_b), &normalized(concept_common)).is_some(),
    "reprocessing must create an edge for newly co-occurring concepts"
);
```

This is the highest-severity gap in the file — the mention-layer is tested but the graph layer is not, despite the graph rebuild being the most expensive and complex part of reprocessing.

#### Defect 2 — Tamper pattern adds limited value in this test

In tests 1 and 2, the tamper-then-assert pattern is load-bearing: it proves the guard truly skips by demonstrating that corrupted state is NOT repaired. In test 3, the file content changes, so the "new" mentions will be written from scratch regardless of what the tamper deleted. The tamper only proves the sync doesn't *append* to existing state — but this behavioral property could be verified more directly by asserting `get_mention_count == 1` (not `> 0`) for both `concept_common` and `concept_b`.

#### Defect 3 — String-coupled assertion (same as tests 1–2)

```rust
assert!(output.contains("concept mentions updated"), "…{output}");
```

The actual production string is `"Synced {file_uri}: {N} concept mentions updated, {M} filtered, {R} affected relations rebuilt"`. The substring `"concept mentions updated"` is stable but still unconstrained — a refactor could produce `"1 mention updated"` and this would silently become a false negative.

#### Overlap assessment

`sync_unification_single_file.rs::process_file_content_changed_reprocesses_all_mentions` covers the same scenario. Until test 3 adds graph-edge assertions (defect 1), `sync_unification_single_file.rs` provides equivalent or better coverage.

---

## Cross-Cutting Observations

### Global mutex serializes tests that could be parallel

All three tests call `acquire_test_lock()` which blocks on `TEST_MUTEX`. Each test also calls `isolated_config()` which creates a fresh `TempDir` with its own database. The tests are therefore fully isolated at the filesystem and database level — the only reason for the mutex is `Config::override_root()` and `Config::set_database_path()` writing to global state. This is a **design smell in production code**: global config state forces all integration tests to run serially.

### Guard output strings should be constants

The strings `"mtime guard"`, `"hash guard"`, and `"concept mentions updated"` appear in both production source and tests as independent string literals. Any refactor of the production message requires a coordinated hunt across test files. Extract:

```rust
// sync/mod.rs
pub const SKIP_MTIME_GUARD_MARKER: &str = "mtime guard";
pub const SKIP_HASH_GUARD_MARKER: &str = "hash guard";
pub const SYNCED_MENTIONS_MARKER: &str = "concept mentions updated";
```

### Shared test concepts should be module-level constants

`"Incremental-Mtime-ConceptA"`, `"Incremental-Mtime-ConceptCommon"`, `"Incremental-Mtime-ConceptB"` appear as inline strings in every test. A naming collision between test files (if another file writes the same concept name to a shared DB) would be silently masked by the mutex isolation, but is still a hygiene issue.

---

## Recommendations

1. **Fix test 3 immediately (CRITICAL)**: Add `get_edge_source_files` assertions for both the removed `concept_a ↔ concept_common` edge and the new `concept_b ↔ concept_common` edge. The graph-reprocessing path is currently untested.

2. **Extract guard marker constants in production code**: Create `pub const` values for the guard output strings in `sync/mod.rs` and import them in tests. This prevents silent test rot on message refactoring.

3. **Replace post-sync `file_mtime_rfc3339` with pre-computed `new_mtime` in tests 2 and 3**: Compare `after.last_indexed` against the `DateTime<Utc>` that was explicitly set, not the re-read filesystem value.

4. **Remove the redundant second `set_file_mtime_utc` in test 1**: Delete the `let original_mtime = ...` / inner `set_file_mtime_utc` block or add an explanatory comment if it is intentional.

5. **Add `file_content` row immutability assertion in test 1**: After the mtime guard fires, assert that `last_indexed`, `file_md5`, and `file_size` all equal their pre-guard values to catch any regression that accidentally writes to the DB on the mtime-skip path.

6. **Consolidate with `sync_unification_single_file.rs`**: Tests 2 and 3 in this file largely duplicate tests in `sync_unification_single_file.rs`. After applying the above fixes (particularly adding edge assertions to test 3), document in a header comment what unique guarantees this file provides versus its siblings, or absorb the unique assertions into the unification file.

7. **Add `file_size` assertion to test 3**: After reprocessing with content_v2, assert `after.file_size != before.file_size` (or compare exact sizes) to verify the metadata update path.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. They cover the three decision branches of a performance-critical optimization in `sync_single_file()`. Without them, regressions in the mtime guard (unexpected reads), hash guard (incorrect `last_indexed` update), or full reprocessing (missed mention/edge updates) would be invisible to the test suite. The defects identified above are **correctness gaps**, not redundancy — each test is doing something necessary, just not fully.

The file as a whole is **structurally sound but incomplete**. Its most important contribution (the `UnreadableFileGuard` read-skip proof in test 1) is unique in the codebase. Its most significant weakness (no graph-edge assertions in test 3) creates a blind spot for the most expensive code path.

**Status: REWRITE all three (targeted, surgical rewrites — do not replace wholesale).**
