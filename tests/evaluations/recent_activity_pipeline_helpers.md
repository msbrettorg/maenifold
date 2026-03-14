# Test Evaluation: recent_activity_pipeline_helpers

**File:** `crates/maenifold-core/tests/recent_activity_pipeline_helpers.rs`
**Traceability:** T-2.3 / FR-4 / RTM-025 / RTM-029
**Test count:** 13
**Evaluated:** 2026-07-15

## Summary

Seven of thirteen tests are genuinely load-bearing with correct assertions. Six are either assertion-weak (passing when the interesting behavior is broken), redundant across uniform code paths begging for parameterization, or testing the wrong granularity. The `extract_snippet` tests are mostly solid; the session-id and file-type classifier tests are seven tests doing the work of two parameterized ones.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `extract_snippet_empty_input_returns_empty_string` | **KEEP** | Correct boundary. Empty input → empty output is a real edge: the impl filters/joins lines and `"".chars().count() <= 0` guard could be absent. Would catch a regression. |
| `extract_snippet_short_content_returns_full_content` | **KEEP** | Verifies passthrough identity when content fits within max_length. Distinct from the truncation path. |
| `extract_snippet_long_content_truncates_with_ellipsis` | **KEEP** | Tests both the exact char count and the ellipsis suffix. Correct assertions. |
| `extract_snippet_bullet_only_content_falls_back_to_all_lines` | **REWRITE** | `!result.is_empty()` + `result.contains("Bullet")` are too weak. If the fallback returns only the first bullet, both assertions pass. The test should assert that ALL lines are present (e.g., all three bullet values appear in the result), pinning the "fall back to all lines" contract, not just "some bullet content exists". |
| `extract_snippet_mixed_content_skips_bullet_lines` | **KEEP** | Both a positive (`contains("prose content")`) and negative (`!contains("Metadata bullet")`) assertion. Correctly specifies the filter behavior. |
| `extract_snippet_uses_default_length_when_minus_one` | **REWRITE** | Asserts `<= 1000` — this is trivially true if the function returns an empty string. The input is 2,000 chars with no bullets, so the expected output is exactly 1,000 chars (997 + "…"). Should be `assert_eq!(result.chars().count(), 1000)`. The current assertion cannot distinguish a correct implementation from one that returns "". |
| `extract_session_id_standard_sequential_path_extracts_id` | **MERGE** | All three `extract_session_id_*` tests exercise the same single-branch implementation: `trim_start_matches("memory://")`, `split('/')`, `next_back()`, `trim_end_matches(".md")`. No conditional logic varies between tests. Collapse into one `#[test_case]` / `rstest` parameterized test with three (input, expected) rows. |
| `extract_session_id_simple_memory_path_extracts_filename` | **MERGE** | See above. |
| `extract_session_id_chat_path_extracts_filename` | **MERGE** | See above. |
| `determine_file_type_sequential_path_returns_sequential` | **MERGE** | All four `determine_file_type_*` tests drive a pure string-match classifier with a single `if/else if/else` chain. No unique code path is exercised by having four separate functions vs. one parameterized test. Collapse into one `#[test_case]` / `rstest` with four (path, expected_type) rows. Also missing: a path starting with `memory://thinking/` that is NOT sequential or workflow (would return "thinking", not "memory") — the current suite has no coverage of the `Thinking` variant at all. |
| `determine_file_type_workflow_path_returns_workflow` | **MERGE** | See above. |
| `determine_file_type_memory_path_returns_memory` | **MERGE** | See above. |
| `determine_file_type_chat_path_returns_memory` | **MERGE** | See above. Also the only one testing the `else` branch, making it the most unique of the four — but that uniqueness doesn't require a separate function. |

## Recommendations

1. **`extract_snippet_bullet_only_content_falls_back_to_all_lines`** — Replace both weak assertions with: `assert!(result.contains("Bullet one"))`, `assert!(result.contains("Bullet two"))`, `assert!(result.contains("Bullet three"))`. This pins the "all lines included" contract.

2. **`extract_snippet_uses_default_length_when_minus_one`** — Change `assert!(result.chars().count() <= 1000, ...)` to `assert_eq!(result.chars().count(), 1000)`. The current inequality assertion accepts an empty string as valid output.

3. **Collapse `extract_session_id_*` (3 tests → 1)** using `rstest` or `test_case`:
   ```rust
   #[rstest]
   #[case("memory://thinking/sequential/2026/02/session-12345.md", "session-12345")]
   #[case("memory://docs/my-note.md", "my-note")]
   #[case("memory://chat/2026/02/chat-session.md", "chat-session")]
   fn extract_session_id_extracts_filename_stem(#[case] path: &str, #[case] expected: &str) { ... }
   ```

4. **Collapse `determine_file_type_*` (4 tests → 1)** and add the missing `Thinking` variant case:
   ```rust
   #[rstest]
   #[case("memory://thinking/sequential/2026/02/session.md", "sequential")]
   #[case("memory://thinking/workflow/2026/02/wf.md", "workflow")]
   #[case("memory://thinking/my-note.md", "thinking")]   // ← MISSING CASE
   #[case("memory://docs/my-note.md", "memory")]
   #[case("memory://chat/2026/02/assumptions.md", "memory")]
   fn determine_file_type_classifies_path_correctly(#[case] path: &str, #[case] expected: &str) { ... }
   ```
   Note: the `Thinking` variant (`memory://thinking/` but not sequential/workflow) is **entirely absent** from the current 13-test suite.

5. No tests cover the `thinking` classification path at all — this is a gap in FR-4 / RTM-029 coverage.

## Load-Bearing Score: 7/13

Tests 1, 2, 3, 5 are correct and load-bearing. Tests 10–13 (the classifier tests) are load-bearing in terms of behavior coverage but not in terms of test count — the same coverage can be achieved with 2 parameterized tests instead of 7 separate functions. Tests 4 and 6 are load-bearing in intent but their assertions are too weak to catch real regressions.
