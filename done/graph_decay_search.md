---
id: graph_decay_search
status: done
pipeline: tdd
---

# Test Evaluation: graph_decay_search
**File:** `crates/maenifold-core/tests/graph_decay_search.rs`
**Support:** `crates/maenifold-core/tests/graph_decay/support.rs`
**Test count:** 4
**Evaluated:** 2026-07-09
**Traceability:** T-GRAPH-DECAY-001.1 · RTM FR-7.5 · RTM NFR-7.5.2

---

## Summary

This suite validates that [[SearchMemories]] applies [[decay-weighting]] across all three
search modes (Hybrid, Semantic, FullText) and that files within the grace window carry a
neutral `decayWeight` of 1.0. The scope is narrow and clearly defined.

Skeptical examination identifies **three structural problems** that reduce the suite's
precision without removing its load-bearing role:

1. **Tests 1–3 are structurally isomorphic.** They differ only in `mode` string, query
   text, and age numbers. Each sets up two files with identical body content, ages them
   differently, and asserts that the newer file appears earlier in results. This is one
   parameterized test wearing three costumes. The triplication adds noise to the failure
   signal: a regression in *all* modes would produce three identical failures, and a
   regression in a *single* mode is caught with equal confidence by one test or three.

2. **Tests 1–3 assert rank order but never inspect score values.** `title_index(new) <
   title_index(old)` catches the correct ordering but cannot distinguish between "decay
   correctly shifted the scores" and "decay is broken but the files happened to rank in
   this order for another reason." A single additional assertion — that
   `result["decayWeight"]` is materially different between the two files — would tighten
   the claim from "newer ranks higher" to "newer ranks higher *because of decay*."

3. **The name of Test 1 is inaccurate.** `search_memories_newer_content_ranks_higher_when_scores_are_equal`
   implies that the test controls for equal base scores. It does not. `create_test_file_with_age`
   embeds the title inside an `# {title}` H1 heading, so "Old Machine Learning Doc" and
   "New Machine Learning Doc" produce slightly different vector embeddings. The test relies
   on a 59-day age gap being large enough to overwhelm any embedding divergence — a safe
   bet in practice, but the "when scores are equal" qualifier is a promise the test cannot
   make. A reader auditing this later may trust that premise and accept a weaker age gap
   that isn't actually safe.

**What the suite does not cover (and should):**
- Ranking behavior *at* the grace period boundary: what happens at `age_days ==
  grace_period_days + 1`? Test 4 only shows both files within the window; no test in this
  file covers the transition out of it.
- Verification that `fusedScore` (not just rank position) changes between decayed and
  non-decayed files. This is tested in `proximity_decay_search.rs` but not here.

**Overlap with adjacent suites:** `proximity_decay_search.rs::search_memories_orders_results_by_decay_weighted_score`
covers a directly analogous integration scenario — two files, different decay paths,
search ordering asserted — and also checks `decayWeight` and `fusedScore` values.
`decay_calculator_path_grace.rs` covers the grace-period unit behavior tested by Test 4.
The overlap is not fatal (integration scope differs), but it means failures in this file
will often co-fire with failures in those files, obscuring diagnostic specificity.

**On traceability:** All four tests cite `T-GRAPH-DECAY-001.1`, which does not appear in
`TODO.md` or `RTM.md`. It is presumably in the completed sprint archive (`CHANGELOG.md`).
The `RTM FR-7.5` comment used on Tests 1–3 cannot be verified against current RTM entries
(FRs are numbered 1–31 as integers). Test 4 adds `RTM NFR-7.5.2`, also not in current
`RTM.md`. These are likely aliases from an older naming scheme. Traceability is *present*
in spirit but *unverifiable* against living project documents.

**The support module is well-designed.** `TEST_MUTEX` + `isolated_env()` correctly
serializes process-wide env state. `create_test_file_with_age` writes syntactically
correct frontmatter that the sync path reads via `Frontmatter::get("created")` in
`file_index.rs` — the `created` date actually reaches the DB and therefore drives the
decay calculation. The mechanical setup is sound; only the assertion layer needs work.

---

## Verdicts

| Test | Verdict | Reason |
|---|---|---|
| `search_memories_newer_content_ranks_higher_when_scores_are_equal` | **REWRITE** | Name is inaccurate (scores are not equal); no decayWeight assertion; should be the canonical form of the merged trio with corrected name and added score inspection |
| `search_memories_semantic_mode_applies_decay_weighting` | **MERGE** | Structurally identical to Test 1; fold into a parameterized or macro-driven test, keeping Semantic as one case |
| `search_memories_fulltext_mode_applies_decay_weighting` | **MERGE** | Same as above for FullText mode |
| `search_memories_files_within_grace_period_have_equal_ranking_weight` | **KEEP** | Unique: only test in this file that asserts `decayWeight` field values directly; correctly verifies grace window contract |

