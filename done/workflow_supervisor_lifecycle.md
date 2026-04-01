---
id: T-80.192
title: "Test Evaluation: workflow_supervisor_lifecycle"
status: done
pipeline: tdd
---

# Test Evaluation: workflow_supervisor_lifecycle
**File:** `crates/maenifold-core/tests/workflow_supervisor_lifecycle.rs`
**Test count:** 6
**Evaluated:** 2026-07-14
**Traceability:** T-W1-7 · T-12.7 · NFR-1 · FR-10.1 · FR-10.2 · FR-10.3 · FR-10.4

---

## Summary

Six tests covering the full supervisor lifecycle of the workflow engine: initial field initialisation, submachine registration, blocking while active, resumption after completion, resumption after cancellation, and backward compatibility with legacy sessions lacking the `phase` field. Every test targets a meaningfully distinct scenario and no two tests are pure duplicates. Three specific concerns deserve attention:

1. **Asymmetric assertion depth between tests 4 and 5**: `continue_after_submachine_completed_clears_supervisor_fields_and_resumes` verifies all three supervisor fields are cleared on resumption (`phase`, `activeSubmachineType`, `activeSubmachineSessionId`). Its cancelled-state twin, test 5, only checks `phase`. If the cancelled path contained a bug where submachine fields are not cleared, test 5 would silently pass. This is an active gap against FR-10.1.

2. **String-based step-progression assertions are pervasive and brittle**: Four tests (`3`, `4`, `5`, `6`) assert on `result.contains("Step 2/")` or `!result.contains("Step 2/")` to determine whether step advancement occurred. Any output-format refactoring (e.g., `"Step 2 of"`, `"step-2"`, `"2/3"`) silently undermines these guards. The backing `currentStep` frontmatter value is a more durable signal — it is already read in tests 2 and 3 and should be used uniformly.

3. **`wsup-beta` is never exercised**: `isolated_env()` creates both `wsup-alpha` (3 steps) and `wsup-beta` (2 steps), but every test in this file uses `wsup-alpha`. The second fixture exists only to confirm the support helper doesn't break on multiple workflows. This is not a correctness defect but indicates the parameter space is incomplete — in particular, the `wsup-beta` boundary (a 2-step workflow where step advancement reaches the final step) is untested here.

4. **Moderate cross-file overlap with `workflow_supervisor_registration.rs`**: Test 2 (`register_submachine_enters_waiting_state_without_advancing_step`) shares assertion territory with `response_persisted_during_register` in the sibling file (both verify `phase = "waiting"` and `currentStep` freeze post-registration). This is not redundancy worthy of removal — the lifecycle file exercises the scenario as a *precondition setup* rather than a *primary target* — but it should be acknowledged so future maintainers do not add further overlap.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `workflow_start_writes_supervisor_frontmatter_fields` | **KEEP** | Only test that verifies the initial supervisor field state on workflow creation (FR-10.3). Assertions cover `phase`, `activeSubmachineType`, and `activeSubmachineSessionId` — the three fields owned by the supervisor. Minor gap: `currentStep` is not verified here (it is confirmed indirectly in tests 2 and 3), which is acceptable given FR-10.3's scope. No changes required. |
| `register_submachine_enters_waiting_state_without_advancing_step` | **KEEP** (harden string assertions) | The entry-point lifecycle test for submachine registration. Correctly captures `step_before` before registration and confirms it is unchanged afterward — the right pattern. Three string-based assertions (`"Registered submachine"`, `thinking_session_id`, `"waiting"`) are brittle: the last two are useful; the first is a message-phrasing dependency. Frontmatter assertions are solid. Cross-file overlap with the registration suite is acceptable since both files have different primary concerns. |
| `continue_while_waiting_on_active_submachine_blocks_and_does_not_advance_step` | **KEEP** (harden step assertion) | The sole test for the blocking invariant — a critical guard against accidental step advancement while a submachine is running (FR-10.4). The `assert!(!block_result.contains("Step 2/"))` negative string match is the weakest assertion in the file. If the output format changes, this silently passes. The frontmatter read at the bottom (`yaml_usize(..., "currentStep") == step_after_register`) is the authoritative check — the string assertion should be removed or replaced with a named constant. |
| `continue_after_submachine_completed_clears_supervisor_fields_and_resumes` | **KEEP** | The canonical happy-path test for the full supervisor lifecycle. Covers initiation, registration, terminal transition, and resumption in sequence. All three supervisor fields are verified on both the result text and the frontmatter. The `!resume_result.contains("ERROR")` assertion is a useful broad-stroke guard even if it cannot catch panics or unusual error forms. No defects; the pattern is the reference implementation for tests 5 and 6. |
| `continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes` | **REWRITE** | Covers a semantically distinct terminal state ("cancelled") but is missing the `activeSubmachineType` and `activeSubmachineSessionId` cleared-field assertions that test 4 carries. If the cancelled resumption path contains a bug where supervisor fields survive, this test will not catch it — a direct gap against FR-10.1. The fix is two lines. Once hardened, these two tests could be merged into a single parameterized test over `["completed", "cancelled"]` to reduce setup duplication, but parity of assertions is the higher priority. |
| `backward_compatibility_old_session_without_phase_field_proceeds_normally` | **KEEP** (add post-advance assertion) | The sole regression guard for pre-supervisor sessions. Correctly verifies the precondition (`!frontmatter.contains_key("phase")`) before exercising the legacy path. The `!result.contains("ERROR")` and `result.contains("Step 2/")` checks confirm forward progress. A missing assertion: after the advance, does the session now carry `phase = "running"`? If the backward-compat path migrates the session on first use that is a meaningful invariant worth pinning; if it deliberately operates without the field, that too should be documented. The test currently leaves the post-run state undefined. |

