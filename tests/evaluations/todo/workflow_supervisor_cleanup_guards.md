# Test Evaluation: workflow_supervisor_cleanup_guards
**File:** `crates/maenifold-core/tests/workflow_supervisor_cleanup_guards.rs`
**Traceability:** T-W1-7 · T-12.7 · RTM FR-17.7 · RTM NFR-1
**Test count:** 2
**Evaluated:** 2026-07-13

## Summary

Two tests. One is a **near-duplicate** of a sibling test that covers the identical code path with a
simpler fixture and a more accurate name. The other is a **genuine idempotency guard** covering a
terminal status not exercised anywhere else in the cleanup suite.

The file name `cleanup_guards` is well-chosen for the second test but misleading for the first:
the first test names its discriminating variable (`type: memory`) when the actual guard is entirely
path-based — the `type` frontmatter field plays no role in `cleanup_abandoned_sessions`' SQL
filter or in `abandon_session_candidate`'s logic.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `handle_session_cleanup_memory_type_no_change` | **REMOVE** | Near-duplicate of `workflow_supervisor_cleanup_core.rs::handle_session_cleanup_null_type_no_change`. Both plant a stale `status: active` row at `memory://notes/` and assert zero abandoned + unchanged mtime. The `type: memory` frontmatter field does not participate in the `LIKE 'memory://thinking/%'` SQL filter or in any branch of `abandon_session_candidate`. The type variation is noise, not signal. Removing it loses zero coverage. |
| `handle_session_cleanup_already_abandoned_no_change` | **KEEP** | Unique. Verifies that an `abandoned`-status session at a `memory://thinking/` path — which would otherwise match the path filter — is correctly excluded by the `status = 'active'` SQL guard. No other test in the cleanup suite covers the `abandoned` status specifically. `session_cleanup_sweep.rs::completed_and_cancelled_sessions_are_not_touched_regardless_of_age` covers `completed` and `cancelled`; this is the only test for `abandoned`. A regression on the status filter (e.g., `!= 'abandoned'` mistyped) would ship undetected without it. Assertions are thorough (count, frontmatter status, body). |

---

## Recommendations

### 1. Remove `handle_session_cleanup_memory_type_no_change`

The `type` frontmatter field does not influence the cleanup predicate. The only question being
tested is whether a file at `memory://notes/` survives cleanup — and `cleanup_core.rs` already
answers it with `handle_session_cleanup_null_type_no_change`. Both tests:

- Use the same `write_raw_markdown` + `seed_file_content` setup pattern
- Seed `status: active`, `minutes_ago: 130` (well past the 30-min default threshold)
- Assert `abandoned == 0` and `before_mtime == after_mtime`

The `null_type` test is strictly more minimal and therefore a better test: it rules out the path
mechanism without implying the type field has anything to do with it. Keeping both propagates the
false impression that `type: memory` is what protects a memory note.

**Before removal checklist:**
- Confirm `handle_session_cleanup_null_type_no_change` is present and passing in
  `workflow_supervisor_cleanup_core.rs` (verified: lines 113–148).
- Confirm the SQL path filter `LIKE 'memory://thinking/%'` in `cleanup.rs:24` is the sole
  mechanism; type field is never read (verified: no `type` access in `cleanup_abandoned_sessions`
  or `abandon_session_candidate`).

### 2. Strengthen `handle_session_cleanup_already_abandoned_no_change` (optional)

The test currently asserts count, frontmatter status, and body. Consider adding a DB status
assertion:

```rust
assert_eq!(
    read_db_status_or_null(&env.cfg, file_uri),
    Some("abandoned".to_string()),
    "DB status must not be reset"
);
```

This would catch a regression where the file is rewritten to disk but the DB record is updated
(or cleared). The session_cleanup_sweep support module has `read_db_status`; if the
`workflow_supervisor/support.rs` module gains an equivalent helper, this assertion closes the gap.

### 3. Consider merging the `abandoned` and `completed`/`cancelled` cases

Rather than having `handle_session_cleanup_already_abandoned_no_change` in this file and
`completed_and_cancelled_sessions_are_not_touched_regardless_of_age` in
`session_cleanup_sweep.rs`, a single parameterized test (or one test with three fixture URIs)
would consolidate all terminal-status idempotency coverage:

```rust
// One test, three terminal statuses
for (status, uri) in [
    ("abandoned",  "memory://thinking/workflow/.../already-abandoned"),
    ("completed",  "memory://thinking/sequential/.../already-completed"),
    ("cancelled",  "memory://thinking/workflow/.../already-cancelled"),
] {
    create_session(&env.cfg, uri, status, 130, "...");
}
let abandoned = sessions::cleanup_abandoned_sessions(&env.cfg).expect("cleanup");
assert_eq!(abandoned, 0);
// assert all three unchanged
```

This is a refactor suggestion, not a requirement. The current split is defensible; the sweep
test and the guards test live in different support module contexts.

### 4. Fix the misleading `// T-W1-7: RTM FR-17.7` annotation on test 1

If test 1 is kept, its annotation correctly identifies `FR-17.7` (session cleanup sub-requirement).
But if it is removed, verify the sibling `null_type` test in `cleanup_core.rs` also carries a
`FR-17.7` annotation so traceability is preserved for path-guard coverage.

---

## Load-Bearing Score: 1/2

Only `handle_session_cleanup_already_abandoned_no_change` would catch a real regression if
removed. `handle_session_cleanup_memory_type_no_change` is fully covered by a sibling test that
exercises the identical code path with a simpler, less misleading fixture.

---

## SWE Remediation
- REMOVE `handle_session_cleanup_memory_type_no_change`: confirmed duplicate of `cleanup_core::handle_session_cleanup_null_type_no_change` (line 114); `type` frontmatter field has no effect on cleanup predicate. Removed unused imports (`std::fs`, `Frontmatter`, `serde_yaml::Value`).
- KEEP `handle_session_cleanup_already_abandoned_no_change`: untouched per verdict.
- Verified sibling test `cleanup_core.rs:112` already carries `// T-W1-7: RTM FR-17.7` — traceability preserved (Rec #4).
- Test result: 1 passed, 0 failed, 882 filtered out.
