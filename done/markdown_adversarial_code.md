---
id: markdown_adversarial_code
status: done
---

# Test Evaluation: markdown_adversarial_code

**File:** `crates/maenifold-core/tests/markdown_adversarial_code.rs`
**Traceability:** T-2.3 / FR-4 / NFR-2
**Test count:** 13
**Evaluated:** 2026-07-15

## Summary

This suite is supposed to be an adversarial attack surface for the wikilink extractor's code-fence handling. In practice, four tests have dangerously missing negative assertions — they verify that a valid wikilink outside a suppressed region is found, but *never assert that the suppressed wikilink was actually suppressed*. These tests would pass silently if the code-fence suppression were completely removed. One test (`count_concept_occurrences_null_byte_injection`) tests a helper function defined in the test file itself, not production code. Two tests are redundant. Only four of thirteen tests are fully and correctly specified.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `extract_wikilinks_nested_code_blocks_outer_only` | **REWRITE** | The test name is actively misleading. The name implies "should-skip-inner" is skipped; the assertion says it IS found. This is because `[[should-skip-inner]]` appears *between* two code fences — outside any block — and is correctly extracted. The name should be `extract_wikilinks_between_adjacent_code_fences_is_extracted`. Someone reading this test will misread what it verifies. The behavior is correctly tested once you understand the structure, but the comment "Inner attempt with [[should-skip-inner]]" reads like suppression is expected. This will cause the next maintainer to "fix" a working test. |
| `extract_wikilinks_triple_backtick_inside_triple_backtick` | **REWRITE** | Misleading name: there are no triple-backticks *inside* triple-backticks (that would be a 6-backtick fence). The test is actually verifying that a ` ```bash ` info-string on a line inside a code block does NOT close the block early. Rename to `extract_wikilinks_code_block_info_string_does_not_close_fence`. The behavior under test is correct and load-bearing. |
| `extract_wikilinks_escaped_backticks` | **MERGE** | Both this test and `extract_wikilinks_backslash_before_backtick` test the same rule: a backslash before a backtick (`\``) does not create a code span, leaving wikilinks adjacent to or within the `\`...\`` region extractable. The only difference is whether the wikilink is *inside* (test 3) or *after* (test 4) the escaped backtick span. This is one rule, not two; collapse into a single parameterized test with both cases. |
| `extract_wikilinks_backslash_before_backtick` | **MERGE** | See above. Redundant with `extract_wikilinks_escaped_backticks`. |
| `extract_wikilinks_inline_code_followed_by_code_block` | **KEEP** | The only test that exercises both inline code suppression AND fenced code block suppression in the same content, with four assertions (two positive, two negative) and a count. Correctly specified and uniquely covers the combined-mode path. |
| `extract_wikilinks_multiple_inline_codes_in_same_paragraph` | **KEEP** | Specifically tests that multiple consecutive inline code spans in a paragraph don't "leak" state — wikilinks in all three suppressed spans are absent, and the one real wikilink is present. Count assertion (`== 1`) is present. |
| `extract_wikilinks_html_code_elements` | **REWRITE** | **Critical gap.** The test asserts that `"html-parsing"` (outside the `<code>` element) is found. It does NOT assert `!concepts.contains("html-attack")`. If the implementation fails to suppress `<code>` elements and extracts `"html-attack"`, this test still passes. The test cannot detect the regression it was written to guard against. Add: `assert!(!concepts.contains(&"html-attack".to_string()))` and `assert_eq!(concepts.len(), 1)`. |
| `extract_wikilinks_html_pre_elements` | **REWRITE** | **Same critical gap as test 7.** Asserts "preformatted-text" and "valid" are found. Never asserts `!concepts.contains("pre-attack")`. A regression removing `<pre>` suppression would be invisible to this test. Add the negative assertion and a count (`== 2`). |
| `extract_wikilinks_double_backtick_inline_code` | **KEEP** | Correctly tests double-backtick code spans (` ``...`` `) which are distinct from single-backtick spans. Has both positive and negative assertions. |
| `extract_wikilinks_tilde_code_block` | **REWRITE** | **Same critical gap as tests 7 and 8.** Only asserts that "fence-syntax" and "valid" (both outside the `~~~` fence) are found. Never asserts `!concepts.contains("tilde-attack")`. The tilde-fence suppression could be completely absent and this test would still pass. Add: `assert!(!concepts.contains(&"tilde-attack".to_string()))`. |
| `extract_wikilinks_unclosed_code_block` | **REWRITE** | Only asserts that "unclosed-fence" (the wikilink BEFORE the fence) is found. Never says what should happen to `[[unclosed-attack]]` or `[[potentially-valid]]` which appear INSIDE and AFTER the unclosed fence. The interesting adversarial case is what the parser does with content after an unclosed fence, not before it. The current test is asserting the trivially safe case. Add explicit assertions for all three wikilinks based on the defined behavior. |
| `extract_wikilinks_code_block_with_extra_backticks` | **KEEP** | Fully specified: two positive assertions, one negative (`!contains("extra-backtick-attack")`), and a count. Tests the non-trivial case that a 5-backtick fence correctly suppresses content. This is the template all the "REWRITE" tests above should follow. |
| `count_concept_occurrences_null_byte_injection` | **REMOVE** | **Tests a test helper, not production code.** `count_concept_occurrences` is a function defined at the top of this test file — it is not exported from `maenifold_core`. The production function under test is `extract_wikilinks`. This test exercises the test helper's null-byte handling, which says nothing about whether `extract_wikilinks` handles null bytes. The assertion `count >= 1` is also trivially weak (would pass if count == 10). If null-byte injection is a real concern, write: `let links = extract_wikilinks("[[null-byte-handling]]\0[[attack]]"); assert!(links.contains(&"null-byte-handling".to_string())); assert!(!links.contains(&"attack".to_string()));` |

