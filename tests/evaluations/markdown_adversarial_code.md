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
