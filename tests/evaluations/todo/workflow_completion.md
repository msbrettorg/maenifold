# Test Evaluation: workflow_completion

**File:** `crates/maenifold-core/tests/workflow_completion.rs`
**Test count:** 10
**Evaluated:** 2026-03-14
**Traceability:** T-12.0 / RTM NFR-1
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions.

---

## Summary

The weakest of the three evaluated files. Of 10 tests, 3 are redundant with other tests in the same file, and 2 contain "wrong-thing" assertions (emoji containment checks) that test presentation format rather than behavioral outcome. The genuine load-bearing count is **7**, but even those 7 have naming and assertion quality issues worth fixing. The file covers real behavior — WikiLink validation, completion/cancellation state transitions, disk persistence of conclusions — but the coverage is diluted by duplicate scenarios that differ only in which test fixture environment they use.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `workflow_response_without_wikilinks_returns_error` | KEEP | Unique: missing WikiLinks on a plain `response` → validation error with `[[WikiLink]]` guidance. Core validation path. |
| `status_transition_active_to_completed` | KEEP | Unique: `status: "completed"` persists frontmatter `status=completed`, `completed` timestamp, and conclusion body to disk. The primary state-transition test. |
| `completion_without_conclusion_returns_error` | KEEP | Unique: `status: "completed"` without `conclusion` field → error containing "Conclusion required". Distinct validation from WikiLink absence. |
| `completion_with_conclusion_lacking_wikilinks_returns_error` | KEEP | Unique: `conclusion` present but contains no WikiLinks → error. Tests that WikiLink validation applies specifically to the `conclusion` field too. |
| `status_transition_active_to_cancelled` | KEEP | Unique: `status: "cancelled"` persists `status=cancelled` and `cancelled` timestamp without requiring a conclusion. Distinct contract from completion. |
| `conclusion_with_wikilinks_is_stored_in_session_content` | **MERGE → `status_transition_active_to_completed`** | Adds assertion depth (more WikiLinks checked) but covers no new behavioral territory. `status_transition_active_to_completed` already verifies conclusion text and WikiLinks appear in content. Move the multi-WikiLink assertions into that test. |
| `continue_with_completion_status_finalizes_session` | **REWRITE** | Covers a genuinely distinct scenario (completion after a prior intermediate step). KEEP the substance. **REMOVE** `assert!(output.contains('✅'))` — this tests emoji choice, not behavior. The semantic outcome is already proven by frontmatter assertions. |
| `continue_with_cancellation_status_terminates_early` | **MERGE → `status_transition_active_to_cancelled`** | Same behavioral contract (cancel → status + timestamp) against a different fixture env (`workflow_operations_env`). **The env difference is not justified as a unique test case.** Also has a wrong-thing `assert!(output.contains('❌'))` assertion. |
| `continue_without_concepts_returns_error` | **REMOVE** | Exact duplicate of `workflow_response_without_wikilinks_returns_error` in a different fixture environment. The WikiLink validation logic is independent of workflow definition; running it against `workflow_operations_env` adds zero coverage. |
| `continue_with_invalid_session_id_returns_error` | KEEP | Unique: nonexistent session ID → error containing the ID and "doesn't exist". Tests an entirely different failure mode from validation errors. |

---

## Wrong-Thing Assertions — Detailed

### Emoji containment (`tests 7 and 8`)

```rust
assert!(output.contains('✅'), "unexpected output: {output}");
assert!(output.contains('❌'), "unexpected output: {output}");
```

**What these test:** Whether the MCP response string contains a specific Unicode code point.

**What they do NOT test:** Whether the workflow was completed or cancelled (that's tested by the frontmatter `status` assertions that follow).

**Why this is wrong:**
- A change to `"✅ Workflow completed"` → `"✓ Workflow completed"` would break these tests while leaving all semantics intact.
- A broken implementation that writes `status=active` but includes `✅` in the output would pass these assertions.
- The semantic outcome is *already proven* by `read_workflow_session` + frontmatter assertions. The emoji adds noise, not signal.

**Fix:** Delete the emoji assertions. If the formatted output contract genuinely matters (e.g., it's rendered in a UI), document that intent explicitly and test it in a dedicated output-format test.

### Exact error message strings (`tests 3, 9, 10`)

```rust
assert!(output.contains("Conclusion required"), ...);
assert!(output.contains("knowledge graph"), ...);
assert!(output.contains("doesn't exist"), ...);
```

These are fragile against wording changes. However, they have more justification than the emoji checks: the error messages are user-facing guidance in an MCP tool response, and verifying that they contain actionable direction (not just "ERROR") is a reasonable contract. The key question is *whether the exact wording matters* or just the presence of key terms.

**Verdict:** Acceptable as-is for guidance strings. The "knowledge graph" assertion in test 9 (which is being removed anyway) is the weakest — it tests a phrase rather than a behavior.

---

## Deeper Concerns

### 1. Two fixture environments used, but the difference is never explained

`workflow_tools_env()` creates 3 workflows (`wtt-alpha`, `wtt-beta`, `wtt-gamma`).
`workflow_operations_env()` creates 3 different workflows (`test-workflow-1`, etc.).

Several tests use `workflow_tools_env` and several use `workflow_operations_env`, but there's no documented reason why specific tests need one versus the other. If the distinction doesn't affect the behavior under test (and for validation tests like test 9, it clearly doesn't), then the arbitrary split creates false impressions of coverage diversity.

**Recommendation:** Either document why each test needs its specific env, or consolidate to one env per file.

### 2. `TEST_MUTEX` (parking_lot) vs `acquire_test_lock` (file-lock) — two different serialization mechanisms for similar problems

`workflow_completion.rs` uses `TEST_MUTEX` (in-process). `proximity_weight_sections.rs` uses a file-level lock. Neither can share the global `Config` state with the other file's tests if they run in parallel. The in-process mutex is invisible to the test runner's parallelism across files.

### 3. No test for reopening a completed or cancelled session

Tests verify that `status=completed` and `status=cancelled` are persisted. But no test verifies that attempting to add a response to an already-completed session fails with an appropriate error. This is a meaningful edge case — the session lifecycle should have a terminal state.

### 4. `start_session` creates fresh sessions — no test verifies multi-step intermediate state

`continue_with_completion_status_finalizes_session` (test 7) does two invocations, but the first step's content is never independently verified before the second. There's no test that verifies intermediate responses accumulate correctly across multiple steps before final completion.

---

## Recommendations

1. **REMOVE** `continue_without_concepts_returns_error` — exact duplicate of test 1 against a different env.
2. **MERGE** `conclusion_with_wikilinks_is_stored_in_session_content` into `status_transition_active_to_completed` — add the additional WikiLink assertions there.
3. **MERGE** `continue_with_cancellation_status_terminates_early` into `status_transition_active_to_cancelled` — same behavioral contract; deduplicate.
4. **REWRITE** `continue_with_completion_status_finalizes_session` — remove `assert!(output.contains('✅'))`, rename to `completion_after_intermediate_step_persists_state`.
5. **ADD** test: `completed_session_rejects_further_responses` — verify terminal state is enforced.
6. **ADD** test: `cancelled_session_rejects_further_responses` — same.
7. **Document or remove** the `workflow_tools_env` / `workflow_operations_env` distinction — if it doesn't affect behavior, collapse to one.

---

## Load-Bearing Score: 7/10

Tests 1, 2, 3, 4, 5, 7 (rewritten, minus emoji), and 10 cover genuinely distinct behavioral contracts. Tests 6, 8, and 9 are redundant with existing tests and could be removed without allowing any real bug to ship undetected.
