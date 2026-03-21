# Test Evaluation: content_preview_tests.rs

> **Traceability**: T-1.6 (RTM NFR-1 — C# oracle parity for `create_smart_preview`)
> **Evaluator**: Gray (Skeptical Inquiry)
> **File**: `crates/maenifold-core/tests/content_preview_tests.rs`
> **Date**: 2025-07

---

## Summary

This file contains **8 tests** covering `create_smart_preview`, which implements a
sentence-aware content truncation algorithm with four ordered fallback levels: sentence
boundary → paragraph boundary → word boundary → hard truncation. The function resides in
`crates/maenifold-core/src/graph/preview.rs` and is re-exported through two façades
(`graph/mod.rs` and `search/mod.rs`). Tests import it via `maenifold_core::search::create_smart_preview`.

The fallback hierarchy is structurally sound and each test exercises a distinct code path.
**However, the file has three meaningful quality failures:**

1. **One test is vacuously correct** (Test 6 — WikiLinks): the chosen parameters return the
   *entire input string* without truncation, making the "preservation" assertions trivially
   true. This test says nothing about whether wikilinks survive truncation.

2. **Three tests use under-constrained assertions** (Tests 3, 4, 7): `contains` and
   `ends_with` checks that pass even when the implementation returns subtly wrong output.
   Test 7 is the worst offender — it only checks the first two sentences while the result
   contains four, and it never asserts what is *excluded*.

3. **No coverage of the `tolerance` window boundary condition**: no test has a sentence
   ending that falls *below* `minimum_acceptable_boundary`, which is the only path that
   causes the sentence-fallback to fall through to paragraph or word. All sentence tests
   are written so the sentence boundary comfortably exceeds the minimum. The tolerance
   mechanism is implemented but unverified.

Other gaps: `!` and `?` as sentence terminators, Unicode content (the implementation
correctly uses `.chars().count()`, untested), `tolerance = 0`, `target_length = 0`, empty
input, and wikilinks that *straddle* a truncation boundary (the genuinely hard preservation
case).

**Net assessment**: 5 of 8 tests are load-bearing and would catch a regression in their
stated code path. 3 have assertion failures serious enough that a buggy refactor could slip
through silently.

---

## Verdicts

| # | Test Name | Verdict | Load-Bearing | Unique | Assertions Correct | Name Clear | Parameterizable |
|---|-----------|---------|:---:|:---:|:---:|:---:|:---:|
| 1 | `create_smart_preview_short_content_returns_full_content` | **KEEP** | ✅ | ✅ | ✅ | ✅ | Partial |
| 2 | `create_smart_preview_with_complete_sentence_truncates_at_sentence` | **KEEP** | ✅ | ✅ | ✅ | ✅ | No |
| 3 | `create_smart_preview_no_sentence_boundary_uses_paragraph` | **REWRITE** | ⚠️ | ✅ | ❌ | ⚠️ | Yes |
| 4 | `create_smart_preview_no_paragraph_boundary_uses_word_boundary` | **REWRITE** | ⚠️ | ✅ | ❌ | ⚠️ | Yes |
| 5 | `create_smart_preview_no_boundaries_hard_truncates` | **KEEP** | ✅ | ✅ | ✅ | ✅ | No |
| 6 | `create_smart_preview_with_wikilinks_preserves_links` | **REWRITE** | ❌ | ❌ | ❌ | ⚠️ | No |
| 7 | `create_smart_preview_multiple_short_sentences_truncates_at_natural_boundary` | **REWRITE** | ⚠️ | ✅ | ❌ | ⚠️ | Yes |
| 8 | `create_smart_preview_sentence_ending_at_limit_returns_full_sentence` | **KEEP** | ✅ | ✅ | ✅ | ✅ | No |

---

## Per-Test Analysis

---

### Test 1 — `create_smart_preview_short_content_returns_full_content`

**Lines**: 5–12 | **Verdict**: KEEP

#### What it does
Calls `create_smart_preview("Short content.", 200, 50)` where content (14 chars) is far
below the target (200). Asserts `result == content`.

#### Strengths
- Tests the unconditional early-return guard `if content_len <= target_length`. Without this
  test, a refactor that removes the guard would pass every other test (all other inputs
  exceed their target) while silently reprocessing short content through the fallback chain.
- Exact equality assertion — the strongest form possible here.
- Correctly exercises the `search` re-export path, incidentally verifying the dual façade
  (`graph/mod.rs` → `graph/preview.rs`) is not broken.

#### Issues / Gray Skepticism
- *Does this test prove the early return is* correct *or merely that it* exists*?* At 14
  chars vs. target 200, the tolerance floor `(200 − 50 = 150)` would also be far exceeded
  by any sentence boundary in the content. A test with `content_len == target_length`
  (exact boundary) would be more precise, but this test is still load-bearing as a
  regression guard on the guard itself.
- **Parameterizable with edge cases**: content_len == target, content_len == target + 1
  (enters the main path), and content_len == 0. None are covered elsewhere.

---

### Test 2 — `create_smart_preview_with_complete_sentence_truncates_at_sentence`

**Lines**: 14–21 | **Verdict**: KEEP

#### What it does
Content: `"First sentence. Second sentence. Third sentence that goes on and on."` (68 chars).
`target=30`, `tolerance=20`. Expected: `"First sentence. Second sentence."`.

Trace:
- `max_length = min(50, 68) = 50` → search space = first 50 chars
- `min_acceptable = 10`
- Last sentence boundary in search space: `'.'` at char index 31, next char is `' '` → `boundary = Some(31)`
- `31 >= 10` ✓ → return `truncate_chars(content, 32).trim_end()`
- Result: `"First sentence. Second sentence."`

#### Strengths
- Uses `assert_eq!` — the only exact equality assertion that checks the sentence-truncation
  path. This is the canonical test for the sentence fallback branch.
- Parameters are well-chosen: the search space contains two clean sentence boundaries, and
  the algorithm correctly selects the *later* one (exercising `last_index` update logic
  in `find_last_sentence_boundary`).

#### Issues / Gray Skepticism
- **Tolerance boundary not stressed**: `min_acceptable = 10`, but the sentence boundary is
  at `31`. No test exercises the case where a sentence boundary is found but falls below
  `min_acceptable` — i.e., the path where `find_last_sentence_boundary` returns `Some(x)`
  but `x < minimum_acceptable_boundary`, forcing fallthrough. This is the most fragile
  branch in the algorithm and remains untested.

---

### Test 3 — `create_smart_preview_no_sentence_boundary_uses_paragraph`

**Lines**: 23–34 | **Verdict**: REWRITE

#### What it does
Content: `"First paragraph without ending\n\nSecond paragraph here"` (53 chars).
`target=20`, `tolerance=15`. Expects the result to end with `"..."` and contain
`"First paragraph"`.

Trace:
- `max_length = min(35, 53) = 35` → search space = `"First paragraph without ending\n\nSec"`
- No sentence-ending punctuation → sentence branch skipped
- `rfind("\n\n")` at byte 30 → `paragraph_char_end = 30`
- `30 >= 5` ✓ → result = `"First paragraph without ending..."`

#### Why REWRITE
The actual result is deterministic: `"First paragraph without ending..."`. The test only
asserts `ends_with("...")` and `contains("First paragraph")`. Both assertions would also
pass if the result were `"First paragraph X..."` (extra content), or even
`"First paragraph without ending and more stuff..."` (a bug that ignores `\n\n`).

**Fix**: Replace both assertions with:
```rust
assert_eq!(result, "First paragraph without ending...");
```

#### Additional Issues
- **Name is misleading**: "no sentence boundary" — the content genuinely has no `.`, `!`,
  or `?`, which is correct. But the name implies the test is about the *paragraph* fallback
  specifically. The name should be `uses_paragraph_boundary_when_no_sentence_found` to
  clarify which code path is under test.
- The tolerance is so wide (`tolerance=15` means `min_acceptable=5`) that even a 5-char
  paragraph boundary would pass. This does not stress the floor.

---

### Test 4 — `create_smart_preview_no_paragraph_boundary_uses_word_boundary`

**Lines**: 36–47 | **Verdict**: REWRITE

#### What it does
Content: `"ThisIsAVeryLongWordWithNoSpaces FollowedByMore Words"` (52 chars).
`target=35`, `tolerance=10`.

Trace:
- `max_length = min(45, 52) = 45` → search space = `"ThisIsAVeryLongWordWithNoSpaces FollowedByMor"`
- No sentence boundaries, no `\n\n`
- `rfind(' ')` → last space at char index 31 (`word_end > 0` ✓)
- Result: `"ThisIsAVeryLongWordWithNoSpaces..."`

#### Why REWRITE
Assertions: `ends_with("...")` and `!result.contains("FollowedByMore")`. The negative
assertion is the stronger of the two, but neither confirms what is *actually* included.

A regression that returned `"ThisIsAVery..."` (truncating too early) would:
- `ends_with("...")` → ✅ pass
- `!contains("FollowedByMore")` → ✅ pass

The test would pass silently despite incorrect output.

**Fix**:
```rust
assert_eq!(result, "ThisIsAVeryLongWordWithNoSpaces...");
```

#### Additional Issues
- **Name drift**: "no_paragraph_boundary" is accurate — but the content also has no
  sentence boundary. The cascade being tested is sentence-miss → paragraph-miss →
  word-hit, yet only the final step is named.
- Exercises only `rfind(' ')` with one space. No test covers a content string with
  multiple spaces to verify `rfind` selects the *last* space in the search window.

---

### Test 5 — `create_smart_preview_no_boundaries_hard_truncates`

**Lines**: 49–60 | **Verdict**: KEEP

#### What it does
Content: `"NoSpacesOrPunctuationHereJustOneVeryLongString"` (46 chars).
`target=20`, `tolerance=5`. Expects `ends_with("...")` and `chars().count() <= 23`.

Trace:
- `max_length = min(25, 46) = 25`; no sentence, no paragraph, no space
- Falls through to: `format!("{}...", truncate_chars(content, 20))`
- Result: `"NoSpacesOrPunctuatio..."` (23 chars)

#### Strengths
- Tests the last-resort fallback exclusively. No other test in this file reaches the
  `format!("{}...", truncate_chars(content, target_length))` branch.
- The `<= 23` bound is arithmetically tight (`target + len("...") = 20 + 3 = 23`) and
  correctly reasons about the upper bound.

#### Issues / Gray Skepticism
- *Why `<= 23` instead of `== 23`?* The result is deterministic: 20 chars + `"..."` = 23.
  Using `==` would be more precise and would catch a regression where fewer chars are
  included. The `<=` passes vacuously if the implementation returns `"..."` (3 chars).
- **The `word_end > 0` guard is untested**: the implementation skips the word-boundary
  branch when `rfind(' ')` returns `Some(0)`. No test in this file contains a string where
  the only space is at position 0, so this guard's correctness is unverified.

---

### Test 6 — `create_smart_preview_with_wikilinks_preserves_links`

**Lines**: 62–76 | **Verdict**: REWRITE

#### What it does
Content: `"The [[authentication]] system uses [[JWT]] tokens. More content here."` (69 chars).
`target=60`, `tolerance=20`. Asserts `contains("[[authentication]]")` and
`contains("[[JWT]]")`.

#### Why REWRITE — This Test Is Vacuous

**The parameters cause the entire string to be returned.** Trace:
- `content_len = 69`, `target = 60` → truncation branch entered
- `max_length = min(80, 69) = 69` → search_space = entire content
- Last sentence boundary: `'.'` at char index 68 (end of `"here."`) → `Some(68)`
- `min_acceptable = 60 − 20 = 40`; `68 >= 40` ✓
- `return truncate_chars(content, 69).trim_end()` = full content

**No truncation occurs.** Both wikilinks are present because the full string is returned.
The assertions are trivially satisfied by *any* implementation that correctly handles the
early-sentence-boundary case — they say nothing about preservation *during truncation*.

**What this test should do**: set parameters so the truncation boundary falls *before*
`"[[JWT]]"` but *after* `"[[authentication]]"`, and assert that the sentence-aware
algorithm either includes both (by extending to the sentence boundary) or excludes both
(by cutting at a sentence boundary before them). The interesting property is whether a
wikilink that *starts* inside the truncation window but *ends* outside it is handled
gracefully — currently completely untested.

**Suggested rewrite** (forces truncation at "tokens." sentence boundary):
```rust
let content = "The [[authentication]] system uses [[JWT]] tokens. More content here.";
let result = create_smart_preview(content, 30, 20);
// sentence boundary at "tokens." (index 49) < max_length(50), >= min_acceptable(10)
assert!(result.contains("[[authentication]]"));
assert!(result.contains("[[JWT]]"));
assert!(!result.contains("More content"));
assert!(result.ends_with("tokens.") || result.ends_with("tokens. "));
```

---

### Test 7 — `create_smart_preview_multiple_short_sentences_truncates_at_natural_boundary`

**Lines**: 78–86 | **Verdict**: REWRITE

#### What it does
Content: `"First. Second. Third. Fourth. Fifth. Sixth. Seventh."` (52 chars).
`target=25`, `tolerance=10`. Asserts `contains("First.")` and `contains("Second.")`.

Trace:
- `max_length = min(35, 52) = 35` → search_space = `"First. Second. Third. Fourth. Fifth"`
- Sentence boundaries at indices: 5, 13, 20, 28 (all `>= min_acceptable = 15`)
- `last_index = Some(28)` → result = `"First. Second. Third. Fourth."`

#### Why REWRITE
The result is `"First. Second. Third. Fourth."` — 4 sentences. The test checks only that
`"First."` and `"Second."` are present. It does **not** assert:

1. That `"Third."` and `"Fourth."` are present (would catch an under-truncation bug)
2. That `"Fifth."`, `"Sixth."`, `"Seventh."` are absent (would catch an over-extension bug)

A regression that returned `"First. Second. Third. Fourth. Fifth."` (selecting the wrong
boundary) would still pass both assertions. The name says "truncates at natural boundary"
but the test doesn't verify *which* natural boundary.

**Fix**:
```rust
assert_eq!(result, "First. Second. Third. Fourth.");
```

Or if the exact value is intentionally soft:
```rust
assert!(result.contains("Fourth."), "should include up to Fourth: {result}");
assert!(!result.contains("Fifth."), "should not include Fifth: {result}");
```

---

### Test 8 — `create_smart_preview_sentence_ending_at_limit_returns_full_sentence`

**Lines**: 88–102 | **Verdict**: KEEP

#### What it does
Content: `"This is exactly fifty characters for testing now. And more after."` (65 chars).
`target=50`, `tolerance=10`. Asserts the first sentence is included and "And more" is not.

Trace:
- `max_length = min(60, 65) = 60`; sentence boundary at `'.'` after "now." = char index 48
- `min_acceptable = 40`; `48 >= 40` ✓
- Result: `"This is exactly fifty characters for testing now."`

#### Strengths
- Tests a near-boundary condition: the sentence ending (index 48) is within the
  `[min_acceptable=40, max_length=60]` window, which is the realistic scenario the
  algorithm is designed for.
- Uses both a positive assertion (`contains` the first sentence) and a negative assertion
  (`!contains` the overflow content) — the negative assertion provides meaningful regression
  value.
- The comment in the content ("exactly fifty characters") is informative. Counting reveals
  the first sentence is 49 chars (the content author appears to have miscounted by one),
  but the test logic is unaffected by this.

#### Issues / Gray Skepticism
- **`contains` instead of `==`**: the positive assertion would pass even if the result
  appended extra content. `assert_eq!` is tighter and this is a deterministic case.
- **`"And more"` is not the exact sentence**: the negative assertion checks for `"And more"`
  but the second sentence is `"And more after."`. While correct, checking for
  `"And more after."` would be more precise.

---

## Missing Coverage

The following behaviors are exercised by the implementation but have **zero tests**:

| Missing Case | Risk | Code Path |
|---|---|---|
| `!` and `?` as sentence terminators | Medium — `find_last_sentence_boundary` handles these, but only `.` is used in tests | `find_last_sentence_boundary` lines 59–68 |
| Sentence boundary falls **below** `min_acceptable` (forced fallthrough) | **High** — the tolerance mechanism's core purpose is never stress-tested | `if sentence_end >= minimum_acceptable_boundary` branch |
| `content_len == target_length` exact boundary | Low — trivially returns full content, but edge case of early return | early-return guard |
| `target_length = 0` | Medium — produces `""...` (3 chars) via hard-truncation branch | `truncate_chars(content, 0)` |
| `tolerance = 0` | Medium — `min_acceptable == target`, any boundary exactly at target passes | tolerance window |
| `word_end == 0` guard (`if word_end > 0`) | Low — only possible if content starts with a space; code falls through to hard truncation | word-boundary branch |
| Unicode / multibyte content | Medium — impl uses `.chars().count()` (correct), but no test exercises CJK or emoji inputs | `truncate_chars` char-based indexing |
| **Wikilink straddling truncation boundary** | **High** — a wikilink like `[[long-concept-name]]` that *starts* inside but *ends* outside the window could produce malformed `[[long...` output | All truncation paths |
| Multiple paragraphs, paragraph below `min_acceptable` | Medium — only one `\n\n` test exists and the tolerance is wide | paragraph-boundary condition |

---

## Recommendations

1. **Fix Test 6 immediately** (vacuous). Change parameters so truncation actually fires
   and the wikilink is near the boundary. Add a companion test for a wikilink that
   *straddles* the truncation point — the current implementation will silently split
   a wikilink like `[[long-name]]` if the boundary falls inside it.

2. **Tighten assertions on Tests 3, 4, 7** to use `assert_eq!` rather than `contains`.
   All three produce deterministic output; there is no reason to use soft assertions.

3. **Add a sentence-boundary-below-minimum test** — this is the most critical missing
   case. Without it, a bug in the `if sentence_end >= minimum_acceptable_boundary`
   branch could cause fallthrough to skip paragraph/word boundaries entirely.

4. **Add `!` and `?` coverage** — one test each. The C# oracle tests presumably covered
   these; they were dropped in the port.

5. **Add `tolerance = 0` and `target_length = 0` edge cases** to a dedicated boundary
   test, or fold them into a parameterized structure.

6. **Consider parameterization** for Tests 3, 4, 7 — these all follow the same shape
   (input → `create_smart_preview(content, target, tolerance)` → assert result). A
   `#[rstest]` or manual table-driven loop would reduce boilerplate and make coverage
   gaps visible by inspection.

---

## Load-Bearing Score: 5/8

Tests 1, 2, 5, and 8 reliably detect regressions in their stated code paths with
adequate assertion strength. Test 7 covers the correct code path but would miss
boundary-selection bugs. Tests 3 and 4 cover unique code paths (paragraph and word
fallbacks) but their weak assertions allow wrong outputs to pass silently. Test 6
contributes no meaningful coverage — it is a vacuous test that must be rewritten before
it earns load-bearing status.

> **Tags**: [[test-quality]], [[maenifold-rs]], [[content-preview]], [[create_smart_preview]]

---
## SWE Remediation
- REWRITE test 3 (no_sentence_boundary_uses_paragraph): `assert_eq!` replaces `contains`+`ends_with` per P0
- REWRITE test 4 (no_paragraph_boundary_uses_word_boundary): `assert_eq!` replaces `contains`+`ends_with` per P0
- REWRITE test 6 (with_wikilinks_preserves_links): fixed vacuous test — params changed from target=60 to target=30 so truncation fires; exact `assert_eq!` plus positive/negative assertions per P0
- REWRITE test 7 (multiple_short_sentences): `assert_eq!` replaces partial `contains` per P0
- NEW sentence_below_minimum_falls_through_to_word: covers sentence boundary below `min_acceptable` forcing fallthrough (P1 Rec 3)
- NEW exclamation_as_sentence_boundary: covers `!` terminator in `find_last_sentence_boundary` (P1 Rec 4)
- NEW question_mark_as_sentence_boundary: covers `?` terminator in `find_last_sentence_boundary` (P1 Rec 4)
- NEW zero_tolerance_uses_word_boundary: covers `tolerance=0` edge case (P2 Rec 5)
- NEW zero_target_produces_ellipsis_only: covers `target_length=0` edge case (P2 Rec 5)
- Tests: 13/13 pass (was 8, added 5)
- Commit: f0155fa