---

## Detailed Findings

### T1: `search_memories_newer_content_ranks_higher_when_scores_are_equal` — REWRITE

```rust
let identical_content = "This document discusses [[machine-learning]] algorithms...";
create_test_file_with_age(&env.cfg, "…/old-ml-doc.md", "Old Machine Learning Doc", identical_content, 60);
create_test_file_with_age(&env.cfg, "…/new-ml-doc.md", "New Machine Learning Doc", identical_content, 1);
...
assert!(
    title_index(&search_result, "New Machine Learning Doc")
        < title_index(&search_result, "Old Machine Learning Doc"),
    "newer document should rank ahead of older document: {search_result}"
);
```

**Name accuracy problem.** The test name promises "when scores are equal." In practice,
`create_test_file_with_age` inserts `# {title}` into the body, so the stored content for
the two files differs at the heading. Their vector embeddings and text-score contributions
are therefore *not* equal — they are merely *similar*. The age gap (59 days) is large
enough to dominate in any realistic scoring function, but the test name creates a false
guarantee. A future developer reducing the age gap (e.g., to make the test run faster or
to probe a boundary) would trust the "equal scores" premise and draw incorrect conclusions
from a failure.

**Missing assertion on score values.** The test only checks `index_new < index_old`. It
does not verify that `result["decayWeight"]` for the old file is actually less than 1.0,
nor that the old and new files carry different scores. If the decay multiplier were always
applied as 1.0 (a logic bug that left decay inert), the 59-day age gap means the files
might still rank in creation order due to some implementation artifact. The rank-ordering
assertion would pass while the decay mechanism was silently broken.

**What a rewrite should look like:**
```rust
#[test]
fn search_memories_decay_raises_newer_file_above_older_file_in_hybrid_mode() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    // Title kept identical so embeddings are maximally similar
    let content = "Same body [[machine-learning]] [[deep-learning]] neural networks.";
    create_test_file_with_age(&env.cfg, "…/old-ml-doc.md", "Decay Test Doc", content, 60);
    create_test_file_with_age(&env.cfg, "…/new-ml-doc.md", "Decay Test Doc", content, 1);
    sync_graph(&env);

    let result = search_memories_json(&env, "machine learning deep learning", "Hybrid");
    assert!(result["success"].as_bool() == Some(true), "{result}");

    let old = find_result(&result, "Decay Test Doc"); // disambiguate by path if needed
    let new_idx = title_index(&result, "Decay Test Doc (new)"); // adjust names as appropriate

    // Primary: ordering
    assert!(new_idx < title_index(&result, "Decay Test Doc (old)"), "newer should rank higher");

    // Secondary: scores confirm decay is the cause, not arbitrary ordering
    let old_decay = old["decayWeight"].as_f64().expect("decayWeight");
    assert!(old_decay < 1.0, "60-day-old file should carry a decayed weight, got {old_decay}");
}
```

Note: identical titles would require path-based disambiguation in `find_result`. Keeping
*slightly* different titles is acceptable; the rewrite's point is to (a) drop the false
"equal scores" claim from the name and (b) assert on score values, not only position.

---

### T2: `search_memories_semantic_mode_applies_decay_weighting` — MERGE

The body of this test is identical to Test 1 in structure:

```
create_test_file_with_age(…, 45 days);  // T1: 60 days
create_test_file_with_age(…, 2 days);   // T1: 1 day
sync_graph(&env);
search_memories_json(&env, "…", "Semantic");  // T1: "Hybrid"
assert! title_index(new) < title_index(old);
```

The only varying dimension is the `mode` argument. This means that if the shared ranking
infrastructure breaks, Tests 1 and 2 fail together — tripling the output without tripling
the diagnostic information. If Semantic-mode decay were specifically broken (while Hybrid
was fine), this test would catch it; but that's the one meaningful contribution of
"Semantic" as a separate case, and it can be preserved inside a parameterized form.

**Merge path:** Fold T1, T2, T3 into a single `#[cfg(test)]` helper or a table-driven
loop. Each entry supplies `(mode, age_old, age_new, query)`. Three assertions, one body.

---

### T3: `search_memories_fulltext_mode_applies_decay_weighting` — MERGE

