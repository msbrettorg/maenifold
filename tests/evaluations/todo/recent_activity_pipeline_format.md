# Test Evaluation: recent_activity_pipeline_format
**File:** `crates/maenifold-core/tests/recent_activity_pipeline_format.rs`
**Test count:** 8
**Evaluated:** 2026-03-14
**Traceability:** T-2.3 (FR-4, RTM-025 through RTM-027)

---

## Summary

This file tests the `format_recent_activity_report` formatting pipeline — a pure function that converts `RecentActivityRecord` structs into markdown output. Because format IS the C# parity contract here, string-contains assertions are more defensible than in tool tests. However, the tests collectively assert that **labels are present** without asserting that the **values those labels introduce are correct**. No test checks that "Thoughts: N" contains the right N, that "Sections: N" counts correctly, that timestamps are formatted, or that snippet content is faithfully extracted. Four tests are also parameterizable into two. The suite provides basic shape coverage but would allow silent value regressions to ship.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `format_activity_report_empty_results_returns_no_activity_message` | KEEP | Load-bearing: the empty-result path is tested with an exact string equality assertion (`assert_eq!`), which is the correct strength for this case. If someone silently changes the empty message, MCP clients parsing this output will see unexpected content. The only question is whether exact equality is too strict vs. `contains` — for a boundary/parity test, exact is correct. |
| `format_activity_report_sequential_entry_includes_expected_fields` | KEEP (strengthen) | Behavioral: verifies session-id extraction from path, type classification (`(sequential)`), presence of `Thoughts:` and `Status:` labels. These are real C# parity assertions. **Weakness**: `assert!(output.contains("Thoughts:"))` checks the label exists but not the value. The fixture has 2 thoughts — the test should assert `"Thoughts: 2"`. A missing value regression (e.g., `"Thoughts: 0"`) would pass. |
| `format_activity_report_sequential_entry_include_content_true_shows_h2_content_fields` | MERGE with `..._include_content_false_shows_snippet` | The `include_content=true` and `include_content=false` cases for the same entry type are parameterizable. Both tests create a sequential fixture, differ only in one boolean, and assert different but complementary output fields. Collapse into one parameterized test. **Weakness**: neither test verifies that the H2 content or snippet text is correct — only that the label appears (e.g., `"First H2 Content:"` but not `"First H2 Content: First prose."`). |
| `format_activity_report_sequential_entry_include_content_false_shows_snippet` | MERGE (see above) | Counterpart to the test above. The `!output.contains("First H2:")` negative assertion is the only unique assertion here. On its own it verifies that `include_content=false` suppresses the H2-detail labels — this is real. Merge the two into one parameterized test with a table of `(include_content, expected_present, expected_absent)`. |
| `format_activity_report_workflow_entry_includes_steps_and_status` | KEEP (strengthen) | Tests workflow path classification and step counting label. Real behavioral assertions: session-id extraction, `(workflow)` type label, `Steps:`, `Status: active`. **Weakness**: same as the sequential test — `"Steps:"` is present but step count value is not asserted. The fixture has one "Step 1: Init" and one "Response" section. The implementation counts steps via headings containing "Step" or "Response" — assert `"Steps: 2"` to catch a counting regression. |
| `format_activity_report_memory_entry_includes_title_and_sections` | KEEP (strengthen) | Tests memory path classification. Verifies `(memory)`, `Title: Memory Format Test`, `Sections:`. The `Title:` assertion includes the actual title value — this is good. **Weakness**: `"Sections:"` label is checked but not the count. The fixture has 2 sections; assert `"Sections: 2"`. |
| `format_activity_report_memory_entry_include_content_true_shows_h2_fields` | MERGE with `..._memory_entry_includes_title_and_sections` | Same pattern as the sequential merge pair: two tests for the same entry type differing only on `include_content`. The `include_content=true` case also uniquely asserts `"Last H2: Last Section"` (multi-section Last rendering). **Strengthen**: assert `"First H2 Content: First section content."` (not just the label) to verify actual content extraction. |
| `format_activity_report_multiple_entries_all_entities_present` | KEEP | Load-bearing multi-entry rendering test. Verifies that two entries from different types (`sequential` and `memory`) both appear correctly in combined output, with type labels preserved. This is the only test that exercises the multi-entry join path. However, it does not verify ordering (most-recent first) or the separator between entries. |

---

## Recommendations

1. **Strengthen count assertions across all tests**: Every test that asserts `"Thoughts:"`, `"Steps:"`, or `"Sections:"` should assert the full `"Thoughts: N"` string, not just the label. The fixture content is controlled, so counts are deterministic. The current tests are blind to counting bugs.

2. **MERGE the two `include_content` test pairs into parameterized tests**:
   - Merge `format_activity_report_sequential_entry_include_content_true_shows_h2_content_fields` + `format_activity_report_sequential_entry_include_content_false_shows_snippet` into one test with a `for (include_content, expected, unexpected) in [...]` loop.
   - Merge `format_activity_report_memory_entry_include_content_true_shows_h2_fields` + `format_activity_report_memory_entry_includes_title_and_sections` similarly.
   - This reduces 4 tests to 2 while making the boolean contrast explicit and reducing fixture duplication.

3. **Assert content values, not just labels**: After merging, the surviving tests should check that H2 content fields contain the actual fixture content. For example: `assert!(output.contains("First H2 Content: First prose."))` rather than just `assert!(output.contains("First H2 Content:"))`. This closes the gap where a formatter emitting empty values after the label would pass all current tests.

