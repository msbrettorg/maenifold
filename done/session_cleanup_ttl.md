# Test Evaluation: `session_cleanup_ttl`

**File**: `crates/maenifold-core/tests/session_cleanup_ttl.rs`  
**Traceability**: T-4.5b · RTM FR-4.2 (all three tests)  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**Date**: 2025-05-24

---

## Summary

The file contains **3 tests** covering the TTL-based session abandonment logic in
`sessions::cleanup_abandoned_sessions`. All three call the same production function
against isolated tempdir environments, using a shared support module at
`tests/session/session_cleanup_support.rs`.

The tests sit in a small cluster alongside `session_cleanup_sweep.rs` (multi-session
sweeps, empty-DB, non-active statuses) and `workflow_supervisor_cleanup_thresholds.rs`
(threshold precision with `align_to_second_boundary()`). Understanding that cluster
context is essential to judging redundancy.

**Overall verdict**: 1 test should be **REWRITTEN** to fix a misleading name and a
latent timing fragility; 1 test should be **REWRITTEN** to add missing assertions; 1
test should be **KEPT** as-is. No test should be removed. Three important coverage
gaps exist and are called out as missing tests.

---

## Per-Test Analysis

---

### Test 1 — `active_session_within_default_ttl_is_not_marked_abandoned`

**Traceability**: T-4.5b · RTM FR-4.2  
**Lines**: 10–33

#### What it does

Seeds `last_indexed = Utc::now() - 30 minutes` (default TTL = 30 min), then runs
cleanup and asserts the session is NOT abandoned. Checks the return count, frontmatter
status, DB status, and body.

#### Load-bearing analysis

The test is load-bearing for the SQL strict-less-than (`<`) boundary:

```sql
WHERE datetime(last_indexed) < datetime(?1, '-30 minutes')
```

A session at exactly `now - 30min` produces **equal timestamps on both sides** after
SQLite's `datetime()` truncates fractional seconds. Strict `<` is therefore false and
the session escapes cleanup. If the operator were changed to `<=`, this test would
fail — it is the only guard of that invariant in this file.

#### Correctness of assertions

Assertions are correct and complete for a negative case:

- `assert_eq!(abandoned, 0)` ✓
- `read_frontmatter_status == "active"` ✓
- `read_db_status == "active"` ✓
- `!body.contains("## Session Abandoned")` ✓

#### Naming problem

The test name says **"within default TTL"** but it seeds the session at **exactly the
TTL boundary** (30 minutes for a 30-minute TTL). "Within" conventionally implies
strictly less than. There is no test for a session genuinely inside the window (say 15
minutes old). The name creates a false belief that interior-window behavior is tested.

#### Latent timing fragility

`iso_minutes_ago(30)` calls `Utc::now()` at test setup time (T₀). Inside
`cleanup_abandoned_sessions`, `now_iso8601_roundtrip()` calls `Utc::now()` again (T₁).
The SQL boundary check only "safely" excludes the session when `floor(T₀) == floor(T₁)`
at second granularity — i.e., when T₁ − T₀ < 1 second. On a slow CI runner where
`create_session` (file I/O + DB insert) crosses a second boundary, T₁'s truncated
second ticks forward and the SQL condition becomes true, selecting the session as a
candidate.

If that happens, the filesystem guard (`should_preserve_recent_filesystem_edit`) runs:
the file was just written (mtime ≈ now), so `modified_at < abandonment_cutoff` is
false. Then `metadata_change_lags_mtime` checks `ctime - mtime > 1ms`. For a freshly
created file, ctime ≈ mtime (≈ 0 delta), so it returns false. The filesystem guard
does **not** protect the session. The session gets abandoned, and the test fails with
`abandoned == 1` instead of 0.

Compare: `workflow_supervisor_cleanup_thresholds.rs::try_mark_abandoned_from_last_indexed_exactly_at_threshold_returns_false`
solves this correctly by calling `align_to_second_boundary()` before seeding, ensuring
T₀ and T₁ share the same second. This file has no such protection.

