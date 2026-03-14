# Test Evaluation: workflow_append
**File:** `crates/maenifold-core/tests/workflow_append.rs`
**Traceability:** T-12.0 · RTM NFR-1
**Test count:** 7
**Evaluated:** 2026-07-13

## Summary

The file covers important append semantics (single, multi, invalid session, missing workflow, atomicity) but carries **two duplicate pairs** using different fixture environments (`workflow_tools_env` vs `workflow_operations_env`) that test identical behavior. The error-path tests are well-constructed and distinct.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `append_adds_workflow_to_queue` | **MERGE** with `append_valid_workflow_to_queue` | Uses `workflow_tools_env` (wtt-* fixtures). Asserts: output contains "Added 1 workflow(s) to queue", name, no "ERROR"; queue.len()==2; queue[1] is the appended id. `append_valid_workflow_to_queue` is an exact behavioral twin using `workflow_operations_env`. The two fixture sets have no semantic difference for append coverage. |
| `append_adds_multiple_workflows_as_json_array` | **MERGE** with `append_multiple_workflows_as_json_array` | Uses `workflow_tools_env`. Checks count message and names. `append_multiple_workflows_as_json_array` (test 6) covers the same path with `workflow_operations_env` AND stronger assertions (exact queue vector, "New queue:" output). Keep test 6's assertions, delete this test. |
| `append_valid_workflow_to_queue` | **KEEP** (after merge — becomes the canonical single-append test) | This or `append_adds_workflow_to_queue` should survive as the single canonical happy-path test. Prefer this one's fixture set (`workflow_operations_env`) because test 6 (the stronger multi-append) also uses it, giving fixture consistency within this file. Rename to `append_single_valid_workflow_succeeds_and_updates_queue`. |
| `append_with_invalid_session_id_returns_error` | **KEEP** | Unique error path: non-existent session ID. Asserts output contains "ERROR", the invalid id, and "doesn't exist". No other test probes this path. Load-bearing for session-lookup failure handling. |
| `append_with_missing_workflow_file_returns_error` | **KEEP** | Unique error path: valid session but workflow file not found. Asserts `invoke_workflow` returns `Err`, error contains name and "not found", AND queue remains length 1 (no partial mutation). That last assertion is critical — it validates transactional integrity on the error path. |
| `append_multiple_workflows_as_json_array` | **KEEP** (after merge — becomes the canonical multi-append test) | Strongest multi-append test: exact queue vector assertion (`assert_eq!(queue, vec![...])`) plus "New queue:" output check. Absorbs `append_adds_multiple_workflows_as_json_array`. |
| `append_validates_all_workflows_before_queue_update` | **KEEP** | Unique atomicity test: a mixed array (valid + nonexistent) should fail entirely, leaving the queue unchanged. This covers the all-or-nothing validation contract. Critical for preventing partial queue corruption. |

---

## Recommendations

1. **Delete `append_adds_workflow_to_queue`** (test 1). `append_valid_workflow_to_queue` covers the same behavior with identical structural assertions. No unique failure mode is exercised.

2. **Delete `append_adds_multiple_workflows_as_json_array`** (test 2). `append_multiple_workflows_as_json_array` (test 6) is strictly stronger: it checks the exact queue vector, not just length. The weaker test adds no value.

3. **Rename surviving tests** for clarity:
   - `append_valid_workflow_to_queue` → `append_single_workflow_succeeds_and_updates_queue`
   - `append_multiple_workflows_as_json_array` → `append_json_array_of_workflows_succeeds_and_updates_queue_atomically`

4. **The two fixture environments (`workflow_tools_env` / `workflow_operations_env`) provide no differential value** for append testing. The workflows' internal step counts and names are irrelevant to append behavior. Consider rationalizing to a single environment across this file.

5. **Consider a parameterized table** for error paths (tests 4, 5, 7) if a test framework with `test_case` support is adopted. All three share the pattern: setup → invoke → assert error → assert queue unchanged.

6. **String-label assertions** (`output.contains("Added 1 workflow(s) to queue")`) are partially format-testing rather than behavior-testing. The queue data structure assertions (`queue.len()`, `queue[1]`) are the real load-bearing assertions. If the human-readable output format changes, tests 1/3 would fail for the wrong reason. Prefer the data assertion as primary; make string assertions informational only.

---

## Load-Bearing Score: 5/7

Tests 3 (`append_valid_workflow_to_queue`), 4, 5, 6, 7 are load-bearing. Tests 1 and 2 are redundant duplicates that would pass/fail in lockstep with tests 3 and 6 respectively — removing them loses no fault-detection capability.
