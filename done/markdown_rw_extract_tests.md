---
id: T-80.95
title: "Test Evaluation: markdown_rw_extract_tests"
status: done
pipeline: tdd
---

# Test Evaluation: markdown_rw_extract_tests

**File:** `crates/maenifold-core/tests/markdown_rw_extract_tests.rs`
**Test count:** 10
**Evaluated:** 2025-07-15
**Traceability:** T-12.7.1, NFR-1, NFR-2

## Summary

Seven tests guard the public `extract_wikilinks` API with genuine behavioral coverage; three tests guard a private test-helper function (`count_concept_occurrences` in `markdown_rw_support/mod.rs`) — these protect test infrastructure, not production behavior. The fenced-code-block exclusion is tested three times with marginal differentiation. The whole file is ripe for a parameterized macro collapse on the code-skipping variants.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `extract_wikilinks_skips_code_blocks` | **KEEP** | Load-bearing: verifies that bare fenced blocks are excluded from extraction. Asserts correct cardinality (len == 2) and correct membership. Would allow a regression if the parser stops recognizing bare ` ``` ` as a code boundary. |
| `extract_wikilinks_skips_inline_code` | **KEEP** | Load-bearing: verifies inline backtick exclusion — distinct code path from fenced blocks. Unique coverage. |
| `extract_wikilinks_extracts_from_headings` | **MERGE** with `extract_wikilinks_extracts_from_lists` | Not load-bearing independently. Both tests assert that "wikilinks in non-code contexts are extracted." Since `pulldown_cmark` events for `Heading` and `List` both ultimately yield `WikiLink` events through the same extraction loop, these two tests collapse into one "extracts from all structural contexts" test. Neither catches a bug the other does not. |
| `extract_wikilinks_extracts_from_lists` | **MERGE** with `extract_wikilinks_extracts_from_headings` | See above. |
| `extract_wikilinks_normalizes_concepts` | **KEEP** | Load-bearing and unique. The deduplication of `[[Machine Learning]]` and `[[machine-learning]]` into one entry is a correctness invariant for the entire graph/search subsystem. If this regresses, concept graph edges diverge. No other test in this file catches it. |
| `extract_wikilinks_skips_fenced_code_block_with_language` | **KEEP** | Marginally distinct from `skips_code_blocks`: the language annotation (` ```csharp `) may be parsed via a different token path in `pulldown_cmark`. Worth keeping but name it clearly — `extract_wikilinks_skips_language_tagged_fenced_code_block`. |
| `extract_wikilinks_skips_indented_code_blocks` | **REWRITE** | Correct intent but the 4-space indented block syntax is context-sensitive in CommonMark (requires a blank line before it). The fixture uses `\n\n    This is indented code block\n` which is valid. However, the test does not verify whether `pulldown_cmark` (with `Options::all()`) even recognizes this as a code block — if the option set changes, this could silently start passing for the wrong reason. Add an assertion that the parser sees zero code blocks in a control input to confirm the test fixture is well-formed. Name: `extract_wikilinks_skips_four_space_indented_code_block`. |
| `count_concept_occurrences_skips_code_blocks` | **REWRITE** | Tests a *test helper*, not the production API. `count_concept_occurrences` lives in `markdown_rw_support/mod.rs` and is never invoked by production code. If this helper has a bug, the symptom is a misleading test failure elsewhere, not a user-visible defect. The correct fix is: if this helper is complex enough to need its own tests (and it is — it has a dual-path pulldown+regex logic), it should be extracted into a tested utility in the production crate. Otherwise, remove these three tests and accept that the helper is implicitly validated by the tests that call it. Current verdict: REWRITE to either (a) promote `count_concept_occurrences` to a public `markdown` utility and add a `#[doc(hidden)]` public test, or (b) remove and rely on integration coverage. |
| `count_concept_occurrences_skips_inline_code` | **REWRITE** | Same diagnosis as above. Also: `count_concept_occurrences` has a non-trivial dual-path (pulldown `WikiLink` event → counts directly; regex fallback for unrecognized link syntax → counts again). The test does not distinguish which path is exercised. A bug in the regex branch would not be caught here if `pulldown_cmark` handles the inline case first. |
| `count_concept_occurrences_handles_normalization` | **REWRITE** | Same diagnosis. Additionally, this test asserts count == 2 for `[[API Design]]` + `[[api-design]]`. It passes as long as normalization returns the same key for both. But it does NOT assert that the *normalized key* is `"api-design"` — it only verifies the count. If normalization changed to lowercase-only (making both normalize to `"api design"` with a space), the count would still be 2 but the downstream concept graph would use a different key. Add an assertion on the normalized form. |

## Recommendations

1. **Parameterize the code-block exclusion suite.** `skips_code_blocks`, `skips_inline_code`, and `skips_fenced_code_block_with_language` are all "input with wikilinks in code → assert those are absent in output." Collapse into one test with a `test_cases` array: `(label, content, expected_extracted, expected_absent)`. This makes it trivially easy to add `<!-- HTML comments -->` or YAML frontmatter exclusion later.

2. **Resolve the test-helper paradox.** `count_concept_occurrences_*` tests guard a 150-line function in the support module. Either promote that function to production (it has genuine utility as a mention-counting primitive) or accept the implicit coverage and delete the three tests. Hiding complex logic in test helpers without testing it directly is a maintenance debt.