#### Duplication

`workflow_supervisor_cleanup_thresholds.rs` has:
- `try_mark_abandoned_from_last_indexed_recent_session_returns_false` (30 min, count = 0)
- `try_mark_abandoned_from_last_indexed_exactly_at_threshold_returns_false` (exact 5-min boundary)

These cover the same SQL exclusion semantics but for workflow sessions and with greater
precision. The sequentialy-typed session path is distinct enough to warrant a test here,
but the current form is less rigorous than the supervisor counterpart.

---

### Test 2 — `active_session_beyond_default_ttl_is_marked_abandoned`

**Traceability**: T-4.5b · RTM FR-4.2  
**Lines**: 35–64

#### What it does

Seeds `last_indexed = Utc::now() - 180 minutes` (180 >> 30-min TTL), runs cleanup,
and asserts the session IS abandoned. Checks all four observable surfaces: count,
frontmatter, DB status, and both body and DB content for the abandonment notice.

#### Load-bearing analysis

This is the primary positive-path test — the only test in the entire session-cleanup
cluster that verifies **both storage layers** (file and DB content column) are updated
together. Removing it would leave the dual-write behaviour unjustified by any test.

#### Correctness of assertions

All five assertions are correct and meaningful:

- `assert_eq!(abandoned, 1)` ✓
- `read_frontmatter_status == "abandoned"` ✓
- `read_db_status == "abandoned"` ✓
- `body.contains("## Session Abandoned")` ✓
- `db_content.contains("## Session Abandoned")` ✓ — uniquely present in this test

**Minor gap**: The abandonment notice also embeds the inactivity duration in minutes
(`⚠️ Session marked as abandoned due to {N} minutes of inactivity`). No test checks
the numeric value. A bug that inverts `max(inactivity, abandonment_minutes)` or
substitutes zero would go undetected.

#### Filesystem guard interaction

Same freshly-written file issue as test 1 — `should_preserve_recent_filesystem_edit`
returns false because ctime ≈ mtime for the fresh write. This is correct here: with
180 minutes of apparent inactivity, the intent is to abandon the session. The guard
transparently does not interfere. The test does not have a timing fragility risk because
the session's `last_indexed` is 150 minutes beyond the threshold — any sub-second drift
is irrelevant.

---

### Test 3 — `custom_ttl_from_env_var_is_respected`

**Traceability**: T-4.5b · RTM FR-4.2  
**Lines**: 66–89

#### What it does

Sets `MAENIFOLD_SESSION_ABANDON_MINUTES=5` via `SessionCleanupEnvGuard`, seeds a
session 10 minutes old, runs cleanup. Under the default TTL (30 min) the session would
NOT be abandoned; under the custom TTL (5 min) it SHOULD be.

#### Load-bearing analysis

This is the **only test** covering the env-var TTL override path (lines 13–17 of
`cleanup.rs`). If removed, a regression in `std::env::var("MAENIFOLD_SESSION_ABANDON_MINUTES")`
parsing would go undetected.

#### Correctness of assertions

**Missing assertions compared to test 2**:

| Assertion | Test 2 | Test 3 |
|-----------|--------|--------|
| `abandoned == N` | ✓ | ✓ |
| `read_frontmatter_status` | ✓ | ✓ |
| `read_db_status` | ✓ | **MISSING** |
| `body.contains("## Session Abandoned")` | ✓ | ✓ |
| `db_content.contains("## Session Abandoned")` | ✓ | **MISSING** |

The two missing DB assertions mean a regression where `cleanup_abandoned_sessions`
correctly updates the filesystem but fails to update the database (or updates it under
the wrong TTL branch) would not be caught by this test specifically.

#### `unsafe` env-var ordering

The test locks `TEST_MUTEX` first, then creates `SessionCleanupEnvGuard`. This order is
correct — the lock is held before the env is mutated, consistent with the safety
comment. The guard restores the previous value on drop before the mutex is released.
This is sound.

---

## Verdicts

