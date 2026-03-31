---
id: proximity_decay_relations
status: done
pipeline: tdd
---

# Test Evaluation: proximity_decay_relations

**File:** `crates/maenifold-core/tests/proximity_decay_relations.rs`
**Test count:** 4
**Evaluated:** 2025-07-14
**Traceability:** T-12.0 / RTM NFR-1, NFR-2
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions, find duplication.

---

## Summary

This file contains 4 integration tests that exercise the proximity-weight-to-relation-storage pipeline end-to-end via `invoke_sync()` → `concept_graph` query. The question it tries to answer is: *does the sync pipeline correctly persist proximity weights (1.0 vs 0.5) into the `source_files` JSON column of `concept_graph`?*

**Critical finding: three of the four tests are functional duplicates of superior tests already present in `proximity_weight_sections.rs`.** The duplication is not approximate — tests 1 and 2 use *identical markdown content and identical assertions*, with the only addition being a `DecayEnvGuard::with_defaults()` that does not affect proximity weight calculation (proximity is determined purely by section structure, not by temporal decay configuration). Test 3 is a degraded duplicate: it covers the same "flat-doc → 0.5" contract but omits the `beta`–`gamma` pair assertion that `sync_flat_docs_store_half_weight_for_all_pairs` in `proximity_weight_sections.rs` correctly includes.

Test 4 is genuinely load-bearing and unique: it is the only test in the codebase that verifies **multi-file aggregation** in the `source_files` JSON dictionary — specifically that contributions from two files with different proximity weights are merged under a single relation key with correct per-URI weight values and a correct total `len()`.

**Secondary finding: traceability annotations are misapplied.** All four tests carry `// T-12.0: RTM NFR-1`. NFR-1 is "C# Oracle Parity" — these are behavioral contract tests, not oracle-comparison tests. The correct anchor is NFR-2 (Test Coverage) or the specific FR governing proximity weight semantics (FR-23.2 per proximity_weight_sections.rs).

---

## Verdicts

| Test Function | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `sync_same_h2_section_relations_get_full_proximity_weight` | ❌ No — covered by `sync_same_section_pairs_store_weight_one` in `proximity_weight_sections.rs` | ❌ No — identical content, identical assertion, irrelevant `DecayEnvGuard` addition | ✅ Yes — 1.0 is correct | ✅ Adequate | ✅ Yes | **REMOVE** |
| `sync_cross_h2_section_relations_get_half_proximity_weight` | ❌ No — covered by `sync_cross_section_pairs_store_weight_half` in `proximity_weight_sections.rs` | ❌ No — identical file content, identical assertion | ✅ Yes — 0.5 is correct | ✅ Good | ✅ Yes | **REMOVE** |
| `sync_without_h2_headings_stores_cross_section_pair_weights` | ❌ No — covered by `sync_flat_docs_store_half_weight_for_all_pairs` in `proximity_weight_sections.rs` (which also covers `y`–`z` pair) | ❌ No — inferior version; missing `beta`–`gamma` assertion | ⚠️ Partial — `alpha`–`beta` and `alpha`–`gamma` at 0.5 are correct, but `beta`–`gamma` at 0.5 is never asserted | ❌ Poor — name says "cross_section" but all three concepts are in the *same* implicit root section; misleading | ✅ Yes | **REMOVE** |
| `sync_stores_source_files_as_json_weight_dictionary` | ✅ Yes — only test verifying multi-file aggregation in `source_files` JSON | ✅ Yes — no other test asserts `len() == 2` with two distinct source files at different weights | ✅ Yes — checks count, key presence, and both weight values (1.0 and 0.5) | ✅ Good | ⚠️ Could be extended | **KEEP** |

---

## Per-Test Analysis

### `sync_same_h2_section_relations_get_full_proximity_weight`

**What it tests:** Writing a single file with two WikiLinks in the same `## Identity` section, running sync, and asserting the stored relation weight is 1.0.

**The problem:** This is byte-for-byte the same behavioral contract as `sync_same_section_pairs_store_weight_one` in `proximity_weight_sections.rs`, which uses the same markdown content (`## Identity\nPair [[alpha]] with [[beta]]`) and the same assertion (`1.0`). The only distinguishing element is `DecayEnvGuard::with_defaults()`.

