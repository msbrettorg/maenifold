# Test Evaluation: workflow_view
**File:** `crates/maenifold-core/tests/workflow_view.rs`
**Test count:** 7
**Evaluated:** 2025-07-16
**Traceability:** T-12.0 · T-15.14 · RTM NFR-1 · RTM FR-8

---

## Summary

Three of the seven tests are near-duplicates of each other: two test initial queue display, and two test position advancement after one step — distinguished only by which fixture environment (`workflow_tools_env` vs `workflow_operations_env`) they use. The behavioral contract they verify is identical. One test (`view_via_workflow_tools_entry_point`) asserts a strict subset of what an existing test already asserts and can be removed entirely. The two error-path tests are load-bearing and unique. The assertions throughout rely exclusively on `contains`-style string matching, never on structural or exact output verification — this is appropriate for an integration-level output test but means subtle formatting regressions (e.g., wrong step count) would need to match a very specific substring.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `view_queue_shows_current_position_and_next_step` | **KEEP** | Tests initial queue display with `workflow_tools_env` (`wtt-*` fixtures). Asserts workflow names present, `Position:`, and `step 1/3`. The fixture names (`wtt-alpha` etc.) are meaningfully different from the operations fixtures and test a different environment setup. Keep as the canonical initial-view test for this fixture set. |
| `view_reflects_updated_position_after_step_advancement` | **MERGE** → `view_after_progressing_shows_updated_position` | Near-duplicate of `view_after_progressing_shows_updated_position`. Both advance one step and assert `step 2/3` and `Next:`. Only difference: fixture environment (`wtt-alpha` vs `test-workflow-1/2`). Keep the operations-env version (which uses the more standard `test-workflow-*` fixtures shared with other tests); merge the assertion pattern into it and drop this one. |
| `view_displays_queue_status_with_current_position` | **MERGE** → `view_queue_shows_current_position_and_next_step` | Near-duplicate of `view_queue_shows_current_position_and_next_step`. Tests the same initial-view behavior with `workflow_operations_env`. The only unique assertion is `workflow 1/3` (vs `step 1/3` only in test 1). Merge: add `assert!(output.contains("workflow 1/3"))` to test 1 and delete this test, OR make one canonical 3-workflow initial-view test covering both display strings. |
| `view_with_invalid_session_id_returns_error` | **KEEP** | Unique error-path coverage. Verifies that an unknown session ID returns `ERROR`, echoes the invalid ID, and says `"doesn't exist"`. Load-bearing: error messages silently becoming generic or missing the session ID would go undetected without this test. |
| `view_after_progressing_shows_updated_position` | **KEEP** (absorbs merged `view_reflects_updated_position_after_step_advancement`) | Tests position advancement after one workflow step. Unique behavioral contract: position counter increments and `Next:` section appears. Keep this version; absorb the `wtt-*` version's assertions if they differ. |
| `view_via_workflow_tools_entry_point` | **REMOVE** | Asserts only `!ERROR`, `Queue:`, and `Position:` — a strict subset of what `view_displays_queue_status_with_current_position` (or the merged successor) already asserts. Contributes zero unique coverage. The name implies it's testing a different entry point, but the `invoke_workflow` helper is the same in all tests. Remove. |
| `view_without_session_id_throws_helpful_error` | **KEEP** | Unique and load-bearing. Tests the `view=true` + no `sessionId` parameter combination. Verifies the specific error message: `"Invalid parameter combination"`, `"view=true"`, `"sessionId"`. This is a regression guard for a named parameter-validation contract (T-15.14 / FR-8). |

---

## Recommendations

1. **Consolidate the two initial-view tests** (`view_queue_shows_current_position_and_next_step` + `view_displays_queue_status_with_current_position`) into a single test that asserts both `step 1/3` and `workflow 1/3`, uses a consistent fixture, and covers all three workflow names. The fixture-environment split (`wtt-*` vs `test-workflow-*`) does not cover any distinct code path — it is accidental duplication from the support module having two setup helpers.

2. **Consolidate the two advancement tests** (`view_reflects_updated_position_after_step_advancement` + `view_after_progressing_shows_updated_position`) into one. Both advance a single step and assert identical output tokens. The only variation is fixture names — not a meaningful behavioral distinction.

3. **Delete `view_via_workflow_tools_entry_point`** unconditionally. It has no unique assertions and no rationale for existing alongside the other view tests. The comment "via workflow tool" suggests it was testing a specific invocation path, but the invocation is `invoke_workflow(...)` in all seven tests.

4. **Add a structural assertion to the advancement tests**: currently they only assert `step 2/3` and `Next:` appear somewhere in the output. A stronger test would also assert that the *previous* step label is no longer the "current" step — i.e., the view is not just accumulating output but correctly tracking position. Example: `assert!(!output.contains("step 1/3") || output.contains("step 2/3"), ...)`.

5. **Question the `TEST_MUTEX` serialization strategy**: all 7 tests acquire the same global mutex. This serializes the entire file, defeating any parallelism benefit. If the mutex is needed due to global `Config` state (as in the decay tests), that coupling should be documented. If each test truly uses an isolated `TempDir` + `Config::override_root()`, verify whether the mutex is *actually* needed or is cargo-cult protection from earlier non-isolated implementations.

6. **Assert the `view_with_invalid_session_id_returns_error` message more tightly**: currently asserts `output.contains("doesn't exist")` — consider asserting the full expected phrase (e.g., `"Session 'workflow-nonexistent-12345' doesn't exist"`) to catch regressions in message formatting that might still contain the substring `"doesn't exist"`.

---

## Load-Bearing Score: 4/7

Tests covering unique behavioral contracts: `view_queue_shows_current_position_and_next_step` (initial display), `view_with_invalid_session_id_returns_error` (error path), `view_after_progressing_shows_updated_position` (advancement), and `view_without_session_id_throws_helpful_error` (parameter validation error). The remaining three (`view_reflects_updated_position_after_step_advancement`, `view_displays_queue_status_with_current_position`, `view_via_workflow_tools_entry_point`) cover no distinct code paths and can be removed without any loss of behavioral coverage.
