# Test Evaluation: dynamic_model_db
**File:** `crates/maenifold-core/tests/dynamic_model_db.rs`
**Test count:** 2
**Evaluated:** 2026-06-06
**Traceability:** T-9.6 · RTM FR-8 · RTM-057 · RTM-060

---

## Summary

This file contains exactly two tests. On the surface they appear well-structured — each carries a traceability tag, uses an isolated `TestContext`, and avoids global state pollution. Under scrutiny, both tests have meaningful flaws that reduce their defect-detection power below what their names and comments imply.

**Test 1** (`t9_6_model_metadata_table_created_and_readable`) has one genuinely load-bearing assertion (`table_exists`) and two padding assertions that test SQLite round-trip behavior using manually-injected data. The production write path — `upsert_model_metadata` called from `ensure_vec0_tables` — is never exercised. The test provides false confidence that metadata is populated correctly in production conditions.

**Test 2** (`t9_6_no_model_db_setup_skips_vec0_tables_without_crashing`) contains a causation error. The vec0 tables are skipped because the vec0 *extension binary* is absent from the isolated temp-dir `assets_path`, not because the ONNX model is hidden. The `ctx.hide_packaged_model()` call is likely cargo-culted — the test outcome is determined by the isolated `Config::assets_path` pointing to a fresh temp dir with no `native/{platform}/vec0.{ext}` binary, independent of any ONNX model file presence. The test name and comment attribute the behavior to the wrong cause. Anyone attempting to diagnose a regression from this test would look in the wrong place.

A third gap applies to both tests: RTM-058 and RTM-059 (dimension-aware vec0 recreation) are cited in `ensure_vec0_tables` source comments but have no corresponding test anywhere in this file.

---

## Verdicts

| # | Test Function | Verdict | Reason |
|---|---|---|---|
| 1 | `t9_6_model_metadata_table_created_and_readable` | **REWRITE** | Only one of three assertions is load-bearing. The `get_model_metadata` assertions use manually-inserted data (raw `params![]`) not the production write path. Missing: `None` returned for absent key; production-path upsert verification. See detail below. |
| 2 | `t9_6_no_model_db_setup_skips_vec0_tables_without_crashing` | **REWRITE** | The negative vec0 assertions are load-bearing but the stated cause is wrong. Vec0 is skipped because the vec0 extension binary is absent from the temp-dir `assets_path`, not because the ONNX model is hidden. `hide_packaged_model()` is likely redundant here and introduces unnecessary global filesystem mutation. See detail below. |

---

## Detailed Findings

### Test 1: `t9_6_model_metadata_table_created_and_readable`

**RTM coverage claimed:** RTM-057 — `model_metadata` table always created after db init; `get_model_metadata` readable.

**What is actually tested:**
1. `table_exists(&conn, "model_metadata")` — **load-bearing**. Directly validates the schema setup contract from `GraphDatabase::initialize_database → ensure_vec0_tables → MODEL_METADATA_TABLE_SQL`.
2. `assert_eq!(db::get_model_metadata(&conn, "model_filename").as_deref(), Some("test-model.onnx"))` — **padding**. The test manually inserts `model_filename` via raw `conn.execute()`. This is a SQLite round-trip test, not a production-path test. `upsert_model_metadata` (the function that writes metadata in production) is never called.
3. `assert_eq!(db::get_model_metadata(&conn, "embedding_dimensions").as_deref(), Some("384"))` — **padding**, same problem. The value `"384"` is also the default constant, so this assertion would pass even if production code never wrote it.

**Missing tests:**
- `get_model_metadata` returns `None` for a key that was never inserted. This is a critical behavior — `ensure_vec0_tables` branches on `get_model_metadata(...).and_then(|v| v.parse::<usize>().ok())` returning `None`. If `get_model_metadata` silently returns a default instead of `None`, the fallback chain in `ensure_vec0_tables` breaks invisibly.
- The production write path: call `ensure_vec0_tables` with a loaded model and verify that `get_model_metadata(conn, "embedding_dimensions")` returns the runtime-discovered dimension, not just the hardcoded 384 default.

