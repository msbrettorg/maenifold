# Test Evaluation: memory_tools_wikilink_replacements

**File**: `crates/maenifold-core/tests/memory_tools_wikilink_replacements.rs`
**Traceability**: T-12.6 · RTM NFR-1 (C# Oracle Parity)
**Evaluator**: Red Team (🔘 Gray — Skeptical Inquiry)
**Method**: Full read of test file + implementation (`memory/editing.rs`, `markdown/wikilinks.rs`) + overlap analysis against sibling files (`memory_tools_edit_ops.rs`, `memory_tools_editing_parity.rs`, `wikilink_regex_tests.rs`)

---

## Summary

This file contains **4 integration tests** that probe the `find_replace` operation of `EditMemory` specifically from a WikiLink correctness perspective. They exercise three distinct semantic surfaces:

1. **Malformed-bracket detection** — the `contains_malformed_wikilinks` post-operation safety gate (private fn in `editing.rs`, no standalone unit tests).
2. **Whole-WikiLink replacement** — replacing `[[Concept A]]` with `[[Concept B]]` without disturbing adjacent WikiLinks.
3. **Partial-text-within-WikiLink replacement** — `find_replace` where `findText` is a substring of one or more `[[WikiLink]]` inner labels, and the result is still valid WikiLinks.
4. **Multi-WikiLink replacement content** — where the replacement string itself contains several `[[Link]]` patterns.

**Sibling coverage audit**: `memory_tools_edit_ops.rs` (9 tests) covers `find_replace` for `expectedCount` validation, plain-text replacement, multiline patterns, and append/prepend operations — none of which specifically exercise WikiLink bracket correctness. `wikilink_regex_tests.rs` tests extraction but not the editing gate. No overlap was found on any of the four scenarios above.

All 4 tests are **integration-level** (they write to an isolated DB via `isolated_env()`, call `edit_memory`, then read back) and rely on the serialized `TEST_MUTEX`. They are appropriately placed in the integration test layer (NFR-2 boundary tier).

**Skeptical concerns raised and answered**:

- *Are these testing the right layer?* Yes. `contains_malformed_wikilinks` has no unit tests in `editing.rs` — this file is the only coverage for that path under the find_replace operation.
- *Is the error message in Test 1 accurate to the failure cause?* **No.** The implementation emits "must contain at least one [[concept]]" for BOTH the zero-concept case AND the malformed-bracket case (same code path). The test correctly captures the observable behavior, but the conflated message is a documentation debt.
- *Does Test 4 fully verify preservation of the original document?* **No.** The original `[[AI]]` WikiLink is not asserted present after the replacement, leaving a preservation gap.
- *Is the `expectedCount: 4` in Test 3 load-bearing or incidental?* Incidental to the file's purpose — `expectedCount` validation is already covered in `memory_tools_edit_ops.rs`. The WikiLink transformation behavior is the unique claim.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `find_replace_replacing_text_inside_wikilink_creates_nested_brackets` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | 🔵 Low value | **KEEP** |
| 2 | `find_replace_replacing_entire_wikilink_works_correctly` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | 🔵 Low value | **KEEP** |
| 3 | `find_replace_multiple_wikilinks_with_same_inner_text_replaces_all` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Misleading | 🟡 Moderate | **KEEP** (rename) |
| 4 | `find_replace_replacing_with_multiple_wikilinks_works_correctly` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | 🔵 Low value | **KEEP** (add assertion) |

---

## Per-Test Analysis

### Test 1 — `find_replace_replacing_text_inside_wikilink_creates_nested_brackets`

**Scenario**: `findText = "Learning"` targets the inner text of `[[Machine Learning]]`. Replacement content `[[Deep Learning]]` would produce `[[ [[Deep Learning]] ]]` — a nested bracket structure. Implementation's `contains_malformed_wikilinks` should fire and return an error.

**Load-bearing**: **Yes.** This is the only integration test that exercises `contains_malformed_wikilinks` as an editing gate. The function has no unit tests inside `editing.rs`, and no other test file covers this path for the `find_replace` operation.

**Unique**: **Yes.** `memory_tools_edit_ops.rs::edit_memory_without_concepts_returns_error` tests a no-concept `append` (the *pre-operation* check, different code path). This test exercises the *post-operation* malformed check.

**Correct assertions**:
- `edit_result.starts_with("ERROR:")` — ✅ correct boundary assertion
- `edit_result.contains("must contain at least one [[concept]]")` — ⚠️ **factually misleading**. The error fires because `contains_malformed_wikilinks` returned true (nested brackets), not because there are zero concepts. The implementation emits the same message for both conditions (conflation in source). The test is asserting the current observable behavior accurately, but any reader is likely to misinterpret *why* the error fires. This is a source-level issue worth noting separately from test quality.

**Good name**: ✅ Yes. "creates_nested_brackets" clearly states the mechanism under test.

**Recommendation**: Add a brief inline comment: `// Partial-text replacement inside [[WikiLink]] produces nested brackets — caught by contains_malformed_wikilinks post-op gate`. Consider filing a follow-up to distinguish the error message for malformed brackets from the no-concept message.

---

### Test 2 — `find_replace_replacing_entire_wikilink_works_correctly`

**Scenario**: `findText = "[[Machine Learning]]"` (full bracket syntax) replaced with `[[Artificial Intelligence]]`. Verifies the replacement succeeds, the new WikiLink appears, the replaced one is gone, and an adjacent `[[Deep Learning]]` is untouched.

**Load-bearing**: **Yes.** This is the canonical happy path for WikiLink renaming via `find_replace`. No other test file verifies that full-bracket `findText` works, nor that adjacent WikiLinks survive.

**Unique**: **Yes.** The edit_ops tests only use plain-text `findText` values.

**Correct assertions**:
- `edit_result.starts_with("Updated memory FILE:")` — ✅
- `read_result.contains("[[Artificial Intelligence]]")` — ✅ new WikiLink present
- `!read_result.contains("[[Machine Learning]]")` — ✅ replaced WikiLink absent
- `read_result.contains("[[Deep Learning]]")` — ✅ untouched WikiLink preserved

All four assertions are well-chosen. No gaps identified.

**Good name**: ✅ Yes.

**Recommendation**: None. This test is exemplary for the file.

---

### Test 3 — `find_replace_multiple_wikilinks_with_same_inner_text_replaces_all`

**Scenario**: `findText = "Neural Networks"` (a substring shared across 4 WikiLinks: `[[Neural Networks]]`, `[[Convolutional Neural Networks]]`, `[[Recurrent Neural Networks]]`, `[[Neural Networks Architecture]]`). After replacement with `"NN"`, all four become valid WikiLinks with the substring substituted.

**Load-bearing**: **Yes.** This is the only test verifying that `str::replace` propagates correctly *within* WikiLink brackets and that the resulting WikiLinks pass `contains_malformed_wikilinks`. Without this test, a regression that accidentally broke inner-substring replacement would go undetected.

**Unique**: **Yes.** edit_ops tests only replace plain tokens, not tokens embedded in WikiLink bracket syntax.

**Correct assertions**:
- All four transformed WikiLinks asserted present: `[[NN]]`, `[[Convolutional NN]]`, `[[Recurrent NN]]`, `[[NN Architecture]]` — ✅
- `!read_result.contains("Neural Networks")` — ✅ complete replacement confirmed

**Good name**: ⚠️ **Misleading**. The name "multiple_wikilinks_with_same_inner_text" implies the WikiLinks share *identical* inner content. They do not — they share a *substring*. A more precise name: `find_replace_partial_text_within_wikilinks_updates_all_occurrences`.

**Parameterizable**: Moderately. The four assertions checking specific WikiLink forms (`[[NN]]`, `[[Convolutional NN]]`, etc.) could be table-driven if similar tests were added for other substring patterns. Not worth doing in isolation.

**Note on `expectedCount: 4`**: This is not the focus of the test (it's already covered in edit_ops), but it is necessary for the test to pass because the default behavior errors if count mismatches.

**Recommendation**: Rename the function. The concept is clear; only the name needs correction.

---

### Test 4 — `find_replace_replacing_with_multiple_wikilinks_works_correctly`

**Scenario**: `findText = "fundamentals"` (plain text, one occurrence) replaced with `"[[Machine Learning]], [[Deep Learning]], and [[Neural Networks]]"` — a replacement string containing multiple WikiLinks. Verifies all three new WikiLinks appear.

**Load-bearing**: **Yes.** `memory_tools_edit_ops.rs::find_replace_with_correct_expected_count_succeeds` replaces with only `"[[bar]]"` (single WikiLink). This test verifies the multi-WikiLink replacement path is accepted by the post-operation validation gate.

**Unique**: **Yes.** The distinction is the multi-WikiLink replacement string, not tested elsewhere.

**Correct assertions**:
- `edit_result.starts_with("Updated memory FILE:")` — ✅
- Three assertions for `[[Machine Learning]]`, `[[Deep Learning]]`, `[[Neural Networks]]` — ✅

**Gap**: ❌ Missing assertion: `assert!(read_result.contains("[[AI]]"))`. The original file content contains `[[AI]] fundamentals`; after replacement, `[[AI]]` should be preserved unchanged. The absence of this assertion means a regression that also destroys the original WikiLink would not be caught.

**Good name**: ✅ Yes.

**Recommendation**: Add `assert!(read_result.contains("[[AI]]"), "original [[AI]] WikiLink must be preserved after replacement");`.

---

## Recommendations

### High priority

1. **Test 4 — add preservation assertion**: Add `assert!(read_result.contains("[[AI]]"))` to verify the original WikiLink survives the replacement. This closes an observable regression gap.

2. **Test 1 — clarify error message conflation**: The source error `"must contain at least one [[concept]]"` is emitted for two distinct failure modes in `contains_malformed_wikilinks` (nested brackets) and the zero-concept check. Consider distinct messages or a comment in the test explaining the observed message does not accurately describe the cause. *(Source-level debt, surfaces via this test.)*

### Medium priority

3. **Test 3 — rename**: `find_replace_multiple_wikilinks_with_same_inner_text_replaces_all` → `find_replace_partial_text_within_wikilinks_updates_all_occurrences`. The current name implies identical inner text, not shared substring.

4. **Unit test for `contains_malformed_wikilinks`**: The function in `editing.rs` has no dedicated unit tests. Test 1 is the only coverage. Recommend adding `#[cfg(test)]` unit tests in `editing.rs` for edge cases: triple brackets `[[[x]]]`, unclosed `[[open`, bracket inside `[[a[b]c]]`, and empty `[[]]`. This reduces the blast radius if Test 1 is ever removed or refactored.

### Low priority

5. **Consolidation consideration**: All 4 tests could logically live in `memory_tools_edit_ops.rs` under a `// WikiLink-specific scenarios` section comment, reducing the file count in the test directory. However, the separation is defensible as the file name makes the focus explicit. This is a style preference, not a correctness issue.

---

## Load-Bearing Score: 4/4

All four tests are load-bearing. Each exercises a distinct WikiLink-specific code path in `find_replace` that is not covered by any sibling test file. Removing any one of them would leave an undetected regression surface in `contains_malformed_wikilinks` (Test 1), whole-WikiLink renaming (Test 2), partial-substring-within-WikiLink replacement (Test 3), or multi-WikiLink replacement content (Test 4).
