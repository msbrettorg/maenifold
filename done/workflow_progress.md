---
id: workflow_progress
status: done
---

# Test Evaluation: workflow_progress.rs

**File**: `crates/maenifold-core/tests/workflow_progress.rs`
**Traceability**: T-12.0 · RTM NFR-1
**Evaluator color**: 🔘 Gray — Skeptical Inquiry
**Related concepts**: [[test-quality]] · [[maenifold-rs]] · [[workflow-execution]] · [[step-progression]]

---

## Summary

`workflow_progress.rs` contains **4 integration tests** covering the "continue an in-progress
workflow" path: step advancement, frontmatter persistence after a continue, thoughts content
preservation, and the behavior of thoughts without a conclusion at an intermediate step.

The module is lean and focused, but cross-file overlap with `workflow_lifecycle.rs` and internal
duplication between tests 3 and 4 dilute the independent value of two tests. Assertion quality
varies sharply: test 3 is exemplary; test 2 makes seven assertions and proves almost nothing
meaningful because every one is an existence check.

The shared mutex (`TEST_MUTEX`) coupled with `Config::override_root()` isolation is correctly
sequenced — `env` drops before `_guard` (Rust reverse-declaration order), so `reset_overrides()`
executes while the lock is still held. No concurrency hazard is present, but the mutex is
functionally redundant given each test creates a fully isolated `TempDir`. It should be kept for
safety (global `ToolRegistry`), not removed silently.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `continue_to_next_step_in_same_workflow` | ✅ Yes | ⚠️ Partial | ⚠️ Mostly | ✅ Yes | ✅ Possible | **KEEP** |
| 2 | `continue_creates_proper_frontmatter_metadata` | ⚠️ Partial | ⚠️ Partial | ❌ Weak | ✅ Yes | ❌ No | **REWRITE** |
| 3 | `continue_preserves_thoughts_in_content` | ✅ Yes | ✅ Yes | ✅ Good | ✅ Yes | ✅ Possible | **KEEP** |
| 4 | `continue_with_thoughts_but_no_conclusion_at_intermediate_step` | ❌ No | ❌ No | ⚠️ Weak | ⚠️ Misleading | ❌ No | **MERGE → 3** |

---

## Detailed Findings

### Test 1 — `continue_to_next_step_in_same_workflow`

**Load-bearing**: Yes. This is the canonical happy-path for step advancement.

**Overlap concern**: `workflow_lifecycle.rs::workflow_advances_from_step1_to_step2` also starts a
session, submits a response, and asserts `output.contains("Step 2/3")` and
`currentStep == 1`. The tests are not identical — `workflow_progress` additionally verifies the
persisted **content** ("Step 1/3 Response" heading, `[[workflow-execution]]` WikiLink), which the
lifecycle test omits. That distinction is meaningful and justified.

**Assertion weakness**: `assert!(!output.contains("ERROR"))` is a negative string scan. A custom
error format that avoids the word "ERROR" (e.g., `"Err:"`, `"❌"`, or a structured error block)
would silently pass. Replace with a positive assertion about what the output *must* contain (e.g.,
the step prompt text or a "Session:" line), or at minimum complement with
`assert!(output.contains("Step 2/3"))` only, dropping the negative check.

**Parameterization opportunity**: The response WikiLink names are hardcoded. A small table of
`(workflow_id, response_text, expected_step_label)` tuples could cover multi-step spans cheaply.
Not urgent.

---

### Test 2 — `continue_creates_proper_frontmatter_metadata`

**Load-bearing**: Borderline. The intent — verifying post-continue frontmatter completeness — is
legitimate. The execution defeats that intent.

**Assertion quality problems** (7 assertions, 4 are vacuous):

| Assertion | Problem |
|-----------|---------|
| `frontmatter.get("title").is_some()` | Does not verify the value. A blank title passes. |
| `type == "workflow"` | This was already true at session creation (lifecycle test). Does not prove it survives a continue. |
| `permalink == session_id` | Valid and load-bearing — unique to this test. |
| `status == "active"` | Redundant: lifecycle test asserts `"active"` at creation; completion tests assert it changes. |
| `frontmatter.get("queue").is_some()` | Does not verify queue contents. An empty queue passes. |
| `frontmatter.get("currentWorkflow").is_some()` | Does not verify value. Should be `== 0` for a single-workflow session. |
| `frontmatter.get("currentStep").is_some()` | Strictly redundant: test 1 already asserts `frontmatter_usize(&frontmatter, "currentStep") == 1`. |

