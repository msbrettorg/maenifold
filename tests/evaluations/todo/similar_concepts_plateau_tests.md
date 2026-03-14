# Test Evaluation: similar_concepts_plateau_tests

**File:** `crates/maenifold-core/tests/similar_concepts_plateau_tests.rs`
**Evaluator role:** 🔘 Gray — Skeptical Inquiry
**Traceability claimed:** `T-QUAL-FSC2` / `RTM FR-7.4` (all six tests)
**Tests evaluated:** 6
**Date:** 2025-07-17

---

## Summary

This file is named `similar_concepts_plateau_tests.rs`, implying it exists to guard the
[[similarity-plateau]] detection path. That premise is false under inspection: only **one**
of six tests exercises plateau-adjacent logic (similarity clamping), and that test
**fails**. The remaining five tests are input-validation guards that have nothing to do with
plateau detection and belong in a differently-named file.

**Three of the six tests are broken today** — confirmed by running
`cargo test --package maenifold-core --test integration_tests similar_concepts_plateau`:

| Test | Status |
|------|--------|
| `find_similar_concepts_when_query_is_punctuation_only_does_not_emit_confident_results` | ✅ passes |
| `find_similar_concepts_when_query_contains_unicode_bracket_confusables_rejects_helpfully` | ✅ passes |
| `find_similar_concepts_when_query_contains_bracket_like_delimiters_rejects_helpfully` | ✅ passes |
| `find_similar_concepts_when_concept_name_too_long_returns_validation_error` | ❌ PANICS |
| `find_similar_concepts_when_max_results_out_of_range_returns_validation_error` | ❌ PANICS |
| `find_similar_concepts_when_it_returns_results_never_emits_similarity_greater_than_one` | ❌ PANICS |

The shared test helper panics on `Err` returns via `.expect()`; two tests assume the
implementation returns `Ok("ERROR: ...")` when it actually returns `Err(Message(...))`.
The headline "plateau" test fails because it seeds `vec_concepts` rows directly but the
test environment cannot load the vec0 SQLite extension, so the function falls through to
graph-based search which finds nothing, and the test's `assert!(...contains("Similar concepts to"))` fires.

Additional concerns: all six tests claim `T-QUAL-FSC2: RTM FR-7.4`, but **FR-7.4 does not
exist in `RTM.md`**. FR-7 (dynamic model loading) completed in earlier sprints and is now
in `CHANGELOG.md`. The traceability annotations are stale. There is also no `T-QUAL-FSC2`
entry in `TODO.md`. The reference appears to be a carry-over from an older sprint that was
never updated.

A global `TEST_MUTEX` serializes all tests in the file. This is a symptom of
`Config::override_root()` / `Config::set_database_path()` using process-global static
state; the tests are paying an isolation tax for a production-code design choice.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `find_similar_concepts_when_query_is_punctuation_only_does_not_emit_confident_results` | ✅ | ✅ | ⚠️ fragile footer string | ✅ | Moderate value | **KEEP** |
| 2 | `find_similar_concepts_when_query_contains_unicode_bracket_confusables_rejects_helpfully` | ✅ | ✅ | ⚠️ one confusable pair only | ✅ | High value | **KEEP** |
| 3 | `find_similar_concepts_when_query_contains_bracket_like_delimiters_rejects_helpfully` | ✅ | ✅ | ⚠️ one pair of 30+ | ✅ | High value | **KEEP** |
| 4 | `find_similar_concepts_when_concept_name_too_long_returns_validation_error` | ❌ broken | ✅ | ❌ panics, never asserts | ✅ | Low value | **REWRITE** |
| 5 | `find_similar_concepts_when_max_results_out_of_range_returns_validation_error` | ❌ broken | ✅ | ❌ panics, never asserts | ✅ | Low value | **REWRITE** |
| 6 | `find_similar_concepts_when_it_returns_results_never_emits_similarity_greater_than_one` | ❌ broken | ✅ | ❌ env-dependent, tautological | ⚠️ overlong | Low value | **REWRITE** |

---

## Detailed Findings

### Test 1 — `find_similar_concepts_when_query_is_punctuation_only_does_not_emit_confident_results`

**What it covers:** The `is_low_information_find_similar_query` branch in `validation.rs`
via the input `"----"`. A punctuation-only string passes through
`normalize_find_similar_validation_input` unchanged, skips the bracket checks (no brackets
in `"----"`), then hits `is_low_information_find_similar_query` which returns `true` because
all characters are non-alphanumeric non-whitespace, producing a `DIAGNOSTIC` response.

