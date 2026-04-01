---
id: T-80.80
title: "Test Evaluation: `incremental_sync_boundary_guards.rs`"
status: done
pipeline: tdd
---

# Test Evaluation: `incremental_sync_boundary_guards.rs`

> **Color**: 🔘 Gray — Skeptical Inquiry
> **Traceability**: T-5.7 · RTM FR-5.7 · NFR-1 · NFR-3
> **Related features**: [[incremental sync]] · [[mtime guard]] · [[hash guard]] · [[watcher lifecycle]] · [[test quality]]
> **Evaluator notes**: Three tests. Two earn their keep. One is a misplaced compatibility artefact that is actively shaping production source code.

---

## Summary

`incremental_sync_boundary_guards.rs` contains exactly **three `#[test]` functions** covering the skip-path behaviour of `sync_single_file`. Two tests (`mtime_guard`, `hash_guard`) are well-targeted boundary tests for FR-5.7 guard logic. The third (`watcher_start_stop_lifecycle_contract_still_holds`) is a **structural misfit**: it belongs in `incremental_sync_tools_watcher.rs`, its stop-message assertion relies on a compatibility shim baked into production code (see `stop_watcher_tool` in `sync/watchers.rs`, comment `// T-0.1: RTM NFR-3`), and it is rendered strictly weaker by the more comprehensive watcher lifecycle test that already exists in the dedicated watcher file.

A cross-file mutex isolation defect was also discovered during test execution: `incremental_sync_boundary_guards` and `incremental_sync_boundary_mutations` each instantiate **their own copy** of `TEST_MUTEX` from the shared `support.rs` path, meaning the two files provide zero mutual exclusion and produce "database is locked" / "another process is leader" failures when run together under a broad filter (`-- incremental_sync_boundary`).

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Quality | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `sync_single_file_skips_unchanged_file_with_mtime_guard` | ✅ Yes | ✅ Yes | ✅ Yes (minor weak spot) | ✅ Clear | ⬜ Low value | **KEEP** |
| 2 | `sync_single_file_skips_file_with_same_hash` | ✅ Yes | ✅ Yes | ⚠️ Mostly (one weak assert) | ✅ Clear | ⬜ Low value | **KEEP** |
| 3 | `watcher_start_stop_lifecycle_contract_still_holds` | ❌ No | ❌ No | ⚠️ Fragile OR | ⚠️ Misleading file | ⬜ N/A | **REWRITE → MOVE** |

---

## Per-Test Analysis

---

### Test 1 — `sync_single_file_skips_unchanged_file_with_mtime_guard`

**What it does**: Writes a fixture, runs a baseline `Sync`, then immediately calls `sync_single_file` on the same file without modifying it. Asserts the mtime guard fires, that `last_indexed` is unchanged, and that the concept graph is unchanged.

**Is the setup correct?** Yes. `baseline_sync()` indexes the file and persists `last_indexed = file mtime`. The second `sync_one` reads a disk mtime that equals the stored timestamp → `should_skip_by_mtime` returns `true` (`modified_at <= last_indexed` with equality). The path is fully exercised.

**Are the assertions load-bearing?**

| Assertion | Verdict |
|---|---|
| `output.contains("mtime guard")` | ✅ Confirms the right code path was taken (not hash guard, not full process). Structural. |
| `fetch_last_indexed == Some(before_last_indexed)` | ✅ Distinguishes mtime guard from hash guard — hash guard *does* refresh `last_indexed`, mtime guard must not. Load-bearing behavioral contract. |
| `concept_names == before_concepts` | ✅ Guards against accidental graph mutation in the skip path. |

**Minor gap**: Does not assert `file_concepts(cfg, &target_uri)` against the known `{alpha, beta}` set. The `concept_names` check is wider (whole-DB scan), so a bug that cleared only this file's mentions would pass. Low risk but worth noting.

**Is it unique vs. `incremental_sync_unification_guards.rs::incremental_sync_mtime_guard_skips_unchanged_files`?** Yes — complementary. The unification test is adversarial (tampers DB, verifies nothing is restored). This boundary test exercises the normal skip path. Different angles; both warranted.

**Verdict**: **KEEP**

---

### Test 2 — `sync_single_file_skips_file_with_same_hash`

**What it does**: Establishes a baseline sync, manually backdates `last_indexed` to defeat the mtime guard, re-writes the file with identical content (updating disk mtime), then calls `sync_one`. Asserts the hash guard fires, `last_indexed` is refreshed, `file_md5` is preserved, and concepts are unchanged.

**Is the setup correct?** Yes. `set_last_indexed(..., "2000-01-01T00:00:00+00:00")` then `write_memory_fixture(..., content)` (same content, new mtime) creates the exact condition: `disk mtime >> last_indexed` (mtime guard passes), same checksum (hash guard fires). The two-step guard sequence is correctly exercised.

**Are the assertions load-bearing?**

