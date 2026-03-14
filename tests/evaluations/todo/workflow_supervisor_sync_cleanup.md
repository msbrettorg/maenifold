# Test Evaluation: workflow_supervisor_sync_cleanup.rs

> **Traceability**: T-W1-7 · RTM FR-14.6 · file header cites T-12.7 / NFR-1 (see §Observations)
> **Color lens**: 🔘 Gray — Skeptical Inquiry
> **Scope**: `crates/maenifold-core/tests/workflow_supervisor_sync_cleanup.rs` — 3 `#[test]` functions

---

## Summary

This file tests the **intersection of two independent systems**: the incremental-sync mtime
guard (which skips re-reading files whose mtime matches `last_indexed`) and the staleness-based
session-abandonment logic (`cleanup_abandoned_sessions`). The core claim under test is:

> Even when the mtime guard would cause sync to skip a file entirely, the abandonment pass
> must still fire and rewrite the session as `abandoned` — and, conversely, must *not* touch
> plain memory files solely because they are old.

All three tests are load-bearing. However Tests 1 and 2 are structural duplicates
parameterizable by session type, each test carries a hidden dependency on the default
`MAENIFOLD_SESSION_ABANDON_MINUTES` value, and an intermediate assertion in both tests
verifies test-scaffolding state rather than production behaviour.

---

## Verdicts

| # | Test Name | Verdict | Severity of Issue |
|---|-----------|---------|-------------------|
| 1 | `sync_stale_workflow_session_marked_abandoned_even_when_mtime_guard_would_skip_read` | **MERGE** with #2 | Medium |
| 2 | `sync_stale_sequential_session_marked_abandoned_even_when_mtime_guard_would_skip_read` | **MERGE** with #1 | Medium |
| 3 | `sync_does_not_rewrite_normal_memory_file_solely_due_to_age_mtime_guard_remains` | **KEEP** (minor improvements) | Low |

---

## Detailed Findings

### Test 1 — `sync_stale_workflow_session_marked_abandoned_even_when_mtime_guard_would_skip_read`

**Referencing**: `T-W1-7 · FR-14.6`

#### What it does
1. Creates a `workflow`-type thinking session via `create_workflow_session_fixture`.
2. Runs a full `invoke_sync` to index it (establishes a baseline `last_indexed`).
3. Sets the file's mtime to 130 minutes in the past using `set_file_mtime_utc`.
4. Calls `update_last_indexed_and_status` so the DB `last_indexed` matches that same old mtime —
   simulating the state where sync believes the file is "already up-to-date."
5. Asserts the DB row reflects the manipulation (setup guard).
6. Runs `invoke_sync` again.
7. Asserts `status == "abandoned"` in frontmatter and `"Session Abandoned"` in body.

#### Unique?
**Yes.** The sibling files `workflow_supervisor_cleanup_core.rs` and
`workflow_supervisor_cleanup_guards.rs` also test abandonment, but they call
`sessions::cleanup_abandoned_sessions` directly and seed the DB with `create_session`/
`seed_file_content` — they never exercise the scenario where `invoke_sync` must bypass
its own mtime guard to reach the cleanup pass. This is a distinct integration path.

#### Correct Assertions?
**Mostly.** The two terminal assertions are correct and sufficient:
```rust
assert_eq!(read_frontmatter_status(&env.cfg, &file_uri), "abandoned");
assert!(read_body(&env.cfg, &file_uri).contains("Session Abandoned"));
```
The **intermediate assertion** is noise:
```rust
assert_eq!(last_indexed_db, last_indexed);
```
This only verifies that `update_last_indexed_and_status` worked — i.e., that the *test's own
scaffolding* is correct, not that the production code behaves correctly. If this assertion ever
fires, it would indicate a bug in the test helper, not in the system under test. It should be
removed or replaced with a `debug_assert!` / explanatory comment.

#### Hidden Threshold Dependency
The test sets mtime to **130 minutes ago** but never pins `MAENIFOLD_SESSION_ABANDON_MINUTES`.
The real default in `cleanup.rs` is **30 minutes**. The test happens to work because 130 > 30,
but this assumption is undocumented and fragile:
- If the default rises above 130 minutes, the test **silently changes meaning** (session is now
  within the active window and would not be abandoned — assertion failure).
- A reader cannot tell from the test alone *why* 130 was chosen.
- `SessionCleanupEnvGuard::set_abandonment_minutes` exists in `support.rs` precisely for this
  purpose and is unused here.