3. **Strengthen indented-code-block test.** The test fixture is syntactically correct but fragile — indented code blocks are the most context-sensitive CommonMark feature. Add a sanity assertion that `extract_wikilinks("    [[no-code-here]]")` (no preceding blank line) is NOT treated as a code block.

4. **Merge headings + lists.** Rename the merged test `extract_wikilinks_extracts_from_all_structural_contexts` and cover headings, list items, and block quotes in one fixture.

## Load-Bearing Score: 6/10

Tests 1, 2, 5, 6, 7 are load-bearing for the production `extract_wikilinks` API. Tests 8–10 guard a test helper. Tests 3 and 4 overlap.

---

## SWE Remediation

- **KEEP** `extract_wikilinks_skips_code_blocks`: unchanged
- **KEEP** `extract_wikilinks_skips_inline_code`: unchanged
- **MERGE** `extract_wikilinks_extracts_from_headings` + `extract_wikilinks_extracts_from_lists` → `extract_wikilinks_extracts_from_all_structural_contexts`: covers headings, list items, and block quotes in one fixture (rec #4)
- **KEEP** `extract_wikilinks_normalizes_concepts`: unchanged
- **KEEP+RENAME** `extract_wikilinks_skips_fenced_code_block_with_language` → `extract_wikilinks_skips_language_tagged_fenced_code_block`
- **REWRITE** `extract_wikilinks_skips_indented_code_blocks` → `extract_wikilinks_skips_four_space_indented_code_block`: added control assertion for CommonMark context-sensitivity (rec #3)
- **REMOVE** `count_concept_occurrences_skips_code_blocks`: guarded test helper, not production API
- **REMOVE** `count_concept_occurrences_skips_inline_code`: same; accept implicit coverage (rec #2 option b)
- **REMOVE** `count_concept_occurrences_handles_normalization`: same
- Test count: 10 → 6 (3 removed, 2 merged into 1, 1 rewritten, 1 renamed)
- All 6 remaining tests pass
- Commit: ab8a232

---

## Red-team Audit

**Traceability:** T-12.7.1, NFR-1, NFR-2

### Item-by-Item

- ✅ **KEEP** `extract_wikilinks_skips_code_blocks`: unchanged, present at lines 5–22. Matches eval.
- ✅ **KEEP** `extract_wikilinks_skips_inline_code`: unchanged, present at lines 24–33. Matches eval.
- ✅ **MERGE** `extracts_from_headings` + `extracts_from_lists` → `extracts_from_all_structural_contexts`: new fixture covers headings, body text, list items, and block quotes (5 concepts asserted). Satisfies eval rec #4 ("cover headings, list items, and block quotes in one fixture"). Both originals removed; no unique coverage lost — both hit the same extraction loop.
- ✅ **KEEP** `extract_wikilinks_normalizes_concepts`: unchanged. Matches eval.
- ✅ **KEEP+RENAME** `skips_fenced_code_block_with_language` → `skips_language_tagged_fenced_code_block`: only the function name changed. Test logic identical. Matches eval-requested name.
- ✅ **REWRITE** `skips_indented_code_blocks` → `skips_four_space_indented_code_block`: renamed per eval. Added control assertion (lines 107–112) confirming that 4-space indent **without** preceding blank line is NOT treated as a code block — this is the CommonMark context-sensitivity guard the eval demanded (rec #3). Control input `"Paragraph text\n    [[not-actually-code]]"` is more robust than the eval's suggested `"    [[no-code-here]]"` because it explicitly tests the lazy-paragraph-continuation path.
- ✅ **REMOVE** `count_concept_occurrences_skips_code_blocks`: deleted. Eval offered option (a) promote or (b) remove; SWE chose (b). Helper still used by `wikilink_regex_tests.rs` and `sync_perf_baseline.rs` — support module correctly preserved.
- ✅ **REMOVE** `count_concept_occurrences_skips_inline_code`: deleted per same rationale. No unique production coverage lost.
- ✅ **REMOVE** `count_concept_occurrences_handles_normalization`: deleted per same rationale. The eval's specific concern (count assertion without normalized-key assertion) is moot after removal — the normalization invariant is already covered by `extract_wikilinks_normalizes_concepts` which asserts the exact normalized form `"machine-learning"`.

### Scope Check

- Only `crates/maenifold-core/tests/markdown_rw_extract_tests.rs` modified (33 insertions, 44 deletions).
- No unrelated changes introduced.
- Support module `markdown_rw_support/mod.rs` correctly left in place (still consumed by 2 other test files).
- `#[path]` import and `use` of `count_concept_occurrences` removed from this file — consistent with test removal.

### Bookkeeping Fix

- SWE remediation summary claimed "Test count: 10 → 7" but actual `#[test]` count is 6 (3 removed + 1 merge-net = 4 fewer). The same section says "All 6 remaining tests pass" — contradicting the "→ 7" claim. Corrected to "10 → 6" above.

### Test Execution

- All 6 `extract_wikilinks_*` tests pass (33 of 33 `extract_wikilinks` tests passed; 5 unrelated `proximity_weight_sections` failures are pre-existing lock-timeout infrastructure issues, not regressions from this change).

### Verdict: PASS

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_extract`: 6 passed, 0 failed
- Verdict: PASS