| # | Test Name | Verdict | Reason |
|---|-----------|---------|--------|
| 1 | `active_session_within_default_ttl_is_not_marked_abandoned` | **REWRITE** | Name misrepresents semantics (boundary, not "within"); latent timing fragility absent in the supervisor counterpart; add genuine interior-window case (e.g. 15 min) |
| 2 | `active_session_beyond_default_ttl_is_marked_abandoned` | **KEEP** | Most thorough test in file; unique dual-storage assertion; correct and load-bearing |
| 3 | `custom_ttl_from_env_var_is_respected` | **REWRITE** | Only test for env-var TTL path (must keep); add `read_db_status` and `read_db_content` assertions to match test 2's thoroughness |

---

## Coverage Gaps (Missing Tests)

These behaviors exist in `cleanup.rs` but are not tested anywhere in this file or its
immediate siblings:

### MISSING-1: Filesystem guard preserving a genuinely-recent-edit session

The `should_preserve_recent_filesystem_edit` / `metadata_change_lags_mtime` path (T-4.6
/ T-0.3) has no test. The scenario it guards: a session whose `last_indexed` in the DB
is stale but whose on-disk mtime was touched recently AND whose ctime lags mtime by
> 1ms (indicating a deliberate backdated DB timestamp, not a fixture side-effect). No
test exercises `metadata_change_lags_mtime` returning `true`.

### MISSING-2: Invalid / zero / negative env var falls back to default

The implementation guards:
```rust
.and_then(|value| value.trim().parse::<i64>().ok())
.filter(|value| *value > 0)
.unwrap_or(30);
```
No test exercises `MAENIFOLD_SESSION_ABANDON_MINUTES=abc`, `=0`, or `=-5`. Each should
silently fall through to the 30-minute default.

### MISSING-3: Session path not under `memory://thinking/%`

The SQL `LIKE 'memory://thinking/%'` filter would ignore, for example, a file at
`memory://notes/active-note`. No test verifies that non-matching paths are never
touched regardless of age.

### MISSING-4: Idempotency — session body already contains `## Session Abandoned`

`append_abandoned_notice` is idempotent:
```rust
if content.contains("## Session Abandoned") {
    return content.to_string();
}
```
But this path is unreachable in practice because the SQL already filters
`status = 'active'` (abandoned sessions have a different status). The combination of
these two guards is never explicitly tested, and the redundancy could become a latent
bug if the SQL filter is ever relaxed.

---

## Recommendations

1. **Rename test 1** to `active_session_at_exact_ttl_boundary_is_not_marked_abandoned`
   and add a companion test `active_session_well_within_ttl_is_not_marked_abandoned`
   (e.g. 15 minutes) to cover the interior case independently.

2. **Fix timing fragility in test 1** by calling `align_to_second_boundary()` (already
   used in `workflow_supervisor_cleanup_thresholds.rs`) before seeding, so that T₀ and
   T₁ reliably share the same truncated second.

3. **Add missing DB assertions to test 3**: `read_db_status` and `read_db_content`
   checks matching what test 2 already verifies, so the env-var path is held to the
   same standard.

4. **Add inactivity-minutes assertion to test 2**: assert that the body contains the
   specific inactivity duration (e.g. `"180 minutes"`) not just the section heading.

5. **Add MISSING-2 test**: parameterise over `["abc", "0", "-1", ""]` and confirm each
   triggers a 30-minute-TTL abandonment of a 31-minutes-old session.

6. **Consider parameterizing tests 1 and 2** with `rstest` or a table-driven helper;
   the structural pattern (create → cleanup → assert) is identical and lends itself to
   a `#[rstest]` data table with `(minutes_old, expected_abandoned, expected_status)`.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. Each covers a distinct invariant:

| Test | What breaks if removed |
|------|------------------------|
| Test 1 | No guard on SQL `<` strict boundary; `<=` regression goes undetected |
| Test 2 | No test that both storage layers (file + DB content) are updated together |
| Test 3 | No test that `MAENIFOLD_SESSION_ABANDON_MINUTES` env var is read at all |