**Assertion quality:**
- The `assert_eq!` calls against manually-inserted data provide zero signal about whether `ensure_vec0_tables` correctly populates metadata. They could be replaced by assertions on a fresh empty table (before any insert) to test the `None`-return behavior, which is actually meaningful.

**Parameterizable?** Yes. The two `get_model_metadata` assertions (assertions 2 and 3) are structurally identical — same function, different key/value pairs. If these were retained, they belong in a parameterized helper `assert_metadata_readable(conn, key, expected_value)` rather than duplicated inline.

**Name accuracy:** The name `_table_created_and_readable` is accurate for what is tested but misleads about the write origin — "readable" implies the production path wrote the data.

---

### Test 2: `t9_6_no_model_db_setup_skips_vec0_tables_without_crashing`

**RTM coverage claimed:** RTM-060 — without ONNX model, db init skips vec0 tables without crashing.

**What is actually tested:**
1. `assert!(table_exists(&conn, "model_metadata"))` — **load-bearing**. Confirms `MODEL_METADATA_TABLE_SQL` executes even when the vec0 extension is absent.
2. `assert!(!table_exists(&conn, "vec_concepts"))` — **load-bearing** (asserts correct negative). But the mechanism driving this assertion is mislabeled.
3. `assert!(!table_exists(&conn, "vec_memory_files"))` — **load-bearing** (asserts correct negative). Same concern.

**The causation error (critical):**

The test calls `ctx.hide_packaged_model()`, which renames `assets/models/all-MiniLM-L6-v2.onnx` to a `.dynamic-model-tests` backup. The comment claims this is why vec0 tables are skipped.

The actual execution path:
```
initialize_database(ctx.config())
  → create_vector_tables(conn, config)
    → ensure_vec0_tables(conn, config.assets_path)   // assets_path = temp dir
      → load_vec0_extension(conn, assets_path)
          // looks for native/{platform}/vec0.{ext} under the temp-dir assets_path
          // the temp dir has NO native/ subdirectory → returns Err
      → returns Ok(()) early, no vec0 DDL executed
```

The ONNX model (`.onnx` file) is never consulted during this code path. `load_vec0_extension` fails because `assets_path` is a fresh temp dir with no `native/` subdirectory — regardless of whether the packaged model is hidden.

**Consequence of the error:** `hide_packaged_model()` is almost certainly a no-op for the vec0 skip behavior. It mutates a file outside the test's isolated sandbox, relying on `TEST_MUTEX` for serialization and `TestContext::Drop` for cleanup. If the cleanup fails (panic before drop, process kill), the packaged model file is left renamed until the next test run. This is a real operational risk with zero payoff.

**What the test *actually* proves:**
> When `Config::assets_path` points to a directory containing no `native/{platform}/vec0.{ext}` binary, `initialize_database` creates `model_metadata` but skips `vec_concepts` and `vec_memory_files` without panicking.

This is a valid and load-bearing behavioral contract — but it should say so clearly and not involve the packaged ONNX model at all.

**Missing coverage:**
- The positive counterpart: when the vec0 extension *is* available, both `vec_concepts` and `vec_memory_files` are created. This is the affirmative case for RTM-057/RTM-059 and is not tested anywhere in this file.
- RTM-058 and RTM-059 (dimension-preserving recreation: if `embedding_dimensions` changes, old tables are dropped and new ones created with correct dimension) have zero test coverage in this file.

**Global state concern:** `hide_packaged_model()` writes outside the `_tmp: TempDir` scope and relies on `Drop` restoration. This pattern is fragile under test-process crashes and unsafe if multiple test binaries from different crates run concurrently (the mutex only guards within-process serialization). If the ONNX model hiding is confirmed to be redundant, it must be removed.

---

## Recommendations

