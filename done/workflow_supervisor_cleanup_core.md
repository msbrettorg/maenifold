---
id: T-80.189
title: "Test Evaluation: workflow_supervisor_cleanup_core"
status: done
pipeline: tdd
---

# Test Evaluation: workflow_supervisor_cleanup_core

**File:** `crates/maenifold-core/tests/workflow_supervisor_cleanup_core.rs`
**Traceability:** T-W1-7 · FR-17.7 · RTM NFR-1
**Test count:** 6
**Evaluated:** 2026-07-14

## Summary

This file covers the core happy-path and negative-path behavior of `sessions::cleanup_abandoned_sessions`. Four of the six tests exercise the **same functional scenario** (stale session → abandoned) split across two session types and two assertion targets. This produces duplicated fixture setup with no isolation benefit. The four stale-path tests are strong candidates for merging or parameterization.

Two tests are unambiguously well-placed and uniquely load-bearing: the temporal guard case (recent session is preserved) and the path-filter case (files outside `memory://thinking/` are excluded). The thresholds sibling file (`workflow_supervisor_cleanup_thresholds.rs`) previously evaluated as largely redundant **with this file**, confirming these six tests are the canonical source of truth for FR-17.7 abandonment behavior.

A skeptical read surfaces two further concerns:

1. **Tests 1 and 2 do not independently isolate** the return count from the frontmatter mutation. If `abandoned_count` were incremented but `write_markdown` silently failed, Test 1 passes and Test 2 fails — but the inverse failure (write succeeds, count wrong) is also possible. The assertions are not orthogonal in the meaningful sense — they're just split across fixtures that happen to be identical.

2. **Test 6's name is actively misleading.** The test is called `null_type_no_change`, but the implementation never inspects the `type` frontmatter field. The property under test is purely **URI path exclusion** (`memory://notes/` vs `memory://thinking/`). A reader following the name would look for type-field logic in `cleanup.rs` and find none.

---

## Verdicts

| # | Test Function | Verdict | Reason |
|---|---|---|---|
| 1 | `handle_session_cleanup_stale_workflow_session_marks_status_abandoned` | **MERGE** into test 2 | Identical fixture to test 2. Asserts count (`== 1`) and frontmatter status (`== "abandoned"`). Both assertions belong alongside test 2's body check in a single three-assertion test. Splitting adds no isolation — a broken `write_markdown` fails both, and a broken count fails only test 1. |
| 2 | `handle_session_cleanup_stale_workflow_session_appends_abandoned_section` | **MERGE** with test 1 | Identical fixture to test 1. Asserts `body.contains("Session Abandoned")`. The three assertions across tests 1 and 2 form a coherent single scenario — one setup, three `assert!` calls. |
| 3 | `handle_session_cleanup_stale_sequential_session_marks_status_abandoned` | **MERGE** into test 4, then evaluate for **REMOVE** | URI path is `/thinking/sequential/` instead of `/thinking/workflow/`. The SQL query uses `LIKE 'memory://thinking/%'` — sequential coverage is real but thin. Merge with test 4 first. If `session_cleanup_ttl.rs::active_session_beyond_default_ttl_is_marked_abandoned` is confirmed to cover both path types, this combined test can be removed. |
| 4 | `handle_session_cleanup_stale_sequential_session_appends_abandoned_section` | **MERGE** with test 3 | Identical duplicate of test 2 but with a sequential URI. `append_abandoned_notice` does not branch on URI type — the body content is identical regardless. The body-content assertion adds zero incremental coverage over tests 1+2. |
| 5 | `handle_session_cleanup_recent_active_session_no_change` | **KEEP** | Genuinely unique scenario. Three orthogonal assertions (count == 0, frontmatter status unchanged, body unchanged) on one setup. Note: the 30-minute fixture sits precisely at the default threshold boundary — it is protected by the filesystem mtime check (the file was just written), not necessarily by the SQL `<` predicate. This makes the test less precise than the threshold-boundary test in `cleanup_thresholds.rs`, but it still provides meaningful regression coverage for the "recent session is left alone" contract. |
| 6 | `handle_session_cleanup_null_type_no_change` | **KEEP** + **RENAME** | Unique: only test that verifies files outside `memory://thinking/` are excluded by the path predicate. The mtime equality check (`before_mtime == after_mtime`) is the correct assertion — it proves the filesystem was not touched at all. The name `null_type_no_change` is wrong: `type` is absent from the frontmatter, but the implementation never reads `type` during cleanup. The invariant is URI-path exclusion, not type-field absence. Rename to `handle_session_cleanup_non_thinking_path_is_not_touched`. |

---

## Recommendations

