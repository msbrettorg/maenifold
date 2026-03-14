# Test Evaluation: wikilink_regex_tests.rs

**File:** `crates/maenifold-core/tests/wikilink_regex_tests.rs`
**Traceability:** T-W1-3 · RTM NFR-1 (C# Oracle Parity) · RTM NFR-2 (Test Coverage)
**Evaluator color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

This file contains **6 `#[test]` functions** and **2 large private helper functions**
(`count_concept_occurrences`, `count_regex_wikilinks`) defined entirely within the test
file. The helpers are byte-for-byte reimplementations of production logic that already
exists in `crates/maenifold-core/src/markdown/wikilinks.rs`.

The central question for each test is: **does it actually verify production behaviour, or
does it verify an assumption that has already been baked into the test itself?**

### What the tests claim to cover (per inline comments)

All six tests carry the tag `T-W1-3: RTM NFR-1 / NFR-2`, asserting they port WikiLink
regex coverage from the C# oracle and satisfy the parity + coverage NFRs.

### What the tests actually cover

| Area | Covered | Not covered |
|---|---|---|
| `extract_wikilinks` — triple-bracket rejection | ✅ | — |
| `extract_wikilinks` — quadruple-bracket rejection | ⚠️ partial | `is_empty()` not asserted |
| `extract_wikilinks` — mixed valid/invalid | ⚠️ partial | negative assertions absent |
| `extract_wikilinks` — nested bracket handling | ⚠️ partial | negative assertions absent |
| `count_concept_occurrences` — test-local helper | ❌ wrong target | production API untested |
| `normalize_concept` via extraction | ✅ | underscore→hyphen rule |
| WikiLinks inside code blocks | ❌ | not present |
| Empty `[[]]` pattern | ❌ | not present |
| Whitespace `[[ concept ]]` trimming | ❌ | not present |
| Deduplication (same concept twice) | ❌ | not present |
| `[[concept\|alias]]` pipe syntax | ❌ | not present |

### Structural problem: duplicated production logic in test infrastructure

`count_concept_occurrences` (68 lines) and `count_regex_wikilinks` (45 lines) in this
file reimplement the same logic as `extract_wikilinks` + `extract_wikilinks_regex_only`
in production. This is a **test-smell trifecta**:

1. Tests of the helper verify the helper, not the production function.
2. Any divergence between helper and production code is invisible.
3. Maintenance burden doubles: both must be kept in sync with any production change.

The NFR-1 parity claim requires proving that production code matches the C# oracle.
Proving that a test-local reimplementation matches itself proves nothing.

---

## Verdicts

| # | Test name | Load-bearing | Unique | Assertions correct | Name accurate | Parameterizable | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | `extract_wikilinks_rejects_triple_bracket_pattern` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | Partially | **KEEP** |
| 2 | `extract_wikilinks_rejects_quadruple_bracket_pattern` | ⚠️ Partial | ✅ Yes | ❌ No | ❌ No | Partially | **REWRITE** |
| 3 | `extract_wikilinks_extracts_only_valid_concepts` | ✅ Yes | ✅ Yes | ❌ No | ❌ No | ✅ Yes | **REWRITE** |
| 4 | `extract_wikilinks_handles_nested_brackets_correctly` | ⚠️ Partial | ❌ No | ❌ No | ⚠️ Partial | Partially | **MERGE into 3** |
| 5 | `count_concept_occurrences_ignores_malformed_brackets` | ❌ No | ❌ No | N/A | ❌ No | N/A | **REMOVE** |
| 6 | `extract_wikilinks_allows_hyphens_underscores_in_concepts` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | **KEEP** |

---

## Per-Test Analysis

---

### `extract_wikilinks_rejects_triple_bracket_pattern`

**Input:** `"This has a malformed [[[triple-attack]]] pattern."`
**Assertion:** `concepts.is_empty()`

**Load-bearing?** Yes. This is the only test that makes the *strong* claim that a
specific malformed pattern produces **zero** extracted concepts. The assertion
`is_empty()` is unambiguous.

**Unique?** Yes. No other test in this file or in `wikilink_filter_*` tests covers the
triple-bracket case with a complete-rejection assertion.

**Assertions correct?** Yes. The intent (triple brackets must not extract anything)
directly matches the assertion.

**Name accurate?** Yes.

**Parameterizable?** This test could be one row in a table-driven test covering multiple
rejection patterns (`[[[x]]]`, `[[[[x]]]]`, `[x]]`, `[[x]`, etc.), but it is legible
as-is.

**Verdict: KEEP.** Optionally extend with additional triple-bracket variants (e.g.,
`[[[x]]]` with text before and after, `[[[space content]]]`).

---

### `extract_wikilinks_rejects_quadruple_bracket_pattern`

**Input:** `"This has [[[[quadruple]]]] brackets."`
**Assertions:** `all(|c| !c.contains('['))` and `all(|c| !c.contains(']'))`

**Load-bearing?** Partially. It tests that no bracket characters escape into concept
names. It does NOT test whether `quadruple` (clean, without brackets) is extracted or
not. Whether the clean string is extracted depends entirely on what `pulldown_cmark`
emits for this input and how the PARITY-INT002 `visible` reconstruction behaves.

**Unique?** Yes in intent, but weaker than test 1.

**Assertions correct?** No, for this claim. The test name says "rejects" — implying
nothing is extracted. But the assertions would pass if `quadruple` (no brackets) is
extracted. The inline comment clarifies the intent is narrower: "must not leak bracket
characters". The name and the assertion are misaligned.

**Name accurate?** No. "rejects" implies `is_empty()`. If the intended guarantee is
only "no bracket leakage", the name should say so.

**Parameterizable?** Yes — this and test 1 are natural rows in the same table.

**Verdict: REWRITE.** Clarify the intended contract:
- If the contract is "nothing extracted": add `assert!(concepts.is_empty())` and
  rename to `extract_wikilinks_rejects_quadruple_bracket_pattern`.
- If the contract is "brackets do not leak into concept names": rename to
  `extract_wikilinks_does_not_leak_brackets_from_quadruple_pattern` and document
  explicitly that `quadruple` may or may not be extracted.

---

### `extract_wikilinks_extracts_only_valid_concepts`

**Input:** Multi-line string with `[[neural-network]]`, `[[machine-learning]]`,
`[[[invalid]]]`, `[[[[also-invalid]]]]`, `[[data-science]]`.
**Assertions:**
- `concepts.contains("neural-network")` ✅
- `concepts.contains("machine-learning")` ✅
- `concepts.contains("data-science")` ✅
- `all(!contains('['))` and `all(!contains(']'))`

**Load-bearing?** Yes — it is the primary test for mixed-content extraction.

**Unique?** Yes.

**Assertions correct?** No. The test name claims "extracts **only** valid concepts" but
there are no negative assertions. If the production code had a regression that also
extracted `invalid` or `also-invalid` (without bracket characters), every assertion
would still pass. The "only" guarantee is unverified.

**Name accurate?** No. The test name overpromises vs. the assertions.

**Parameterizable?** Yes — the three positive concept checks are a natural `#[test_case]`
or `rstest` table.

**Verdict: REWRITE.** Add negative assertions:
```rust
assert!(!concepts.contains(&"invalid".to_string()),
    "Triple-bracket content must not be extracted");
assert!(!concepts.contains(&"also-invalid".to_string()),
    "Quadruple-bracket content must not be extracted");
assert_eq!(concepts.len(), 3,
    "Exactly 3 valid concepts expected; extra extractions indicate a regression");
```

---

### `extract_wikilinks_handles_nested_brackets_correctly`

**Input:** Multi-line string with `[[algorithm]]`, `[[[nested]]]`, `[[[before]]]`,
`[[[[after]]]]`.
**Assertions:**
- `concepts.contains("algorithm")` ✅
- `all(!contains('['))` and `all(!contains(']'))`

**Load-bearing?** Partially. It verifies that a valid link (`algorithm`) survives in a
noisy input. But it does not assert that `nested`, `before`, and `after` are absent.

**Unique?** No. Test 3 already covers the same "valid surrounded by malformed" pattern.
This test provides marginally different inputs but the same logical structure and the
same incomplete assertion strategy.

**Name accurate?** Partially. "Handles correctly" implies both extraction of valid links
AND rejection of invalid ones. Only the former is checked.

**Verdict: MERGE into test 3 (or REWRITE as a standalone).** If kept as standalone,
add:
```rust
assert!(!concepts.contains(&"nested".to_string()));
assert!(!concepts.contains(&"before".to_string()));
assert!(!concepts.contains(&"after".to_string()));
assert_eq!(concepts.len(), 1);
```
Merging with test 3 as a parameterized case would eliminate the duplication and enforce
the same assertion discipline across all mixed-content scenarios.

---

### `count_concept_occurrences_ignores_malformed_brackets`

**Calls:** `count_concept_occurrences(content, "regex-pattern")` — a function defined
only in this test file, not in any production module.

**Load-bearing?** No. `count_concept_occurrences` is not a public production API. It is
a 68-line test helper that reimplements `extract_wikilinks` logic. Testing it verifies
the reimplementation's self-consistency, not the production code's correctness.

**Unique?** No. The production function `extract_wikilinks` would serve the same
verification purpose and provide real coverage.

**Assertions correct?** Vacuously — the assertion `assert_eq!(count, 2)` is correct
against the test helper's own logic, which was written to match what the author already
believed the count should be. Circular.

**Name accurate?** No. The name implies production behaviour is being verified.

**Verdict: REMOVE.** Replace with a direct test against `extract_wikilinks`:
```rust
#[test]
fn extract_wikilinks_counts_only_valid_occurrences_ignoring_malformed() {
    let content = "Valid: [[regex-pattern]]\nInvalid: [[[regex-pattern]]]\nValid again: [[regex-pattern]]";
    // extract_wikilinks deduplicates, so only 1 unique concept regardless of count.
    // For occurrence counting, verify via len() on repeated inputs or test the
    // relevant production counting surface if one exists.
    let concepts = extract_wikilinks(content);
    assert!(concepts.contains(&"regex-pattern".to_string()));
    assert_eq!(concepts.len(), 1, "Deduplication must hold");
}
```
> **Note:** If occurrence counting (vs. unique extraction) is a genuine product
> requirement, `count_concept_occurrences` should be promoted to a production function,
> documented, and tested against its public signature. Hiding it in test infrastructure
> makes the requirement invisible.

---

### `extract_wikilinks_allows_hyphens_underscores_in_concepts`

**Input:** `[[machine-learning]]`, `[[neural_network]]`, `[[deep-learning_model]]`,
`[[gpt-4]]`.
**Assertions:**
- `contains("machine-learning")` — hyphen passthrough ✅
- `contains("neural-network")` — underscore→hyphen normalization ✅
- `contains("deep-learning-model")` — mixed→all-hyphen normalization ✅
- `contains("gpt-4")` — digit tolerance ✅

**Load-bearing?** Yes. This is the only test in the file that directly exercises
`normalize_concept` behavior through the extraction path.

**Unique?** Yes — normalization is not covered by the other five tests.

**Assertions correct?** Yes. Each expected normalized form is unambiguous and correct
against the `normalize_concept` implementation.

**Name accurate?** Yes.

**Parameterizable?** Yes — this is an excellent candidate for `rstest`:
```rust
#[rstest]
#[case("[[machine-learning]]", "machine-learning")]
#[case("[[neural_network]]",   "neural-network")]
#[case("[[deep-learning_model]]", "deep-learning-model")]
#[case("[[gpt-4]]",             "gpt-4")]
#[case("[[  spaced  ]]",        "spaced")]   // bonus: trim normalization
fn normalize_concept_via_extraction(#[case] input: &str, #[case] expected: &str) {
    let concepts = extract_wikilinks(input);
    assert!(concepts.contains(&expected.to_string()));
}
```

**Verdict: KEEP.** Parameterize when `rstest` is added as a dev-dependency.

---

## Recommendations

### Priority 1 — Remove ghost infrastructure (test 5)

`count_concept_occurrences` and `count_regex_wikilinks` are test-only reimplementations
of production logic. They exist nowhere in the public API. Remove test 5 and the two
helper functions entirely. If occurrence-counting is a product need, surface it as a
first-class production function with its own dedicated test file.

### Priority 2 — Add negative assertions to tests 2, 3, 4

Tests 3 and 4 claim to verify that "only valid" concepts are extracted but make no
negative assertions. Any extraction of malformed content that happens to not contain
bracket characters would be silently accepted. Add:
- `assert_eq!(concepts.len(), N)` exact-count guards
- `assert!(!concepts.contains(&"malformed-name"))` explicit rejections

### Priority 3 — Clarify or fix test 2's contract

Decide: does `[[[[quadruple]]]]` extract `quadruple` (clean) or extract nothing? Document
that decision in the test assertion. The current assertion is silent on this question.

### Priority 4 — Fill coverage gaps

The following inputs have no tests in this file or in the existing `wikilink_filter_*`
evaluations for the unit-test layer:

| Gap | Suggested test name |
|---|---|
| `[[]]` empty concept | `extract_wikilinks_ignores_empty_brackets` |
| `[[ concept ]]` whitespace trim | `extract_wikilinks_trims_whitespace_in_concept` |
| `[[A]] text [[A]]` deduplication | `extract_wikilinks_deduplicates_repeated_concept` |
| WikiLink inside `` ` `` code span | `extract_wikilinks_ignores_wikilink_in_inline_code` |
| WikiLink inside fenced code block | `extract_wikilinks_ignores_wikilink_in_fenced_code` |
| `[[concept\|alias]]` pipe alias | `extract_wikilinks_handles_pipe_alias_syntax` |

### Priority 5 — Parameterize test 6

When `rstest` is available, convert test 6 to a parameterized test and extend it with
additional normalization edge cases (spaces, slashes, mixed case).

---

## Load-Bearing Score: 3/6

| Test | Load-bearing |
|---|---|
| `extract_wikilinks_rejects_triple_bracket_pattern` | ✅ |
| `extract_wikilinks_rejects_quadruple_bracket_pattern` | ⚠️ Partial (assertion gap) |
| `extract_wikilinks_extracts_only_valid_concepts` | ✅ |
| `extract_wikilinks_handles_nested_brackets_correctly` | ⚠️ Partial (redundant with test 3) |
| `count_concept_occurrences_ignores_malformed_brackets` | ❌ Tests reimplementation |
| `extract_wikilinks_allows_hyphens_underscores_in_concepts` | ✅ |

**3 fully load-bearing / 2 partial / 1 dead weight (6 total).**

The partial tests (2 and 4) carry real intent but are under-asserted: a regression that
extracts malformed content into clean strings would pass them silently. They defend
against only the most obvious failure mode (bracket leakage) while missing the harder
failure mode (silent extraction of content from malformed patterns).

---

*Evaluation performed under 🔘 Gray — Skeptical Inquiry. Every assertion was questioned
for what it actually verifies vs. what it implies. Traceability: T-W1-3 · NFR-1 · NFR-2.*
