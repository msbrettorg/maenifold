# Test Evaluation: content_preview_tests
**File:** `crates/maenifold-core/tests/content_preview_tests.rs`
**Test count:** 8
**Evaluated:** 2025-07-16
**Traceability:** T-1.6 · RTM NFR-1

---

## Summary

Seven of eight tests cover genuinely distinct code paths through `create_smart_preview`, with assertions that range from excellent (exact string equality) to superficial (contains-only checks that pass even if the boundary logic is wrong). One test — `create_smart_preview_with_wikilinks_preserves_links` — is a silent false-positive: it claims to verify WikiLink preservation under truncation but the parameters chosen cause the full string to be returned, so the WikiLinks are never at risk of being cut. No truncation occurs at all.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `create_smart_preview_short_content_returns_full_content` | **KEEP** | Pins the early-return path (`content_len <= target_length`). Exact-equality assertion. Load-bearing. |
| `create_smart_preview_with_complete_sentence_truncates_at_sentence` | **KEEP** | Only test with an exact-equality assertion on a truncated result. Directly tests `find_last_sentence_boundary`. Load-bearing. |
| `create_smart_preview_no_sentence_boundary_uses_paragraph` | **REWRITE** | Asserts `ends_with("...")` and `contains("First paragraph")` — does not assert the exact output. The expected output is deterministic (`"First paragraph without ending..."`); pin it. |
| `create_smart_preview_no_paragraph_boundary_uses_word_boundary` | **REWRITE** | Uses a *negative* assertion (`!contains("FollowedByMore")`) to verify word-boundary fallback. This does not verify what the output *is*, only one word that isn't in it. Expected output is deterministic (`"ThisIsAVeryLongWordWithNoSpaces..."`); use `assert_eq!`. |
| `create_smart_preview_no_boundaries_hard_truncates` | **KEEP** | Tests the last-resort hard-truncation path. The `chars().count() <= 23` bound is a legitimate behavioral contract for maximum output size, and no other test reaches this path. |
| `create_smart_preview_with_wikilinks_preserves_links` | **REWRITE** | **Silent false-positive.** Content is 69 chars; `target=60, tolerance=20` → `max_length=min(80,69)=69` → the entire string is returned, so both `[[WikiLinks]]` are trivially present. No truncation occurs. The test passes even if the implementation blindly strips `[[...]]` on truncation. Use content long enough that truncation *would* split a WikiLink mid-bracket to actually test preservation. |
| `create_smart_preview_multiple_short_sentences_truncates_at_natural_boundary` | **REWRITE** | Only asserts the first two sentences are present (`contains("First.")`, `contains("Second.")`). The deterministic expected output is `"First. Second. Third. Fourth."` — assert it with `assert_eq!` or `assert!(result == ...)`. Contains-checks alone don't verify that truncation stopped at a sentence boundary rather than elsewhere. |
| `create_smart_preview_sentence_ending_at_limit_returns_full_sentence` | **KEEP** | Meaningful boundary-condition test: sentence ends exactly within the tolerance window. Contains + negative-contains together are sufficient here because the semantic contract is directional ("includes first sentence, excludes trailing sentence"). |

---

## Recommendations

1. **Replace `contains`-only assertions with `assert_eq!`** in tests 3, 4, and 7. `create_smart_preview` is a pure function with deterministic output for these inputs — there is no excuse for inexact assertions. Every fuzzy assertion here is a future regression that won't be caught.

2. **Rewrite `create_smart_preview_with_wikilinks_preserves_links`** with content that forces truncation across a WikiLink boundary. Example:
   ```rust
   // 100-char content; truncation at ~60 chars would split "[[authentication]]"
   // unless the implementation respects WikiLink boundaries.
   let content = "Uses [[authentication]] for all endpoints and [[JWT]] for all tokens at the service layer boundary.";
   let result = create_smart_preview(content, 30, 5);
   // Assert: does NOT split a WikiLink mid-bracket
   assert!(!result.contains("[[auth") || result.contains("[[authentication]]"),
       "truncation must not split WikiLink: {result}");
   ```

3. **Parameterize or consolidate tests 3, 4, 5** (paragraph, word, and hard-truncation fallbacks) into a single table-driven structure once exact assertions are added. Each row: `(content, target, tolerance, expected)`. Reduces 3 tests to 1 `#[test]` with 3 cases.

4. **Verify the `wikilinks_preserves_links` assumption** with the actual implementation: if `search_space` trims into the middle of `[[WikiLink]]`, the current implementation will output a broken `[[auth` fragment. No test currently catches this. This is a **gap in coverage**, not just a naming issue.

---

## Load-Bearing Score: 6/8

Tests 1, 2, 5, 8 are clearly load-bearing. Tests 3 and 7 have correct *intent* but assertions too weak to catch a subtly wrong boundary choice. Test 6 is not load-bearing as written — it would pass even if WikiLinks were stripped. Test 4 is partially load-bearing (negative assertion catches obvious regression but not subtle truncation-position bugs).
