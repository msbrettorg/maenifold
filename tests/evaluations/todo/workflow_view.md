# Test Evaluation: `workflow_view.rs`

> **Reviewer mode**: 🔘 Gray — Skeptical Inquiry
> **Traceability**: T-12.0 (NFR-1), T-15.14 (FR-8)
> **File**: `crates/maenifold-core/tests/workflow_view.rs`
> **Total tests**: 7
> **Evaluation date**: 2025-07-28

---

## Summary

This module tests the `view: true` path of the `Workflow` tool — specifically that the queue status, current position, and error states are correctly rendered. The module was created under T-12.0 / NFR-1 to isolate queue-view coverage from other workflow concerns. It uses two fixtures (`workflow_tools_env` / `workflow_operations_env`) which differ only in workflow naming conventions — `wtt-*` vs `test-workflow-*`.

**Central problem**: the module has a clear duplication axis. Three behavior scenarios (initial queue view, view after advancement, single-step view) are each covered by nearly-identical test pairs using different fixtures, without any comment justifying why both fixtures are needed. A fourth test (`view_via_workflow_tools_entry_point`) has the weakest assertion set of all seven tests and adds no unique coverage. The two negative-path tests (`view_with_invalid_session_id_returns_error`, `view_without_session_id_throws_helpful_error`) are well-targeted and unique.

**Questions the Gray lens demands answers to**:
- What does `workflow_tools_env` prove that `workflow_operations_env` does not, and vice versa? There is no comment or structural signal explaining this split.
- Why does `view_via_workflow_tools_entry_point` only assert structural markers (`Queue:`, `Position:`) when all other tests assert content? The test name implies a routing distinction that does not exist — both fixtures route through the same `invoke_workflow` helper.
- Why does `view_reflects_updated_position_after_step_advancement` omit the `!output.contains("ERROR")` guard present in its near-twin `view_after_progressing_shows_updated_position`?

---

## Verdicts

| # | Test Name | Assertions | Env Fixture | Traceability | Load-Bearing? | Unique? | Correct? | Good Name? | Parameterizable? | **Verdict** |
|---|-----------|:----------:|-------------|-------------|:-------------:|:-------:|:--------:|:----------:|:----------------:|-------------|
| 1 | `view_queue_shows_current_position_and_next_step` | 7 | `workflow_tools_env` | T-12.0 / NFR-1 | ❌ Subsumed by #3 | ❌ Duplicate scenario | ✅ | ⚠️ Name says "next step" but no `Next:` assertion | ✅ Yes | **MERGE → #3** |
| 2 | `view_reflects_updated_position_after_step_advancement` | 2 | `workflow_tools_env` | none | ⚠️ Borderline | ⚠️ Near-twin of #5 | ⚠️ Missing `!contains("ERROR")` guard | ✅ | ✅ Yes | **REWRITE** |
| 3 | `view_displays_queue_status_with_current_position` | 8 | `workflow_operations_env` | none | ✅ Yes | ⚠️ Superset of #1 | ✅ | ✅ | ✅ Yes | **KEEP** |
| 4 | `view_with_invalid_session_id_returns_error` | 3 | `workflow_operations_env` | none | ✅ Yes | ✅ Only invalid-session test | ✅ | ✅ | ➖ No | **KEEP** |
| 5 | `view_after_progressing_shows_updated_position` | 3 | `workflow_operations_env` | none | ⚠️ Borderline | ⚠️ Near-twin of #2 | ✅ | ✅ | ✅ Yes | **MERGE → #2** |
| 6 | `view_via_workflow_tools_entry_point` | 3 | `workflow_operations_env` | none | ❌ Fully subsumed | ❌ No unique assertions | ⚠️ Weakest set | ❌ Name implies routing distinction that does not exist | ➖ | **REMOVE** |
| 7 | `view_without_session_id_throws_helpful_error` | 3 | `workflow_operations_env` | T-15.14 / FR-8 | ✅ Yes | ✅ Only no-sessionId error test | ✅ Uses `expect_err` correctly | ✅ | ➖ No | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `view_queue_shows_current_position_and_next_step`

**Scenario**: Start a 3-workflow queue, immediately view it.

**Problems**:

1. **Name misleads**: The name promises "next step" behaviour, but there is no `assert!(output.contains("Next:"))`. The name conflates "next step in the queue" (which has not been prompted) with the step-advancement behaviour tested in #2/#5. Readers looking for "next step" coverage will find this test and trust it incorrectly.

