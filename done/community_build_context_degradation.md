# Test Evaluation: community_build_context_degradation

> **Reviewer perspective:** 🔘 Gray — Skeptical Inquiry  
> **Source file:** `crates/maenifold-core/tests/community_build_context_degradation.rs`  
> **Traceability:** T-W1-8 (NFR-2), T-12.7 (NFR-1), FR-13.10, FR-16.10  
> **Production code under test:** `src/graph/build_context.rs`, `src/graph/formatter.rs`

---

## Summary

This file tests the **graceful-degradation path** of `BuildContext`: what happens when
community-enrichment data is absent or incomplete at query time. The production code gates
all community enrichment on a single boolean flag (`include_community_data =
query_community_id.is_some()`, `build_context.rs:61`). Three distinct scenarios are
covered:

| # | Scenario | Code path |
|---|----------|-----------|
| 1 | `concept_communities` table is entirely empty | `query_community_id = None` → `include_community_data = false` |
| 2 | Table has rows but queried concept was never assigned a community | Same flag — but via orphan look-up |
| 3 | Queried concept **has** a community; one of its *neighbors* does not | `include_community_data = true` but a neighbor's `community_id` column is NULL |

All three are exercised end-to-end through `ToolRegistry::try_invoke("BuildContext", …)`.
No mocking of the storage layer is used, which gives high confidence in the full call
chain but also means each test spins a real SQLite database via `TestEnv`.

---

## Verdicts

| Test name | Load-bearing? | Unique? | Assertions correct? | Name quality | Parameterisable? | Verdict |
|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| `build_context_graceful_degradation_no_community_data` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Good | ➖ Not needed | **KEEP** |
| `build_context_graceful_degradation_concept_not_in_communities` | ✅ Yes | ✅ Yes | ⚠️ Partial — one assertion deliberately omitted | ✅ Good | ➖ Not needed | **REWRITE** |
| `build_context_null_community_id_does_not_crash` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Name undersells intent | ➖ Not needed | **KEEP** |

---

## Per-Test Analysis

### `build_context_graceful_degradation_no_community_data`

**Scenario:** `concept_communities` is verified empty before invocation. Two concepts
share an edge. BuildContext is called on one of them.

**What it probes:**
- `include_community_data = false` when `load_concept_community_id` returns `None` for
  an empty table.
- Formatter emits `"CommunityId": null` and `"CommunitySiblings": []` (formatter.rs:64–65).
- Core graph path still returns `DirectRelations` (no silent swallowing of results).
- Each relation entry in `DirectRelations` also carries `"CommunityId": null` because
  `render_relation_json` receives `None` when `include_community_data = false`.

**Assertion audit (5 assertions):**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | Precondition: `COUNT(*) FROM concept_communities == 0` | ✅ | ✅ |
| 2 | `data["CommunityId"].is_null()` | ✅ | ✅ |
| 3 | `data["CommunitySiblings"]` is empty | ✅ | ✅ |
| 4 | `data["DirectRelations"]` is **non-empty** | ✅ | ✅ — verifies graph still works without community data |
| 5 | Each relation's `CommunityId` is null | ✅ | ✅ — catches the formatter loop at `formatter.rs:27–35` |

All five assertions are independently meaningful. No assertion is redundant with another.
The precondition check (assertion 1) is especially valuable: it fails fast if a previous
test leaked community rows (shared singleton database is a real risk in this suite).

**Concerns:** None material. Minor: the setup uses hardcoded names `degrade-a` / `degrade-b`
and weight `5`. These are arbitrary but the test doesn't depend on their values, so no issue.

**Verdict: KEEP** — clean, complete, correctly targeted.

---

### `build_context_graceful_degradation_concept_not_in_communities`

**Scenario:** A triangle (`existing-a/b/c`) runs through `detect_and_persist`, producing
real community rows. An `orphan-concept` is then inserted with an edge to `existing-a`
but *no* community detection rerun. BuildContext is called on the orphan.

**What it probes:**
- `include_community_data = false` when a concept is missing from `concept_communities`
  even though the table has other rows.
- Orphan's own `CommunityId` and `CommunitySiblings` are empty/null.
- The edge to `existing-a` is still traversed and returned.

**Assertion audit:**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | Precondition: community data exists for existing concepts | ✅ | ✅ |
| 2 | Precondition: orphan not in `concept_communities` | ✅ | ✅ |
| 3 | Orphan's `CommunityId` is null | ✅ | ✅ |
| 4 | `CommunitySiblings` is empty | ✅ | ✅ |
| 5 | `existing-a` appears in `DirectRelations` | ✅ | ✅ |
| 6 | `existing-a`'s `CommunityId` in `DirectRelations` — **deliberately not asserted** | ✅ needed | ❌ **missing** |

**Finding — skipped assertion (line 147–151):**

```rust
// existing-a's CommunityId is null because include_community_data=false for orphan query.
let _ = existing_a_rel; // documented difference from C# — no assertion on CommunityId here.
```

The test locates `existing_a_rel` and then silently discards it with `let _ = …`. This is
dead code in a test body — a code smell. The comment explains *why* the value is null
(because `include_community_data = false` suppresses community look-ups for **all** relations
when the query concept has no community), but it never asserts that claim.

**Why this matters:** The production behaviour described — *"when the query concept is not
in a community, its direct relations are also served with null CommunityId"* — is a
non-obvious, intentional design decision. It is the exact thing this test should lock in.
Without the assertion, a future refactor that accidentally starts populating CommunityId
for relations even when `include_community_data = false` would not be caught here.