What this test *should* verify, given it fires after a continue: (a) title is non-empty, (b) queue
structure is valid (e.g., `["test-workflow-1"]`), (c) `currentWorkflow == 0`, (d) `currentStep ==
1`. Fields like `type` and `permalink` that don't change after a continue belong in the lifecycle
test, not here.

---

### Test 3 — `continue_preserves_thoughts_in_content`

**Load-bearing**: Yes. No other test in the workflow suite verifies thoughts content format,
the `"Thoughts:"` section header, or WikiLink preservation within the `thoughts` parameter.

**Assertions**: All five are meaningful — response verbatim, thoughts verbatim, `"Thoughts:"`
header, specific WikiLink `[[ambient-thinking]]`. The inclusion of a named WikiLink (not just any
`[[...]]`) is appropriately specific.

**Missing**: Does not assert `output.contains("Step 2/3")` (step advancement). This absence is
partially covered by test 4, but test 4 is being recommended for MERGE here. Add the advancement
check when absorbing test 4.

---

### Test 4 — `continue_with_thoughts_but_no_conclusion_at_intermediate_step`

**Load-bearing**: No. Every behavior this test validates is already covered:

| Assertion in test 4 | Where it is already covered |
|---------------------|------------------------------|
| `!output.contains("ERROR")` | Implicitly by test 3 (no error path triggered) |
| `output.contains("Step 2/3")` | Test 1 (`continue_to_next_step_in_same_workflow`) |
| `content.contains(thoughts)` | Test 3 (`continue_preserves_thoughts_in_content`) |

**Name is misleading**: The name implies the test asserts that a conclusion is *not required* at
an intermediate step. It does not. It never asserts `!output.contains("conclusion")` or anything
negative about conclusion enforcement. The name describes motivation, not behavior.

**Unique contribution**: The `output.contains("Step 2/3")` check in the context of a
thoughts-bearing request is the only non-redundant signal — and it belongs in test 3 after merge.

---

## Recommendations

1. **KEEP test 1** as-is. Replace `assert!(!output.contains("ERROR"))` with a stronger positive
   assertion (step prompt or session marker). Add a comment distinguishing it from
   `workflow_lifecycle::workflow_advances_from_step1_to_step2`.

2. **REWRITE test 2** (`continue_creates_proper_frontmatter_metadata`). Replace the four
   existence-only `is_some()` checks with value assertions. Remove the `status` and `type` checks
   (already covered by lifecycle). Keep only assertions that are uniquely sensitive to post-continue
   state: `permalink == session_id`, non-empty `title`, `currentStep == 1`,
   `currentWorkflow == 0`, and queue contents.

   ```rust
   // After rewrite (sketch):
   assert_eq!(frontmatter.get("permalink").and_then(YamlValue::as_str), Some(session_id.as_str()));
   assert!(frontmatter.get("title").and_then(YamlValue::as_str).map(|t| !t.is_empty()).unwrap_or(false), "title should be non-empty");
   assert_eq!(frontmatter_usize(&frontmatter, "currentStep"), 1);
   assert_eq!(frontmatter_usize(&frontmatter, "currentWorkflow"), 0);
   let queue = queue_from_frontmatter(&frontmatter);
   assert_eq!(queue, vec!["test-workflow-1"]);
   ```

3. **KEEP test 3** and add `assert!(output.contains("Step 2/3"))` to absorb test 4's only
   non-redundant signal.

4. **REMOVE test 4** after absorbing its `output.contains("Step 2/3")` check into test 3.

5. **Future**: Consider parameterized coverage (e.g., using a macro or a helper that runs
   the same assertions across `test-workflow-1/2/3` shapes) for step count boundary conditions
   (single-step, last-step, mid-workflow). Currently no test covers "continue at the *last* step
   before completion" within this file — that gap is covered by `workflow_completion.rs`, but the
   coupling is invisible here.