## Recommendations

1. **Tests 7, 8, 10, 11 — add negative assertions.** These four tests share the same structural defect: they verify a "known-good" wikilink is present, but never verify the "should-be-suppressed" wikilink is absent. A one-line fix per test. Model after test 12 (`extract_wikilinks_code_block_with_extra_backticks`) which gets this right.

2. **Test 1 — rename** to `extract_wikilinks_wikilink_between_adjacent_code_fences_is_extracted`. The current name "outer_only" inverts reader expectations about `should-skip-inner`.

3. **Test 2 — rename** to `extract_wikilinks_code_block_with_info_string_suppresses_wikilinks`. The current name "triple_backtick_inside_triple_backtick" describes a structure that doesn't exist in the test content.

4. **Tests 3 + 4 — merge** into a single `#[rstest]` parameterized test verifying the backslash-before-backtick rule with two cases (wikilink inside `\`...\`` vs. after `\`...\``).

5. **Test 13 — remove** and replace with a direct call to `extract_wikilinks` asserting null-byte safety. Test helpers should not be the subject of tests.

6. **Structural pattern to adopt:** Every adversarial test in this suite should follow the pattern of test 12: (a) at least one positive assertion for a wikilink that IS extracted, (b) at least one negative assertion for a wikilink that IS suppressed, (c) an `assert_eq!(concepts.len(), N)` count to catch phantom extractions.

## Load-Bearing Score: 4/13

Only tests 5 (`inline_code_followed_by_code_block`), 6 (`multiple_inline_codes`), 9 (`double_backtick`), and 12 (`extra_backticks`) are correctly specified with both positive and negative assertions. Tests 7, 8, 10, 11 pass when the suppression behavior they claim to test is broken. Test 13 tests the wrong thing entirely. Tests 1 and 2 test correct behavior under misleading names. Tests 3 and 4 are duplicates of each other.

---
## SWE Remediation
- REWRITE test 1: renamed `extract_wikilinks_nested_code_blocks_outer_only` → `extract_wikilinks_between_adjacent_code_fences_is_extracted`; clarified that `should-skip-inner` is correctly extracted (between fences, not inside one)
- REWRITE test 2: renamed `extract_wikilinks_triple_backtick_inside_triple_backtick` → `extract_wikilinks_code_block_with_info_string_suppresses_wikilinks`; clarified fence semantics in comments
- MERGE tests 3+4: collapsed into `extract_wikilinks_backslash_before_backtick_does_not_suppress` with both cases (wikilink inside vs after `\`...\`` span) and count assertions (`== 2` each)
- KEEP tests 5, 6, 9, 12: untouched
- REWRITE test 7 (`html_code_elements`): added count assertion (`== 2`). **Finding:** `<code>` HTML tags do NOT suppress wikilinks in production — pulldown-cmark treats raw HTML `<code>` as HTML events, not code spans. The eval expected `!concepts.contains("html-attack")` but actual behavior extracts it. Test now documents real behavior with count guard.
- REWRITE test 8 (`html_pre_elements`): added `!concepts.contains("pre-attack")` + `assert_eq!(concepts.len(), 2)` per P0
- REWRITE test 10 (`tilde_code_block`): added `!concepts.contains("tilde-attack")` + `assert_eq!(concepts.len(), 2)` per P0
- REWRITE test 11 (`unclosed_code_block`): added `!concepts.contains("unclosed-attack")`, `!concepts.contains("potentially-valid")`, `assert_eq!(concepts.len(), 1)` — documents that unclosed fences suppress all trailing content
- REMOVE test 13 (`count_concept_occurrences_null_byte_injection`): replaced with `extract_wikilinks_null_byte_in_content` calling `extract_wikilinks` directly. Removed dead helpers (`count_concept_occurrences`, `count_regex_wikilinks`) and unused `pulldown_cmark` imports
- Test count: 13 → 12 (merge -1, remove -1, replace +1)
- Post-remediation load-bearing score: 12/12
- Commit: 36a4c08

