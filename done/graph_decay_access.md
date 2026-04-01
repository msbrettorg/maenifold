---
id: T-80.77
title: "Test Evaluation: graph_decay_access.rs"
status: done
pipeline: tdd
---

# Test Evaluation: graph_decay_access.rs

**Evaluator:** Red Team — Gray (Skeptical Inquiry)  
**Traceability:** T-GRAPH-DECAY-002.1 · RTM NFR-7.6.1  
**File under review:** `crates/maenifold-core/tests/graph_decay_access.rs`  
**Test count:** 2  
**Date:** 2025-07

---

## Summary

`graph_decay_access.rs` contains two integration tests that probe whether
`ReadMemory` boosts the computed decay weight and whether `SearchMemories`
leaves it unchanged. The file sits in a dense ecosystem of decay-related tests
(`access_boosting_reads.rs`, `access_boosting_search.rs`,
`decay/scoring.rs` inline tests, `config_decay_weights.rs`, etc.), so every
verdict must answer one hard question: **does removing this file leave a
gap that no other test fills?**

### Pre-read infrastructure findings

The test harness is sound. `support.rs` contains:

- `decay_weight_for_uri`: reads `last_accessed` + `status` from the live DB
  and calls `compute_decay_weight` — a clean cross-layer probe.
- `get_last_accessed`: returns `Option<DateTime<Utc>>` — typed, not string,
  enabling `assert_eq!` instead of string comparison.
- `assert_approx_eq`: float tolerance helper used in Test 2.

The sync SQL in `sync/file_index.rs` is critical context:

```sql
INSERT INTO file_content (..., created, last_accessed)
VALUES (
  ...,
  COALESCE(?8, ?4),              -- created = frontmatter_created ?? indexed_at
  COALESCE(
    (SELECT last_accessed FROM file_content WHERE file_path = ?1),
    COALESCE(?8, ?4)
  )                              -- last_accessed = existing ?? frontmatter_created ?? indexed_at
)
ON CONFLICT DO UPDATE SET
  ...,
  last_accessed = COALESCE(file_content.last_accessed, excluded.last_accessed)
```

Consequence: on **first sync of a file with 60-day-old frontmatter `created`**,
`last_accessed` is seeded to 60 days ago — making `weight_before_access < 0.8`
a valid initial condition in Test 1. This is non-obvious; the test should
document this assumption.

### Traceability gap

Both tests cite `RTM NFR-7.6.1`. This identifier does **not exist** in
`RTM.md`. The closest items are RTM-179 through RTM-194 (NFR-7 family). The
`access_boosting_reads.rs` and `access_boosting_search.rs` files cite
`T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.1` and `NFR-7.6.2` respectively.
The sub-numbering scheme `-3` vs. the file's plain `002.1` is inconsistent.
**NFR-7.6.x is an unregistered requirement.**

---

## Verdicts

| # | Test name | Load-bearing? | Unique? | Assertions correct? | Name clear? | Parameterizable? | Verdict |
|---|-----------|:-------------:|:-------:|:-------------------:|:-----------:|:----------------:|---------|
| 1 | `get_decay_weight_after_read_memory_returns_higher_weight` | ✅ Yes | ⚠️ Partial | ⚠️ Weak upper bound | ✅ Yes | ⚠️ Age hardcoded | **REWRITE** |
| 2 | `get_decay_weight_after_search_memories_weight_is_unchanged` | ❌ No | ❌ No | ⚠️ Redundant assertion | ✅ Yes | N/A | **MERGE** |

---

## Per-Test Analysis

---

### Test 1 — `get_decay_weight_after_read_memory_returns_higher_weight`

**Traceability:** T-GRAPH-DECAY-002.1 · RTM NFR-7.6.1

**What it actually tests (chain):**
```
create 60-day file → sync (seeds last_accessed = 60 days ago)
  → decay_weight_for_uri   [assert < 0.8]
  → ReadMemory             [triggers update_last_accessed(now)]
  → decay_weight_for_uri   [assert > before AND >= 0.95]
```

**Is it load-bearing?**  
Yes — with a caveat. `access_boosting_reads.rs` already proves that
`ReadMemory` updates the `last_accessed` DB timestamp. `decay/scoring.rs`
inline tests prove that `compute_decay_weight` returns high values for
recently-accessed files. This test is the only one that **bridges both
layers end-to-end** at the weight level: it confirms that a live `ReadMemory`
call causes a measurable change in the output of `decay_weight_for_uri`. If
either the DB write or the weight calculation regressed, the other test files
would catch it, but the integration glue here is distinct.

**Unique coverage:**  
Unique at the *weight-layer* integration level. Not unique at the
*timestamp-layer* (`access_boosting_reads.rs` covers that three ways).

**Are the assertions correct?**

| Assertion | Verdict | Issue |
|-----------|---------|-------|
| `!read_result.starts_with("ERROR:")` | ✅ Correct | — |
| `weight_before_access < 0.8` | ✅ Correct | Magic number; no comment explaining why 60 days → < 0.8 (depends on default half-life config). Fragile if defaults change. |
| `weight_after_access > weight_before` | ✅ Correct | Necessary but not sufficient on its own |
| `weight_after_access >= 0.95` | ⚠️ **Too permissive** | A file accessed 3–4 days ago (within grace period) would also satisfy this. The real expectation is that `update_last_accessed` sets `last_accessed = Utc::now()`, which means `age_days = 0 ≤ grace_period → weight = 1.0`. The threshold should be `>= 0.99` or `== 1.0`. The 0.95 floor leaves a false-negative window. |