**Secondary finding — stale file reference (line 147):**

```rust
// This is the correct Rust behaviour (confirmed by reading search.rs line ~1162).
```

`search.rs` at line 1162 does not exist. The only `search.rs` in the codebase is
`src/benchmark/search.rs`, which is unrelated. The relevant logic lives at
`src/graph/build_context.rs:61` (`let include_community_data = query_community_id.is_some()`).
This stale reference erodes comment credibility.

**Verdict: REWRITE** — Add the missing assertion; fix the stale file reference.

Suggested replacement for the dead-variable block:

```rust
// Rust behaviour: when the query concept has no community assignment,
// include_community_data = false, so ALL DirectRelation CommunityIds are null.
// (build_context.rs:61, formatter.rs:28–35)
assert!(
    existing_a_rel["CommunityId"].is_null(),
    "FR-13.10: when query concept is not in a community, relation CommunityIds \
     must also be null (include_community_data=false path)"
);
```

---

### `build_context_null_community_id_does_not_crash`

**Scenario:** A triangle (`null-comm-a/b/c`) has community data. An orphan
(`null-comm-orphan`) is added *after* the last detection run and gains edges to `a` and
`b`. BuildContext is called on `null-comm-a` — a concept that **has** a community — while
one of its direct relations (`null-comm-orphan`) does not.

**What it probes:**
- `include_community_data = true` (query concept is in a community).
- The formatter calls `load_concept_community_id` per relation inside
  `render_build_context_json` (formatter.rs:27–35).
- Mixed null/non-null `CommunityId` in a single `DirectRelations` list.
- The SQL look-up returning `None` for a missing concept does not produce a panic or
  serialisation error (FR-16.10).

**Assertion audit (5 assertions):**

| # | Assertion | Necessary? | Correct? |
|---|-----------|:---:|:---:|
| 1 | Precondition: orphan has no community row | ✅ | ✅ |
| 2 | Orphan appears in `DirectRelations` | ✅ | ✅ |
| 3 | Orphan's `CommunityId` is null | ✅ | ✅ — the core FR-16.10 claim |
| 4 | `null-comm-b` appears in `DirectRelations` | ✅ | ✅ |
| 5 | `null-comm-b`'s `CommunityId` is **non-null** | ✅ | ✅ — validates that community data for assigned concepts is not accidentally suppressed |

Assertion 5 (the positive control) is particularly valuable: it distinguishes "null because
orphan" from "null because something silently broke for everyone".

**Name concern:** `…_does_not_crash` describes a weaker contract than this test actually
enforces. The test also verifies specific structural output (`CommunityId` correctness for
both null and non-null cases). A more accurate name:

```
build_context_mixed_community_state_in_direct_relations_is_correct
```

This is a minor nit. The existing name does not mislead, it only undersells.

**Verdict: KEEP** — structurally sound, assertions correct. Optional rename.

---

## Recommendations

1. **[P0 — REWRITE]** `build_context_graceful_degradation_concept_not_in_communities`:
   Replace `let _ = existing_a_rel;` with an explicit assertion that `existing_a_rel["CommunityId"].is_null()`. This locks in the documented Rust-vs-C# behavioural difference as a verified contract rather than a comment. Fix the stale `search.rs line ~1162` reference to `build_context.rs:61`.

2. **[P2 — Optional rename]** `build_context_null_community_id_does_not_crash` →
   `build_context_mixed_community_state_in_direct_relations_is_correct`. The current name
   is accurate but misleadingly weak.

3. **[P3 — Consider]** Tests 1 and 2 share the same high-level assertion shape (null
   CommunityId, empty CommunitySiblings, DirectRelations intact). A shared helper such as
   `assert_no_community_enrichment(data)` would make the shared contract explicit and
   reduce duplication. This is stylistic; the tests are readable as-is.

4. **[P3 — Consider]** The precondition check in test 1 (`COUNT(*) == 0`) should arguably
   exist in test 2 as well — currently test 2 verifies the orphan is absent but does not
   verify the row counts are as expected before the main invocation. Low risk given the
   mutex, but belt-and-suspenders.

---

## Load-Bearing Score: 3/3

All three tests in this file are load-bearing. Each covers a distinct code path through
`build_context.rs` and `formatter.rs`, and none is a duplicate of the other or of any
test in the companion file `community_build_context_siblings.rs` (which covers the
positive community-data path). Removing any one of these tests would leave a degradation
scenario unverified.

| Test | Load-bearing | Reasoning |
|------|:---:|---------|
| `…no_community_data` | ✅ | Only test that covers total absence of community rows |
| `…not_in_communities` | ✅ | Only test that covers orphan-in-populated-table scenario; distinct `include_community_data` trigger |
| `…does_not_crash` | ✅ | Only test exercising `include_community_data = true` with a null-community neighbor in `DirectRelations` |

---
## SWE Remediation
- KEEP `build_context_graceful_degradation_no_community_data`: unchanged
- REWRITE `build_context_graceful_degradation_concept_not_in_communities`: replaced dead `let _ = existing_a_rel;` with `assert!(existing_a_rel["CommunityId"].is_null(), ...)` locking in the include_community_data=false contract (P0, FR-13.10). Fixed stale `search.rs line ~1162` comment → `build_context.rs:61, formatter.rs:28–35`.
- KEEP+RENAME `build_context_null_community_id_does_not_crash` → `build_context_mixed_community_state_in_direct_relations_is_correct` (P2 — name undersold intent)
- Commit: 33da4ed