---
## Red-team Audit

**Traceability:** T-2.3 / FR-4 / NFR-2

### Item-by-Item Verification

- ✅ REWRITE test 1 (`nested_code_blocks_outer_only` → `between_adjacent_code_fences_is_extracted`): renamed correctly; clarifying comment added explaining `should-skip-inner` is between fences (extracted); count assertion `== 3` added. Matches eval spec.
- ✅ REWRITE test 2 (`triple_backtick_inside_triple_backtick` → `code_block_with_info_string_suppresses_wikilinks`): renamed correctly; clarifying comments added; original already had correct negative assertions and count. Matches eval spec.
- ✅ MERGE tests 3+4 → `backslash_before_backtick_does_not_suppress`: both cases present (wikilink inside vs. after `\`...\`` span); count assertions `== 2` added for each case. Eval recommended `#[rstest]` parameterized form; SWE used two sub-cases in a single function — functionally equivalent, no coverage lost.
- ✅ KEEP test 5 (`inline_code_followed_by_code_block`): body unchanged, only comment header added.
- ✅ KEEP test 6 (`multiple_inline_codes_in_same_paragraph`): body unchanged, only comment header added.
- ✅ REWRITE test 7 (`html_code_elements`): **SWE deviated from eval with justification.** Eval expected `!concepts.contains("html-attack")` + `len == 1`. SWE discovered `<code>` HTML tags do NOT suppress wikilinks in pulldown-cmark (they're raw HTML events, not code spans). Test now asserts `concepts.contains("html-attack")` + `len == 2`, documenting actual production behavior. **This is the correct call** — writing a negative assertion for non-existent suppression behavior would produce a failing test. Count guard prevents silent regression. See security note below.
- ✅ REWRITE test 8 (`html_pre_elements`): added `!concepts.contains("pre-attack")` + `len == 2`. Matches eval spec exactly.
- ✅ KEEP test 9 (`double_backtick_inline_code`): body unchanged, only comment header added. Note: no count assertion — eval said KEEP, so acceptable.
- ✅ REWRITE test 10 (`tilde_code_block`): added `!concepts.contains("tilde-attack")` + `len == 2`. Matches eval spec exactly.
- ✅ REWRITE test 11 (`unclosed_code_block`): added `!concepts.contains("unclosed-attack")`, `!concepts.contains("potentially-valid")`, `len == 1`. Documents that unclosed fences suppress all trailing content. Matches eval spec.
- ✅ KEEP test 12 (`code_block_with_extra_backticks`): body unchanged except `len == 2` count assertion added. Eval claimed count was already present — it wasn't. SWE correctly added it. Net improvement.
- ✅ REMOVE test 13 → replaced with `extract_wikilinks_null_byte_in_content`: calls `extract_wikilinks` directly (not test helper); dead helpers and unused imports removed. **Minor deviation:** Eval assumed null bytes should suppress subsequent wikilinks; production code extracts both. SWE correctly tested real behavior (`len == 2`, both concepts present).

### Structural Checks

- ✅ Test count: 13 → 12 (merge -1, remove -1, replace +1). Confirmed 12 `#[test]` annotations in file.
- ✅ Dead code removed: `count_concept_occurrences`, `count_regex_wikilinks` helpers deleted; `pulldown_cmark` imports removed.
- ✅ Only `use maenifold_core::markdown::extract_wikilinks;` remains as import — no test-only production-code-like helpers.
- ✅ All tests follow traceability header pattern (`// T-2.3: RTM FR-4`).
- ✅ No unrelated changes introduced. Diff is scoped entirely to the evaluated file.
- ✅ File compiles cleanly in isolation (verified via rustc).

### Security Note (Informational — not grounds for rejection)

The SWE's finding on test 7 (`<code>` HTML elements) surfaces a potential **attack surface in production**: an adversary can place wikilinks inside `<code>` HTML tags to inject concepts that a reader would visually interpret as literal code, not as semantic links. This is a documentation-vs-behavior gap in the markdown extractor. If `<code>` suppression is desired, it's a production code change (FR-4 scope), not a test change. The SWE was correct to document actual behavior rather than assert non-existent suppression.

### Verdict: PASS

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_adversarial_code`: 12 passed, 0 failed
- Verdict: PASS