4. **Add a timestamp formatting assertion**: No test verifies that `"Modified: 2026-02-18 10:00"` appears (the formatter converts `"2026-02-18 10:00:00"` to `"2026-02-18 10:00"`). The `format_recent_activity_timestamp` function has three parsing branches — none of them are tested through these pipeline-level tests. Add at least one `output.contains("Modified: 2026-02-18 10:00")` assertion to the existing tests.

5. **Add ordering assertion to `format_activity_report_multiple_entries_all_entities_present`**: The multi-entry test verifies presence but not position. If the formatter reorders entries (e.g., alphabetically rather than by recency), the test passes. Assert that `session-multi-a` appears before `note-multi-b` in the output, since it has a later timestamp in the fixture.

6. **Add a `Thinking` type path test**: The `recent_activity_format.rs` source has a `Thinking` variant (`memory://thinking/` prefix without `/sequential/` or `/workflow/`). None of the eight tests exercise this code path — it falls through to `format_memory_entry` but with `file_type.as_str()` = `"thinking"` rather than `"memory"`. Add a test with a `memory://thinking/` path to verify type detection and label emission.

---

## Load-Bearing Score: 5/8

The empty-result test (1), the multi-entry test (8), and the workflow/memory entry tests (5, 6) are load-bearing for their respective paths. The two `include_content` pairs (tests 3/4 and tests 6/7) are partially redundant and mergeable. Across all eight tests, the systematic absence of **value assertions** (counts, content, timestamps) means a formatter that emits all the right labels with wrong values would achieve a full green suite.

---

## SWE Remediation

- KEEP `format_activity_report_empty_results_returns_no_activity_message`: unchanged
- STRENGTHEN `format_activity_report_sequential_entry_includes_expected_fields`: assert `"Thoughts: 2"` (not just label), added `"Modified: 2026-02-18 10:00"` timestamp assertion (Rec #1, #4)
- MERGE `format_activity_report_sequential_entry_include_content_true_shows_h2_content_fields` + `format_activity_report_sequential_entry_include_content_false_shows_snippet` → `format_activity_report_sequential_entry_include_content_toggle`: parameterized with `(include_content, expected_present, expected_absent)` table; asserts actual content values like `"First H2 Content: First prose."` (Rec #2, #3)
- STRENGTHEN `format_activity_report_workflow_entry_includes_steps_and_status`: assert `"Steps: 2"` (not just label), added timestamp assertion (Rec #1, #4)
- MERGE `format_activity_report_memory_entry_includes_title_and_sections` + `format_activity_report_memory_entry_include_content_true_shows_h2_fields` → `format_activity_report_memory_entry_include_content_toggle`: parameterized; asserts `"Sections: 2"`, actual H2 content values like `"First H2 Content: Overview content."` (Rec #1, #2, #3)
- STRENGTHEN `format_activity_report_multiple_entries_all_entities_present`: added positional ordering assertion (`pos_a < pos_b`) (Rec #5)
- ADD `format_activity_report_thinking_entry_classified_as_thinking`: new test for `memory://thinking/` prefix (without `/sequential/` or `/workflow/`), verifies `(thinking)` label, title, section count, and timestamp (Rec #6)
- Test count: 8 → 7 (4 merged into 2, 1 added)
- Commit: 0bd2c27

---
## Red-team Audit
- ✅ KEEP `format_activity_report_empty_results_returns_no_activity_message`: unchanged, `assert_eq!` preserved
- ✅ STRENGTHEN `format_activity_report_sequential_entry_includes_expected_fields`: `"Thoughts: 2"` asserted (not just label), timestamp `"Modified: 2026-02-18 10:00"` added (Rec #1, #4)
- ✅ MERGE `sequential_entry_include_content_true` + `sequential_entry_include_content_false` → `sequential_entry_include_content_toggle`: parameterized with `(bool, &[&str], &[&str])` table, content values asserted (`"First H2 Content: First prose."`, `"Last H2 Content: Third prose."`), negative assertions preserved, count `"Thoughts: 3"` in both branches (Rec #2, #3)
- ✅ STRENGTHEN `format_activity_report_workflow_entry_includes_steps_and_status`: `"Steps: 2"` asserted, timestamp added (Rec #1, #4)
- ✅ MERGE `memory_entry_includes_title_and_sections` + `memory_entry_include_content_true_shows_h2_fields` → `memory_entry_include_content_toggle`: parameterized, `"Sections: 2"` asserted, content values asserted (`"First H2 Content: Overview content."`, `"Last H2 Content: Detailed content here."`) (Rec #1, #2, #3)
- ✅ STRENGTHEN `format_activity_report_multiple_entries_all_entities_present`: positional ordering assertion via `output.find()` positions, `pos_a < pos_b` (Rec #5)
- ✅ ADD `format_activity_report_thinking_entry_classified_as_thinking`: `memory://thinking/` prefix (no `/sequential/` or `/workflow/`), asserts `(thinking)` label, Title, `Sections: 2`, timestamp (Rec #6)
- ✅ Test count verified: 7 `#[test]` functions, 7/7 pass (`cargo test -p maenifold-core recent_activity_pipeline_format`)
- ✅ No unrelated changes: commit 0bd2c27 touches only the test file (1 file, 163 insertions, 69 deletions)
- ✅ All 6 recommendations addressed: count assertions (Rec #1), merge pairs (Rec #2), content values (Rec #3), timestamps (Rec #4), ordering (Rec #5), thinking path (Rec #6)
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- recent_activity_pipeline_format`: 7 passed, 0 failed
- Verdict: PASS