**Hidden assumption not documented:** Test 1 relies on `sync_graph` seeding
`last_accessed` from frontmatter `created` (60 days ago). This is true because
of the `COALESCE((SELECT last_accessed ...), ...)` SQL on first insert — but
it is not stated anywhere in the test. If the schema migration changed this
default, the precondition assertion would silently flip.

**Parameterizability:** The `60` days and `0.8` threshold are both magic
numbers. The 60-day age is not critical to the semantic (any age > grace_period
would work), but if `decay_half_life_days` config changes, the `< 0.8` bound
could fail.

**Recommended rewrite:**
```rust
// Assert weight is 1.0 (not just >= 0.95) — file was accessed moments ago
// File age > grace period is explicitly documented with the formula
assert!(
    weight_before_access < weight_for_age_days(60, &env.cfg),
    "precondition: 60-day-old file has decayed weight"
);
assert_approx_eq(weight_after_access, 1.0, 0.01,
    "weight after ReadMemory should be ~1.0 (accessed moments ago, within grace period)");
```

---

### Test 2 — `get_decay_weight_after_search_memories_weight_is_unchanged`

**Traceability:** T-GRAPH-DECAY-002.1 · RTM NFR-7.6.1

**What it actually tests (chain):**
```
create 60-day file → sync
  → get_last_accessed, decay_weight_for_uri  [capture baseline]
  → SearchMemories (Hybrid mode)
  → assert last_accessed unchanged (DateTime equality)
  → assert weight unchanged (tolerance 1e-6)
```

**Is it load-bearing?**  
No. `access_boosting_search.rs` already contains three tests — Hybrid,
Semantic, and FullText — that prove `SearchMemories` does not update
`last_accessed`. This test covers only Hybrid mode and adds nothing those
tests do not already provide.

**Unique coverage:**  
None. The `last_accessed` equality assertion duplicates
`access_boosting_search.rs::search_memories_does_not_update_last_accessed`.
The weight equality assertion is **purely derived**: if
`last_accessed_before == last_accessed_after`, then
`compute_decay_weight(last_accessed_before)` and
`compute_decay_weight(last_accessed_after)` called with `Utc::now()` within
the same test run will differ by at most a few nanoseconds in the age
calculation — the tolerance of `1e-6` is so tight that this is effectively
checking identity of a deterministic function over equal inputs. It adds no
semantic value beyond the timestamp equality check.

**Are the assertions correct?**

| Assertion | Verdict | Issue |
|-----------|---------|-------|
| `assert_eq!(after_last_accessed, initial_last_accessed)` | ✅ Correct | Duplicates `access_boosting_search.rs` |
| `assert_approx_eq(weight_after, weight_before, 0.000_001)` | ⚠️ **Redundant** | Proves nothing beyond the `last_accessed` equality. Also: `compute_decay_weight` calls `Utc::now()` internally; both calls happen within milliseconds, so the result will be the same regardless of whether `last_accessed` changed. This assertion cannot distinguish the "search updated last_accessed to a new value" case from the "search left it unchanged" case if the new value is also recent. |

**The assert_approx_eq tolerance is arguably both too tight and too loose at once:**
- Too tight: any slow CI runner could fail due to clock granularity differences
  between the two `Utc::now()` calls inside `compute_decay_weight`.
- Too loose (semantically): 1e-6 cannot catch a 1-second shift in
  `last_accessed`; a file accessed 1 second ago vs. now produces
  `Δweight ≈ 9e-8`, well within tolerance.

**Recommended action:** Fold the `last_accessed` check (only) into
`access_boosting_search.rs` as an additional Hybrid-mode coverage note. Drop
the weight-equality assertion entirely; it is derivable and fragile.

---

## Recommendations

### Priority 1 — Fix traceability (both tests)

`RTM NFR-7.6.1` is not registered in `RTM.md`. Either:
- Add `RTM-XXX` entries for NFR-7.6.x (access-boost precision requirements), or
- Change citations to the nearest registered items (RTM-184 for turn timeout
  or the appropriate decay NFR in `config_decay_weights.rs` lineage).

### Priority 2 — Rewrite Test 1

Strengthen `weight_after_access >= 0.95` → `assert_approx_eq(weight_after_access, 1.0, 0.01)`.  
Document the sync precondition: *"On first sync, `last_accessed` is seeded
from frontmatter `created`; this is the initial decay baseline."*  
Consider extracting `60` and `0.8` as named constants or a helper function
`expected_decay_weight_for_days(age, config)`.

### Priority 3 — Merge Test 2 / Remove

Test 2 provides no unique coverage. Options:
1. **Remove** — `access_boosting_search.rs` already covers this three ways.
2. **Merge** — If weight-level verification is desired, add a single
   weight-unchanged assertion to `access_boosting_search.rs`'s existing
   `search_memories_does_not_update_last_accessed` test rather than
   maintaining a separate file.

### Priority 4 — Missing coverage in this area

`graph_decay_access.rs` (and its ecosystem) does not test:
- `BuildContext` not updating `last_accessed` (only SearchMemories and ReadMemory are tested)
- `FindSimilarConcepts` not boosting access weight
- `WriteMemory` (update path) not resetting `last_accessed` of an already-indexed file
- The case where ReadMemory **fails** — does a failed read incorrectly update
  `last_accessed`? (Access boost should only happen on successful reads.)

---

## Load-Bearing Score: 1/2

One of two tests (`get_decay_weight_after_read_memory_returns_higher_weight`)
contributes distinct integration coverage that is not fully replicated
elsewhere. The other (`get_decay_weight_after_search_memories_weight_is_unchanged`)
is redundant with the `access_boosting_search.rs` suite and should be merged
or removed. The file as a whole is **below average** for its size: 50%
load-bearing, with a critical assertion weakness in the test that IS
load-bearing.