---

## Recommendations

### P1 — Add missing supervisor-field clear assertions to test 5 (REWRITE)

`continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes` omits verification that `activeSubmachineType` and `activeSubmachineSessionId` are cleared after a cancelled submachine is resolved. Add the two missing assertions from test 4:

```rust
assert_eq!(yaml_string(&frontmatter, "activeSubmachineType"), "");
assert_eq!(yaml_string(&frontmatter, "activeSubmachineSessionId"), "");
```

This makes the test's assertion depth match test 4 and closes the gap against FR-10.1.

### P2 — Replace brittle `contains("Step 2/")` checks with frontmatter reads

Tests 3, 4, 5, and 6 all use `result.contains("Step 2/")` or its negation to probe step advancement. This couples the tests to output-format details. The `currentStep` frontmatter field is already read in tests 2 and 3; the same pattern should be applied uniformly. For tests that currently do string-based checks:

```rust
// Instead of:
assert!(resume_result.contains("Step 2/"));

// Prefer:
let post_frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&workflow_session_id));
assert_eq!(yaml_usize(&post_frontmatter, "currentStep"), 1);
```

The `"Step 2/"` string assertion may remain as a secondary sanity check but should not be the *only* advancement signal.

### P3 — Consider merging tests 4 and 5 into a parameterized test

After P1 is applied, tests 4 and 5 will be structurally identical except for the terminal state string. Rust's `rstest` or a hand-rolled helper function can express both:

```rust
fn submachine_resume_after_terminal_state(terminal_status: &str) {
    // ... shared setup and assertions
}

#[test] fn resumes_after_completed() { submachine_resume_after_terminal_state("completed"); }
#[test] fn resumes_after_cancelled() { submachine_resume_after_terminal_state("cancelled"); }
```

This removes ~40 lines of duplication and makes it trivially easy to add future terminal states (e.g., `"errored"`, `"timed_out"`).

### P4 — Pin the post-advance state in the backward-compat test

`backward_compatibility_old_session_without_phase_field_proceeds_normally` does not specify what state the session is in after the advance. Add an assertion on the post-run frontmatter to document the expected behaviour:

```rust
// Either: the session migrates the phase field on first use
let post_frontmatter = read_frontmatter(&env.cfg, &workflow_uri(&session_id));
assert_eq!(yaml_string(&post_frontmatter, "phase"), "running");
// Or: phase is absent and currentStep advanced
assert_eq!(yaml_usize(&post_frontmatter, "currentStep"), 1);
```

Whichever is correct, it should be explicit — the test is the specification for backward-compat behaviour.

### P5 — Remove or replace the `contains("Registered submachine")` assertion in test 2

`assert!(register_result.contains("Registered submachine"))` in test 2 binds the test to an exact phrase in the response string. This is a presentation-layer dependency. The structural state (`phase = "waiting"`, submachine fields set) is the load-bearing signal; the phrase check adds noise without adding coverage. If message-content coverage is desired, extract a constant:

```rust
pub(crate) const SUBMACHINE_REGISTERED_MSG: &str = "Registered submachine";
```

### P6 — Exercise `wsup-beta` in at least one test

The `isolated_env()` fixture creates both `wsup-alpha` (3 steps) and `wsup-beta` (2 steps), but only `wsup-alpha` is used. A 2-step workflow's final step boundary (step 2/2 advancing to completion) is a different code path from a mid-workflow step. Adding a single test using `wsup-beta` would validate that the supervisor lifecycle is correct at workflow completion, not just mid-workflow.

---

## Load-Bearing Score: 5/6

Test 5 (`continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes`) is load-bearing in intent — "cancelled" is a meaningfully distinct terminal state — but as written it does not assert on all the fields it claims to test (FR-10.1). It passes for a partially-correct implementation. The remaining five tests are each the unique owner of a distinct invariant and all carry correct, sufficient assertions for their stated purpose.

---

## SWE Remediation

- REWRITE `continue_after_submachine_cancelled_clears_supervisor_fields_and_resumes`: added `currentStep`, `activeSubmachineType`, and `activeSubmachineSessionId` assertions after cancelled-state resumption.
- Hardened lifecycle progression checks in tests 3, 4, 5, and 6 by asserting `currentStep == 1` from frontmatter instead of relying on `"Step 2/"` output text.
- Hardened `register_submachine_enters_waiting_state_without_advancing_step` by dropping the exact `"Registered submachine"` phrase assertion while keeping the session-id and waiting-state checks.
- Validation: `cargo test --test integration_tests workflow_supervisor_lifecycle` (workspace uses the consolidated `integration_tests` harness in `tests/main.rs`).
- Commit: 5b10637