Same analysis as T2. Age numbers (50 vs 3 days) are the largest gap of the trio, which is
unnecessary — the test adds no new discriminating information over T2. Different query
terms (`authentication jwt token`) are cosmetic; they do not test distinct logic paths in
the decay or ranking pipeline.

---

### T4: `search_memories_files_within_grace_period_have_equal_ranking_weight` — KEEP

```rust
let day1 = find_result(&search_result, "Day 1 File");
let day10 = find_result(&search_result, "Day 10 File");

support::assert_approx_eq(day1["decayWeight"].as_f64().expect("day1 decay weight"), 1.0, 0.000_001);
support::assert_approx_eq(day10["decayWeight"].as_f64().expect("day10 decay weight"), 1.0, 0.000_001);
```

This is the only test in the file that inspects `decayWeight` values directly (not just
rank order). The tolerance `0.000_001` is appropriately tight. The ages chosen (1 and 10
days) are comfortably within the 28-day default grace window
(`MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT = 28`), and the assertion correctly models the
production rule `if age_days <= grace_period_days { return 1.0; }`.

**Minor gap:** The test name promises "equal ranking weight" but only proves that both
weights equal 1.0. It does not assert the two files appear in any particular order
relative to each other. That distinction is fine — the spec says "within grace period →
weight 1.0," not "all grace-period files rank equally." The name is slightly over-broad
but not misleading.

**Missing boundary test:** This test does not cover `age_days = grace_period_days + 1` (29
days), which should produce `decayWeight < 1.0`. That boundary case is untested in this
file. `proximity_decay_search.rs::compute_decay_weight_drops_below_one_after_grace_period`
covers it at the unit level, but an integration-level counterpart here would catch
regressions where the search path doesn't propagate the decay correctly.

**Overlap note:** `proximity_decay_search.rs::compute_decay_weight_is_one_within_grace_period`
tests the same logical property at the `compute_decay_weight` unit level. Both should
remain — they operate at different abstraction layers.

---

## Positive Observations

- **`create_test_file_with_age` correctly drives decay.** The frontmatter `created:
  {rfc3339}` field is read by `file_index.rs::index_file_content` and stored in the
  `file_content.created` column. `search/api.rs` then passes it to
  `compute_decay_weight_with_created`, which falls back from `last_accessed` to `created`
  when `last_accessed` is NULL. The test infrastructure and production wiring are coherent.

- **`TEST_MUTEX` + `isolated_env()` + `Config::override_root`** correctly serialize
  process-wide env state and provide real filesystem isolation. No test bleeds into
  another via database or config.

- **`minScore: 0.0`** is passed to `search_memories_json`, which prevents score-based
  filtering from hiding either test document. This is correct defensive practice.

- **`find_result` panics if the expected title is absent.** In Test 4, this acts as an
  implicit assertion that both files appear in results at all, not just that their weights
  are correct if found. This is a useful implicit check.

- **Age gaps are large enough to be flake-resistant.** 60 vs 1 day, 45 vs 2 days, 50 vs
  3 days: at these scales, neither CI timing noise nor grace-period rounding can invert
  the expected ordering. The tests will not produce intermittent failures.

---

## Recommendations

