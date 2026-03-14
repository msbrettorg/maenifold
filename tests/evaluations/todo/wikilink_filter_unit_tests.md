# Test Evaluation: wikilink_filter_unit_tests

**File:** `crates/maenifold-core/tests/wikilink_filter_unit_tests.rs`
**Test count:** 6
**Evaluated:** 2026-07-14
**Traceability:** T-10.16 (FR-19, RTM-102, RTM-103, RTM-104)

## Summary

This file is the original C# → Rust port of `WikiLinkFilter` unit coverage (T-10.16). Since its creation, a parallel `#[cfg(test)]` module has grown inside `filter.rs` itself — now containing 10 inline tests that cover more behavior, including prefix patterns (T-15.24), symlink rejection, and empty-normalization edge cases. That growth has rendered half of this external file redundant.

Three critical observations drive this evaluation:

1. **Significant inline duplication.** Tests 1 and 2 are directly superseded by inline tests in `filter.rs`. Keeping them creates maintenance surface without adding coverage.

2. **The helper abstraction creates a hidden asymmetry.** The `check_filter()` helper wraps `try_check()` and `.expect()`s on the result — meaning it silently panics if the JSON is invalid rather than returning a controlled failure. This makes the helper *structurally unsuitable* for testing the fail-open `check()` vs. fail-strict `try_check()` behavioral contract. Test 6 correctly bypasses the helper for exactly this reason — but nothing in the file warns the reader.

3. **One test has an actively misleading name** (Test 5). "Empty concepts list" implies the filter file is empty; in reality, the filter is populated and the *content string* is empty. A future maintainer editing test 5 may confidently misread what is being exercised.

Three tests carry genuine, unique load and have no counterpart in the inline module. Three do not.

## Verdicts

| # | Test Function | Verdict | Reason |
|---|---|---|---|
| 1 | `check_filter_with_filter_file_blocks_matching_concepts` | **MERGE** | Strict subset of Test 4. Asserts 1 blocked + 1 not-blocked; Test 4 asserts 2 blocked + 2 not-blocked + `len() == 2`. No coverage gap if removed and Test 4 is kept. Fold the RTM-102/103 traceability comment into Test 4's header. |
| 2 | `check_filter_missing_file_returns_empty` | **REMOVE** | Exact behavioral duplicate of inline `wikilink_filter_allows_content_when_filter_file_is_missing` in `filter.rs`. Same setup (no `.wikilink-filter.json`), same assertion (`Ok(())`). Only difference is this file uses the `check_filter` helper wrapper — which adds no coverage value. |
| 3 | `check_filter_case_insensitive_matching` | **KEEP** | Unique. Tests normalization of the JSON *key* (`"Tool"` → `"tool"`) during filter load, not just the wikilink text. Inline tests all use lowercase keys and mixed-case wikilinks — they test link-side normalization. This tests key-side normalization; a distinct code path in `load_blocked_concepts()`. If removed, a regression in key normalization would go undetected. Rename to `check_filter_json_key_is_normalized_to_lowercase` for precision. |
| 4 | `check_filter_mixed_filtered_and_unfiltered` | **KEEP** | Most load-bearing test in the file. Asserts exact count (`len() == 2`), both positive matches (blocked with correct reasons), and both negative matches (unblocked concepts absent). No inline test checks unblocked concept absence — inline `wikilink_filter_blocks_normalized_concepts_with_reasons` only asserts what IS blocked. This is the only test providing false-positive guard coverage. |
| 5 | `check_filter_empty_concepts_list_returns_empty` | **REWRITE** | The behavior is worth testing (populated filter + zero wikilinks in content → `Ok(())`), but the name is actively misleading. "Empty concepts list" implies the filter/concepts list is empty — it is not (the filter has `{"tool":..., "agent":...}`). What is empty is the content string, which produces no wikilinks to check. A reader following the name will misidentify what code path is exercised. Rename to `check_filter_no_wikilinks_in_content_returns_ok`. |
| 6 | `check_filter_invalid_json_returns_empty` | **KEEP** | Unique. The only test in this file — or anywhere in the test suite — that exercises `check()` (not `try_check()`) against invalid JSON. It is testing the fail-open contract: `check()` swallows parse errors and returns `Ok(())`; `try_check()` propagates them. The inline `wikilink_filter_returns_error_for_invalid_json_in_try_check` tests the opposite side. These two together document the full behavioral contract of both methods. Add a comment naming the `check()` vs. `try_check()` distinction explicitly so the asymmetry with the `check_filter()` helper is visible. |

