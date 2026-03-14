# Test Evaluation: memory_tools_editing_parity

**File:** `crates/maenifold-core/tests/memory_tools_editing_parity.rs`
**Test count:** 9
**Evaluated:** 2026-07-14
**Traceability:** T-15.1 (FR-15), T-15.2 (FR-15), T-15.3 (FR-15), T-15.4 (FR-15)

## Summary

The most structurally flawed of the three files. Four tests (3, 4, 5, 6) exist solely to verify case-insensitive operation name parsing, but their assertions stop at the return-value prefix (`"Updated memory FILE:"`) — they never read the file back to confirm the operation was applied. A bug where the operation parses correctly but applies incorrectly would pass all four tests without comment. Tests 1 and 2 are genuinely load-bearing but are parameterizable into a single test. Tests 7, 8, 9 are well-written.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `append_adds_double_newline_separator` | **MERGE** | Load-bearing. Asserts exact `\n\n` separator in read-back content — this is the right assertion. However, structurally identical to `prepend_adds_double_newline_separator`: same setup, same assertion pattern, different operation. Parameterize into one test with two cases (append/prepend). |
| `prepend_adds_double_newline_separator` | **MERGE** | Load-bearing. See above. |
| `operation_uppercase_append_resolves` | **REWRITE** | Only asserts `edit_result.starts_with("Updated memory FILE:")`. This is the same check as the success guard in `write_memory_file`'s internal assertion. It does NOT verify that content was actually appended. A bug where `APPEND` parses correctly but silently no-ops would pass this test. Add a read-back assertion. |
| `operation_titlecase_append_resolves` | **REWRITE + MERGE** | Same defect as test 3. Also: `APPEND` vs `Append` — both are uppercase variants of `append`. These four case-variation tests (3, 4, 5, 6) should be merged into a single parameterized test that (a) tests all case variants and (b) reads back the content to verify the operation applied. |
| `operation_mixed_case_find_replace_resolves` | **REWRITE + MERGE** | Same defect. No read-back. `Find_Replace` passes without verifying "target word" was replaced with `[[replacement]]`. |
| `operation_uppercase_replace_section_resolves` | **REWRITE + MERGE** | Same defect. `REPLACE_SECTION` passes without verifying "My Section" body was updated. All four case tests (3, 4, 5, 6) should be one parameterized test with content verification. |
| `replace_section_partial_heading_match` | **KEEP** | Load-bearing. Verifies: (1) old heading preserved, (2) new content present, (3) old content removed, (4) other sections untouched. Four distinct assertions covering the full expected state. Correctly documents a nuanced behavior. |
| `replace_section_exact_match_still_works` | **KEEP** | Load-bearing as a regression guard. When partial match was added, exact match could have broken. This test documents the invariant that exact match remains the baseline behavior. |
| `replace_section_with_heading_in_new_content_preserves_old_heading` | **KEEP** | Load-bearing. Tests a genuinely surprising edge case — when replacement content starts with its own `##` heading, the old heading must be preserved and the new heading stripped. Without this test, this behavior is invisible in the API contract. |

## Recommendations

1. **Tests 3, 4, 5, 6 are testing operation name parsing, not operation behavior.** The absence of read-back assertions is not a style issue — it is a correctness gap. A fix that parses `APPEND` correctly but appends to the wrong location, uses the wrong separator, or writes the wrong content would pass all four tests. Rewrite as:
   ```rust
   #[test]
   fn case_insensitive_operation_names_apply_correctly() {
       // variants: ("APPEND", expected_append_result), ("Append", ...), etc.
       // For each: write file, apply op with case variant, read back, assert content
   }
   ```

2. **Merge tests 1 and 2** (`append_adds_double_newline_separator` + `prepend_adds_double_newline_separator`) into a single parameterized test. They are byte-for-byte identical in structure, differing only in the operation name and the expected ordering of content.

3. **`operation_mixed_case_find_replace_resolves`**: In particular, `Find_Replace` is a multi-word operation name with an underscore — this is distinct enough from the other two variants that it arguably warrants its own test. If you collapse 3+4+5+6, keep `Find_Replace` as a separate case with its own content verification.

4. **Coverage gap**: None of the case-insensitive tests cover `PREPEND` or `REPLACE_SECTION` with a read-back. Test 6 does check `REPLACE_SECTION` but with no verification. Add read-back for at least one case of each of the 4 operations.

5. **The `edit_result.starts_with("Updated memory FILE:")` pattern** is used as the only assertion in 4 tests. This should be treated as a success-guard precondition (using `assert!` is fine), but it cannot stand alone as the test's only meaningful check.

## Load-Bearing Score: 5/9

Tests 1, 2, 7, 8, 9 are genuinely load-bearing. Tests 3, 4, 5, 6 verify operation-name parsing only — not the actual effect on file content. A regression in the apply logic for any case-normalized operation would be invisible to this suite.

---
## SWE Remediation

- MERGE tests 1+2 (`append_adds_double_newline_separator` + `prepend_adds_double_newline_separator`) → `append_prepend_add_double_newline_separator`: parameterized loop over append/prepend. Fixed pre-existing prepend assertion failure — `write_memory` inserts a `# Title` heading, so prepend places content before the heading, making the original assertion `prepended\n\noriginal` incorrect (the heading sits between them). New assertion verifies ordering + `\n\n` separator after prepended content.
- REWRITE+MERGE tests 3+4+5+6 (`operation_uppercase_append_resolves`, `operation_titlecase_append_resolves`, `operation_mixed_case_find_replace_resolves`, `operation_uppercase_replace_section_resolves`) → `case_insensitive_operation_names_apply_correctly`: all case variants now include read-back content verification per P0. Added PREPEND uppercase variant to fill coverage gap (rec #4). Find_Replace verifies replacement present + original absent. REPLACE_SECTION verifies new body present + old body absent + heading preserved.
- KEEP tests 7+8+9: untouched.
- 9 tests (1 pre-existing failure) → 5 tests (0 failures), load-bearing score 5/5
- Commit: 9aaa5ac