1. **Rewrite Test 1** as the canonical form of the trio. Correct the name (drop "when
   scores are equal" or make the title truly identical in both files). Add an assertion
   that `decayWeight` on the older file is `< 1.0` — this makes the test prove that decay
   caused the ordering, not just that the ordering happened to be correct.

2. **Merge Tests 2 and 3 into the rewritten Test 1** as a mode parameter. A
   `for mode in ["Hybrid", "Semantic", "FullText"]` loop (or a Rust macro table) gives
   mode-specific coverage with a single body. Failure messages should include the mode
   string.

3. **Keep Test 4**, but add a companion case for the grace period boundary exit (`age_days
   = grace_period_days + 1 → decayWeight < 1.0`). The current test demonstrates files
   *inside* the window; the boundary test demonstrates the *transition*.

4. **Reconcile traceability tags.** `T-GRAPH-DECAY-001.1` and `RTM FR-7.5` / `RTM
   NFR-7.5.2` do not appear in current `TODO.md` or `RTM.md`. Either update the comments
   to reference their CHANGELOG-archived equivalents explicitly, or annotate them as
   `(archived: Sprint-N)` to prevent readers from wasting time searching living documents.

---

## Load-Bearing Score: 4/4

| # | Test | Load-Bearing? | Notes |
|---|---|---|---|
| T1 | `search_memories_newer_content_ranks_higher_when_scores_are_equal` | ✅ Yes | Load-bearing but incorrectly named; assertion too shallow |
| T2 | `search_memories_semantic_mode_applies_decay_weighting` | ✅ Yes | Load-bearing but structurally duplicates T1 |
| T3 | `search_memories_fulltext_mode_applies_decay_weighting` | ✅ Yes | Load-bearing but structurally duplicates T1 |
| T4 | `search_memories_files_within_grace_period_have_equal_ranking_weight` | ✅ Yes | Unique assertion layer; clean and correct |

All four tests are load-bearing — each covers a real behavior (per-mode decay integration,
grace period contract) that would go undetected without it. The problem is not that tests
should be removed, but that the per-mode coverage could be consolidated without losing
signal and the rank-order assertions should be strengthened with score-value checks.

Net after applying verdicts: 3 merged/rewritten tests replace 4, with stronger assertions
and accurate names. Coverage is preserved; noise is reduced; the grace-period transition
boundary is added as a new case.

---
## SWE Remediation
- REWRITE T1 + MERGE T2/T3: Collapsed three structurally identical per-mode tests into `search_memories_decay_raises_newer_file_across_all_modes` — a table-driven loop over `["Hybrid", "Semantic", "FullText"]` with per-mode content/query pairings. Corrected name (dropped false "equal scores" claim). Added `decayWeight < 1.0` assertion on the older file per P0 recommendation. Semantic mode gracefully skips when ONNX embedding model is absent (mirrors `access_boosting_search.rs` pattern); guard asserts at least Hybrid + FullText are fully exercised.
- KEEP T4 (`search_memories_files_within_grace_period_have_equal_ranking_weight`): unchanged
- 4 tests → 2 tests; all mode coverage preserved with stronger assertions
- Commit: a017810

---
## SWE Remediation (pass 2)
- ADD `search_memories_file_just_past_grace_period_has_decayed_weight`: integration-level boundary test for grace period exit (Rec 3). Uses 30-day age (grace=28, effective_age=2) because power-law `x^(-0.5)` yields 1.0 at effective_age=1 (day 29). Control file at 1 day confirms grace period weight == 1.0.
- ANNOTATE traceability tags: `T-GRAPH-DECAY-001.1` and `RTM FR-7.5`/`NFR-7.5.2` annotated as `(archived: Sprint 15)` per Rec 4 to prevent readers searching living docs.
- 2 tests → 3 tests; grace-period boundary transition now covered at integration level
- Commit: e6524bd

---
## Red-team Audit
- ✅ REWRITE T1 (`search_memories_newer_content_ranks_higher_when_scores_are_equal` → `search_memories_decay_raises_newer_file_across_all_modes`): Name corrected (dropped false "equal scores" claim). `decayWeight < 1.0` assertion added on older file per P0. Matches eval spec.
- ✅ MERGE T2 (`search_memories_semantic_mode_applies_decay_weighting`): Folded into table-driven loop as `"Semantic"` case. Graceful ONNX-absent skip is a sound addition (mirrors `access_boosting_search.rs` pattern). Mode coverage preserved.
- ✅ MERGE T3 (`search_memories_fulltext_mode_applies_decay_weighting`): Folded into table-driven loop as `"FullText"` case. Original content/query pairings carried forward. No unique coverage lost.
- ✅ KEEP T4 (`search_memories_files_within_grace_period_have_equal_ranking_weight`): Unchanged except for traceability annotation. Matches eval spec.
- ✅ ADD `search_memories_file_just_past_grace_period_has_decayed_weight` (Rec 3): Correctly tests grace-period exit at 30 days (grace=28, power-law effective_age=2 ≈ 0.707). Control file at 1 day confirms grace-period weight == 1.0. Addresses the gap identified in the eval.
- ✅ ANNOTATE traceability (Rec 4): `T-GRAPH-DECAY-001.1` and `RTM FR-7.5`/`NFR-7.5.2` annotated as `(archived: Sprint 15)` on all three tests. Prevents readers searching living docs.
- ✅ No unrelated changes: both commits (`a017810`, `e6524bd`) touch only `graph_decay_search.rs`. Support file unchanged. No source code outside the test file was modified.
- ✅ Test count: 4 → 2 (pass 1) → 3 (pass 2). All mode coverage preserved. Assertions strengthened with score-value checks.
- ✅ `modes_exercised >= 2` guard ensures Hybrid + FullText always exercise full assertion path even when ONNX is absent. Sound defensive practice.
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- graph_decay_search`: 3 passed, 0 failed
- Verdict: PASS