| Assertion | Verdict |
|---|---|
| `output.contains("hash guard")` | ✅ Confirms the correct skip path. |
| `assert_ne!(updated_last_indexed, old_timestamp)` | ⚠️ **Weak positive**: only asserts the timestamp changed *from "2000"*. It would pass if `last_indexed` were set to any wrong value. A stronger assertion would be `assert!(updated_last_indexed.contains(current_year))` or compare to approximately the disk mtime. |
| `fetch_file_md5 == Some(original_hash)` | ✅ Confirms hash guard does not corrupt stored checksum. |
| `concept_names == before_concepts` | ✅ Guards against accidental graph mutation in the skip path. |

**Is it unique vs. `incremental_sync_unification_guards.rs::incremental_sync_hash_guard_updates_last_indexed_only`?** Complementary. The unification test also tampers DB state and verifies mentions/edges are NOT restored. This boundary test verifies `file_md5` preservation explicitly (the unification test does not). Different assertion coverage.

**Verdict**: **KEEP** (with recommendation to strengthen `assert_ne` to `assert_ne! + contains year`)

---

### Test 3 — `watcher_start_stop_lifecycle_contract_still_holds`

**What it does**: Calls `StartWatcher` via `ToolRegistry::global()`, asserts start output format, calls `StopWatcher`, asserts stop is either `"Stopped watcher"` or `"Watcher not running"`.

**Is it load-bearing?** No — a more comprehensive watcher lifecycle test already exists in `incremental_sync_tools_watcher.rs::start_watcher_then_stop_watcher_lifecycle_returns_expected_messages`, which additionally verifies:
- Double-start idempotency (`"Watcher already running"`)
- Exact stop message (`"Stopped incremental sync watcher"` at the lower layer)
- Double-stop idempotency (`"Watcher not running"`)
- `watcher_is_running()` state before and after

**Is it correctly placed?** No. The file is `incremental_sync_boundary_guards.rs` — a file about mtime and hash guards on `sync_single_file`. The watcher lifecycle has no conceptual relationship to those guards. It belongs in the dedicated watcher test file.

**Are the assertions correct?**

| Assertion | Verdict |
|---|---|
| `start.starts_with("Started watching ")` | ✅ Correct format check. |
| `start.contains(" for incremental sync (debounce: 250ms)")` | ✅ Correct debounce format (this detail is *not* checked in the tools_watcher test — only positive uniqueness in the whole suite). |
| `stop == "Stopped watcher" \|\| stop == "Watcher not running"` | ⚠️ The `\|\|` is structurally fragile. After a confirmed successful start, only `"Stopped watcher"` should ever appear. The `"Watcher not running"` branch can only be true if a race condition or global state leak caused the watcher to stop between the start and stop calls. This OR is either defensive dead code or a signal of non-determinism. |

**Source-code debt signal**: `sync/watchers.rs` contains a function `stop_watcher_tool` with comment `// T-0.1: RTM NFR-3 — preserve the registry-level StopWatcher contract expected by older boundary tests.` This function exists *specifically* to remap `"Stopped incremental sync watcher"` → `"Stopped watcher"` so this one test stays green. A test should not be forcing a translation layer into production code.

**Verdict**: **REWRITE → MOVE** — migrate the debounce-format assertion (its only unique value) into `incremental_sync_tools_watcher.rs`, drop the weaker OR stop assertion there, and **delete this test from the guards file**. Then evaluate whether `stop_watcher_tool` can be removed from `sync/watchers.rs`.

---

## Cross-Cutting Finding: Shared Mutex Race Condition

**Severity**: High (causes intermittent CI failures)

Both `incremental_sync_boundary_guards.rs` and `incremental_sync_boundary_mutations.rs` use:
```rust
#[path = "incremental_sync_boundary/support.rs"]
mod support;
```

In Rust, each `mod` inclusion of the same `#[path]` creates a **separate static**. The `TEST_MUTEX: Lazy<Mutex<()>>` in `support.rs` is instantiated *twice* — once per test module. These are independent mutexes and provide **zero mutual exclusion** between the guards file and the mutations file.

**Evidence**: Running `cargo test -p maenifold-core --test integration_tests incremental_sync_boundary` (which covers both files) produces intermittent failures:
- `"database is locked"` — SQLite contention on `Config::set_database_path` global override
- `"Skipped: another process is leader"` — leader election global state corrupted by concurrent `isolated_config()` calls

Both files call `Config::override_root(tmp.path())` — a **process-global** side effect — expecting the mutex to guard it. It does not.

**Fix**: Define a single cross-module mutex in a shared location (e.g., `tests/support/global_test_lock.rs`) and have both guard files acquire *the same* static lock.

---

## Recommendations

1. **Test 1 (KEEP)**: Add a `file_concepts(cfg, &target_uri)` assertion to pin the exact file-scoped concept set, not just the whole-DB snapshot. Low effort, eliminates the gap.

2. **Test 2 (KEEP)**: Strengthen `assert_ne!(updated_last_indexed, old_timestamp)` → verify the new timestamp is a plausible current-year RFC-3339 value, e.g.:
   ```rust
   assert!(
       updated_last_indexed.starts_with("202"),
       "hash guard should refresh last_indexed to current time, got: {updated_last_indexed}"
   );
   ```

