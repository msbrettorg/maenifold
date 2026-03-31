---
id: workflow_supervisor_cleanup_thresholds
status: done
pipeline: tdd
---

# Test Evaluation: workflow_supervisor_cleanup_thresholds
**File:** `crates/maenifold-core/tests/workflow_supervisor_cleanup_thresholds.rs`
**Traceability:** T-12.7 · FR-17.7 · RTM NFR-1
**Test count:** 7
**Evaluated:** 2026-07-13

## Summary

Five of the seven tests are **substantively duplicated** by `workflow_supervisor_cleanup_core.rs` and `session_cleanup_ttl.rs` — they exercise the same code paths, same setup helpers, and assert the same properties. Only the boundary-condition test and the content-preservation test earn their place in this file.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `try_mark_abandoned_from_last_indexed_stale_session_returns_true` | **REMOVE** | `cleanup_core.rs::handle_session_cleanup_stale_workflow_session_marks_status_abandoned` already asserts `abandoned == 1` for a 130-min stale session. `session_cleanup_ttl.rs::active_session_beyond_default_ttl_is_marked_abandoned` also covers the count. No unique assertion here. |
| `try_mark_abandoned_from_last_indexed_stale_session_updates_frontmatter_status` | **REMOVE** | `cleanup_core.rs` test 1 asserts both count AND `status == "abandoned"` in one test. `session_cleanup_ttl.rs` test 2 does the same plus DB status. This test splits one of those assertions out into a standalone without adding coverage. |
| `try_mark_abandoned_from_last_indexed_stale_session_appends_abandoned_section` | **REMOVE** | Directly duplicates `cleanup_core.rs::handle_session_cleanup_stale_workflow_session_appends_abandoned_section`. Identical fixture (workflow type, 130 min, `create_session` helper) and identical assertion (`body.contains("Session Abandoned")`). |
| `try_mark_abandoned_from_last_indexed_recent_session_returns_false` | **MERGE** with `_recent_session_frontmatter_unchanged` | Asserting `count == 0` and `body == original` are both properties of "recent session not touched." These two tests share identical fixture setup (30-min session). Combine into one. Also already covered by `cleanup_core.rs::handle_session_cleanup_recent_active_session_no_change`. |
| `try_mark_abandoned_from_last_indexed_recent_session_frontmatter_unchanged` | **MERGE** with `_recent_session_returns_false` | See above. Splits `status == "active"` into a separate test with no added isolation value. Three files now cover the same "recent session is not touched" behavior. |
| `try_mark_abandoned_from_last_indexed_exactly_at_threshold_returns_false` | **KEEP** | Genuinely unique. Uses `SessionCleanupEnvGuard` to set a 5-minute TTL, then seeds `last_indexed` at exactly `anchor - 5 minutes` (with `align_to_second_boundary()` to prevent nanosecond drift). No other test in the suite probes the fencepost: "at the threshold = safe." This would allow a `<` vs `<=` regression to ship. |
| `try_mark_abandoned_from_last_indexed_stale_session_preserves_original_content` | **KEEP** | Unique assertion: `updated.starts_with(original)`. Verifies the abandoned section is *appended* rather than replacing or corrupting the original content. Other tests only check that "Session Abandoned" appears somewhere; none verify the original content survives intact. Load-bearing for an append-vs-overwrite regression. |

---

## Recommendations

1. **Remove tests 1, 2, 3** (`returns_true`, `updates_frontmatter_status`, `appends_abandoned_section`). Coverage is fully held by `workflow_supervisor_cleanup_core.rs` and `session_cleanup_ttl.rs`. Verify no unique assertions are lost before deleting.

2. **Merge tests 4 + 5** (`recent_session_returns_false` + `recent_session_frontmatter_unchanged`) into a single test — one setup, three assertions (`count == 0`, `status == "active"`, `body == original`). Then evaluate whether it is sufficiently differentiated from `cleanup_core.rs` test 5; if not, delete it too.

3. **Rename test 6** to something that exposes the invariant being tested, not the mechanism:
   - Current: `try_mark_abandoned_from_last_indexed_exactly_at_threshold_returns_false`
   - Suggested: `session_at_exact_abandonment_boundary_is_not_marked_abandoned`

4. **Rename test 7** to expose the content-preservation invariant:
   - Current: `try_mark_abandoned_from_last_indexed_stale_session_preserves_original_content`
   - Suggested: `stale_session_abandonment_appends_section_without_corrupting_original_body`

5. **The `try_mark_abandoned_from_last_indexed_` prefix** is misleading — it describes the implementation's internal mechanism (checking `last_indexed` column), not the observable behavior being tested. All tests in this file (and cleanup_core) operate through the same mechanism; the prefix adds noise without helping navigation.

