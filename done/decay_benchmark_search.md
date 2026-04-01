---
id: T-80.49
title: "Test Evaluation: decay_benchmark_search"
status: done
pipeline: tdd
---

# Test Evaluation: decay_benchmark_search

**File**: `crates/maenifold-core/tests/decay_benchmark_search.rs`
**Traceability**: T-12.7 · RTM NFR-1
**Related FR/NFR**: NFR-1 (C# Oracle Parity), NFR-2 (Test Coverage)
**Evaluator color**: 🔘 Gray — Skeptical Inquiry
**Date**: 2025-07-23

---

## Summary

`decay_benchmark_search.rs` contains **2 integration tests** that attempt to verify
[[temporal decay]] prevents stale content from drowning recent [[search ranking]] signals.
Both tests exercise the full end-to-end pipeline: file creation with age-stamped frontmatter
→ `sync_graph` → `search_memories` (Hybrid mode) → rank position assertion.

The tests carry a T-12.7 traceability tag and both reference NFR-1 (C# Oracle Parity).
However, the actual assertions never inspect `decayWeight` or `fusedScore` fields that are
present in the JSON output. Both tests assert only on **rank position** of a document that
would rank equally well (or better) on **pure text relevance** alone, without decay. This is
the central problem: the named mechanism (decay) is untested in isolation, and the
behavioral coverage is already provided — with tighter controls — by `graph_decay_search.rs`
and `proximity_decay_search.rs`.

### Key Findings

| Finding | Affects |
|---|---|
| Neither test inspects `decayWeight` or `fusedScore` fields available in JSON output | Both tests |
| Recent decision docs have the highest keyword density for the query — they would win on text relevance alone, no decay required | Both tests |
| The test names claim to test decay as a mechanism; the assertions only confirm rank position | Both tests |
| `graph_decay_search.rs` already covers the same behavior using **identical content** (properly isolated) | Test 1 |
| `proximity_decay_search.rs` already covers decay-weighted `fusedScore` ordering with `decayWeight` field assertions | Both tests |
| "precision_at_k" is a misnamed concept — P@K measures fraction of top-K that are relevant; this test checks a single document's position | Test 2 |
| Test 2 uses `< 3` (top 3) while Test 1 uses `< 2` (top 2), with no stated rationale for the looser threshold | Test 2 |
| Both tests are structurally identical (create old docs + one fresh decision → sync → search → rank assert) and could be parameterized | Both tests |

---

## Verdicts

| Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `benchmark_context_rot_decay_prevents_old_noise_drowning_recent_signal` | ⚠️ Partial | ❌ No — covered by `graph_decay_search.rs` | ❌ No — rank position doesn't isolate decay | ❌ No — implies decay is the tested mechanism | ✅ Yes | **REWRITE** |
| `benchmark_precision_at_k_relevant_recent_doc_in_top_3` | ⚠️ Partial | ❌ No — scenario overlap with both graph_decay and proximity_decay tests | ❌ No — `< 3` is weaker than justified; "P@K" is a misnomer | ❌ No — precision@K is not computed | ✅ Yes | **REWRITE** |

---

## Detailed Analysis

### Test 1: `benchmark_context_rot_decay_prevents_old_noise_drowning_recent_signal`

#### What it does
Creates 5 stale auth-related docs (50–90 days old) and 1 recent auth decision (1 day old).
Syncs. Searches for `"authentication approach for the API"`. Asserts the decision document
appears in the top 2 results (`decision_index < 2`).

#### What it claims to prove
That **temporal decay** prevents stale documents from burying the recent decision.

#### What it actually proves (or fails to prove)

**Question the premise**: Would the recent decision document rank in the top 2 even with
decay **disabled** (all `decayWeight = 1.0`)?

Almost certainly yes. The decision document's title is
`"DECISION: API Authentication Approach"` and the query is
`"authentication approach for the API"`. The title contains four of the five
significant query tokens verbatim. Even the stale documents are on the same topic
(auth + API), so the decision doc's title match would dominate text scoring alone.
There is no control experiment.

**The available evidence not used**: The JSON output from `search_memories_json`
includes `decayWeight` and `fusedScore` fields for every result (confirmed in
`src/search/api.rs` lines 276–277). The test never reads them. An assertion like:

```rust
let decision_result = &results[decision_index.expect("rank")];
assert!(
    decision_result["decayWeight"].as_f64().expect("decay weight") > 0.95,
    "recent document should have near-full decay weight"
);
assert!(
    old_doc_decay_weight < 0.5,
    "90-day-old document should be heavily suppressed"
);
```

…would actually demonstrate decay is working. Without it, the test is a
**text-relevance tautology** dressed up as a decay test.

**Coverage overlap**: `graph_decay_search.rs::search_memories_newer_content_ranks_higher_when_scores_are_equal`
uses *identical content* for both the old and new document, meaning text relevance is
neutralized and only decay determines the winner. That is a properly isolated decay test.
`decay_benchmark_search.rs` Test 1 adds no incremental coverage of the decay mechanism.

**Name problem**: `context_rot_decay_prevents_old_noise_drowning_recent_signal` is 74
characters of marketing language that overstates what is actually asserted. The test
confirms a document appears in the top 2; it does not confirm that removing decay would
change that outcome.

---

### Test 2: `benchmark_precision_at_k_relevant_recent_doc_in_top_3`

#### What it does
Creates 4 unrelated stale docs, 3 stale semantically-related docs, and 1 recent decision
(2 days old). Syncs. Searches for `"error handling strategy for services"`. Asserts the
decision document appears in the top 3 (`decision_rank < 3`).

#### What it claims to prove
"Precision at K" — that relevant, recent content appears in the top K results.

#### What it actually proves (or fails to prove)

**Misnomer**: Precision@K is an information retrieval metric defined as:
`P@K = (number of relevant documents in top K) / K`. This test does not compute P@K.
It checks one document's rank position. This is "recall at K" for a single target
document, or simply a rank assertion. The name is wrong.

**Weaker threshold without justification**: Test 1 asserts `< 2` (top 2).
Test 2 asserts `< 3` (top 3). The decision document's title
`"DECISION: Error Handling Strategy"` contains `"error handling strategy"` verbatim
from the query. If anything, Test 2 should have the *tightest* assertion (top 1),
not the loosest. The `< 3` threshold means two irrelevant or weakly relevant documents
could outrank the decision and the test would still pass. That is a near-silent failure.

**Same text-relevance problem as Test 1**: The 4 "unrelated" documents cover Docker,
CSS Grid, PostgreSQL migration, and Git workflow — zero overlap with "error handling
strategy for services". Even with decay disabled and the old related docs scoring
maximally, the decision document's verbatim title match would rank it #1 on text
relevance alone.

**No assertion on the stale related docs**: The test captures `old_related_1_rank` and
`old_related_2_rank` and formats them into the failure message, but never asserts on
them. These variables are dead weight in the success path. If decay were broken and a
90-day-old "Error Handling Research" doc ranked #1 above the decision, the test would
still pass as long as the decision lands at index 1 or 2.

**Coverage overlap**: Same as Test 1 — covered by `graph_decay_search.rs` (identical
content), `proximity_decay_search.rs` (direct `decayWeight`/`fusedScore` assertions),
and `decay_benchmark_weights.rs` (`benchmark_access_boosting_rescues_actively_used_old_content`).

---

## Recommendations

### 1. Rewrite Test 1 — Isolate the decay signal

The scenario (stale research vs. fresh decision) is **worth keeping as a narrative test**,
but the assertions need to actually touch the decay mechanism. Proposed changes:

- Add an assertion on `decayWeight` for the decision document (should be `>= 0.95`) and
  for the oldest stale document (should be `< 0.5` at 90 days).
- Or: Add a second stale document with *identical text* to the decision document but 60
  days old, then assert the fresh version outranks the stale twin. This isolates decay
  as the variable.
- Rename to `recent_decision_outranks_stale_research_in_hybrid_search` — accurate and
  concise.

### 2. Rewrite Test 2 — Fix the name and the threshold

- Remove "precision_at_k" from the name. This is not P@K.
- Tighten `< 3` to `< 2` or `< 1`. If the verbatim query match guarantees top 1, assert
  it. A passing test with a loose threshold hides future regressions.
- Add assertions on `old_related_1_rank` and `old_related_2_rank` — they are already
  captured. Assert they rank *below* the decision document.
- Rename to `recent_decision_outranks_stale_related_docs_in_hybrid_search` or similar.

### 3. Consider merging into a parameterized test

Both tests share the same structure. A single `#[test_case(...)]` or data-driven
approach would eliminate duplication and make adding new scenarios cheaper.

```rust
// Proposed parameterized structure
fn assert_recent_decision_wins(
    domain_docs: &[(&str, &str, &str, i64)], // (path, title, content, days_ago)
    decision: (&str, &str, &str, i64),
    query: &str,
    expected_top_k: usize,
) { ... }
```

### 4. Confirm T-12.7 coverage is satisfied by better-isolated tests

`graph_decay_search.rs` (T-GRAPH-DECAY-001.1) already provides isolated decay ranking
tests for all three search modes. If those tests are green, T-12.7 decay-in-search
coverage is satisfied. The benchmark tests should be repositioned as **scenario tests**
(realistic but not proof-of-mechanism), not as the primary evidence for NFR-1 decay
compliance.

---

## Load-Bearing Score: 0/2

> *"Load-bearing" is defined strictly: would removing this test expose a regression that
> no other test in the suite would catch?*

**Test 1**: The decay-in-ranking behavior it purports to test is already covered by
`graph_decay_search.rs::search_memories_newer_content_ranks_higher_when_scores_are_equal`
(identical content, isolated decay variable, all three search modes) and
`proximity_decay_search.rs::search_memories_orders_results_by_decay_weighted_score`
(`decayWeight` + `fusedScore` assertions). Removing Test 1 would not leave any gap.
**Score: 0**

**Test 2**: No unique behavioral coverage. The rank-position-only assertion with a
looser threshold (`< 3`) catches strictly fewer regressions than the tests it duplicates.
Removing Test 2 would not leave any gap.
**Score: 0**

**Total: 0/2**

Both tests are safe to remove *as currently written*. They should be **rewritten**
(not removed) to provide genuine decay-isolation assertions and to eliminate the
naming debt. Once rewritten to assert on `decayWeight`, `fusedScore`, and the relative
rank of old-vs-new docs with equal text relevance, their load-bearing score would rise
to 2/2 — covering a **realistic multi-document scenario** that the unit-level isolated
tests do not fully replicate.