2. **Duplicate scenario**: Test #3 covers an identical scenario (start a 3-workflow queue, immediately view) using `workflow_operations_env` and adds one more assertion — `"workflow 1/3"`. Test #1 is therefore a strict subset of Test #3. If Test #1 fails alone, the diagnostic value is nil: Test #3 will also fail, and Test #3's output is more informative.

3. **Fixture split unexplained**: The only difference is workflow names (`wtt-alpha/beta/gamma` vs `test-workflow-1/2/3`). There is no comment suggesting this exercises distinct logic.

**Verdict: MERGE → Test #3**. Transfer the T-12.0 traceability comment to Test #3.

---

### Test 2 — `view_reflects_updated_position_after_step_advancement`

**Scenario**: Start a single-workflow session (`wtt-alpha`, 3 steps), advance one step, view.

**Problems**:

1. **Missing error guard**: Every other test in this file that uses `invoke_workflow(...).expect(...)` on the view call also adds `assert!(!output.contains("ERROR"), ...)`. Test #2 skips this guard. If the view returns an error string but does not return `Err(...)`, the `step 2/3` assertion will fail with a confusing message rather than a clear "unexpected ERROR" message.

2. **Near-twin of Test #5**: Test #5 does almost exactly this — advance a step in a queue, view, assert `step 2/3` and `Next:`. The distinction is: Test #2 uses a single-workflow session; Test #5 uses a 2-workflow queue. This _could_ be a meaningful distinction (queue position display vs. single-workflow display) but is not named or commented to clarify it.

3. **Only 2 assertions**: The thinnest assertion set among non-error-path tests.

**Verdict: REWRITE**. Add the `!contains("ERROR")` guard. Add a comment distinguishing the single-workflow case from the queue case. Promote to 3 assertions. If the single-workflow vs. queue distinction is not intentional, MERGE with Test #5.

---

### Test 3 — `view_displays_queue_status_with_current_position`

**Scenario**: Start a 3-workflow queue (`test-workflow-1/2/3`), immediately view.

**Strengths**: Strongest assertion set for the initial-view scenario (8 assertions). Asserts all 3 workflow names appear, correct `workflow 1/3` counter, correct `step 1/3` counter. No ERROR guard.

