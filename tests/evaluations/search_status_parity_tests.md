# Test Evaluation: search_status_parity_tests

**File:** `crates/maenifold-core/tests/search_status_parity_tests.rs`
**Test count:** 12
**Evaluated:** 2026-03-14
**Traceability:** T-13.7 (FR-6 search mode parity), T-13.8 (FR-7 MemoryStatus envelope, FR-8 hint wiring)

## Summary

Seven of twelve tests are genuinely load-bearing; the remaining five either carry vacuous conditional assertions, test redundant paths, or could be collapsed into parameterized tests without losing coverage. The biggest structural defect is `invalid_mode_json_reports_hybrid_envelope`, which silently passes when the result set is empty — precisely the case most likely to occur if the code is broken.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `invalid_mode_falls_through_to_hybrid` | **REWRITE** | The `result.contains("Search Results") \|\| result.contains("Hybrid Search")` assertion is a disjunction that accepts either string — if the heading label changes tomorrow, one half of the OR keeps the test green. More critically, this test doesn't assert *which* documents were found, only that *some* results appear. Rewrite to: (a) assert exactly one of the two heading variants to pin the expected behaviour, and (b) verify the seeded documents appear in results so a silent empty-results case doesn't masquerade as success. |
| `invalid_mode_json_reports_hybrid_envelope` | **REWRITE** | The entire assertion body is wrapped in `if !results.is_empty()`. If the code under test regresses and returns zero results, the test passes vacuously — exactly the wrong behaviour for a regression guard. The name promises "reports hybrid envelope" but never checks the `mode` field in the JSON envelope. Rewrite to: assert `results` is non-empty unconditionally, *then* check `fusedScore` presence and the `mode` field value. |
| `fulltext_mode_returns_nonempty_results` | **MERGE** with `hybrid_mode_returns_nonempty_results` | Both tests: seed identical data, call `search_json`, assert `!results.is_empty()`. The only difference is the mode string. Collapse into one parameterized test over `["FullText", "Hybrid"]`. |
| `hybrid_mode_returns_nonempty_results` | **MERGE** with `fulltext_mode_returns_nonempty_results` | See above. As a standalone test it provides zero structural coverage beyond the FullText variant. |
| `semantic_mode_degrades_gracefully_without_model` | **KEEP** | Unique scenario: exercises the ONNX-absent code path. Correctly asserts `mode="Semantic"` in the JSON envelope (behaviour, not just label — this field drives client-side branching). Asserts `results` is array, tolerating 0 entries. Load-bearing for the degraded-mode contract. |
| `memory_status_json_wraps_in_success_envelope` | **MERGE** with `memory_status_json_preserves_graph_section` | Both invoke `MemoryStatus` with an identical environment, parse the same JSON response, and assert on different keys in `json["data"]`. Running the same expensive tool call twice in separate tests for non-conflicting assertions is wasteful and brittle. Merge into a single comprehensive test that asserts the full envelope shape in one pass. |
| `memory_status_json_preserves_graph_section` | **MERGE** with `memory_status_json_wraps_in_success_envelope` | See above. The section-level assertions here (`### Graph Database`, `- Concepts:`) are purely additive to the above test. Merging eliminates duplication without losing a single assertion. |
| `search_results_contain_next_steps_hint` | **KEEP** | The only test verifying FR-8 wiring into the Hybrid search path. Asserts three specific string anchors: the emoji, "ReadMemory", and "BuildContext". These are load-bearing because they catch the case where the hint is wired to the wrong tool names or missing entirely. Also validates structural position (hint is last line) — correctly checking *format* not just *presence*. |
| `fulltext_search_results_contain_next_steps_hint` | **REWRITE** | Currently only asserts `result.contains("💡 Next: Consider using")` — a strict subset of what `search_results_contain_next_steps_hint` already asserts for Hybrid. The unique claim (FullText *also* gets the hint) is valid, but the test should at minimum also assert "ReadMemory" and "BuildContext" to prevent a future regression where FullText gets a different/empty hint text. Consider merging with the Hybrid variant into a parameterized mode-coverage test. |
| `write_memory_contains_hint` | **KEEP** | Unique: exercises FR-8 wiring into the WriteMemory path. Asserts creation succeeded, hint emoji present, "ReadMemory" present, and `GetHelp("WriteMemory")` present. The `GetHelp` assertion is legitimately load-bearing — it catches the case where the tool name in the hint is wrong. |
| `search_next_steps_does_not_corrupt_results` | **KEEP** | Load-bearing structural integrity test. Verifies the seeded document title survives in the output above the hint, and that the hint is the *last* line (not interleaved). This catches the specific regression where `with_next_steps` prepends instead of appends, or corrupts the result body. |
| `write_memory_hint_does_not_corrupt_content` | **KEEP** | Mirrors `search_next_steps_does_not_corrupt_results` for the WriteMemory path. Asserts URI (`Created memory FILE: memory://`) and checksum survive, hint is last line, and URI is first line. All four assertions are structurally distinct and non-redundant. Load-bearing. |

## Recommendations

1. **Fix the vacuous assertion in `invalid_mode_json_reports_hybrid_envelope` immediately.** The `if !results.is_empty()` guard inverts the purpose of the test. A broken implementation returning 0 results silently passes. Remove the guard; assert non-empty unconditionally.

2. **Parameterize the mode-returns-nonempty pair.** `fulltext_mode_returns_nonempty_results` and `hybrid_mode_returns_nonempty_results` are character-for-character identical modulo the mode string. Use a helper or a `for mode in ["FullText", "Hybrid"]` loop inside a single test.

3. **Merge the two MemoryStatus JSON tests.** They share setup, tool invocation, and JSON parsing overhead. A single `memory_status_json_output_shape` test with all assertions is cleaner and faster.

4. **Strengthen `fulltext_search_results_contain_next_steps_hint`.** It currently asserts only the emoji string — add "ReadMemory" and "BuildContext" assertions to match the Hybrid variant's rigour.

5. **Tighten `invalid_mode_falls_through_to_hybrid`.** The `result.contains("Search Results") || result.contains("Hybrid Search")` disjunction is too permissive. Pin the expected heading to whichever string the implementation actually emits, and add an assertion that the seeded document title appears in the output.

6. **Add a `mode` field assertion to `invalid_mode_json_reports_hybrid_envelope`.** The test name promises the envelope reports something about mode, but no assertion verifies the `data["mode"]` field. Either the field should equal `"Hybrid"` or this behaviour should be explicitly documented as unspecified.

## Load-Bearing Score: 7/12

Five tests either have vacuous/missing assertions (`invalid_mode_json_reports_hybrid_envelope`), are pure duplicates collapable without coverage loss (`hybrid_mode_returns_nonempty_results`, `memory_status_json_preserves_graph_section`), or assert a strict subset of a sibling test (`fulltext_search_results_contain_next_steps_hint`). Seven tests would catch real regressions if removed.