1. **Merge tests 1 + 2** into a single `handle_session_cleanup_stale_workflow_session_is_abandoned` test with three assertions:
   ```rust
   assert_eq!(abandoned, 1);
   assert_eq!(read_frontmatter_status(&env.cfg, file_uri), "abandoned");
   assert!(read_body(&env.cfg, file_uri).contains("Session Abandoned"));
   ```
   One fixture, one scenario, all three observable properties verified. No coverage is lost.

2. **Merge tests 3 + 4** into a single `handle_session_cleanup_stale_sequential_session_is_abandoned`. Then cross-check against `session_cleanup_ttl.rs::active_session_beyond_default_ttl_is_marked_abandoned` — if that test runs both URI types, the merged test 3+4 is a REMOVE candidate.

3. **Rename test 6** from `handle_session_cleanup_null_type_no_change` to `handle_session_cleanup_non_thinking_path_is_not_touched`. The current name directs readers toward a non-existent type-field check. The new name correctly identifies the SQL path-predicate as the mechanism.

4. **Add a missing scenario: idempotency.** `append_abandoned_notice` has an early-return guard (`if content.contains("## Session Abandoned") { return content.to_string() }`), but no test in this file (or any sibling file) calls `cleanup_abandoned_sessions` twice on the same already-abandoned session. A double-cleanup regression — duplicate "Session Abandoned" sections appended, count incremented a second time — would not be caught. Add:
   ```rust
   fn handle_session_cleanup_already_abandoned_session_is_idempotent()
   ```
   Assert: second call returns `abandoned == 0` and body contains exactly one `"Session Abandoned"` section.

5. **Add a missing scenario: status-based exclusion in this file.** `session_cleanup_sweep.rs` covers `completed` and `cancelled` sessions. The SQL filters on `status = 'active'`, so those are excluded. Tests 1–4 only set up `active` sessions. The complement (non-active → not abandoned) is tested elsewhere but would be a natural addition to this file if it is the canonical cleanup reference.

6. **Parameterize if tests 1+2 and 3+4 are kept separately for any reason.** The four stale-session tests share identical structure: `(session_type, uri_path, minutes_ago)` → `(count, status, body_contains)`. A `#[rstest]` or `#[test_case]` table would eliminate the fixture duplication while preserving both path variants.

7. **The `T-W1-7: RTM FR-17.7` annotation** appears on all six tests without differentiation. If FR-17.7 has sub-requirements (e.g., one for status mutation, one for body append, one for path exclusion, one for temporal guard), those sub-requirement IDs should appear on the individual tests that verify them. As written, the annotation is a blanket label that offers no navigability.

---

## Load-Bearing Score: 4/6

| Test(s) | Load-Bearing? | Rationale |
|---|---|---|
| Test 1 + Test 2 (together) | ✅ Yes | Canonical happy-path for stale workflow session. Referenced as source of truth by `workflow_supervisor_cleanup_thresholds.md` which removed duplicates. Removal would leave no explicit test of the return count + status + body triple for the workflow path. |
| Test 3 + Test 4 (together) | ⚠️ Marginal | Adds sequential-URI path coverage over tests 1+2. Genuinely useful if someone changes the SQL predicate to a more specific pattern. However, body-content check (test 4) adds zero incremental value — `append_abandoned_notice` is path-agnostic. Combined, these are one test worth of coverage for a marginal concern. |
| Test 5 | ✅ Yes | Only test in this file asserting the temporal guard (recent sessions are not abandoned). The three-assertion structure is clean. |
| Test 6 | ✅ Yes | Only test in the entire suite asserting that non-thinking-path files are excluded. Without it, a SQL predicate regression (`LIKE 'memory://%'` instead of `LIKE 'memory://thinking/%'`) would not be caught. The mtime check is the correct level of evidence. |

---
## SWE Remediation
- MERGE tests 1+2 → `handle_session_cleanup_stale_workflow_session_is_abandoned`: single fixture, 3 assertions (count, frontmatter status, body contains notice)
- MERGE tests 3+4 → `handle_session_cleanup_stale_sequential_session_is_abandoned`: same triple-assert pattern for sequential URI path; kept because `session_cleanup_ttl.rs::active_session_beyond_default_ttl_is_marked_abandoned` only covers sequential, not both URI types
- KEEP test 5 (`handle_session_cleanup_recent_active_session_no_change`): unchanged
- RENAME test 6 → `handle_session_cleanup_non_thinking_path_is_not_touched`: reflects actual URI-path-exclusion invariant, not type-field absence
- ADD `handle_session_cleanup_already_abandoned_session_is_idempotent`: asserts second cleanup returns 0 and body contains exactly one "Session Abandoned" notice (recommendation 4)
- Test count: 6 → 5 (net -1 from merges, +1 from idempotency)
- Commit: c17525b