## Recommendations

### 1. Remove Test 2 — it is a maintenance liability with no payoff
`check_filter_missing_file_returns_empty` has an exact counterpart in `filter.rs`. Two tests for the same behavior using different helper layers means two tests to update when behavior changes. Remove this one; the inline test is closer to the production code and easier to reason about.

### 2. Merge Test 1 into Test 4, preserve its RTM comment
`check_filter_with_filter_file_blocks_matching_concepts` adds nothing beyond what `check_filter_mixed_filtered_and_unfiltered` already asserts. Before deletion, move the `RTM-102 / RTM-103` traceability comment to Test 4's header so the requirement link is not orphaned.

### 3. Rename Test 5 to prevent misreading
```rust
// Before
fn check_filter_empty_concepts_list_returns_empty()

// After
fn check_filter_no_wikilinks_in_content_returns_ok()
```
Update the comment from "RTM FR-19 / RTM-103" to also note: "populated filter, empty content string — no wikilinks extracted → Ok(())".

### 4. Annotate Test 6 with the `check()` vs `try_check()` contract
The helper `check_filter()` wraps `try_check()` and `.expect()`s on it — it cannot test `check()`'s fail-open path. Test 6 correctly bypasses the helper. Make this explicit:
```rust
// check() — not try_check() — is called here intentionally.
// check() has fail-open semantics: parse errors return Ok(()) rather than propagating.
// See also: wikilink_filter_returns_error_for_invalid_json_in_try_check (filter.rs)
// for the fail-strict counterpart.
```

### 5. Document the helper's scope limitation at the top of the file
The `check_filter()` helper wraps `try_check()` with `.expect()`. Add a comment:
```rust
// NOTE: check_filter() calls try_check() and panics on parse errors.
// Tests that require check()'s fail-open semantics must call WikiLinkFilter directly.
// See check_filter_invalid_json_returns_empty for an example.
```

### 6. Consider whether this file should exist at all
Now that `filter.rs` has a robust inline test module (10 tests) covering prefix patterns, symlink rejection, empty normalization, and both API surfaces, the question is whether an external integration test file adds value that inline unit tests do not. After removing Tests 1 and 2, only 4 tests remain — all addressing scenarios the inline module does not. A smaller, precisely-scoped file is fine; just confirm there is no plan to eventually consolidate into the inline module.

### 7. Gap not addressed by either file
`is_concept_blocked()` — a public method added in T-22.1 (RTM-151, RTM-152, RTM-155) — has **zero test coverage** in this file or in the inline module. It handles both exact-match and prefix-scan logic independently of `check()`/`try_check()`. At minimum, add:
- `is_concept_blocked_returns_true_for_exact_match`
- `is_concept_blocked_returns_true_for_prefix_match`
- `is_concept_blocked_returns_false_when_no_match`
- `is_concept_blocked_returns_false_when_filter_file_missing`

## Load-Bearing Score: 3/6

Tests 3, 4, and 6 carry coverage that is not replicated elsewhere in the test suite. Tests 1, 2, and 5 either duplicate inline tests or are strict subsets of another test in the same file. Two of the three non-load-bearing tests (1 and 2) should be removed outright. The third (5) should be renamed to eliminate its misleading framing before it causes a misunderstanding.

