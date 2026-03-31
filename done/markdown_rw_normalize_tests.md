---
id: markdown_rw_normalize_tests
status: done
---

# Test Evaluation: markdown_rw_normalize_tests

**File:** `crates/maenifold-core/tests/markdown_rw_normalize_tests.rs`
**Test count:** 10
**Evaluated:** 2026-03-14
**Traceability:** T-12.7.1 / RTM NFR-1
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions.

---

## Summary

These 10 tests exercise `normalize_concept`, a pure deterministic function of ~20 lines. The function has exactly 5 behavioral properties: trim, lowercase, map `{' ', '_', '/'}` → `'-'`, deduplicate consecutive hyphens, and trim leading/trailing hyphens. Nine of the ten tests are load-bearing and cover distinct input classes. One test (`many_consecutive_hyphens`) is **unconditionally redundant** with its neighbor (`triple_hyphens`) — the implementation has no count-sensitive logic that could make 10 hyphens behave differently from 3.

---

## Implementation Under Test

```rust
pub fn normalize_concept(concept: &str) -> String {
    // trim → lowercase → map {' '|'_'|'/'} to '-' → dedup '-' → trim_matches('-')
}
```

Five distinct behavioral properties. The test suite covers all five. The only criticism is naming and one true redundancy.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `normalize_concept_double_space_compresses_to_single_hyphen` | KEEP | Unique: covers multi-space deduplication AND lowercasing together. Load-bearing. |
| `normalize_concept_double_underscore_compresses_to_single_hyphen` | KEEP | Unique: verifies underscore (`_`) mapping specifically — one of the three distinct separator chars. |
| `normalize_concept_multiple_slashes_compresses_to_single_hyphen` | KEEP + RENAME | Load-bearing, but **name is wrong**: input is `"auth / / token"` (spaces AND slashes). Name implies slash-only. Rename to `normalize_concept_slash_and_space_mix_deduplicates_to_single_hyphen`. |
| `normalize_concept_mixed_separators_compresses_to_single_hyphen` | KEEP | Unique: exercises all three separator types (`' '`, `'_'`, `'/'`) simultaneously. Best single test of the full separator map. |
| `normalize_concept_leading_trailing_separators_trims_hyphens` | KEEP + RENAME | Load-bearing (covers `.trim_matches('-')`), but name is ambiguous. Input is leading/trailing **whitespace**, which becomes hyphens, which are then trimmed. Rename to `normalize_concept_leading_trailing_spaces_produce_trimmed_result`. |
| `normalize_concept_only_separators_returns_empty` | KEEP | Unique edge case: all-separator input returns `""`. Tests `.trim_matches('-')` exhausting the whole string. |
| `normalize_concept_single_separators_works_as_expected` | KEEP + RENAME | Load-bearing (single spaces → single hyphens, no dedup path exercised). But **`works_as_expected` is a non-name** — rename to `normalize_concept_single_spaces_produce_hyphen_delimited_lowercase`. |
| `normalize_concept_no_separators_works_as_expected` | KEEP + RENAME | Load-bearing (pure lowercase path, no separator logic at all). **`works_as_expected` is a non-name** — rename to `normalize_concept_no_separators_returns_lowercased_input`. |
| `normalize_concept_triple_hyphens_compresses_to_single_hyphen` | KEEP | Unique: verifies that **already-present hyphens** (not mapped from other chars) are also deduplicated. Non-obvious property. |
| `normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen` | **REMOVE** | **Unconditionally redundant with `triple_hyphens`.** The implementation has no count-sensitive logic — a run of 10 hyphens cannot produce a different result than a run of 3. Removing this test cannot allow any real bug to ship. |

---

## Deeper Concerns

### 1. All 10 tests could be 1–2 parameterized tests

The entire file is a set of `(input, expected_output)` pairs over the same pure function. With `rstest` or a simple macro:

```rust
#[rstest]
#[case("API  Gateway", "api-gateway")]
#[case("user__profile", "user-profile")]
#[case("auth / / token", "auth-token")]
// ... etc.
fn normalize_concept_cases(#[case] input: &str, #[case] expected: &str) {
    assert_eq!(normalize_concept(input), expected);
}
```

This would reduce ~60 lines to ~15, make it trivial to add new cases, and eliminate the naming problem entirely. The current structure requires 10 separate test functions that all do `assert_eq!(normalize_concept(X), Y)` — this is exactly what parameterized tests were designed for.

### 2. No test covers special characters

The function maps `' '`, `'_'`, `'/'` but passes **all other characters through unchanged**. No test verifies what happens with characters like `!`, `@`, `-` pre-existing in the middle of the string, Unicode letters, digits, or numbers. Is `normalize_concept("user-123")` expected to return `"user-123"` (preserved) or something else? The implementation says yes (other chars pass through), but there is zero test coverage for this.

### 3. No test covers empty input

`normalize_concept("")` is untested. The function trims and returns — result should be `""`. This is a trivially reachable input (empty WikiLink `[[]]` would produce it) and the behavior should be pinned.

---

## Recommendations

