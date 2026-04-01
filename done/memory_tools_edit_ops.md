---
id: T-80.107
title: "Test Evaluation: memory_tools_edit_ops"
status: done
pipeline: tdd
---

# Test Evaluation: memory_tools_edit_ops

**File:** `crates/maenifold-core/tests/memory_tools_edit_ops.rs`
**Test count:** 9
**Traceability:** T-12.6 / RTM NFR-1
**Evaluated:** 2026-07-14

---

## Summary

This suite tests the `find_replace`, `append`, and `prepend` operations for `edit_memory`, with particular focus on the `expectedCount` guard. The business logic under test is genuinely important and most tests are load-bearing. However, 5 of 9 tests assert a string output label (`"Updated memory FILE:"`) as a *primary* fast-path check — a fragile proxy for success that would survive a rename without catching it. Three error-path tests are strong candidates for parameterization since they differ only in input values.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `find_replace_with_correct_expected_count_succeeds` | **KEEP** | Load-bearing: verifies actual content mutation and that `expectedCount=N` with correct N permits the replace. The `starts_with("Updated memory FILE:")` assertion is redundant given the read-back, but not harmful. |
| `find_replace_with_incorrect_expected_count_throws_argument_error` | **MERGE** → parameterize with tests 3 & 5 | Error message assertions are correct (checking error text, not just `is_err()`), but this test differs from tests 3 and 5 only in the numeric values of `expectedCount` and actual match count. |
| `find_replace_with_expected_count_zero_throws_when_matches_found` | **MERGE** → parameterize with tests 2 & 5 | Unique edge: `expectedCount=0` when 1 match exists. Same pattern as test 2 — a table-driven test covering `(expected=0, actual=1)`, `(expected=1, actual=3)`, `(expected=2, actual=0)` would be cleaner and catch gaps. |
| `find_replace_with_null_expected_count_succeeds` | **KEEP** | Distinct semantic from test 1 — `null` means "replace all without count validation", not "expect N". This is a different code path and worth a dedicated test. |
| `find_replace_with_no_matches_throws_when_expecting_matches` | **MERGE** → parameterize with tests 2 & 3 | Third variant of the expectedCount mismatch pattern. Zero matches vs non-zero expected. Merge into parameterized table. |
| `find_replace_with_multiline_pattern_works_correctly` | **REWRITE** | **Name is wrong.** The needle `"paragraph"` is a single-word term, not a multiline pattern. The test verifies replacement occurs across distinct paragraphs in the document body, not that a needle spans newlines. Rename to `find_replace_replaces_all_occurrences_across_paragraphs`. Also: this test is essentially a second success-path test identical in structure to test 1. Consider whether it adds coverage test 1 doesn't already provide. |
| `append_with_concepts_succeeds` | **MERGE** → parameterize with test 8 | Tests `append` op. Identical structure to `prepend_with_concepts_succeeds`. Collapse into one parameterized test: `edit_memory_succeeds_for_op(op: &str)` with ops `["append", "prepend"]`. |
| `prepend_with_concepts_succeeds` | **MERGE** → parameterize with test 7 | Tests `prepend` op. Identical structure to `append_with_concepts_succeeds`. See above. |
| `edit_memory_without_concepts_returns_error` | **KEEP** | Load-bearing: enforces the WikiLink-required business rule. Tests the error path for a constraint that all edit operations share. Cannot be merged without losing clarity. |

---

## Detailed Observations

### 1. String-label assertions are weak proxies

Five tests check `edit_result.starts_with("Updated memory FILE:")` before reading back content. This pattern asserts the *format string* of the success message, not the operation's correctness. It will:
- **Survive** a content mutation bug (passes even if content is wrong, as long as the message prefix survives)
- **Break** on a cosmetic rename of the output message (false negative)

Since every one of these tests then reads back the file and asserts actual content, the `starts_with` check is redundant. It should be removed or replaced with `assert!(edit_result.contains(&uri), ...)` — asserting the returned URI is present, which is a behavioral guarantee.

### 2. Three error-path tests are parameterizable

Tests 2, 3, and 5 share this structure:
```rust
let err = memory::edit_memory(&env.ctx(), &json!({..., "expectedCount": X}))
    .expect_err("...");
assert!(err.to_string().contains("Expected X matches but found Y"));
assert!(err.to_string().contains("Find text: '...'"));
```
They cover `(expected=1, actual=3)`, `(expected=0, actual=1)`, and `(expected=2, actual=0)`. A `#[test_case]` or manually-written loop covering all three rows would shrink this from ~60 lines to ~25 and make adding new edge cases trivial.

### 3. `find_replace_with_multiline_pattern_works_correctly` is misnamed

"Multiline pattern" implies the *search needle* spans newlines (e.g., `"First paragraph\n\nSecond"`). It does not. The test uses `"paragraph"` as the needle. The interesting thing it's actually testing is that `find_replace` correctly replaces a word that appears in structurally different locations in the document. A better name: `find_replace_replaces_single_word_across_document_sections`.

