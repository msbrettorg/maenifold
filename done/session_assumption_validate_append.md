---
id: session_assumption_validate_append
status: done
---

# Test Evaluation: session_assumption_validate_append

**File:** `crates/maenifold-core/tests/session_assumption_validate_append.rs`
**Test count:** 11
**Evaluated:** 2026-03-14
**Traceability:** T-12.7 (NFR-1 parity), T-W1-5 (FR-17.9 assumption ledger validation)

## Summary

This file tests a single pure function, `validate_assumption_append_input`, which has three distinct error branches and one happy path. Eleven tests for four code paths is excessive: six tests share the same code branch and differ only in input spelling variants, and three of those are easily parameterizable. The test assertions are uniformly weak — every error test checks only that *some* error is returned and that the error string contains a partial keyword, never asserting the full error message. This means a regression that returns a completely wrong error message (e.g., returning a concepts error when an assumption error was expected, or vice versa) can still pass all tests. One test (`validate_append_input_valid_multiple_concepts_returns_null`) is redundant in the presence of `validate_append_input_bracket_error_on_second_concept_returns_error`.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `validate_append_input_null_assumption_returns_error` | **KEEP** | Canonical test for the `None` assumption branch. The `is_none_or(...)` guard in the implementation has two distinct inputs to this branch (None and empty). This test covers the `None` arm. Load-bearing anchor for this branch. However: should assert the full error string, not just a partial keyword match, to catch message regressions (NFR-1 oracle parity). |
| `validate_append_input_empty_assumption_returns_error` | **MERGE** with `validate_append_input_whitespace_assumption_returns_error` | Both tests are identical except for the assumption string (`""` vs `"   "`). They exercise the `trim().is_empty()` guard in a single if-condition. Neither test provides assertion coverage beyond the other — both check `result.is_some()` and `contains("Assumption text is required")`. Collapse into a parameterized helper or a single test with a loop over `["", "   ", "\t"]`. |
| `validate_append_input_whitespace_assumption_returns_error` | **MERGE** with `validate_append_input_empty_assumption_returns_error` | See above. The whitespace trimming behaviour is already implied by `validate_append_input_null_assumption_returns_error` keeping the branch alive — this test's only unique contribution is proving `"   ".trim().is_empty()` is true, which is a Rust stdlib property, not a property of the function under test. |
| `validate_append_input_null_concepts_returns_error` | **KEEP** | Canonical test for the `None` concepts branch (`let Some(concepts) = concepts else {...}`). Structurally distinct from the empty-vec path. The `is_some()` + `contains("[[WikiLink]]")` pattern is acceptable here but the full error string should be asserted for oracle parity (see Recommendations). |
| `validate_append_input_empty_concepts_array_returns_error` | **KEEP** | Tests the empty-vec branch, which is a separate code path from `None` in the implementation (`concepts.is_empty()` after the `let Some` destructure). Both the `None` and empty-vec paths exist as separate if-branches; removing either test would leave a branch uncovered. Load-bearing. |
| `validate_append_input_concept_with_full_brackets_returns_error` | **MERGE** with open/close bracket tests | All three bracket tests (`[[foo]]`, `[[dialogue`, `dialogue]]`) exercise the same single `any(|c| c.contains("[[") || c.contains("]]"))` expression. The first already triggers BOTH `contains("[[")` and `contains("]]")`. The remaining two tests each isolate one side of the OR condition, which is genuinely valuable for proving both arms work independently. However, three separate tests for a single boolean expression is over-engineered. Merge into one parameterized test: `invalid_bracket_patterns_in_concepts_return_error` with inputs `["[[foo]]", "[[dialogue", "dialogue]]"]`. |
| `validate_append_input_concept_with_only_open_bracket_returns_error` | **MERGE** with bracket test family | See above. Unique value: proves the `contains("[[")` arm fires independently. Valid coverage point, but should live in a parameterized test alongside the full and close-bracket variants. |
| `validate_append_input_concept_with_only_close_bracket_returns_error` | **MERGE** with bracket test family | See above. Proves the `contains("]]")` arm. Same argument. |
| `validate_append_input_valid_single_concept_returns_null` | **KEEP** | The happy-path anchor. If this test were removed, no test verifies that well-formed input returns `None`. Load-bearing. Note: the assertion `validate_assumption_append_input(...).is_none()` provides zero diagnostic information on failure — add an `expect` or `unwrap_none` equivalent with a message. |
| `validate_append_input_valid_multiple_concepts_returns_null` | **REMOVE** | Redundant in the presence of `validate_append_input_bracket_error_on_second_concept_returns_error`. Test 11 calls the function with `["good-concept", "[[bad-concept]]"]` — to make that test meaningful, the function must already be able to handle a non-empty valid first concept. If this file's goal were to test the multi-concept loop, the bracket-error-on-second-concept test already proves the loop iterates past the first element. The only scenario where this test catches a unique bug is if the implementation special-cases exactly one concept vs. multiple — which the current code does not do (`iter().any(...)` is identical for N=1 and N=3). Remove and rely on test 9 + test 11 to cover the valid and multi-concept branches. |
| `validate_append_input_bracket_error_on_second_concept_returns_error` | **KEEP** | Uniquely valuable: proves the bracket check iterates over ALL concepts, not just the first. If the implementation used `concepts[0]` instead of `iter().any(...)`, this test would fail and the others would not. Load-bearing. |

