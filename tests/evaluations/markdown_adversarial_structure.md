# Test Evaluation: markdown_adversarial_structure

**File:** `crates/maenifold-core/tests/markdown_adversarial_structure.rs`
**Test count:** 9
**Traceability:** T-2.3 / RTM FR-4, T-W1-3 / RTM NFR-2
**Evaluated:** 2026-07-14

---

## Summary

This suite targets adversarial inputs to `extract_wikilinks` — malformed brackets, normalization edge cases, and code-block bypass. The normalization tests (6, 7, 8) are solid and genuinely load-bearing. However, **two tests (4 and 5) apply a client-side filter before asserting**, which masks the very regression they exist to catch: if `extract_wikilinks` starts emitting empty strings for `[[]]` or `[[   ]]`, these tests still pass. The malformed-bracket tests (1, 2, 3) are correct but could be collapsed into a single parameterized test.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `extract_wikilinks_triple_brackets` | **MERGE** → parameterize with tests 2 & 3 | Correctly asserts rejected concept is absent. But identical pattern to tests 2 and 3: "malformed input → not in results." Three `(input, rejected_concept, expected_kept)` rows in a table-driven test would cover this group. |
| `extract_wikilinks_missing_closing_bracket` | **MERGE** → parameterize with tests 1 & 3 | Same pattern as test 1. Correct assertion. |
| `extract_wikilinks_missing_opening_bracket` | **MERGE** → parameterize with tests 1 & 2 | Same pattern. Adds a `len() == 1` assertion that could be part of the table. |
| `extract_wikilinks_empty_brackets` | **REWRITE** | **Critical flaw: client-side filter masks regression.** The test filters `!concept.trim().is_empty()` from the result before asserting. The implementation already performs this filter at `wikilinks.rs:77` (`!link.is_empty()`). If that guard is ever removed, `extract_wikilinks` would return `["whitespace-validation", "", "another"]` and this test would **still pass** because the filter hides the empty string. Remove the filter — test the raw output of `extract_wikilinks`. |
| `extract_wikilinks_whitespace_only_brackets` | **REWRITE** | Same critical flaw as test 4. Client-side `!concept.trim().is_empty()` filter hides the regression case. If `normalize_concept("   ")` returns `""` but `extract_wikilinks` fails to call `!link.is_empty()` before pushing, the test would still pass after the filter. Remove the filter and assert on raw output. |
| `extract_wikilinks_normalization_collision` | **KEEP** | Correctly asserts `len() == 1` and `contains("test-concept")` on raw output. Tests the deduplication + normalization interaction. Load-bearing: removing this test allows a regression where all three variants are returned as distinct entries. |
| `extract_wikilinks_leading_trailing_hyphens` | **MERGE** → parameterize with tests 8 | Both test normalization: input → expected canonical form. Could be a table: `[("--data-model--", "data-model"), ("test-----concept", "test-concept")]`. |
| `extract_wikilinks_multiple_adjacent_hyphens` | **MERGE** → parameterize with test 7 | Same pattern as test 7. See above. |
| `extract_wikilinks_only_code_blocks` | **KEEP** | Load-bearing: guards against concepts in fenced code blocks being indexed as real WikiLinks — a correctness and graph-pollution concern. Correctly asserts `len() == 0` on raw output. |

---

## Detailed Observations

### 1. Tests 4 and 5 assert a filtered view, not the function's behavior

This is the most significant finding in the file. The tests do:

```rust
let concepts = extract_wikilinks(content)
    .into_iter()
    .filter(|concept| !concept.trim().is_empty())  // ← hides the bug
    .collect::<Vec<_>>();
```

The implementation at `wikilinks.rs:77` already guards:
```rust
if !link.is_empty() && seen.insert(link.clone()) {
    links.push(link);
}
```

So currently, both tests pass correctly *because the implementation is right*. But the client-side filter **decouples the test from the implementation guard**. The test asks: "after you remove empty strings from the output, do you get 2?" That's always true regardless of whether `extract_wikilinks` returns `["a", "b"]` or `["a", "", "b"]`.

**The correct tests should be:**
```rust
let concepts = extract_wikilinks(content); // no filter
assert!(concepts.contains(&"whitespace-validation".to_string()));
assert!(concepts.contains(&"another".to_string()));
assert_eq!(concepts.len(), 2); // fails if "" is included → catches regression
```

If the implementation regresses, the `len() == 2` assertion with the filter won't catch it. Without the filter, it will. The filter must be removed.

### 2. Tests 1, 2, 3 are parameterizable

All three tests follow the same pattern:
- Set up `content` with one valid wikilink and one malformed construction
- Assert the valid concept is present
- Assert the malformed concept is absent

A table-driven test would be:

| Input fragment | Rejected concept | Kept concept |
|---|---|---|
| `[[[triple-attack]]]` | `triple-attack` | `bracket-validation` |
| `[[missing-bracket text continues.` | `missing-bracket text continues` | `malformed-input` |
| `wrong]] text.` | — | `edge-case-handling` (len=1) |

Three rows, one test function. Adding a new malformed input takes one line instead of a new test.

### 3. Tests 7 and 8 are parameterizable

Both test that normalization collapses variant spellings to a canonical form. Table:

| Input | Expected |
|---|---|
| `[[--data-model--]]` + `[[data-model]]` | `"data-model"`, len=1 |
| `[[test-----concept]]` + `[[test-concept]]` | `"test-concept"`, len=1 |

Test 6 (`normalization_collision`) is a stronger form of this pattern — three variants colliding. It could be the third row in the same table.

### 4. Missing adversarial cases

The suite doesn't cover:
- **Nested brackets inside content**: `[[outer [[inner]] end]]` — what does `extract_wikilinks` return?
- **Very long concept names**: Does the parser have length limits?
- **Unicode normalization**: `[[café]]` vs `[[cafe]]` — are these the same concept?
- **Mixed code-block types**: Inline code `` `[[inline-code]]` `` — is it excluded like fenced blocks?

---

## Recommendations

1. **Rewrite tests 4 and 5**: Remove the `.filter(|c| !c.trim().is_empty())` call. Assert on raw `extract_wikilinks` output. The `len() == 2` assertion is the one that catches the regression — do not pre-filter it away.
2. **Parameterize tests 1, 2, 3** into `extract_wikilinks_rejects_malformed_brackets` with a data table.
3. **Parameterize tests 7, 8** (and optionally 6) into `extract_wikilinks_normalizes_to_canonical_form`.
4. **Add inline code test**: Verify `` `[[inline-code]]` `` is excluded from wikilink extraction.
5. **Add a nested-brackets test**: Verify `[[outer [[inner]] end]]` does not produce `"outer"` or `"inner"` as concepts.

---

## Load-Bearing Score: 6/9

Tests 6, 7, 8, 9 are correctly asserted and load-bearing. Tests 1, 2, 3 are load-bearing but redundant with each other (parameterizable). Tests 4 and 5 are **not load-bearing as written** — their client-side filter prevents them from catching the exact regression they exist to guard against.
