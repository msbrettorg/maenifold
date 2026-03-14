# Test Evaluation: session_cleanup_sweep

**File:** `crates/maenifold-core/tests/session_cleanup_sweep.rs`
**Traceability:** T-4.5b · FR-4.2
**Evaluator color:** 🔘 Gray — Skeptical Inquiry
**Support module:** `crates/maenifold-core/tests/session/session_cleanup_support.rs`
**Implementation under test:** `crates/maenifold-core/src/sessions/cleanup.rs`

---

## Summary

`session_cleanup_sweep.rs` contains **3 tests** that together cover the branching and batching
behavior of `cleanup_abandoned_sessions`: terminal-status immunity, multi-session sweep
completeness, and empty-database robustness. A companion file, `session_cleanup_ttl.rs`,
owns the TTL boundary logic (default threshold, custom env-var, within-vs-beyond threshold).
The split is sensible but not perfectly consistent in assertion depth.

The test infrastructure is deliberately serialized via a process-global `parking_lot::Mutex`
because `isolated_env()` mutates global `Config` state. This is necessary and correctly
implemented, but it means **all tests in this module run serially**, directly impacting CI
throughput for the session-cleanup test surface.

A subtle correctness dependency exists in the implementation: `abandon_session_candidate`
performs a filesystem `mtime` vs. `ctime` comparison (`should_preserve_recent_filesystem_edit`,
T-0.3/NFR-4) before acting on a DB-stale candidate. Tests rely on the fact that freshly
written fixtures do **not** trigger this guard (ctime ≈ mtime for a brand-new write). This
assumption is correct but implicit — no test comment explains the dependency, making it a
latent trap for future fixture refactors.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Quality | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `completed_and_cancelled_sessions_are_not_touched_regardless_of_age` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Good | ✅ Yes (add `abandoned`) | **REWRITE** |
| 2 | `multiple_abandoned_sessions_are_detected_in_one_sweep` | ✅ Yes | ✅ Yes | ❌ Incomplete | ✅ Good | ⚠️ Minor | **REWRITE** |
| 3 | `empty_database_returns_zero_abandoned_sessions` | ⚠️ Smoke | ✅ Yes | ⚠️ Partial | ✅ Good | ❌ No | **KEEP** |

---

## Per-Test Analysis

---

### Test 1 — `completed_and_cancelled_sessions_are_not_touched_regardless_of_age`

**What it claims:** Completed and cancelled sessions are immune to cleanup regardless of age.

**What it actually does:** Creates one `completed` (600 min old) and one `cancelled`
(600 min old) session, then verifies both counts and individual session states.

**Gray inquiry — assumptions challenged:**

1. **SQL filter is the actual guard.** The implementation filters `WHERE status = 'active'` in
   SQL, so this test is effectively verifying that the WHERE clause has not been widened to
   include terminal statuses. The assertions confirm correctness; the mechanism is sound.

2. **The "regardless of age" claim is backed.** Both sessions are 600 minutes old — well past
   the 30-minute default and any plausible custom TTL. The name is honest.

3. **Missing: `abandoned` status coverage.** The test covers `completed` and `cancelled` but
   not `abandoned`. An already-abandoned session should also be immune. The three terminal
   statuses are `completed`, `cancelled`, `abandoned` (see `thinking/session.rs:92` and
   `workflow/submachine.rs:72`). Omitting `abandoned` leaves a gap — if the SQL WHERE clause
   were changed to `status NOT IN ('abandoned')` it would pass this test but incorrectly
   re-process an already-abandoned session.

4. **Assertion completeness:** Checks `frontmatter_status`, `db_status`, and body content
   for both sessions. This is good three-layer verification. Consistent with TTL test
   assertion depth.

5. **Parameterizable?** Yes — iterate over `[("completed", 600), ("cancelled", 600),
   ("abandoned", 600)]` to close the gap. This would be a strict improvement without
   overcomplicating the test.

**Verdict: REWRITE** — extend the terminal-status set to include `abandoned`.

---

### Test 2 — `multiple_abandoned_sessions_are_detected_in_one_sweep`

**What it claims:** Multiple stale sessions are detected in a single pass; a recent session
is untouched.

**What it actually does:** Seeds two old active sessions and one recent active session,
asserts `abandoned == 2`, then checks frontmatter status on the two old sessions only.

**Gray inquiry — assumptions challenged:**

1. **Count assertion is load-bearing.** `assert_eq!(abandoned, 2)` verifies the loop in
   `cleanup_abandoned_sessions` doesn't short-circuit after the first match. This is the
   unique value this test adds over the TTL tests.

2. **DB status not verified for abandoned sessions.** Tests 1 and TTL-test-2 check both
   `read_frontmatter_status` and `read_db_status`. This test checks only `frontmatter_status`
   for the two old sessions. A bug that abandoned markdown but failed to UPDATE the DB row
   would pass this test.

3. **Recent session not explicitly asserted "active".** The test proves `abandoned == 2` and
   checks the two old sessions, but never asserts that `recent_active` remains `"active"` in
   either frontmatter or DB. If the implementation accidentally set `recent_active` to
   `"abandoned"` while still returning count 2 (e.g., through a side-effect), this test
   would pass.

4. **Body / notice content not checked** for either abandoned session. TTL-test-2 verifies
   `body.contains("## Session Abandoned")` and `db_content.contains(...)`. This test does
   not. The `append_abandoned_notice` path is left unverified here.

5. **Boundary value concern — `minutes_ago = 15`.** Default threshold is 30 minutes. A value
   of 15 is safely inside the preservation window. However the gap between 15 and 30 is
   tight — on a slow CI machine where the test itself might take minutes, this remains safe,
   but 5 would be more obviously conservative.