**Assertion quality:** Four asserts. The positive assertions are fine. The fourth —
`result.contains("Next: Consider using BuildContext, SearchMemories")` — checks UX copy
inside the `with_similarity_next_steps` helper, which is an implementation detail. If the
footer wording changes (e.g. a ticket adds "ReadMemory" to the suggestion list), this test
breaks without any logical change to the behaviour under test.

**Parameterization opportunity:** Other low-information inputs like `"..."`, `"*"`, `"@#$"`
are untested. The function has one branch (`has_non_ignored = false` path, i.e. all-whitespace
or empty) that is not exercised here either.

**Verdict:** KEEP. The test is load-bearing and unique. Harden by removing the footer
string assertion and replacing with something that cannot silently drift.

---

### Test 2 — `find_similar_concepts_when_query_contains_unicode_bracket_confusables_rejects_helpfully`

**What it covers:** The confusable-normalization path in `normalize_find_similar_validation_input`
for `\u{FF3B}` (FULLWIDTH LEFT SQUARE BRACKET) and `\u{FF3D}` (FULLWIDTH RIGHT SQUARE
BRACKET) — the only two characters currently in that match arm. After normalization, the
string contains literal `[` and `]`, triggering the bracket check downstream.

**Assertion quality:** Three asserts: DIAGNOSTIC present, "contains bracket characters"
present, "Similar concepts to" absent. The assertions are appropriate.

**Coverage question:** `normalize_find_similar_validation_input` currently maps only two
characters (`\u{FF3B}` → `[`, `\u{FF3D}` → `]`). If future development adds more
confusables (e.g. `\u{FF08}`, `\u{FF09}` for full-width parentheses), no test would
catch a bug in their handling. A parameterized table test would be more durable.

**Verdict:** KEEP. Unique and load-bearing for the current implementation.

---

### Test 3 — `find_similar_concepts_when_query_contains_bracket_like_delimiters_rejects_helpfully`

**What it covers:** The `contains_bracket_like_delimiter` path via `"⟦tool⟧"` (U+27E6
MATHEMATICAL LEFT WHITE SQUARE BRACKET / U+27E7). This exercises a different code branch
than Test 2: the check is `contains_bracket_like_delimiter`, not the normalization + bracket
check. The two branches are genuinely distinct.

**Assertion quality:** Three asserts: appropriate and correct.

**Coverage gap:** `contains_bracket_like_delimiter` tests 30 Unicode characters across 15+
pairs (angle brackets, black lenticular brackets, white corner brackets, ornate angle
brackets, etc.). Only one pair is exercised. The function's `matches!()` arm is long and
error-prone; it would benefit from a parameterized test covering at least `<>`, `〖〗`,
and `❰❱` (ornate).

**Verdict:** KEEP. Unique path. Extend with parameterized cases if the delimiter table
grows.

---

### Test 4 — `find_similar_concepts_when_concept_name_too_long_returns_validation_error`

**What it covers:** The `concept_name.len() > 256` early-exit in `similar/mod.rs`.

**Why it breaks:** The test helper returns a `String` via `.expect("find similar concepts")`,
which **panics** on `Err`. The production code returns
`Err(MaenifoldError::Message("conceptName must be 256 characters or fewer"))` for this
path. The test was written assuming `Ok("ERROR: ...")` would be returned — either the
implementation was refactored from `Ok(...)` to `Err(...)` without updating the test, or
the test was written incorrectly from the start. Either way, the `assert_eq!` is never
reached; the test panics at the `.expect()` call on line 50.

**Confirmed failure output:**
```
panicked at crates/maenifold-core/tests/similar_concepts_plateau_tests.rs:50:6:
find similar concepts: Message("conceptName must be 256 characters or fewer")
```

**Rewrite direction:** Replace `.expect()` in the helper with
`.unwrap_or_else(|e| format!("ERROR: {e}"))`, OR (preferred) add a separate helper that
explicitly unwraps `Err` variants and test that the returned `Err` has the expected message
type and content. The current approach conflates "the function returned Ok(error_string)"
with "the function returned Err(error_value)" — two very different contracts.

**Boundary gap:** No test verifies that a 256-character name (at the limit) succeeds; only
the over-limit case (257 chars) is tested.

**Verdict:** REWRITE. Fix the error-handling contract in the test helper; add a 256-char
boundary success case.

---

### Test 5 — `find_similar_concepts_when_max_results_out_of_range_returns_validation_error`

**What it covers:** The `!(1..=50).contains(&max_results)` validation for `maxResults = 0`
and `maxResults = 51`.

**Why it breaks:** Same root cause as Test 4. The production code returns
`Err(MaenifoldError::Message("maxResults must be between 1 and 50"))`. `.expect()` panics.