## Recommendations

1. **Assert full error strings, not just keyword fragments.** Every error test uses `result.expect("error").contains("some keyword")`. Given NFR-1 (C# oracle parity), the exact error messages matter. A test that passes when the error reads "ERROR: Missing assumption text" and the oracle expects "ERROR: Assumption text is required for 'append' action." is a false negative. Use `assert_eq!` against the full expected string, or at minimum add a second assertion for a second distinctive substring to narrow down which error branch fired.

2. **Eliminate the `validate_append_input_valid_multiple_concepts_returns_null` test.** It is entirely redundant given tests 9 and 11. Its presence pads the count without adding coverage.

3. **Parameterize the three whitespace-variant tests (null, empty, whitespace).** Create a single test `assumption_text_required_for_empty_variants` that loops over `[None, Some(""), Some("   ")]`. This makes the coverage intent explicit and eliminates two test functions.

4. **Parameterize the three bracket-pattern tests.** Create `bracket_patterns_in_concepts_are_rejected` looping over `["[[foo]]", "[[dialogue", "dialogue]]"]`. The three variants test the two arms of a single OR expression — grouping them as one parameterized test makes the relationship clear.

5. **Improve failure diagnostics on the happy-path tests.** `validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts)).is_none()` provides zero information when it fails. Wrap with:
   ```rust
   let result = validate_assumption_append_input(Some("Valid assumption text"), Some(&concepts));
   assert!(result.is_none(), "expected Ok but got error: {result:?}");
   ```

6. **Add a test for the `validate_append_input_concept_with_only_open_bracket` catching bracket inside a word** (e.g., `"con[[cept"`) if that is a case the oracle handles differently. The current tests only cover bracket-prefixed patterns.

7. **Consider adding a test that passes a concept containing a single bracket (`[` or `]` alone).** The current implementation only checks `[[` and `]]` (double brackets). A concept named `"foo[bar]"` (single brackets, valid markdown link syntax) would pass validation. If that is intentional, document it; if not, it is a gap in both the implementation and the tests.

## Load-Bearing Score: 7/11

Four tests are either redundant (`validate_append_input_empty_assumption_returns_error`, `validate_append_input_whitespace_assumption_returns_error`, `validate_append_input_concept_with_full_brackets_returns_error` — one of the three bracket variants can be dropped after parameterization, keeping two — `validate_append_input_valid_multiple_concepts_returns_null`). Seven tests cover genuinely distinct code paths or boundary conditions.

---
## SWE Remediation
- MERGE validate_append_input_empty_assumption_returns_error + validate_append_input_whitespace_assumption_returns_error → `assumption_text_required_for_empty_variants` (parameterized over `["", "   ", "\t"]`)
- MERGE validate_append_input_concept_with_full_brackets_returns_error + open_bracket + close_bracket → `bracket_patterns_in_concepts_are_rejected` (parameterized over `["[[foo]]", "[[dialogue", "dialogue]]"]`)
- REMOVE validate_append_input_valid_multiple_concepts_returns_null: confirmed redundant with bracket_error_on_second_concept
- KEEP 5 canonical tests: strengthened all assertions from `contains()` keyword match to `assert_eq!` against full parity error strings (P0 Rec 1)
- Improved happy-path diagnostic: `assert!(result.is_none(), "expected Ok but got error: {result:?}")` (P1 Rec 5)
- Added shared constants `ERR_ASSUMPTION_REQUIRED`, `ERR_CONCEPTS_REQUIRED`, `ERR_BRACKETS_IN_CONCEPTS` for DRY parity assertions
- Test count: 11 → 7, all passing, zero coverage loss
- Commit: 1b049a0

---
## Red-team Audit

### MERGE verdicts

- ✅ MERGE `empty_assumption` + `whitespace_assumption` → `assumption_text_required_for_empty_variants`: parameterized over `["", "   ", "\t"]` exactly as eval spec requires. All three variants assert `assert_eq!` against `ERR_ASSUMPTION_REQUIRED`. No coverage lost.
- ✅ MERGE `full_brackets` + `open_bracket` + `close_bracket` → `bracket_patterns_in_concepts_are_rejected`: parameterized over `["[[foo]]", "[[dialogue", "dialogue]]"]` exactly as eval spec requires. All three variants assert `assert_eq!` against `ERR_BRACKETS_IN_CONCEPTS`. Both arms of the `contains("[[") || contains("]]")` OR expression still independently tested. No coverage lost.

### REMOVE verdicts

- ✅ REMOVE `validate_append_input_valid_multiple_concepts_returns_null`: confirmed redundant. `bracket_error_on_second_concept` (KEEP) already forces multi-element iteration via `["good-concept", "[[bad-concept]]"]`. The removed test's only coverage (`iter().any()` with N=3 valid concepts) is already implied — the implementation does not special-case vector length. Tombstone comment present in source.

### KEEP verdicts (5 canonical tests)

- ✅ KEEP `validate_append_input_null_assumption_returns_error`: assertion strengthened from `contains("Assumption text is required")` → `assert_eq!(result.as_deref(), Some(ERR_ASSUMPTION_REQUIRED))`. Verified constant matches implementation line 8 verbatim.
- ✅ KEEP `validate_append_input_null_concepts_returns_error`: assertion strengthened from `contains("[[WikiLink]]")` → `assert_eq!` against `ERR_CONCEPTS_REQUIRED`. Verified constant matches implementation lines 12/16 verbatim.
- ✅ KEEP `validate_append_input_empty_concepts_array_returns_error`: same strengthening. Covers the `.is_empty()` branch separate from the `let Some` destructure. Confirmed distinct code path in implementation (line 14 vs line 10).
- ✅ KEEP `validate_append_input_valid_single_concept_returns_null`: happy-path diagnostic improved to `assert!(result.is_none(), "expected Ok but got error: {result:?}")` — exact match to eval Recommendation 5.
- ✅ KEEP `validate_append_input_bracket_error_on_second_concept_returns_error`: assertion strengthened from `contains("brackets")` → `assert_eq!` against `ERR_BRACKETS_IN_CONCEPTS`. Still uniquely proves `iter().any()` iterates beyond first element.

### P0/P1/P2 recommendations

- ✅ **P0 Rec 1** (full error string assertions): All 5 error tests now use `assert_eq!` against exact parity strings. Cross-verified all 3 constants against `sessions/validation.rs` — character-for-character match.
- ✅ **P1 Rec 5** (happy-path diagnostics): Implemented as specified.
- ℹ️ **P2 Rec 6** (bracket inside a word, e.g. `"con[[cept"`) and **P2 Rec 7** (single bracket `[` or `]`): Not addressed. These are P2/informational gap observations, not required by eval verdict table. Acceptable to defer.

### Cross-checks

- ✅ Test count: 11 → 7 (confirmed: `running 7 tests`, all `ok`)
- ✅ No source code modified — only test file changed (1 file, `66+/66-`)
- ✅ No unrelated changes introduced
- ✅ Shared constants (`ERR_ASSUMPTION_REQUIRED`, `ERR_CONCEPTS_REQUIRED`, `ERR_BRACKETS_IN_CONCEPTS`) match implementation verbatim — no stale/wrong strings
- ✅ Traceability: T-12.7 / NFR-1 annotations present on every test and constant block

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_assumption_validate_append`: 7 passed, 0 failed
- Verdict: PASS