---

## Load-Bearing Score: 3/4

Tests 1, 2 (intent), and 3 carry real coverage weight. Test 4 is fully subsumed. Test 2's
*implementation* is so weak that even though its coverage intent is valid, the assertions as
written would not catch a frontmatter serialization bug that produced empty or wrong values —
making its effective load-bearing contribution closer to 0.5.

**Effective score with implementation quality weighted: ~2.5 / 4**

---
## SWE Remediation
- KEEP test 1 (`continue_to_next_step_in_same_workflow`): replaced `!output.contains("ERROR")` with positive `Step 2/3` assertion (P0); added comment distinguishing from `workflow_lifecycle` overlap
- REWRITE test 2 (`continue_creates_proper_frontmatter_metadata`): replaced 4 vacuous `is_some()` checks with value assertions — non-empty title, `currentStep == 1`, `currentWorkflow == 0`, queue == `["test-workflow-1"]`; removed redundant `type`/`status` checks covered by lifecycle tests
- KEEP test 3 (`continue_preserves_thoughts_in_content`): absorbed test 4's `output.contains("Step 2/3")` check to verify thoughts-bearing requests still advance steps
- REMOVE test 4 (`continue_with_thoughts_but_no_conclusion_at_intermediate_step`): fully subsumed by tests 1+3 after merge
- Tests: 3 passed, 0 failed (down from 4 → 3)
- Commit: 2566c6c

---
## Red-team Audit

**Traceability**: T-12.0 · RTM NFR-1

### Test 1 — KEEP `continue_to_next_step_in_same_workflow`
- ✅ Fragile `assert!(!output.contains("ERROR"))` removed (P0 negative-scan weakness)
- ✅ Positive `assert!(output.contains("Step 2/3"))` retained with improved error message
- ✅ Distinguishing comment added referencing `workflow_lifecycle::workflow_advances_from_step1_to_step2` overlap
- Matches eval spec

### Test 2 — REWRITE `continue_creates_proper_frontmatter_metadata`
- ✅ `frontmatter.get("title").is_some()` → non-empty title string assertion
- ✅ `frontmatter.get("queue").is_some()` → `queue_from_frontmatter() == vec!["test-workflow-1"]`
- ✅ `frontmatter.get("currentWorkflow").is_some()` → `frontmatter_usize("currentWorkflow") == 0`
- ✅ `frontmatter.get("currentStep").is_some()` → `frontmatter_usize("currentStep") == 1`
- ✅ `type == "workflow"` check removed (covered by lifecycle)
- ✅ `status == "active"` check removed (covered by lifecycle)
- ✅ `permalink == session_id` retained (load-bearing, unique to this test)
- All 4 vacuous `is_some()` checks replaced with value assertions; both redundant invariant checks removed
- Matches eval spec sketch exactly

### Test 3 — KEEP + MERGE from test 4
- ✅ `invoke_workflow()` return value now captured into `output`
- ✅ `assert!(output.contains("Step 2/3"))` absorbed from test 4 — the only non-redundant signal
- ✅ Traceability comment documents the MERGE origin
- All 5 original assertions preserved; 1 new assertion added
- Matches eval spec

### Test 4 — REMOVE `continue_with_thoughts_but_no_conclusion_at_intermediate_step`
- ✅ Entire test deleted (19 lines removed)
- ✅ Confirmed no unique coverage lost: `Step 2/3` now in test 3, `content.contains(thoughts)` in test 3, negative error scan was weak and dropped per eval
- No residual references to test 4 remain in the file

### Cross-checks
- ✅ Only `workflow_progress.rs` modified in commit 2566c6c — no unrelated changes
- ✅ `queue_from_frontmatter` was a pre-existing helper (workflow_support/mod.rs:125), not introduced by this commit
- ✅ Tests: 3 passed, 0 failed (down from 4 → 3) — matches SWE claim
- ✅ No new imports beyond `queue_from_frontmatter` which was already in scope

- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- workflow_progress`: 3 passed, 0 failed
- Verdict: PASS