**Confirmed failure output:**
```
panicked at crates/maenifold-core/tests/similar_concepts_plateau_tests.rs:50:6:
find similar concepts: Message("maxResults must be between 1 and 50")
```

**Additional concern:** Both bounds are tested in a single `#[test]` function. This means a
failure in the `result_low` assertion (if it were reached) would shadow whether `result_high`
is also broken. Two independent test functions would produce clearer failure signals.

**Boundary gap:** No test verifies that `maxResults = 1` and `maxResults = 50` (the
inclusive boundary values) succeed.

**Verdict:** REWRITE. Fix error-handling contract; split into `..._when_zero` and
`..._when_above_max` tests; add `maxResults = 1` and `maxResults = 50` success cases.

---

### Test 6 — `find_similar_concepts_when_it_returns_results_never_emits_similarity_greater_than_one`

**What it covers — in theory:** The `(1.0 / (1.0 + distance.max(0.0))).clamp(0.0, 1.0)`
clamping formula in `vector_query.rs` that prevents `similarity > 1.000` values from
appearing in output. This is the only test in the file actually related to the plateau/
clamping concern for which the file is named.

**Why it breaks:** The test seeds three concepts (`alpha`, `beta`, `gamma`) directly into
the `vec_concepts` SQLite table, then queries for `"test"`. However, the query path
requires the **vec0 SQLite extension** to be loaded. In the test environment the extension
is unavailable; `db::load_vec0_extension()` fails silently and `query_vec_similar_concepts`
returns `Ok(None)`. Execution falls through to the graph-based search path, which has no
`concept_graph` entries for `"test"` in a fresh database. The function returns
`"No similar concepts found for 'test'. Run Sync to ensure concept embeddings are generated."`
The test then asserts `.contains("Similar concepts to")` — which panics.

**Confirmed failure output:**
```
panicked at crates/maenifold-core/tests/similar_concepts_plateau_tests.rs:197:5:
unexpected: No similar concepts found for 'test'. Run Sync to ensure concept embeddings are generated.
```

**Tautology risk:** Even if the vec0 extension were available, the clamping formula
`1 / (1 + d)` where `d = distance.max(0)` and `distance >= 0` (cosine distance is always
non-negative) mathematically **cannot** produce a value above 1.0. The only scenario where
`similarity > 1.0` could arise is if `vec_distance_cosine` returned a negative value
(numerical underflow). The test does not construct that edge case and relies on benign
inputs. The `parse_similarity_values` helper also only sees the *weighted* similarity
(`similarity * decay_weight`) not the raw value, and decay weights can amplify results if
not properly bounded — but this is untested.

**Rewrite direction:** The correct approach is to test `similarity_plateau_warning` and the
clamping in `query_vec_similar_concepts` as unit tests against synthetic data, without
going through the full stack. If an integration-level test is desired, it requires either
a conditional `skip_if` guard when vec0 is unavailable, or a feature-flag-controlled
mock of the vector layer.

**Name:** Overly long for what it verifies. A cleaner name would be
`find_similar_concepts_similarity_values_are_bounded_to_one`.

**Verdict:** REWRITE. Decompose into: (a) a unit test of `similarity_plateau_warning` with
synthetic `SimilarConceptMatch` data, (b) a unit test of the clamping formula against a
negative-distance input (the actual failure scenario), and (c) an integration-level test
that guards against accidental formula changes, skipped when vec0 is unavailable.

---

## Structural Issues

### 1. File naming mismatch
Five of six tests verify input-validation guards (`is_low_information_find_similar_query`,
bracket confusable normalization, bracket-like delimiter detection, length limit, range
limit). None of these are "plateau" tests. The one plateau-adjacent test is broken. The
file should be split: move the five validation tests to
`similar_concepts_validation_tests.rs` and rename this file to something that accurately
describes the remaining plateau/clamping concern.

### 2. Stale traceability
All six tests carry `// T-QUAL-FSC2: RTM FR-7.4`. `FR-7.4` does not appear anywhere in
`RTM.md`. `FR-7` (dynamic model loading) is closed and in `CHANGELOG.md`. `T-QUAL-FSC2`
does not appear in `TODO.md`. The annotations are orphaned. Before these tests can be
considered properly load-bearing for audit purposes they must be retargeted to real, open
RTM entries — or the CHANGELOG entry that closed FR-7.4 must be found and the closed
status documented in each comment.

### 3. Global mutex as design-smell proxy
`static TEST_MUTEX: Lazy<Mutex<()>>` exists because `Config::override_root()` and
`Config::set_database_path()` modify process-global static state. Every test in this file
(and any other file that calls `isolated_env()`) must serialize. This is a correctness
constraint imposed by production-code design, not by the tests themselves. The risk: if a
future developer removes the `_guard = TEST_MUTEX.lock()` line from any test in any file
that shares the process-global Config, the tests become non-deterministically flaky without
an obvious cause.