#### Good Name?
Yes — verbose but unambiguous. The name captures both the subject (workflow session) and the
mechanism (mtime guard bypass). Length is acceptable for integration tests.

#### Parameterizable?
**Yes** — see Test 2.

---

### Test 2 — `sync_stale_sequential_session_marked_abandoned_even_when_mtime_guard_would_skip_read`

**Referencing**: `T-W1-7 · FR-14.6`

#### What it does
Byte-for-byte identical logic to Test 1, with two substitutions:
- `"workflow"` → `"sequential"` in `create_workflow_session_fixture`
- `workflow_uri` → `sequential_uri`

#### Unique?
**Marginally.** The `sequential` session type follows a different URI namespace and a slightly
different ID format (nano-second suffix rather than plain milliseconds — see `create_workflow_session_fixture`
in `sync_support.rs`). The production code path that decides whether to abandon a session is
`cleanup_abandoned_sessions`, which queries on `file_path LIKE 'memory://thinking/%'` — the
session type is part of the path segment, so both `workflow` and `sequential` sessions must
match. Covering both types is correct in principle.

However, the incremental-sync mtime-guard logic (the primary thing under test) does **not**
branch on session type — it is identical for any file. The meaningful difference is only
whether `memory://thinking/workflow/…` and `memory://thinking/sequential/…` are both
captured by the `LIKE` query in cleanup. That distinction would be better expressed as a
parameterised test rather than duplicated test bodies.

#### Structural Duplication
Lines 1–45 of this test are functionally identical to Lines 1–45 of Test 1. Any future
change to the test logic (e.g. adding threshold pinning) must be applied to both. This is
the canonical maintenance burden of copy-paste tests.

#### Verdict: MERGE

Collapse Tests 1 and 2 into a single parameterized test:
```rust
#[rstest]
#[case("workflow")]
#[case("sequential")]
fn sync_stale_session_marked_abandoned_even_when_mtime_guard_would_skip_read(
    #[case] session_type: &str,
) {
    let _env_guard = SessionCleanupEnvGuard::set_abandonment_minutes("60");
    // ... shared body parameterized on session_type
}
```
This removes the hidden-threshold fragility, eliminates duplication, and makes the coverage
intent (`both session types`) explicit via the parameterization labels.

---

### Test 3 — `sync_does_not_rewrite_normal_memory_file_solely_due_to_age_mtime_guard_remains`

**Referencing**: `T-W1-7 · FR-14.6`

#### What it does
1. Creates a plain memory file (not in `memory://thinking/`).
2. Syncs it (baseline index).
3. Sets mtime to **7 days** in the past.
4. Aligns `last_indexed` to match that mtime.
5. Runs sync again.
6. Asserts mtime **unchanged** and content **unchanged**.

#### Unique?
**Yes.** This is the negative control: the staleness/abandonment system must be completely
inert for ordinary memory notes. The `cleanup_core.rs` file has `handle_session_cleanup_memory_type_no_change`
which tests a `type: memory` session file at the `cleanup_abandoned_sessions` API level, but
does not go through `invoke_sync`. This test covers the full stack end-to-end and adds the
mtime-unchanged assertion (not just content), which is the stronger claim.

#### Correct Assertions?
**Yes — thorough.** Both `before_mtime == after_mtime` and `before_text == after_text` are
checked. This is the right pairing: a defensive implementation could skip content rewriting
while still updating mtime (e.g. by touching the file), and this test would catch that.

#### Magic Numbers
The `Duration::days(7)` choice is unexplained. Any value > 30 minutes demonstrates the claim.
A comment explaining the margin would clarify intent:
```rust
// Use a very large offset (7 days) to ensure we're well past any abandonment
// threshold, so any unintended cleanup-triggered rewrite would be unambiguous.
let stale = Utc::now() - Duration::days(7);
```

#### Good Name?
Yes — the name correctly signals both the trigger (`solely due to age`) and the expected
outcome (`mtime guard remains`).

#### Parameterizable?
Not meaningfully — this is a single negative-case scenario. The file path used
(`notes/cleanup-001-1/note.md`) could be a named constant for clarity but the test
stands as-is.

---

## Observations

### Traceability Mismatch
The **file header** comment reads:
```rust
// T-12.7: RTM NFR-1
```
All three **test** comments read:
```rust
// T-W1-7: RTM FR-14.6
```
These reference different task items and different requirements. `T-12.7 / NFR-1` (C# Oracle
Parity) is used in the shared support modules; it should not appear as the file-level traceability
for tests that map to `FR-14.6`. The file header should either be removed or corrected to
`T-W1-7: RTM FR-14.6`.