6. **Parameterize if tests 1-3 are kept for any reason:** the stale-session test triple (returns count, updates status, appends section) could collapse into one setup with three assertions, or a `#[test_case]`-style table: `(minutes_ago, expect_abandoned, expect_status, expect_body_contains)`.

---

## Load-Bearing Score: 2/7

Tests 6 and 7 are the only tests where removal would allow a real regression to ship undetected. Tests 1–5 are covered by the cleanup suite in `workflow_supervisor_cleanup_core.rs` and `session_cleanup_ttl.rs`.

---
## SWE Remediation
- REMOVE `try_mark_abandoned_from_last_indexed_stale_session_returns_true`: deleted duplicate stale-session count coverage
- REMOVE `try_mark_abandoned_from_last_indexed_stale_session_updates_frontmatter_status`: deleted duplicate stale-session status coverage
- REMOVE `try_mark_abandoned_from_last_indexed_stale_session_appends_abandoned_section`: deleted duplicate stale-session body-append coverage
- REMOVE merged recent-session pair: dropped duplicated "recent session untouched" coverage because `workflow_supervisor_cleanup_core.rs` already asserts count, status, and body invariants together
- RENAME test 6 to `session_at_exact_abandonment_boundary_is_not_marked_abandoned`
- RENAME test 7 to `stale_session_abandonment_appends_section_without_corrupting_original_body`
- Verified with: `cargo test --test integration_tests workflow_supervisor_cleanup_thresholds`
- Commit: 91447d6

---
## Red-team Audit

### Findings

- ✅ REMOVE `try_mark_abandoned_from_last_indexed_stale_session_returns_true`: confirmed deleted; no unique assertions lost — `cleanup_core.rs` test 1 asserts `abandoned == 1` for a stale session.
- ✅ REMOVE `try_mark_abandoned_from_last_indexed_stale_session_updates_frontmatter_status`: confirmed deleted; `cleanup_core.rs` test 1 asserts both count and `status == "abandoned"`.
- ✅ REMOVE `try_mark_abandoned_from_last_indexed_stale_session_appends_abandoned_section`: confirmed deleted; `cleanup_core.rs::handle_session_cleanup_stale_workflow_session_appends_abandoned_section` is an identical fixture and identical assertion.
- ❌ MERGE `recent_session_returns_false` + `recent_session_frontmatter_unchanged`: **SWE removed both tests without first creating the merged test.** The eval verdict was **MERGE** (not REMOVE). Recommendation #2 prescribes a two-step sequence — (1) merge into one test with three assertions (`count == 0`, `status == "active"`, `body == original`), then (2) *optionally* delete the merged result if sufficiently subsumed by `cleanup_core.rs`. The SWE jumped straight to deletion, skipping the merge step entirely. While `cleanup_core.rs::handle_session_cleanup_recent_active_session_no_change` is confirmed to assert all three properties (`abandoned == 0`, `status == "active"`, `body == before_body`), the prescribed action per the verdicts table was MERGE — not REMOVE. The SWE changed the verdict category without authorization.
- ✅ RENAME test 6 → `session_at_exact_abandonment_boundary_is_not_marked_abandoned`: confirmed; name matches eval recommendation #3 exactly. Dual assertions (`abandoned == 0` + `status == "active"`) present in resulting test body.
- ✅ RENAME test 7 → `stale_session_abandonment_appends_section_without_corrupting_original_body`: confirmed; name matches eval recommendation #4 exactly. `starts_with(original)` assertion preserved.

### Coverage Note

`cleanup_core.rs::handle_session_cleanup_recent_active_session_no_change` **does** assert all three properties the merged test would have carried (`abandoned == 0`, `read_frontmatter_status == "active"`, `read_body == before_body`). The deletion is substantively defensible, but it bypasses the prescribed MERGE step and constitutes an undocumented verdict change from MERGE → REMOVE.

### Verdict: REJECT

The eval verdicts table prescribed **MERGE** for tests 4 and 5. The SWE removed both without producing the intermediate merged test. The merge step must be executed first; deletion of the resulting merged test requires a second explicit evaluation against `cleanup_core.rs`. The SWE collapsed two prescribed steps into one undocumented decision. Resubmit with either (a) the merged test present, or (b) an explicit eval-level addendum changing the verdict for rows 4–5 from MERGE → REMOVE with justification.

---
## SWE Remediation (Resubmission)
- MERGE `try_mark_abandoned_from_last_indexed_recent_session_returns_false` + `try_mark_abandoned_from_last_indexed_recent_session_frontmatter_unchanged`: created `recent_session_before_abandonment_threshold_remains_active_and_unchanged` with the prescribed three assertions (`count == 0`, `status == "active"`, `body == original`)
- Preserved prior renames for the exact-threshold and original-content tests
- Verified with: `cargo test -p maenifold-core --test integration_tests workflow_supervisor_cleanup_thresholds`
- Commit: 9a79207
