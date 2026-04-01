---
id: T-80.186
title: "Test Evaluation: workflow_lifecycle"
status: done
pipeline: tdd
---

# Test Evaluation: workflow_lifecycle
**File:** `crates/maenifold-core/tests/workflow_lifecycle.rs`
**Test count:** 8
**Evaluated:** 2026-06-18
**Traceability:** T-12.0 · T-15.14 · NFR-1 · FR-8

---

## Summary

The tightest of the three files. Every test targets a distinct lifecycle branch: session creation, persistence verification, missing-workflow error, mutual-exclusion error, step advancement, learn-mode short-circuit, and two input-validation guards. Overlap with `workflow_queue.rs` is minimal (step advancement is present in both, but `workflow_advances_from_step1_to_step2` is the only such test here and its persistence assertion is complementary, not redundant). Two tests carry Java-ish naming that should be cleaned up.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `learn_mode_short_circuits_execution_without_creating_session` | **KEEP** | Sole test verifying that `learn: true` bypasses session creation *and* parameter validation entirely. Four distinct negative assertions. Removing this would allow a regression where learn mode accidentally creates a session or throws a validation error. |
| `workflow_with_workflow_id_creates_new_session` | **KEEP** | Verifies the happy-path output surface: `"Created session:"` and `"Step 1/3"`. Distinct from `workflow_created_session_is_readable_via_markdown_io` (which checks persistent state, not output text). Both are needed; they test different layers. |
| `workflow_created_session_is_readable_via_markdown_io` | **KEEP** | Checks five frontmatter fields (`status`, `type`, `currentStep`, `currentWorkflow`) in the persisted markdown file. The only lifecycle test that validates storage-layer correctness independently of tool output text. Load-bearing against persistence regressions. |
| `workflow_with_nonexistent_workflow_id_returns_error` | **KEEP** | Only test that verifies the workflow-not-found error path. Asserts the missing ID appears verbatim in the error and the word "not found" is present. |
| `workflow_with_both_workflow_id_and_session_id_throws` | **KEEP** | Verifies mutual exclusivity of `workflowId`/`sessionId`. Asserts the exact error string `"Cannot provide both workflowId and sessionId"`. Unique coverage; no other test probes this guard. |
| `workflow_advances_from_step1_to_step2` | **KEEP** | Only lifecycle-file test that exercises session continuation. Checks both output (`"Step 2/3"`) and frontmatter (`currentStep == 1`). Complementary to `workflow_progress.rs` tests — this one establishes the baseline step transition works before the queue tests stress it. |
| `workflow_with_no_parameters_throws_invalid_operation_exception` | **KEEP** (rename) | Load-bearing: the only test ensuring a zero-parameter call returns an error rather than silently succeeding or emitting help text (T-15.14). However, **rename** to `workflow_with_no_parameters_returns_invalid_combination_error` — "throws_invalid_operation_exception" is Java terminology and misrepresents Rust's error-return model. |
| `workflow_with_only_session_id_throws_invalid_operation_exception` | **KEEP** (rename) | Load-bearing: ensures that a `sessionId`-only call (without `response`, `view`, etc.) is rejected and the error hints at `view=true`. **Rename** to `workflow_with_only_session_id_returns_invalid_combination_error`. |

---

## Recommendations

1. **Rename tests 7 and 8.** The `_throws_invalid_operation_exception` suffix is Java idiom. Suggested names:
   - `workflow_with_no_parameters_returns_invalid_combination_error`
   - `workflow_with_only_session_id_returns_invalid_combination_error`

2. **Test 7 assertion coverage is complete, but the error message is asserted across three calls.** Consider consolidating into a single multi-assert block rather than calling `error.contains(...)` three times without a shared message. This is a minor style issue, not a correctness one.

3. **`workflow_with_workflow_id_creates_new_session` is pinned to `wtt-alpha`'s step count.** It asserts `"Step 1/3"` which relies on `wtt-alpha` having exactly 3 steps. If the fixture is edited, the test breaks with an opaque mismatch. Consider asserting `"Step 1/"` or the frontmatter `currentStep == 0` as a fixture-agnostic alternative.

4. **No negative test for `learn: true` with invalid other parameters simultaneously.** The current learn-mode test only calls with `{"learn": true}`. A future regression could make learn mode fail when combined with, e.g., `{"learn": true, "workflowId": "x"}`. Consider adding a variant.

---

## Load-Bearing Score: 8/8

> Every test in this file targets a distinct code branch with no cross-test redundancy. This is the model file for the three being evaluated.

---
## SWE Remediation
- RENAME `workflow_with_no_parameters_throws_invalid_operation_exception` → `workflow_with_no_parameters_returns_invalid_combination_error`: Java-ish naming → Rust-idiomatic (Rec 1)
- RENAME `workflow_with_only_session_id_throws_invalid_operation_exception` → `workflow_with_only_session_id_returns_invalid_combination_error`: Java-ish naming → Rust-idiomatic (Rec 1)
- KEEP all 8 tests per verdicts; no removals, rewrites, or merges needed
- `workflow_with_workflow_id_creates_new_session`: assert `"Step 1/"` instead of `"Step 1/3"` for fixture-agnostic robustness (Rec 3)
- Skipped Rec 2 (minor style, not correctness) and Rec 4 (adding new test beyond scope)
- All 8 tests pass: `test result: ok. 8 passed; 0 failed; 0 ignored`
- Commit: fe7bfbe