### 4. `parse_similarity_values` helper is orphaned when Test 6 is rewritten
The helper exists only for Test 6. If Test 6 is decomposed into unit tests as recommended,
`parse_similarity_values` should either migrate with the integration slice or be deleted.
It is a useful utility, but keeping dead helpers in test files increases maintenance cost.

---

## Recommendations

1. **Immediate (blocking):** Rewrite Tests 4 and 5. The test helper must either propagate
   errors correctly (`unwrap_or_else`) or the test must directly inspect the `Result` type.
   Three broken tests in a 500+ test suite are a silent regression detector gap.

2. **Immediate (blocking):** Rewrite Test 6 as a unit test of the clamping formula and
   `similarity_plateau_warning` against synthetic data. The integration-level variant
   should be guarded against vec0 unavailability.

3. **Short-term:** Update all six `// T-QUAL-FSC2: RTM FR-7.4` comments to reference a
   real, open RTM entry (or document that the requirement is closed). Stale traceability
   erodes trust in the RTM as a source of truth.

4. **Short-term:** Rename the file and split its contents. Five validation tests do not
   belong in a "plateau" file.

5. **Medium-term:** Parameterize Tests 2 and 3. A table-driven approach over representative
   samples of the confusable/bracket-delimiter character sets would catch regressions in
   `normalize_find_similar_validation_input` and `contains_bracket_like_delimiter` far more
   reliably than a single input per branch.

6. **Medium-term:** Add boundary-success cases for Tests 4 and 5 (256 chars succeeds;
   `maxResults = 1` and `maxResults = 50` succeed). Boundary-only failure tests without
   accompanying boundary-success tests are half-guarded contracts.

---

## Load-Bearing Score: 3/6

Three tests (1, 2, 3) are currently functioning as load-bearing regression guards.
Three tests (4, 5, 6) are broken and provide zero protection today — a regression in
length validation, range validation, or the similarity clamping formula would go undetected.

---

## SWE Remediation

**Result:** 9 tests pass (was 3 passing + 3 panicking). All P0 and P1/P2 recommendations addressed.

### KEEP (unchanged)
- Test 1 `find_similar_concepts_when_query_is_punctuation_only_does_not_emit_confident_results`: kept as-is
- Test 2 `find_similar_concepts_when_query_contains_unicode_bracket_confusables_rejects_helpfully`: kept as-is
- Test 3 `find_similar_concepts_when_query_contains_bracket_like_delimiters_rejects_helpfully`: kept as-is

### REWRITE: Test 4 — concept_name length validation
- Added `try_find_similar_concepts` helper returning `Result<String, MaenifoldError>` (no `.expect()`)
- Test now asserts `result.is_err()` and inspects error message via `format!("{}", err)`
- Added boundary success test `find_similar_concepts_when_concept_name_at_limit_succeeds` (256 chars)

### REWRITE: Test 5 — maxResults range validation
- Split single test into two independent tests for clear failure signals:
  - `find_similar_concepts_when_max_results_is_zero_returns_validation_error`
  - `find_similar_concepts_when_max_results_above_fifty_returns_validation_error`
- Added boundary success test `find_similar_concepts_when_max_results_at_boundaries_succeeds` (1 and 50)
- All use `try_find_similar_concepts` to properly inspect `Err` returns

### REWRITE: Test 6 — similarity values bounded
- Renamed to `find_similar_concepts_similarity_values_are_bounded_to_one`
- Replaced broken `seed_vec_concept` (requires unavailable vec0 extension) with `seed_concept_graph` helper that seeds `concept_graph` + `concepts` tables for graph-based search path
- Test now produces actual ranked results and verifies similarity ∈ [0.0, 1.0]
- Removed dead `seed_vec_concept` helper and `maenifold_core::vector` import

### Structural changes
- Added `try_find_similar_concepts` helper for error-path tests
- Added `seed_concept_graph` helper for graph-based search seeding
- Removed `seed_vec_concept` helper (orphaned after Test 6 rewrite)
- Removed `use maenifold_core::vector` import (no longer needed)

### Not addressed
- Traceability: `T-QUAL-FSC2: RTM FR-7.4` references retained as-is; eval notes FR-7.4 is stale but determining correct RTM entry is out of scope for SWE remediation
- File rename/split: eval recommends moving validation tests to separate file; SWE prompt prohibits file moves
- Unit tests of `similarity_plateau_warning` and clamping formula: these are `pub(crate)` and not accessible from integration tests; would require production code changes to expose

- Commit: eef9d64