1. **REMOVE** `normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen` — it's identical in behavioral coverage to `normalize_concept_triple_hyphens_compresses_to_single_hyphen`.
2. **RENAME** four tests: `multiple_slashes`, `leading_trailing_separators`, `single_separators_works_as_expected`, `no_separators_works_as_expected` — names don't describe the actual input or property under test.
3. **ADD** test: `normalize_concept_empty_input_returns_empty` — verify `""` → `""`.
4. **ADD** test: `normalize_concept_digits_and_passthrough_chars_preserved` — verify `"user-123"` → `"user-123"` to pin the passthrough behavior.
5. **CONSIDER** collapsing all cases into a single `rstest`-parameterized test — the current structure has high ceremony-to-coverage ratio.

---

## Load-Bearing Score: 9/10

9 out of 10 tests are genuinely load-bearing. The 10th (`many_consecutive_hyphens`) cannot catch any bug the 9th (`triple_hyphens`) would not catch first.

---

## SWE Remediation

- REMOVE `normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen`: confirmed redundant with `triple_hyphens`; implementation has no count-sensitive logic
- RENAME `normalize_concept_multiple_slashes_compresses_to_single_hyphen` → `normalize_concept_slash_and_space_mix_deduplicates_to_single_hyphen`: input is spaces AND slashes
- RENAME `normalize_concept_leading_trailing_separators_trims_hyphens` → `normalize_concept_leading_trailing_spaces_produce_trimmed_result`: describes actual whitespace→trim path
- RENAME `normalize_concept_single_separators_works_as_expected` → `normalize_concept_single_spaces_produce_hyphen_delimited_lowercase`: eliminates non-name
- RENAME `normalize_concept_no_separators_works_as_expected` → `normalize_concept_no_separators_returns_lowercased_input`: eliminates non-name
- ADD `normalize_concept_empty_input_returns_empty`: pins `""` → `""` edge case (P1)
- ADD `normalize_concept_digits_and_passthrough_chars_preserved`: pins `"user-123"` → `"user-123"` passthrough behavior (P1)
- Final count: 11 tests (was 10; -1 REMOVE, +2 ADD), all pass
- Commit: dbb876d

---

## Red-team Audit

**Traceability:** T-12.7.1 / RTM NFR-1

### REMOVE

- ✅ REMOVE `normalize_concept_many_consecutive_hyphens_compresses_to_single_hyphen`: Confirmed deleted in diff. No unique coverage lost — the implementation is a simple `dedup` over hyphens with no count-sensitive branching; `triple_hyphens` exercises the identical code path. The only assertion that existed (`"test----------concept"` → `"test-concept"`) is structurally identical to the surviving `triple_hyphens` test.

### RENAME (4 tests)

- ✅ RENAME `multiple_slashes` → `slash_and_space_mix_deduplicates_to_single_hyphen`: Exact match to eval spec. Assertion body unchanged (`"auth / / token"` → `"auth-token"`). New name accurately describes the mixed-input class.
- ✅ RENAME `leading_trailing_separators_trims_hyphens` → `leading_trailing_spaces_produce_trimmed_result`: Exact match. Assertion unchanged (`"  concept  "` → `"concept"`). New name correctly describes the whitespace→hyphen→trim pipeline.
- ✅ RENAME `single_separators_works_as_expected` → `single_spaces_produce_hyphen_delimited_lowercase`: Exact match. Assertion unchanged (`"REST API Gateway"` → `"rest-api-gateway"`). Non-name eliminated.
- ✅ RENAME `no_separators_works_as_expected` → `no_separators_returns_lowercased_input`: Exact match. Assertion unchanged (`"Authentication"` → `"authentication"`). Non-name eliminated.

### ADD (2 tests)

- ✅ ADD `normalize_concept_empty_input_returns_empty`: `assert_eq!(normalize_concept(""), "")` — pins the empty-input edge case per eval recommendation 3. Correct assertion.
- ✅ ADD `normalize_concept_digits_and_passthrough_chars_preserved`: `assert_eq!(normalize_concept("user-123"), "user-123")` — pins passthrough behavior per eval recommendation 4. Correct assertion.

### KEEP (5 tests, verified untouched)

- ✅ `double_space_compresses_to_single_hyphen` — unchanged
- ✅ `double_underscore_compresses_to_single_hyphen` — unchanged
- ✅ `mixed_separators_compresses_to_single_hyphen` — unchanged
- ✅ `only_separators_returns_empty` — unchanged
- ✅ `triple_hyphens_compresses_to_single_hyphen` — unchanged

### Scope check

- Commit `dbb876d` touches exactly 1 file (`markdown_rw_normalize_tests.rs`). No source code modified. No unrelated changes.
- All 11 tests pass: `cargo test -p maenifold-core normalize_concept` → 11 passed, 0 failed.
- P1 recommendations (empty input, passthrough chars) both addressed. P2 (rstest parameterization) was CONSIDER-tier — not required, correctly deferred.

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_normalize`: 11 passed, 0 failed
- All 11 test names match eval spec post-remediation (4 renames, 1 removal, 2 additions confirmed)
- Verdict: PASS