1. **Rewrite Test 1** as three focused tests:
   - `t9_6_model_metadata_table_created_after_init` — call `initialize_database`, assert `table_exists(&conn, "model_metadata")`. Pure, narrow, impossible to misread.
   - `t9_6_get_model_metadata_returns_none_for_missing_key` — on a fresh table, assert `get_model_metadata(&conn, "nonexistent").is_none()`. This tests the critical fallback path in `ensure_vec0_tables` that is currently unexercised.
   - `t9_6_ensure_vec0_tables_writes_embedding_dimensions_metadata` — call `ensure_vec0_tables` in a context where a vec0 extension is available, assert that `get_model_metadata(&conn, "embedding_dimensions")` returns the correct dimension. This tests the production write path.

2. **Rewrite Test 2** to remove `hide_packaged_model()` and rename it:
   - New name: `t9_6_vec0_tables_skipped_when_extension_unavailable` — accurately describes the actual mechanism.
   - Remove `ctx.hide_packaged_model()`. The isolated temp-dir `assets_path` already has no vec0 binary; hiding the ONNX file has no effect on this code path.
   - Add a comment explaining *why* the tables are absent: "config.assets_path is an isolated temp dir with no native/ subdirectory; load_vec0_extension returns Err; ensure_vec0_tables returns early without issuing DDL."

3. **Add the positive counterpart to Test 2**: `t9_6_vec0_tables_created_when_extension_available` — install the real vec0 extension binary into a temp-dir assets path and assert both `vec_concepts` and `vec_memory_files` are created. This is the RTM-059 affirmative case and is currently absent.

4. **Add RTM-058/059 dimension-recreation tests**: a test that pre-populates `model_metadata` with `embedding_dimensions = "128"` but calls `ensure_vec0_tables` with a runtime dimension of 384 should observe the old tables dropped and new ones created with dimension 384. This validates the most complex branch in `ensure_vec0_tables` and is completely untested.

5. **Audit `hide_packaged_model` across the test suite**: if this method appears in other tests in the `dynamic_model/` suite, verify each one actually requires the ONNX model to be hidden (not just the vec0 extension to be absent). Remove redundant usages to eliminate unnecessary global filesystem mutation.

---

## Load-Bearing Score: 3/5

Of the five distinct assertions across the two tests:
- `table_exists(&conn, "model_metadata")` in Test 1 — **load-bearing**
- `get_model_metadata` returning manually-inserted "test-model.onnx" — **not load-bearing** (padding)
- `get_model_metadata` returning manually-inserted "384" — **not load-bearing** (padding)
- `table_exists(&conn, "model_metadata")` in Test 2 — **load-bearing**
- `!table_exists(&conn, "vec_concepts")` and `!table_exists(&conn, "vec_memory_files")` — **load-bearing** (negative assertions, correct but mislabeled)

3 of 5 assertions carry genuine defect-detection power. The two padding assertions in Test 1 provide false confidence by exercising a manual-insert/read loop instead of the production write path. Both tests have REWRITE verdicts: the assertions they contain are salvageable, but the framing, causation documentation, and coverage scope all need rework.

---
## SWE Remediation
- REWRITE `t9_6_model_metadata_table_created_and_readable` → split into 3 focused tests:
  - `t9_6_model_metadata_table_created_after_init`: pure schema assertion (RTM-057)
  - `t9_6_get_model_metadata_returns_none_for_missing_key`: validates None-return for absent keys, tests critical fallback path in ensure_vec0_tables (RTM-057)
  - `t9_6_ensure_vec0_tables_writes_embedding_dimensions_metadata`: exercises production upsert path, asserts DEFAULT_EMBEDDING_DIMENSIONS persisted (RTM-057) — requires vec0 extension
- REWRITE `t9_6_no_model_db_setup_skips_vec0_tables_without_crashing` → renamed to `t9_6_vec0_tables_skipped_when_extension_unavailable`: removed redundant `hide_packaged_model()` call, fixed causation comment to correctly attribute vec0 skip to missing extension binary in isolated temp-dir (RTM-060)
- ADD `t9_6_vec0_tables_created_when_extension_available`: positive counterpart asserting both vec_concepts and vec_memory_files are created when vec0 extension is present (RTM-059)
- Removed manual INSERT padding assertions (SQLite round-trip tests), replaced with production-path coverage
- Test count: 2 → 5; all 5 pass
- Commit: 9e46737