---
## SWE Remediation
- MERGE test 1 (`check_filter_with_filter_file_blocks_matching_concepts`): RTM-102/103 traceability folded into test 4 header, test 1 deleted.
- REMOVE test 2 (`check_filter_missing_file_returns_empty`): confirmed exact dup of inline `wikilink_filter_allows_content_when_filter_file_is_missing`.
- KEEP+RENAME test 3: `check_filter_case_insensitive_matching` → `check_filter_json_key_is_normalized_to_lowercase`. Added key-normalization comment.
- KEEP test 4 (`check_filter_mixed_filtered_and_unfiltered`): added merged traceability comment from test 1.
- REWRITE test 5: `check_filter_empty_concepts_list_returns_empty` → `check_filter_no_wikilinks_in_content_returns_ok`. Updated comment to clarify populated filter + empty content.
- KEEP+ANNOTATE test 6 (`check_filter_invalid_json_returns_empty`): added `check()` vs `try_check()` contract comment and cross-reference to inline counterpart.
- Rec 5 applied: documented `check_filter()` helper scope limitation at top of file.
- Rec 7 applied: added 4 `is_concept_blocked()` gap tests (exact match, prefix match, no match, missing file).
- Test count: 6 → 8 (−2 removed, +4 new gap coverage). All 8 pass.
- Commit: fa1ee1b

---
## Red-team Audit

**Traceability:** T-10.16 (FR-19, RTM-102, RTM-103, RTM-104), T-22.1 (RTM-151, RTM-152, RTM-155)

| Eval Item | Status | Detail |
|---|---|---|
| MERGE test 1 → test 4 | ✅ | `check_filter_with_filter_file_blocks_matching_concepts` deleted. RTM-102/103 traceability folded into test 4 header comment. Test 4 retains all original assertions (`len()==2`, both blocked with reasons, both unblocked absent). No coverage lost. |
| REMOVE test 2 | ✅ | `check_filter_missing_file_returns_empty` deleted. Confirmed exact behavioral dup of inline `wikilink_filter_allows_content_when_filter_file_is_missing` (filter.rs:188). Both exercise `try_check()` with no filter file → `Ok(())`. |
| KEEP+RENAME test 3 | ✅ | Renamed to `check_filter_json_key_is_normalized_to_lowercase`. Added key-normalization comment matching eval Rec 3. Body unchanged — still tests `"Tool"` JSON key normalizing to `"tool"`. |
| KEEP test 4 | ✅ | `check_filter_mixed_filtered_and_unfiltered` unchanged body. Header now carries merged RTM-102/103 provenance per eval Rec 2. |
| REWRITE test 5 | ✅ | Renamed to `check_filter_no_wikilinks_in_content_returns_ok`. Comment updated to: "Populated filter, empty content string — no wikilinks extracted → Ok(())." Matches eval Rec 3 exactly. Body unchanged. |
| KEEP+ANNOTATE test 6 | ✅ | Full `check()` vs `try_check()` contract annotation added (4-line comment block). Cross-references inline counterpart `wikilink_filter_returns_error_for_invalid_json_in_try_check`. Matches eval Rec 4 verbatim. |
| Rec 5 (helper scope) | ✅ | Lines 8–10 document `check_filter()` wraps `try_check()` + `.expect()`, warning that fail-open tests must bypass the helper. |
| Rec 7 (gap coverage) | ✅ | All 4 `is_concept_blocked()` tests added: exact match, prefix match (`"t-*"` pattern), no match, missing file. Each uses `WikiLinkFilter::new()` directly. Traceability: T-22.1, RTM-151/152/155. |

**Test execution:** `cargo test --test integration_tests -p maenifold-core -- wikilink_filter_unit_tests` → **8 passed, 0 failed** (6 original − 2 removed + 4 new gap coverage).

**Unrelated changes:** None in the test file. Commit also renamed `AGENTS.md → DISPATCH.md` which is outside this eval's scope and does not affect correctness.

- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- wikilink_filter_unit_tests`: 8 passed, 0 failed
- All 8 tests confirmed:
  - `check_filter_json_key_is_normalized_to_lowercase` (kept+renamed from test 3)
  - `check_filter_mixed_filtered_and_unfiltered` (kept, test 4 with merged RTM-102/103)
  - `check_filter_no_wikilinks_in_content_returns_ok` (rewritten from test 5)
  - `check_filter_invalid_json_returns_empty` (kept+annotated, test 6)
  - `is_concept_blocked_returns_true_for_exact_match` (new gap coverage)
  - `is_concept_blocked_returns_true_for_prefix_match` (new gap coverage)
  - `is_concept_blocked_returns_false_when_no_match` (new gap coverage)
  - `is_concept_blocked_returns_false_when_filter_file_missing` (new gap coverage)
- Verdict: **PASS**