---

## Recommendations

1. **Parameterize tests 2, 3, 5** into a single table-driven test `find_replace_expectedcount_mismatch_errors`. Use explicit `(findText, content, expectedCount, actualCount)` rows.
2. **Parameterize tests 7, 8** into `edit_memory_op_with_concepts_succeeds` with `["append", "prepend"]`.
3. **Remove `starts_with("Updated memory FILE:")` assertions** from tests 1, 4, 6, 7, 8. Replace with `assert!(edit_result.contains(&uri))` to assert behavioral output without coupling to format strings.
4. **Rename test 6** to `find_replace_replaces_word_across_document_sections`.
5. **Add a test** for `replace_section` operation — it's the fourth edit operation and has zero coverage here.
6. **Add a test** for `find_replace` with `allow_multiple_occurrences: false` explicitly set — verify single-occurrence enforcement.

---

## Load-Bearing Score: 7/9

Tests 2, 3, 5 are genuinely load-bearing but redundant with each other — they cover three instances of the same error path. Collapsing them to one parameterized test raises the signal-to-noise ratio without reducing coverage.

---

## SWE Remediation

- KEEP `find_replace_with_correct_expected_count_succeeds`: replaced `starts_with("Updated memory FILE:")` with `contains(&uri)` per P0 R3
- MERGE tests 2, 3, 5 → `find_replace_expectedcount_mismatch_errors`: table-driven with (expected=1,actual=3), (expected=0,actual=1), (expected=2,actual=0) per P0 R1
- KEEP `find_replace_with_null_expected_count_succeeds`: replaced `starts_with("Updated memory FILE:")` with `contains(&uri)` per P0 R3
- REWRITE `find_replace_with_multiline_pattern_works_correctly` → `find_replace_replaces_word_across_document_sections`: renamed per P1 R4, replaced `starts_with` with `contains(&uri)` per P0 R3
- MERGE tests 7, 8 → `edit_memory_op_with_concepts_succeeds`: parameterized over `["append", "prepend"]` per P0 R2, replaced `starts_with` with `contains(&uri)` per P0 R3
- KEEP `edit_memory_without_concepts_returns_error`: unchanged (uses `starts_with("ERROR:")` which is correct for error path)
- Test count: 9 → 6 (3 merges reduced by 3)
- All 6 tests pass
- Commit: 4421659

---

## Red-team Audit

| Verdict | Item | Detail |
|---|---|---|
| ✅ | KEEP `find_replace_with_correct_expected_count_succeeds` | `starts_with("Updated memory FILE:")` replaced with `contains(&uri)` per R3. Read-back assertions preserved. |
| ✅ | MERGE tests 2+3+5 → `find_replace_expectedcount_mismatch_errors` | Table-driven with all 3 rows: `(1,3)`, `(0,1)`, `(2,0)`. Each row asserts both "Expected X matches but found Y" and "Find text: '…'" — matching original assertion structure. No coverage lost. |
| ✅ | KEEP `find_replace_with_null_expected_count_succeeds` | `starts_with` replaced with `contains(&uri)`. Read-back assertion preserved. |
| ✅ | REWRITE test 6 → `find_replace_replaces_word_across_document_sections` | Renamed per R4. Name matches eval Recommendation §4. `starts_with` replaced with `contains(&uri)`. Content/assertions unchanged. |
| ✅ | MERGE tests 7+8 → `edit_memory_op_with_concepts_succeeds` | Parameterized over `["append", "prepend"]`. Each iteration asserts URI in result, original content preserved, and new content present — matching both originals. |
| ✅ | KEEP `edit_memory_without_concepts_returns_error` | Unchanged. `starts_with("ERROR:")` retained correctly — R3 targeted success-path format strings, not error-path assertions. |
| ✅ | Test count 9 → 6 | 3 MERGE ops each collapsed 2–3 tests into 1. Confirmed 6 `#[test]` fns in file. |
| ✅ | All 6 tests pass | `cargo test -p maenifold-core --test integration_tests -- edit_ops` — 6 passed, 0 failed. |
| ✅ | No unrelated changes | Diff only touches test logic specified by eval. No source code modified. No stray files. |

**Observations (non-blocking):**
- R5 (add `replace_section` coverage) and R6 (add `allow_multiple_occurrences: false` test) were not addressed. These are additive coverage recommendations, not KEEP/MERGE/REWRITE verdicts, so their absence is not grounds for rejection. They remain open for a future sprint.
- Commit 4421659 bundles this eval's remediation with `memory_path_nested` remediation under the latter's commit message. Minor traceability hygiene issue — separate eval remediations ideally get separate commits. Non-blocking.

- **Verdict: PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_tools_edit_ops`: 6 passed, 0 failed
- Verdict: PASS
