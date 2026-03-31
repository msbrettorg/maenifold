---
id: date_detection_val_tests
status: done
---

# Test Evaluation: date_detection_val_tests

**File:** `crates/maenifold-core/tests/date_detection_val_tests.rs`
**Test count:** 11
**Evaluated:** 2026-07-10
**Traceability:** T-2.3 (RTM FR-4)

---

## Summary

A well-scoped, regression-focused file targeting two pure functions (`is_valid_session_id_format`, `recent_activity_extract_session_id`). Nine of eleven tests are genuinely load-bearing. Two tests have correctness issues: `custom_prefix_returns_true` is parameterizable-away, and `does_not_return_year_as_session_id` carries a redundant `assert_ne!` that adds noise without adding coverage. The entire file could be compressed to two parameterized tests without losing any signal.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `is_valid_session_id_format_valid_sequential_session_id_returns_true` | KEEP | Canonical happy-path for the most common session ID format. Load-bearing anchor for the validation function. |
| `is_valid_session_id_format_valid_workflow_session_id_returns_true` | MERGE | Tests `"workflow-<timestamp>"` (no random suffix). The production function ignores the prefix entirely and only validates `split('-').nth(1)`. The *only* behavioral difference from test 1 is the absence of a third segment — but that doesn't affect `nth(1)`. Merge into test 1 as a parameterized row, or keep if documenting that two-segment IDs are explicitly valid. Current isolation is low-value. |
| `is_valid_session_id_format_non_numeric_timestamp_returns_false` | KEEP | Covers the rejection path when the timestamp segment is alpha. Unique and necessary. |
| `is_valid_session_id_format_no_dash_returns_false` | KEEP | Exercises `nth(1) == None` path. Distinct code path from non-numeric segment. |
| `is_valid_session_id_format_trailing_dash_only_returns_false` | KEEP | Exercises `segment.is_empty()` filter — a distinct branch from `None` and from non-numeric. Without this test, removing the `!segment.is_empty()` guard would not be caught by tests 3 or 4. |
| `is_valid_session_id_format_custom_prefix_returns_true` | MERGE | Tests `"custom-1234567890"` and documents that arbitrary prefixes are valid. The production function is prefix-agnostic by construction (only checks `nth(1)`). This is a specification test, not a regression guard — no realistic refactor would add prefix validation without first being caught by tests 1 and 2. Merge into a single parameterized `is_valid` happy-path test covering `session-*`, `workflow-*`, and `custom-*`. |
| `is_valid_session_id_format_zero_timestamp_returns_true` | KEEP | Edge case: `"0"` is a valid `i64` parse result. Important guard if anyone adds `segment.parse::<i64>().ok().filter(|v| *v > 0)` — this test would immediately fail. Unique and genuinely load-bearing. |
| `extract_session_id_sequential_path_with_date_dirs_returns_session_id` | KEEP | Primary regression for date-nested path extraction: `memory://thinking/sequential/2026/02/12/session-…md`. Exercises the real path structure. |
| `extract_session_id_workflow_path_with_date_dirs_returns_session_id` | MERGE | Same code path as test 8 (`trim_start_matches + split('/').last + trim_end_matches`). The session type (`sequential` vs `workflow`) is invisible to the extractor. The only behavioral difference is the filename prefix, which doesn't affect extraction logic. Merge into a parameterized extraction table alongside test 8. |
| `extract_session_id_path_without_memory_prefix_returns_session_id` | KEEP | Tests the no-prefix case where `trim_start_matches("memory://")` is a no-op. Distinct code path; if someone adds a mandatory-prefix check, only this test fails. Load-bearing. |
| `extract_session_id_path_with_date_dirs_does_not_return_year_as_session_id` | REWRITE | Contains `assert_ne!(result, "2026")` immediately followed by `assert_eq!(result, "session-123-456")`. The `assert_ne` is **strictly redundant** — if the second assertion passes, the value is definitionally not `"2026"`. The `assert_ne` conveys intent (this is a regression against a historical bug) but adds zero logical coverage. Remove the `assert_ne` and document the regression context in a comment above the `assert_eq`. Alternatively rename to `extract_session_id_returns_filename_not_directory_component` and keep only the positive assertion. |

---

## Recommendations

1. **Remove redundant `assert_ne!` in test 11** — replace with a comment `// Regression: previously returned year directory "2026" instead of filename`. The intent is documented; the redundant assertion is noise.
2. **Parameterize the `is_valid_session_id_format` happy-path tests** (1, 2, 6) into one `#[rstest]` table with inputs `["session-1770910549160-79869", "workflow-1770910549160", "custom-1234567890"]`. All three assert `true`; there's no reason for three separate functions.
3. **Parameterize the extraction tests** (8 and 9) into one table with `(file_path, expected_session_id)` rows. The extractor is path-type-agnostic; two tests for sequential vs workflow only documents what the function already doesn't care about.
4. **Naming nit** — the file is called `date_detection_val_tests` but tests session-ID format validation and path extraction, neither of which involves date detection. Consider renaming to `session_id_validation_tests` or similar. The misleading name makes it harder to discover these tests during debugging.
5. **Cover one missing negative extraction case** — what does `extract_session_id` return for an empty string, or a path with no `/` separator? Currently untested. The production code calls `.unwrap_or(file_path)` which would return the full input — is that intended? Add a test for this edge.

---

## Load-Bearing Score: 9/11

Tests 1, 3, 4, 5, 7, 8, 10, 11 (assertion rewrite needed) are clearly load-bearing. Tests 2 and 6 are mergeable with test 1 without losing coverage. Test 9 is mergeable with test 8.