### Coverage Gap: `should_preserve_recent_filesystem_edit` Ctime Branch
`cleanup.rs` contains `metadata_change_lags_mtime` (unix-only), which preserves sessions
whose ctime is significantly newer than their mtime — detecting manually-set-back mtime on a
recently-written file. None of the three tests in this file exercises this branch. The
`set_file_mtime_utc` call in Tests 1 and 2 could potentially trigger this guard on a slow
CI runner (ctime updated by `filetime`, mtime set to 130 min ago, and the ctime could lag
by < 1 ms on some filesystems). The existing tests do not assert this guard was *bypassed*;
they only assert the final state.  A test that explicitly confirms the ctime bypass is
handled would improve determinism.

### Positive: `TEST_MUTEX` Serialization
All three tests correctly acquire `TEST_MUTEX` before mutating the process-wide environment
and filesystem. This is the right pattern given that `MAENIFOLD_SESSION_ABANDON_MINUTES` is
a process-global env var and `Config::override_root` modifies global state.

### Positive: `TempDir` + `Drop` for Cleanup
`isolated_env()` wraps the temp directory in `TestEnv` whose `Drop` impl calls
`Config::reset_overrides()`. No test leaks global state into subsequent tests.

---

## Recommendations

**Priority 1 — Explicit threshold pinning (Tests 1 & 2)**
Add `SessionCleanupEnvGuard::set_abandonment_minutes("60")` as the first line of each test
(or the merged parameterized variant). This makes the 130-minute staleness choice
self-documenting and immune to future default changes.

**Priority 2 — Merge Tests 1 and 2**
Replace both tests with a single `#[rstest]`-parameterized test (or a `for session_type in
["workflow", "sequential"]` loop if avoiding the rstest dependency). The current duplication
will diverge silently under maintenance.

**Priority 3 — Remove intermediate DB assertion**
Delete the `assert_eq!(last_indexed_db, last_indexed)` block from Tests 1 and 2. It is
scaffolding validation, not system-under-test validation. Move it to a support-level unit
test for `update_last_indexed_and_status` if it adds value.

**Priority 4 — Comment magic numbers**
Add a one-line comment to `Duration::minutes(130)` in Tests 1 and 2 and `Duration::days(7)`
in Test 3 explaining the choice relative to the abandonment threshold.

**Priority 5 — Fix file-header traceability**
Change the top-of-file comment from `T-12.7: RTM NFR-1` to `T-W1-7: RTM FR-14.6` to
match the tests it contains.

**Priority 6 — Consider ctime bypass test**
Add a test (possibly in `workflow_supervisor_cleanup_guards.rs`) that explicitly exercises
the `should_preserve_recent_filesystem_edit` path where ctime > mtime, verifying that a
recently-written file with a manipulated mtime is preserved. This would close the coverage
gap identified above.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. Each covers a distinct scenario not fully replicated in
the adjacent cleanup test files:

| Test | Load-Bearing Reason |
|------|---------------------|
| #1 | Only full-stack test of sync mtime-guard bypass → abandonment for `workflow` session type |
| #2 | Extends the same coverage to `sequential` session type (covers the LIKE query scope) |
| #3 | Only end-to-end test asserting the abandonment pass is inert for plain memory files (with mtime-unchanged assertion) |

Score is 3/3 — but Tests 1 and 2 should be considered a single logical test covering two
parameters (current score: 2 tests, 1 distinct scenario). Post-merge the effective score
would be **2/2** distinct scenarios, both load-bearing, both worth keeping.

---
## SWE Remediation
- MERGE `sync_stale_workflow_session_marked_abandoned_even_when_mtime_guard_would_skip_read` + `sync_stale_sequential_session_marked_abandoned_even_when_mtime_guard_would_skip_read`: consolidated into `sync_stale_session_marked_abandoned_even_when_mtime_guard_would_skip_read` with `workflow`/`sequential` coverage in one loop.
- REWRITE merged stale-session test: pinned `MAENIFOLD_SESSION_ABANDON_MINUTES` to `60`, removed the intermediate DB scaffolding assertion, and documented the 130-minute staleness margin.
- KEEP `sync_does_not_rewrite_normal_memory_file_solely_due_to_age_mtime_guard_remains`: added a clarifying comment for the 7-day offset.
- REWRITE file header traceability: corrected to `T-W1-7: RTM FR-14.6`.
- Verification: `cd crates/maenifold-core && cargo test --test integration_tests workflow_supervisor_sync_cleanup`
- Commit: 9b5ede2