3. **Test 3 (REWRITE → MOVE)**:
   - Extract the debounce format check (`start.contains(" for incremental sync (debounce: 250ms)")`) and add it to `start_watcher_then_stop_watcher_lifecycle_returns_expected_messages` in `incremental_sync_tools_watcher.rs`.
   - Delete `watcher_start_stop_lifecycle_contract_still_holds` from this file.
   - Evaluate removing the `stop_watcher_tool` compat shim from `sync/watchers.rs` if no other test references it.

4. **Cross-module mutex (REWRITE infrastructure)**:
   Create a global lock in `tests/support/` (already a shared location, e.g., `tests/support/mod.rs`) and use it in both `guards` and `mutations` files, eliminating the duplicate static and the intermittent `"database is locked"` failures.

---

## Load-Bearing Score: 2/3

| Test | Verdict | Load-Bearing |
|---|---|:---:|
| `sync_single_file_skips_unchanged_file_with_mtime_guard` | KEEP | ✅ |
| `sync_single_file_skips_file_with_same_hash` | KEEP | ✅ |
| `watcher_start_stop_lifecycle_contract_still_holds` | REWRITE → MOVE | ❌ |

Two of three tests are genuinely load-bearing boundary probes for the mtime and hash guard contracts. The watcher test is an artefact of earlier test organisation that is now not only redundant but is exerting pressure on production code to maintain a compatibility shim. It should be dissolved into the dedicated watcher test file and deleted here.

---
## SWE Remediation
- KEEP test 1 (`sync_single_file_skips_unchanged_file_with_mtime_guard`): added `file_concepts(cfg, &target_uri)` assertion pinning exact `{alpha, beta}` set per Rec 1
- KEEP test 2 (`sync_single_file_skips_file_with_same_hash`): strengthened `assert_ne` with `starts_with("202")` year check per Rec 2
- REWRITE→MOVE test 3 (`watcher_start_stop_lifecycle_contract_still_holds`): deleted from guards file; migrated debounce format assertion to `incremental_sync_tools_watcher::start_watcher_then_stop_watcher_lifecycle_returns_expected_messages` per Rec 3
- REMOVE `stop_watcher_tool` compat shim from `sync/watchers.rs`; registry now invokes `stop_watcher` directly (no other test depended on the shim)
- Removed unused imports (`ToolRegistry`, `OutputFormat`, `serde_json::json`) from guards file
- Commit: a63a3ad

---
## Red-team Audit
- ✅ KEEP test 1 (`mtime_guard`): `file_concepts(&cfg, &target_uri)` assertion added — queries `concept_mentions WHERE source_file = ?1`, pins exact `{alpha, beta}` BTreeSet. Matches eval Rec 1.
- ✅ KEEP test 2 (`hash_guard`): `assert!(updated_last_indexed.starts_with("202"), ...)` added alongside preserved `assert_ne!`. Matches eval Rec 2 verbatim.
- ✅ REWRITE→MOVE test 3 (`watcher_lifecycle`): deleted from guards file (2 `#[test]` remain); debounce format assertion `contains(" for incremental sync (debounce: 250ms)")` migrated to `incremental_sync_tools_watcher.rs:52`; fragile OR stop assertion correctly NOT migrated. Matches eval Rec 3.
- ✅ REMOVE `stop_watcher_tool`: function deleted from `sync/watchers.rs`; `StopWatcher` registry entry rewired to `watchers::stop_watcher` (signature `fn(&ToolContext, &Value) -> Result<String>` matches `ToolFn`); zero references to `stop_watcher_tool` remain in codebase (`grep -rn` confirms).
- ✅ No unrelated changes: 4 files touched (`registry.rs`, `watchers.rs`, `boundary_guards.rs`, `tools_watcher.rs`), all directly traceable to eval Recs 1–3.
- ⚠️ NOTE: `integration_tests` binary has pre-existing compilation error in `graph_decay_graph.rs` (unresolved `find_similar_concepts_markdown`, file unchanged since commit `558d5ff`) preventing test execution at HEAD. Not caused by this eval's commit (a63a3ad).
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- incremental_sync_boundary_guards`: 2 passed, 0 failed
- Tests verified:
  - `sync_single_file_skips_unchanged_file_with_mtime_guard` — ok
  - `sync_single_file_skips_file_with_same_hash` — ok
- SWE remediation confirmed: Test 3 (`watcher_start_stop_lifecycle_contract_still_holds`) correctly removed; only 2 `#[test]` functions remain in module, matching eval REWRITE→MOVE verdict
- Eval Rec 1 confirmed: `file_concepts(&cfg, &target_uri)` assertion present at line 48, pinning exact `{alpha, beta}` BTreeSet
- Eval Rec 2 confirmed: `starts_with("202")` year check present at line 90, alongside preserved `assert_ne!`
- Verdict: **PASS**