6. **mtime/ctime guard interaction.** All three sessions are written just before cleanup runs.
   The implementation's `should_preserve_recent_filesystem_edit` guard checks whether
   `ctime` lags `mtime` — for brand-new files they are equal, so the guard correctly allows
   abandonment. This is correct but wholly undocumented in the test. A comment referencing
   T-0.3/NFR-4 would make the fixture's "just written" timing intentional rather than
   accidental.

**Verdict: REWRITE** — add `read_db_status` checks for both abandoned sessions, add explicit
`"active"` status assertion for `recent_active`, add body/notice content checks, reduce
`recent_active` minutes_ago to 5, add comment about mtime/ctime guard expectation.

---

### Test 3 — `empty_database_returns_zero_abandoned_sessions`

**What it claims:** Cleanup on an empty database returns 0 and leaves no rows.

**What it actually does:** Calls cleanup with no sessions seeded, then asserts
`abandoned == 0` and `file_content_row_count == 0`.

**Gray inquiry — assumptions challenged:**

1. **`file_content_row_count == 0` is a structural invariant, not a behavioral assertion.**
   The cleanup implementation only SELECTs and UPDATEs — it never INSERTs. The row count
   check cannot fail unless the cleanup function were radically redesigned to also create
   rows. This assertion documents intent ("cleanup should not add rows") but provides zero
   regression protection for any plausible implementation change.

2. **Smoke test value.** The real load-bearing work here is that `cleanup_abandoned_sessions`
   doesn't panic on an empty `file_content` table (e.g., if it were to call `.next()` on a
   cursor without checking). It exercises the `prepare` → `query_map` → empty-iterate path.
   This is a legitimate, if minimal, edge-case test.

3. **Could a missing table cause a panic?** `isolated_env()` calls `ensure_directories()` and
   `GraphDatabase::initialize_database()` which creates the schema including `file_content`.
   So the "empty" case means zero rows, not a missing table. The test is correctly scoped.

4. **Could this be merged?** It could logically be the degenerate case of test 2 (set N=0),
   but at 6 lines it is too small to warrant merging. Keep it separate.

**Verdict: KEEP** — low assertion richness, but the test has unique smoke value and is
correctly scoped. Add a comment clarifying why `file_content_row_count` is checked.

---

## Recommendations

| Priority | Action | Rationale | Linked Test |
|----------|--------|-----------|-------------|
| **P1** | Add `abandoned` to terminal-status set in test 1 | Closes a real assertion gap; a 3-status iterate covers the complete terminal set | Test 1 |
| **P1** | Add `read_db_status` for both abandoned sessions in test 2 | TTL tests do it; sweep test should be consistent; catches DB-update bugs | Test 2 |
| **P2** | Add explicit `"active"` assertion for `recent_active` in test 2 | Count alone cannot prove the recent session was untouched | Test 2 |
| **P2** | Add body / notice content checks to test 2 | The `append_abandoned_notice` path is exercised but unverified in this test | Test 2 |
| **P3** | Lower `recent_active` minutes_ago from 15 to 5 in test 2 | Increase margin against boundary races on slow CI | Test 2 |
| **P3** | Add T-0.3/NFR-4 comment in test 2 explaining the mtime/ctime guard expectation | Prevents future fixture refactors from breaking tests silently | Test 2 |
| **P4** | Annotate the `file_content_row_count` assertion in test 3 | Clarify it is a documentation-grade invariant, not a behavior regression check | Test 3 |

---

## Positive Observations

- **Mutex serialization is correctly scoped.** `TEST_MUTEX.lock()` is held for the full
  duration of each test, safely serializing the process-global `Config` mutation. The
  `Drop` impl on `TestEnv` restores `Config` overrides. No resource leak risk.

- **Fixture isolation is genuine.** `isolated_env()` uses `tempfile::TempDir`, gives each
  test its own DB and filesystem root, and relies on the `Drop` impl to clean up. No shared
  state between tests.

- **The sweep/TTL split is principled.** TTL-focused concerns (threshold values, env var
  override, boundary detection) live in `session_cleanup_ttl.rs`. Sweep-focused concerns
  (terminal-status immunity, batching, empty DB) live here. This split reduces test surface
  overlap and aids future maintenance.

- **`create_session` double-seeds correctly.** The helper writes both the markdown file and
  the DB row, matching what the real indexer produces. This makes the test environment
  realistic without over-mocking.

- **Test names are verbs-as-propositions.** All three names read as falsifiable claims:
  *"completed and cancelled sessions are not touched regardless of age"*, etc. This is the
  correct naming idiom for behavior-driven tests.

---

## Load-Bearing Score: 6/9

| Test | Score | Rationale |
|------|:---:|-----------|
| `completed_and_cancelled_sessions_are_not_touched_regardless_of_age` | 3/3 | Verifies SQL status filter, frontmatter guard, and DB guard — but missing `abandoned` coverage means one real scenario escapes |
| `multiple_abandoned_sessions_are_detected_in_one_sweep` | 1/3 | Count is load-bearing; frontmatter check for old sessions is partial; DB status and recent-session guard completely absent |
| `empty_database_returns_zero_abandoned_sessions` | 2/3 | Correct smoke test; row-count assertion is structurally vacuous but documents intent |

> **Interpretation:** 6/9 — The sweep test suite carries meaningful signal but has two
> concrete assertion gaps (DB status consistency in test 2, missing `abandoned` status in
> test 1) that would allow real regressions to pass undetected. Both are fixable with
> ≤10 lines each.