**Why `DecayEnvGuard` adds no signal here:** Decay environment variables (`MAENIFOLD_DECAY_FUNCTION`, `MAENIFOLD_DECAY_HALF_LIFE_DAYS`, etc.) control temporal relevance scoring applied *after* the relation is stored. The proximity weight stored in `source_files` is determined solely by section structure in `flush_relation_counts` — the decay env vars are never consulted during sync. Wrapping this test in a decay guard creates a false implication that the proximity weight depends on decay config.

**Verdict:** REMOVE.

---

### `sync_cross_h2_section_relations_get_half_proximity_weight`

**What it tests:** Two WikiLinks in different H2 sections → relation weight 0.5.

**The problem:** `sync_cross_section_pairs_store_weight_half` in `proximity_weight_sections.rs` uses an *identical* markdown structure (`## Identity\n...\n## Sessions\n...`) with the same concepts (`alpha`, `beta`) and the same assertion (0.5). This test adds a `DecayEnvGuard` that is, again, irrelevant to the proximity weight.

**Verdict:** REMOVE.

---

### `sync_without_h2_headings_stores_cross_section_pair_weights`

**What it tests:** A flat file (no H2 headings) with three concepts in the implicit root section → all pairs get weight 0.5.

**Problem 1 — Name:** The name says "cross_section" but the concepts occupy the *same* implicit root section (section_index=0). The implementation comment in `graph_relations.rs:149` clarifies the design intent — "treat that implicit root as cross-section so headingless documents store half-weight pairs" — but the test name conflates the mechanism with the description. A reader seeing the name alongside the same-section test naturally assumes these are different sections; they are not. Correct name: `sync_implicit_root_section_gets_half_weight` or `sync_flat_doc_concepts_get_half_weight_not_full`.

**Problem 2 — Missing pair:** The test asserts `alpha`–`beta` at 0.5 and `alpha`–`gamma` at 0.5 but never tests `beta`–`gamma`. `sync_flat_docs_store_half_weight_for_all_pairs` in `proximity_weight_sections.rs` correctly asserts all three pairs (`x`–`y`, `x`–`z`, `y`–`z`). This test silently permits a regression where `beta`–`gamma` is not stored.

**Problem 3 — Duplication:** The superior version already exists. There is no new behavioral surface here.

**Verdict:** REMOVE (superseded by the complete version in `proximity_weight_sections.rs`).

---

### `sync_stores_source_files_as_json_weight_dictionary`

**What it tests:** Two files (`proximity-source-files-same.md` and `proximity-source-files-cross.md`) contribute to the `alpha`–`beta` relation, and the `source_files` JSON column aggregates both URIs with their respective proximity weights.

**Why this is load-bearing:** This is the only test in the entire suite that exercises multi-file aggregation into `source_files`. It asserts:
1. The column deserializes to a JSON object
2. The object contains exactly 2 keys (no extra entries, no dropped entries)
3. The same-section file contributes weight 1.0
4. The cross-section file contributes weight 0.5

`sync_serializes_source_files_as_weight_dictionary` in `proximity_weight_sections.rs` verifies the JSON structure for a *single-file* relation. It does not test that multi-file merging (`entry.and_modify(|existing| *existing = existing.max(weight))`) preserves distinct per-URI entries. This test is the only regression guard for that code path.

**Minor critique:** The test does not assert anything about the `co_occurrence_count` column (which should be 2.0 = two distinct contributing files, per T-14.4). This is a missed opportunity but not a flaw in what is already asserted.

**Verdict:** KEEP.

---

## Recommendations

### Immediate actions

1. **Remove tests 1–3.** `sync_same_h2_section_relations_get_full_proximity_weight`, `sync_cross_h2_section_relations_get_half_proximity_weight`, and `sync_without_h2_headings_stores_cross_section_pair_weights` are redundant. Their behavioral contracts are fully covered by `proximity_weight_sections.rs` in superior form. The only change introduced — `DecayEnvGuard` — actively misleads readers about which configurations affect proximity weights.

2. **Add `beta`–`gamma` assertion to `sync_flat_docs_store_half_weight_for_all_pairs` if it lacks it.** The existing version in `proximity_weight_sections.rs` already has this, but it is worth noting as the canonical control.

3. **Fix traceability annotations.** The `// T-12.0: RTM NFR-1` annotation on all four tests is incorrect. NFR-1 = C# Oracle Parity. These are behavioral contract tests for proximity weight storage. Correct anchor is NFR-2 (Test Coverage) + the proximity-weight FR (FR-23.2 by analogy with `proximity_weight_sections.rs` annotations at T-4.3).

### What test 4 could gain (optional)