**Weaknesses**:
- Lacks a traceability comment (T-12.0 should migrate here from Test #1).
- "current_position" in the name is somewhat redundant with the test body — position 1 is the _only_ possible value on a freshly-started session.

**Verdict: KEEP**. Accept T-12.0 traceability from Test #1 when that test is merged in.

---

### Test 4 — `view_with_invalid_session_id_returns_error`

**Scenario**: Pass a nonexistent session ID to view; expect ERROR output.

**Strengths**: Unique negative path. Three targeted assertions: ERROR present, session ID echoed back in the message, "doesn't exist" phrasing preserved (locks the error message format). This is a contract test for error shape, not just error presence.

**Minor gap**: Hardcoded literal `"workflow-nonexistent-12345"` — any session ID would do, but using a recognisable name is fine. The `invalid_session_id` variable is extracted, so assertions on its presence are correct.

**Verdict: KEEP** as-is.

---

### Test 5 — `view_after_progressing_shows_updated_position`

**Scenario**: Start a 2-workflow queue, advance one step, view.

**Problems**:

1. **Near-twin of Test #2**: Same core assertion — `step 2/3` and `Next:` — after advancing one step. The structural difference (queue of 2 vs single workflow) is not articulated in the test name or comments.

2. **Queue of 2 but `step 2/3`**: `test-workflow-1` has 3 steps. Advancing 1 step puts us at step 2 of 3. That is correct, but the fact that the _queue_ has 2 workflows does not affect the _step_ counter being asserted. The queue size is decorative in this test.

**Verdict: MERGE → Test #2**. After rewriting Test #2 (add ERROR guard, clarify single-workflow vs queue intent), absorb Test #5 or replace both with a single parameterised test that covers both queue and single-workflow contexts.

---

### Test 6 — `view_via_workflow_tools_entry_point`

**Scenario**: Start a single workflow, view.

**Problems**:

1. **Name implies routing logic that is not tested**: "entry point" suggests this test verifies that the `Workflow` tool's view path is correctly wired, separate from some other entry point. But `invoke_workflow` is the same helper used by all 7 tests. There is no second entry point being distinguished.

2. **Weakest assertion set**: 3 assertions, all structural markers (`!ERROR`, `Queue:`, `Position:`). Every other non-error test asserts workflow names or step counters. This test would pass if the view output contained only `Queue: Position:` with no actual data.

3. **Fully subsumed**: Test #3 starts a session and views, asserting Queue:, Position:, all workflow names, and both counters. Test #6's 3 assertions are a strict subset of Test #3's 8 assertions.

4. **No unique failure mode**: If this test fails, Test #3 will also fail with better diagnostics.

**Verdict: REMOVE**. Zero unique coverage. The "entry point" framing is misleading.

---

### Test 7 — `view_without_session_id_throws_helpful_error`

**Scenario**: Call `view: true` without a `sessionId`; expect a tool-level `Err` (not `Ok` with error text).

**Strengths**:
- Uses `expect_err` correctly — distinguishes between a tool returning `Ok("ERROR: ...")` and returning a real `Err(...)`. This is the only test in the file that exercises `invoke_workflow` as `Err`.
- Asserts 3 specific substrings in the error: "Invalid parameter combination", "view=true", "sessionId". These lock the error message as a contract.
- T-15.14 / FR-8 traceability is explicit and correct.

**Weaknesses**: None significant.

**Verdict: KEEP** as-is.

---

## Recommendations

### Priority 1 — Remove dead weight (low risk, immediate)

- **REMOVE `view_via_workflow_tools_entry_point`** (Test #6). It adds no unique assertions, its name is misleading, and it is fully covered by Test #3.

### Priority 2 — Collapse the duplicate pair (low risk)

- **MERGE `view_queue_shows_current_position_and_next_step`** (Test #1) into `view_displays_queue_status_with_current_position` (Test #3). Migrate the T-12.0 traceability comment to Test #3. Tests #1 and #3 cover the same scenario with different fixture names; the stronger test wins.

### Priority 3 — Fix the advancement pair (medium effort)

- **REWRITE `view_reflects_updated_position_after_step_advancement`** (Test #2): add `!output.contains("ERROR")` guard; add a comment clarifying the single-workflow context (vs. queue context in Test #5).
- **MERGE `view_after_progressing_shows_updated_position`** (Test #5) into the rewritten Test #2, or parameterise both into a single table-driven test:

```rust
#[test]
fn view_reflects_updated_position_after_step_advancement() {
    // single-workflow session
    // queue-of-2 session
    // both should show step 2/3 and Next:
}
```

### Priority 4 — Rename Test #1 (if kept before merge)

The name `view_queue_shows_current_position_and_next_step` asserts `step 1/3` but never asserts `Next:` — the name is incorrect. Before or during the merge, rename the intent to `view_queue_shows_initial_position_and_all_workflow_names`.

### Parameterisation opportunity

Tests #1, #3, #2, and #5 share the same structural pattern:
1. Set up env
2. Optionally advance step
3. Invoke `view: true`
4. Assert a set of strings

These could be driven from a table of `(env_factory, advance_count, expected_strings)` tuples, eliminating the fixture split entirely. This would reduce 4 tests to 1–2 parameterised cases with explicit intent per row.

---

## Load-Bearing Score: 4/7

| Test | Load-Bearing | Reason |
|------|:---:|--------|
| `view_queue_shows_current_position_and_next_step` | ❌ | Strict subset of Test #3; T-12.0 tag is portable |
| `view_reflects_updated_position_after_step_advancement` | ✅ | Single-workflow advancement — distinct from queue context if intentional |
| `view_displays_queue_status_with_current_position` | ✅ | Richest initial-view test; only one asserting `workflow 1/3` |
| `view_with_invalid_session_id_returns_error` | ✅ | Unique negative path; no other test covers invalid session |
| `view_after_progressing_shows_updated_position` | ❌ | Near-twin of #2; queue-of-2 context is incidental |
| `view_via_workflow_tools_entry_point` | ❌ | Zero unique coverage; fully subsumed by #3 |
| `view_without_session_id_throws_helpful_error` | ✅ | Unique `Err` path; T-15.14/FR-8 contract test |

**Minimum viable set after recommendations: 4 tests** (initial queue view, step advancement, invalid session, missing session ID). Current 7 tests represent 75% redundancy in scenario coverage relative to that minimum.
