# Test Evaluation: workflow_queue
**File:** `crates/maenifold-core/tests/workflow_queue.rs`
**Test count:** 8
**Evaluated:** 2026-06-18
**Traceability:** T-12.0 · NFR-1

---

## Summary

The file contains two structurally parallel sets of four tests each — a `serial_queue_*` group using `workflow_tools_env()` (wtt-\* fixtures, 1–3 steps) and a `continue_*` group using `workflow_operations_env()` (test-workflow-\* fixtures, 2 steps each). They exercise the same queue state-machine transitions with different fixture depths. Four of the eight tests are effectively pairwise duplicates that would not independently catch a bug the other wouldn't. The remaining two are unique and load-bearing.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `serial_queue_starts_with_first_workflow_in_array` | **KEEP** | Only test that reads `queue_from_frontmatter` to verify the raw queue array is persisted correctly in the session file. Checks both output and frontmatter state. Unique coverage. |
| `serial_queue_advances_to_next_workflow_after_first_completes` | **MERGE** with `continue_to_next_workflow_when_current_completes` | Both assert `currentWorkflow=1, currentStep=0` after the active workflow completes. Fixtures differ (wtt-gamma=1 step vs test-workflow-3=2 steps) but the code path tested is identical. Retain the deeper-path version (`continue_to_next_workflow_when_current_completes`) and drop this one, or keep only a single parameterized assertion. |
| `serial_queue_completes_when_all_workflows_done_with_conclusion` | **MERGE** with `continue_completes_all_workflows_with_conclusion` | Semantically identical terminal state test. The `continue_*` twin drives more steps (5 invocations vs 2), checks the `✅` emoji, and verifies `content.contains("Conclusion")` — strictly stronger. This version adds nothing beyond what its twin covers. |
| `serial_queue_exhausted_without_conclusion_prompts_for_conclusion` | **MERGE** with `continue_requires_conclustion_when_queue_exhausted` | Both assert the same three strings: `"ERROR"`, `"All workflows complete"`, `"conclusion parameter"`. Fixture depth differs (1-step vs 2-step traversal) but they exercise the same guard branch. Merge into one test; fix the typo in `conclustion` while doing so. |
| `continue_to_next_workflow_when_current_completes` | **KEEP** | Retains after merging with `serial_queue_advances_*`. Exercises a 2-step first workflow before transitioning, checking `Step 2/2` intermediate output — a meaningful additional depth check absent in the twin. |
| `continue_requires_conclustion_when_queue_exhausted` | **KEEP** (after merge + rename) | Retains after merging with `serial_queue_exhausted_*`. Has a typo in the function name (`conclustion`). **Rename to** `queue_exhausted_without_conclusion_prompts_for_conclusion`. |
| `continue_updates_current_step_index_correctly` | **KEEP** | The only test that verifies `currentStep` at every individual increment (0→1→2→reset to 0 on workflow transition). No other test in the file or in `workflow_lifecycle.rs` or `workflow_progress.rs` does this granular step-tracking audit. Highly load-bearing. |
| `continue_completes_all_workflows_with_conclusion` | **KEEP** | The stronger of the two completion twins. Checks `✅` emoji, `"All workflows"`, `status="completed"` in frontmatter, `"Conclusion"` and `"[[final-summary]]"` in content. Retains after the merge above. |

---

## Recommendations

1. **Collapse the four parallel pairs.** `serial_queue_advances_*` / `continue_to_next_*` and `serial_queue_completes_*` / `continue_completes_*` and `serial_queue_exhausted_*` / `continue_requires_*` each cover the same logical branch. Reduce to one test per behavior. **Net reduction: 3 tests**, no coverage lost.

2. **Fix the typo** `conclustion` → `conclusion` in `continue_requires_conclustion_when_queue_exhausted` at the same time as the merge.

3. **Separate environments aren't earning their keep.** `workflow_tools_env()` (wtt-\*) and `workflow_operations_env()` (test-workflow-\*) fixtures exist because of historical module splitting (T-12.0). Now that the split is stable, reconsider whether both fixture families are necessary, or consolidate to one canonical set.

4. **`serial_queue_starts_with_first_workflow_in_array` should assert output text precisely.** The string `"Step 1/3"` is hardcoded against `wtt-alpha` (3 steps). If the fixture changes, the test breaks with a confusing message. Consider asserting `frontmatter_usize(&frontmatter, "currentStep") == 0` instead of (or in addition to) the string check.

---

## Load-Bearing Score: 5/8

> Tests 2, 3, and 4 are duplicated by tests 5, 8, and 6 respectively. Removing either set would not reduce behavioral coverage.

---

## SWE Remediation
- REMOVE `serial_queue_advances_to_next_workflow_after_first_completes`: subsumed by `continue_to_next_workflow_when_current_completes`
- REMOVE `serial_queue_completes_when_all_workflows_done_with_conclusion`: subsumed by `continue_completes_all_workflows_with_conclusion`
- REMOVE `serial_queue_exhausted_without_conclusion_prompts_for_conclusion`: subsumed by `queue_exhausted_without_conclusion_prompts_for_conclusion`
- RENAME `continue_requires_conclustion_when_queue_exhausted` → `queue_exhausted_without_conclusion_prompts_for_conclusion` (typo fix)
- Result: 8 → 5 tests, zero behavioral coverage loss
- Commit: 2566c6c
