---
id: search_tools_tests
status: done
pipeline: tdd
---

# Test Evaluation: search_tools_tests
**File:** `crates/maenifold-core/tests/search_tools_tests.rs`
**Test count:** 8
**Evaluated:** 2026-03-14
**Traceability:** T-0.1 (NFR-3 parity guard), T-13.1 (FR-6 search modes), T-13.2 (FR-6 pre-fusion pagination), T-13.3 (FR-6 normalization)

---

## Summary

Six of eight tests are genuinely load-bearing and assert real behavioral contracts (JSON schema shape, score normalization, pagination bounds). Two tests have substantive defects: one has a conditional guard that makes its core negative assertion vacuous, and one conflates string-label checking with behavioral verification across all three search modes. The file as a whole provides good coverage of the search pipeline's JSON API surface but leaves a gap: the `minScore` filtering correctness at values other than 0.0 can be evaded by an empty result set.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `min_score_zero_returns_all_results` | KEEP | Load-bearing: verifies search returns results at all with no filter. `result.contains("Found")` is a format-label check but the title-presence assertions are real behavioral guards. One annotation needed (see Recommendations). |
| `min_score_one_returns_only_perfect_matches` | REWRITE | **Critical flaw**: the negative assertion (`!result.contains("Cooking Recipes")`) is gated behind `if result.contains("Machine Learning Tutorial")`. When `minScore=1.0` yields zero results (a common, correct outcome), the condition is false and the test passes vacuously — it has never verified anything. The test needs an unconditional structure. |
| `all_search_modes_respect_min_score` | REWRITE | Three of nine assertions check mode display labels ("Semantic Search", "Full-Text Search", "Hybrid Search") — these are format/cosmetic, not behavioral. The six filtering assertions (Cooking Recipes absent at 0.5) are real. Additionally this test is parameterizable: the per-mode pattern is identical. Rewrite as one parameterized loop over modes with only behavioral assertions, and move label checks to the JSON-mode tests that already verify `data["mode"]`. |
| `hybrid_mode_returns_fused_text_semantic_scores` | KEEP | Excellent JSON contract test. Verifies field presence/absence (`fusedScore`, `textScore`, `semanticScore`, absence of plain `score`, envelope counts `textResultCount`/`semanticResultCount`). Would catch any silent schema regression. |
| `semantic_mode_returns_only_score_field` | KEEP | Solid API contract. Verifies per-result field absence for fused fields and envelope field absence. Note: the comment "graceful degradation" is honest — with no ONNX model the loop may never execute, but the structural assertions outside the loop still hold. |
| `fulltext_mode_returns_only_score_field_with_normalized_top` | MERGE with `fulltext_scores_are_max_normalized` | The per-result schema checks (`score` present, `fusedScore`/`textScore`/`semanticScore` absent) and the envelope assertions duplicate work that `fulltext_scores_are_max_normalized` could own. The `top_score > 0.95` assertion is unique and should be folded into the normalization test. Keeping both as separate tests creates inconsistency: a future FullText schema change requires editing two places. |
| `fulltext_scores_are_max_normalized` | KEEP | Strong behavioral test: range `[0.0, 1.0]`, descending order, strict `scores[0] > scores[1]`. These are real algorithmic invariants. Absorb the schema assertions from the test above. |
| `prefusion_multiplier_respects_page_size_bounds` | KEEP (rename) | Load-bearing pagination contract test. However, the name `prefusion_multiplier_respects_page_size_bounds` is misleading: the test **cannot observe** the internal pre-fusion candidate pool size — it only verifies that the returned slice is bounded by `pageSize`. The name claims to test an invisible internal detail. Rename to `page_size_bounds_respected_across_all_modes`. |

---

## Recommendations

1. **REWRITE `min_score_one_returns_only_perfect_matches`**: Remove the conditional guard. The test must unconditionally assert a meaningful outcome. Options: (a) assert `!result.contains("Cooking Recipes")` regardless of ML Tutorial presence, (b) assert empty results is also acceptable when `minScore=1.0` by checking `result.contains("Found") || result.contains("No results")`, or (c) seed a document with an exact title match to guarantee a known positive result. Without this fix, the test is a false green that silently passes on empty result sets.

2. **REWRITE `all_search_modes_respect_min_score`**: Replace the three mode-label `contains` checks with the JSON output path already used in `hybrid_mode_returns_fused_text_semantic_scores`. Use a `for mode in ["Hybrid", "Semantic", "FullText"]` loop with a single behavioral assertion per mode: that the cooking document is absent at `minScore=0.5`. This collapses 9 assertions into 3 real ones with no false positives on cosmetic renames.

3. **MERGE `fulltext_mode_returns_only_score_field_with_normalized_top` into `fulltext_scores_are_max_normalized`**: Consolidate FullText schema + normalization into one authoritative test. The `top_score > 0.95` threshold assumes freshly-indexed content has decay ≈ 1.0 — document this assumption in a comment so future readers don't mistake it for a precision assertion.