4. **Extend test 4 to assert `co_occurrence_count`.** Query the `concept_graph` row and verify `co_occurrence_count = 2.0` (two distinct source files). This would pin the T-14.4 "count distinct files" behavior in the integration context, complementing the unit test in `graph_relations.rs::tests::graph_relations_counts_distinct_files_instead_of_proximity_sums`.

5. **Add a third-file scenario to test 4.** Introduce a third file to verify that `len() == 3` and that the `max()` merging logic is not accidentally summing weights. This would exercise the edge case where the same concept pair appears three or more times across files.

---

## Load-Bearing Score: 1/4

Only `sync_stores_source_files_as_json_weight_dictionary` is uniquely load-bearing. The remaining three tests provide no protection that does not already exist in `proximity_weight_sections.rs`, and test 3 provides *less* protection than its counterpart (missing the `beta`–`gamma` pair). The file as a whole is test debt: it adds maintenance surface, mutual-exclusion lock contention, and misleading `DecayEnvGuard` noise without expanding the regression safety net.

---

## SWE Remediation

- REMOVE `sync_same_h2_section_relations_get_full_proximity_weight`: duplicate of `sync_same_section_pairs_store_weight_one` in `proximity_weight_sections.rs`
- REMOVE `sync_cross_h2_section_relations_get_half_proximity_weight`: duplicate of `sync_cross_section_pairs_store_weight_half` in `proximity_weight_sections.rs`
- REMOVE `sync_without_h2_headings_stores_cross_section_pair_weights`: inferior duplicate of `sync_flat_docs_store_half_weight_for_all_pairs` in `proximity_weight_sections.rs` (missing `beta`–`gamma` assertion)
- KEEP `sync_stores_source_files_as_json_weight_dictionary`: unchanged, uniquely load-bearing
- FIX traceability: `T-12.0: RTM NFR-1` → `T-12.0: RTM NFR-2` on file-level and test-level annotations
- Removed unused `relation_weight_for_uri` import
- Commit: aedad4b

---

## Red-team Audit

| Item | Verdict | Detail |
|---|---|---|
| REMOVE `sync_same_h2_section_relations_get_full_proximity_weight` | ✅ | Deleted in diff. Superior test `sync_same_section_pairs_store_weight_one` at `proximity_weight_sections.rs:96` covers identical markdown + 1.0 assertion. No coverage lost. |
| REMOVE `sync_cross_h2_section_relations_get_half_proximity_weight` | ✅ | Deleted in diff. Superior test `sync_cross_section_pairs_store_weight_half` at `proximity_weight_sections.rs:118` covers identical markdown + 0.5 assertion. No coverage lost. |
| REMOVE `sync_without_h2_headings_stores_cross_section_pair_weights` | ✅ | Deleted in diff. Superior test `sync_flat_docs_store_half_weight_for_all_pairs` at `proximity_weight_sections.rs:140` covers all three pairs (x–y, x–z, y–z at 0.5). Removed test only covered alpha–beta and alpha–gamma — net coverage *increased* by removing the inferior version. |
| KEEP `sync_stores_source_files_as_json_weight_dictionary` | ✅ | Test body unchanged. Confirmed uniquely load-bearing (only multi-file aggregation test). Compiles and passes: `test result: ok. 1 passed`. |
| FIX traceability `NFR-1` → `NFR-2` | ✅ | File-level annotation (line 1) and test-level annotation (line 10) both updated to `// T-12.0: RTM NFR-2`. Correct: these are behavioral contract tests, not C# oracle parity tests. |
| Remove unused `relation_weight_for_uri` import | ✅ | Import removed from `use support::{...}`. Function still exists in `proximity_decay/support.rs` and is consumed by `proximity_weight_sections.rs` and `sync_perf_graph.rs` — no dead code introduced. |
| No unrelated changes | ✅ | `git diff --stat` confirms: 1 file changed, 3 insertions, 75 deletions. Only `proximity_decay_relations.rs` was modified. |
| Compilation + test green | ✅ | `cargo test -p maenifold-core sync_stores_source_files_as_json_weight_dictionary` passes with 0 warnings from this module. |

- **P0 (remove 3 duplicates):** Addressed. All three deleted.
- **P1 (fix traceability):** Addressed. Both annotations corrected.
- **P2 (optional: extend test 4 with co_occurrence_count):** Not addressed — acceptable, eval marked this as optional.

**Verdict: PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- proximity_decay_relations`: 1 passed, 0 failed
- Verdict: PASS