None are safe to delete. Tests 1 and 3 should be rewritten before the next sprint to
remove the timing fragility and assertion gap respectively.

---

*Evaluation performed under [[Gray - Skeptical Inquiry]] color. References: [[test-quality]], [[maenifold-rs]], [[session cleanup TTL]], [[FR-4.2 session abandonment]]*

---
## SWE Remediation
- REWRITE test 1: renamed `active_session_within_default_ttl_is_not_marked_abandoned` → `active_session_at_exact_ttl_boundary_is_not_marked_abandoned`; fixed timing fragility with `align_to_second_boundary()` + anchor-based seeding (mirrors supervisor pattern)
- ADD `active_session_well_within_ttl_is_not_marked_abandoned`: companion interior-window test (15 min vs 30-min TTL) per recommendation 1
- KEEP test 2 (`active_session_beyond_default_ttl_is_marked_abandoned`): unchanged
- REWRITE test 3 (`custom_ttl_from_env_var_is_respected`): added `read_db_status` and `read_db_content` assertions to match test 2's dual-storage thoroughness per recommendation 3
- ADD `invalid_env_var_falls_back_to_default_ttl`: covers MISSING-2 gap — iterates `["abc", "0", "-1", ""]` confirming 30-min default fallback per recommendation 5
- Support module: made `write_session_markdown`/`seed_file_content` pub; added `align_to_second_boundary()` per recommendation 2
- All 5 tests pass: `cargo test --test integration_tests -p maenifold-core -- session_cleanup_ttl`
- Commit: 2f21186

---
## Red-team Audit

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | REWRITE `active_session_within_default_ttl…` → `…at_exact_ttl_boundary…` | ✅ | Renamed correctly; uses `align_to_second_boundary()` + anchor-based `Utc::now().with_nanosecond(0)` seeding to eliminate latent timing fragility; matches supervisor pattern |
| 2 | ADD `active_session_well_within_ttl_is_not_marked_abandoned` | ✅ | Interior-window companion at 15 min; 4 assertions (count, frontmatter, DB status, body) |
| 3 | KEEP `active_session_beyond_default_ttl_is_marked_abandoned` | ✅ | Zero diff in commit — test unchanged with all 5 original assertions intact |
| 4 | REWRITE `custom_ttl_from_env_var_is_respected` | ✅ | Added `read_db_status == "abandoned"` and `db_content.contains("## Session Abandoned")` matching test 2's dual-storage standard |
| 5 | ADD `invalid_env_var_falls_back_to_default_ttl` (MISSING-2) | ✅ | Iterates `["abc", "0", "-1", ""]`; seeds 31-min-old session; asserts `abandoned == 1` and frontmatter status per recommendation 5 |
| 6 | Support module changes | ✅ | Only necessary changes: `align_to_second_boundary()` added, `write_session_markdown`/`seed_file_content` made `pub` |
| 7 | No unrelated changes | ✅ | Commit touches exactly 2 files, both scoped to the eval |
| 8 | Tests pass | ✅ | `running 5 tests … test result: ok. 5 passed; 0 failed` confirmed |

**Eval recommendation 4** (inactivity-minutes assertion on test 2) was not implemented, but this is consistent with the KEEP verdict for test 2 — the recommendation was an enhancement suggestion, not part of the verdict. No fault.

**Eval recommendation 6** (rstest parameterization) was not implemented. The eval used "consider" language, not a directive. No fault.

- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_cleanup_ttl`: 5 passed, 0 failed
- Tests verified:
  1. `active_session_at_exact_ttl_boundary_is_not_marked_abandoned` — ok
  2. `active_session_well_within_ttl_is_not_marked_abandoned` — ok
  3. `active_session_beyond_default_ttl_is_marked_abandoned` — ok
  4. `custom_ttl_from_env_var_is_respected` — ok
  5. `invalid_env_var_falls_back_to_default_ttl` — ok
- Verdict: **PASS**