4. **RENAME `prefusion_multiplier_respects_page_size_bounds`** → `page_size_bounds_respected_across_all_modes` to avoid implying the test verifies internal pool sizes it cannot see.

5. **Add a missing behavioral gap**: No test verifies that a document that genuinely scores below `minScore` is excluded when `minScore` is in the mid-range (e.g., 0.3). The "Cooking Recipes" exclusion in `all_search_modes_respect_min_score` approaches this but is entangled with label checks. A clean, dedicated test would strengthen the `minScore` contract.

---

## Load-Bearing Score: 6/8

Tests 2 and 3 are presently either vacuous under common conditions (test 2) or diluted by format-label assertions (test 3). The remaining six provide genuine regression protection for the search pipeline's behavioral and structural contracts.

---

## SWE Remediation
- REWRITE `min_score_one_returns_only_perfect_matches`: removed conditional guard (`if result.contains(...)`) that made core negative assertion vacuous on empty result sets; now unconditionally asserts both "Cooking Recipes" and "Data Analysis Methods" are absent at minScore=1.0
- REWRITE `all_search_modes_respect_min_score`: replaced 9 assertions (3 cosmetic label checks + 3 error checks + 3 behavioral) with parameterized `for mode in ["Hybrid", "Semantic", "FullText"]` loop containing only 2 behavioral assertions per mode (no error + no Cooking Recipes)
- MERGE `fulltext_mode_returns_only_score_field_with_normalized_top` into `fulltext_scores_are_max_normalized`: consolidated FullText schema contract (score present, fused fields absent, envelope fields absent) + normalization invariants (range, ordering, strict top > second) + top_score > 0.95 into single authoritative test; documented decay assumption in comment
- RENAME `prefusion_multiplier_respects_page_size_bounds` → `page_size_bounds_respected_across_all_modes`: name now reflects the observable behavior (page size bounds) not invisible internals (pre-fusion pool)
- Test count: 8 → 7 (1 MERGE). All 7 passing.
- Commit: adee661

---
## Red-team Audit

**Traceability**: T-0.1 (NFR-3), T-13.1 (FR-6), T-13.2 (FR-6), T-13.3 (FR-6)

### Item-by-item

- ✅ REWRITE `min_score_one_returns_only_perfect_matches`: Conditional guard (`if result.contains("Machine Learning Tutorial")`) removed. Negative assertion `!result.contains("Cooking Recipes")` is now unconditional. SWE added bonus `!result.contains("Data Analysis Methods")` assertion, strengthening coverage beyond eval minimum. Comment documents the empty-result-set invariant. Matches eval Recommendation #1 option (a).
- ✅ REWRITE `all_search_modes_respect_min_score`: Three cosmetic label checks (`"Semantic Search"`, `"Full-Text Search"`, `"Hybrid Search"`) removed. Parameterized `for mode in &["Hybrid", "Semantic", "FullText"]` loop with 2 behavioral assertions per mode (no ERROR, no Cooking Recipes). Eval said "collapse 9 assertions into 3 real ones" — SWE has 6 (kept error guards), but the 3 cosmetic assertions that were the eval's concern are gone. No false positives on cosmetic renames. Matches eval Recommendation #2.
- ✅ MERGE `fulltext_mode_returns_only_score_field_with_normalized_top` into `fulltext_scores_are_max_normalized`: Single authoritative test now owns: (1) per-result schema (score present, fusedScore/textScore/semanticScore absent), (2) envelope schema (textResultCount/semanticResultCount absent), (3) normalization invariants (range [0.0, 1.0], descending order, strict top > second), (4) top_score > 0.95. All assertions from both original tests preserved — no coverage lost. Decay assumption documented in comment (line 318). Minimum result count tightened from `!results.is_empty()` to `results.len() >= 2`. Matches eval Recommendation #3.
- ✅ RENAME `prefusion_multiplier_respects_page_size_bounds` → `page_size_bounds_respected_across_all_modes`: Name now reflects observable behavior. Comment updated to remove internal pool-size claims. Body unchanged — no behavioral regression. Matches eval Recommendation #4.
- ✅ Test count: 8 → 7 (1 MERGE). Confirmed 7 tests in final file. `cargo test -p maenifold-core search_tools` passes all 7.
- ✅ No unrelated changes: Commit `adee661` touches only `crates/maenifold-core/tests/search_tools_tests.rs` (1 file, +45 −81).
- ℹ️ Eval Recommendation #5 (dedicated mid-range `minScore` test) was advisory, not a verdict-driven action. SWE did not add it. Acceptable — the eval marked this as a "missing behavioral gap" suggestion, not a REWRITE/REMOVE/MERGE directive.

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- search_tools_tests`: 7 passed, 0 failed
- Verdict: PASS
