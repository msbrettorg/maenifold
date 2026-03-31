---
id: sync_single_file_vec0_readiness
status: done
---

# Test Evaluation: sync_single_file_vec0_readiness

**Traceability**: T-15.20 · FR-8  
**Evaluated by**: Red-Team / Gray (Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/sync_single_file_vec0_readiness.rs`  
**Support module**: `crates/maenifold-core/tests/sync_unification/support.rs`

---

## Summary

This file contains **2 tests** asserting that `sync_single_file` degrades gracefully when the
`vec0` SQLite extension binary is absent from the assets path. The module-level docstring frames
both tests as verifying that `vector_ready` is set correctly and that embedding generation is
skipped without crashing.

**Gray finding (core):** The stated purpose — verifying `vector_ready=false` and the no-vec0 code
path — is **never validated by any assertion**. Every assertion in both tests is concerned with
general sync correctness (output string match, `file_content` row existence, `concept_mentions`
counts). That contract is already exercised by `sync_unification_single_file.rs`
(`process_file_new_file_inserts_content_mentions_and_index_rows`, T-5.7) using the exact same
`isolated_config()` setup, which also lacks vec0 binaries in its temp dir.

The critical assertion gap: no test queries `vec_concepts` or `vec_memory_files` tables to verify
they remain empty, and `sync_single_file`'s output format does not surface vector diagnostics
(unlike `sync`/`sync_files`, which emit a `Vector sync diagnostics:` block). The tests cannot
distinguish a "vec0 skipped cleanly" outcome from a "vec0 ran and produced embeddings"
outcome — both would produce identical output and identical `file_content` / `concept_mentions`
rows.

**Secondary finding:** The implicit guarantee that `isolated_config()` lacks vec0 is
load-bearing but undocumented. All tests in the entire `sync_unification` family run without
vec0 because the tempdir has no `assets/native/…/vec0.*` binary. If `isolated_config()` were
ever updated to copy native assets into the temp dir (for other test reasons), these tests would
silently start exercising the `vector_ready=true` path while their names still say `_without_vec0`.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique Coverage? | Correct Assertions? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `sync_single_file_without_vec0_processes_file_successfully` | ⚠️ Partial | ❌ No | ⚠️ Correct but insufficient | ❌ No | ⚠️ Marginal | **REWRITE** |
| 2 | `sync_single_file_without_vec0_handles_content_updates` | ⚠️ Partial | ❌ No | ⚠️ Correct but insufficient | ❌ No | ⚠️ Marginal | **REWRITE** |

### Detailed Per-Test Analysis

---

#### Test 1 — `sync_single_file_without_vec0_processes_file_successfully`

**What it asserts:**
1. `output.contains("Synced memory://tests/sync-unification/vec0-readiness")` — fragile string
   match; will pass even if the `.md` extension is in the URI because `contains` is a substring
   check. Passes trivially on any successful sync of this file.
2. `output.contains("concept mentions updated")` — this string is **hard-wired into the
   `sync_single_file` output format string** (`"{mention_count} concept mentions updated"`), so
   any successful sync of any file will satisfy this. Zero specificity to the vec0 scenario.
3. `get_file_content_row(&cfg, &file_uri).is_some()` — correct, confirms indexing happened.
4. `get_mention_count(…) == Some(1)` — correct, confirms mention extraction.

**What it does NOT assert:**
- That `vec_concepts` is empty (no concept embeddings were written).
- That `vec_memory_files` is empty (no file embedding was written).
- That any diagnostic or log message indicates the vec0 path was skipped.
- That `vector_ready` was `false` in the code path exercised.

**Overlap with existing tests:**  
`sync_unification_single_file.rs` → `process_file_new_file_inserts_content_mentions_and_index_rows`
makes the same three DB assertions with the same isolation setup. The *only* differentiator is
the concept name and file name. There is no behavioral difference under test.

**Name accuracy:** The name claims `_without_vec0` as a meaningful qualifier. It is not — the
test is silent on vec0. A reader trusting this test as coverage for FR-8 / T-15.20 is being misled.

---

#### Test 2 — `sync_single_file_without_vec0_handles_content_updates`

**What it asserts:**
1. `output_initial.contains("Synced")` — trivially passes on any successful initial sync.
2. `get_mention_count(concept_a) == Some(1)` after initial sync — correct.
3. Rewrites file, bumps mtime by 1 minute via `set_file_mtime_utc`.
4. `output_updated.contains("concept mentions updated")` — same hard-wired format string problem
   as Test 1; zero vec0 specificity.
5. `get_mention_count(concept_a) == Some(1)` after update — correct.
6. `get_mention_count(concept_b) == Some(1)` after update — correct.

**What it does NOT assert:** (same list as Test 1, plus:)
- That concept_a's `vec_concepts` embedding was **not** created/updated on re-sync.
- No diagnostic output about vector sync skipped.

**Overlap with existing tests:**  
The update-lifecycle pattern (initial sync → write + mtime bump → re-sync) is exercised by:
- `sync_unification_single_file.rs` → `process_file_mtime_differs_hash_differs_*` family.
- `sync_mtime_opt_guards.rs` — dedicated mtime skip/process guard tests.
- `incremental_sync_unification_lifecycle.rs` — lifecycle variants.

The vec0 framing adds no differentiation because all these tests also run in `isolated_config()`
environments that lack vec0 binaries.

**Timing concern:** `+ chrono::Duration::minutes(1)` is correct here, but the mechanism
(explicit `set_file_mtime_utc`) rather than sleeping means the test is fast and non-flaky. This
is the one genuinely good pattern in the file.

---

## Recommendations

### 1. Add `vec_concepts` / `vec_memory_files` query helpers to `support.rs`

The absence of any helper to query the vec0 tables is the root cause of the assertion gap.
Add to `crates/maenifold-core/tests/sync_unification/support.rs`:

```rust
/// Returns the count of rows in vec_concepts, or None if the table does not exist.
pub(crate) fn get_vec_concepts_row_count(cfg: &Config) -> Option<i64> {
    let conn = db_connection(cfg);
    conn.query_row(
        "SELECT COUNT(*) FROM vec_concepts",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .unwrap_or(None)
}

/// Returns the count of rows in vec_memory_files, or None if the table does not exist.
pub(crate) fn get_vec_memory_files_row_count(cfg: &Config) -> Option<i64> {
    let conn = db_connection(cfg);
    conn.query_row(
        "SELECT COUNT(*) FROM vec_memory_files",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .unwrap_or(None)
}
```

### 2. Rewrite Test 1 — add vec0-specific assertions

Replace the two output-string assertions that carry no vec0 signal with assertions that actually
verify the vec0 code path was skipped:

```rust
// After sync completes — vec0 tables must either not exist or be empty:
assert_eq!(
    get_vec_concepts_row_count(&cfg),
    Some(0),
    "vec_concepts must be empty when vec0 extension is absent"
);
assert_eq!(
    get_vec_memory_files_row_count(&cfg),
    Some(0),
    "vec_memory_files must be empty when vec0 extension is absent"
);
```

If the vec0 tables are created as part of `GraphDatabase::initialize_database` regardless of
extension availability, the `Some(0)` pattern is correct. If the tables don't exist at all when
vec0 is absent, the helpers should return `None` and the assertion should be:

```rust
assert!(
    get_vec_concepts_row_count(&cfg).map_or(true, |n| n == 0),
    "no concept embeddings should be written when vec0 is absent"
);
```

### 3. Rewrite Test 2 — same additions + explicit second-sync vector check

After re-syncing with updated content, verify that concept_b also did not produce an embedding:

```rust
assert!(
    get_vec_concepts_row_count(&cfg).map_or(true, |n| n == 0),
    "no embeddings for concept_b when vec0 is absent on re-sync"
);
```

### 4. Rename both tests to match what they actually verify

| Current Name | Proposed Name |
|---|---|
| `sync_single_file_without_vec0_processes_file_successfully` | `sync_single_file_without_vec0_indexes_content_and_skips_embeddings` |
| `sync_single_file_without_vec0_handles_content_updates` | `sync_single_file_without_vec0_update_lifecycle_skips_embeddings_on_resync` |

### 5. Document the vec0-absent invariant of `isolated_config()`

The entire test file's correctness depends on `isolated_config()` producing a config whose
`assets_path` contains no vec0 binary. This is true today but is a silent assumption. Add a
comment to `isolated_config()` in `support.rs`:

```rust
// NOTE: the temp dir has no assets/ tree, so vec0_extension_available() will
// return false for all tests using this config. Tests that require vec0-absent
// behavior rely on this invariant. Do NOT copy native binaries into tmp.
pub(crate) fn isolated_config() -> (TempDir, Config) { ... }
```

### 6. Consider removing `sync_single_file_without_vec0_handles_content_updates` if stronger coverage exists

After Test 1 is rewritten to assert vec0 table emptiness, Test 2 only adds the "update
lifecycle with mtime bump" scenario. If `incremental_sync_unification_lifecycle.rs` or
`sync_unification_single_file.rs` already cover update + re-index cycles (they do), Test 2's
only unique contribution is "vec0 tables remain empty after re-sync." That can be added as a
second assertion in Test 1, or kept as a separate test for test clarity. This is a judgment call
— the cycle test is not harmful to keep if the assertions are strengthened.

---

## Load-Bearing Score: 1/2

**Rationale:**  
Both tests currently assert only general sync correctness — the same behavior validated by
`sync_unification_single_file.rs` T-5.7 tests. In that sense, **0/2** tests are load-bearing on
their *stated* purpose (vec0 readiness). However, the T-15.20 / FR-8 traceability anchored to
this file means removing both would leave a gap in the requirements traceability matrix.
Therefore 1/2 reflects: one test is structurally worth preserving (once rewritten) because it
covers the "new file" case with vec0 specificity, while the second test's update-lifecycle
coverage is fully duplicated elsewhere and would need only the vec0 table assertions to justify
its existence.

If the rewrites are applied (Recommendations 1–3), the revised score becomes **2/2 load-bearing**.
Without the rewrites, the effective load-bearing score on the vec0-specific contract is **0/2**.

---

*Evaluated under Gray (Skeptical Inquiry). Key question applied throughout:*  
*"What evidence do these tests actually provide that vec0 is being skipped, and not merely that sync works?"*

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- sync_single_file_vec0_readiness`: 2 passed, 0 failed
- Verdict: PASS
