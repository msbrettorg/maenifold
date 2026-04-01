---
id: T-80.26
title: "Test Evaluation: community_sync.rs"
status: done
pipeline: tdd
---

# Test Evaluation: community_sync.rs

**File**: `crates/maenifold-core/tests/community_sync.rs`
**Traceability**: T-12.7 (NFR-1 oracle parity), T-W1-8 (NFR-2 test coverage)
**Requirements coverage**: FR-13.4, FR-13.6
**Evaluated by**: Red-team / Gray (skeptical inquiry)
**Test count**: 2

---

## Summary

`community_sync.rs` contains two integration tests that are architecturally different from every
other file in the community test suite: they exercise [[community detection]] through the live
`ToolRegistry` path rather than calling `detect_and_persist` directly. That makes them uniquely
valuable for catching regressions where the detection algorithm is correct but the call-site wiring
inside `Sync` or `BuildContext` breaks silently.

Both tests are **load-bearing** — they cover integration paths that no other file exercises.
However, both share the same structural weakness: their assertions are *sign-of-life* checks
(`rows > 0`, `!is_null()`) rather than *correctness* checks. A regression that silently assigns
every concept to the same community, or populates CommunityId with a stale/wrong value, would
pass both tests without triggering a failure.

Additionally, `build_context_includes_community_id_on_related_concepts` is **misplaced**: the
file is named `community_sync.rs` (implying [[sync pipeline]] scope) but this second test is
concerned purely with `BuildContext` output format — it has nothing to do with the Sync tool. It
belongs alongside the other `community_build_context_*.rs` files.

---

## Verdicts

| # | Test name | Load-bearing? | Unique? | Assertions correct? | Name clear? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `sync_produces_community_assignments` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | Low value | **REWRITE** |
| 2 | `build_context_includes_community_id_on_related_concepts` | ✅ Yes | ⚠️ Partial | ⚠️ Partial | ✅ Yes | Low value | **MERGE** |

---

## Findings Per Test

### Test 1 — `sync_produces_community_assignments`

**FR coverage**: FR-13.4 — _"Sync populates concept_communities"_
**Setup path**: `WriteMemory` (×3) → `Sync` via `ToolRegistry::try_invoke`

#### What it proves

This is the only test in the entire community suite that exercises the real `Sync` tool path.
All other community tests call `detect_and_persist()` directly. If someone wired community
detection into a separate helper that is never called by `Sync`, every unit test would remain
green while this tool silently regressed. This test is the canary.

The topology it constructs is reasonable: two clusters, one triangle-ish (a–b–c, with a–b
appearing in two files) and one isolated pair (d–e). The expected result is two communities.

#### What's missing

**Assertion 1** — `rows > 0` is the weakest possible proof of correctness. A bug that assigns
all concepts to `community_id = 0` passes. A bug that writes garbage community IDs passes.
A bug that persists only _one_ row (partial write) might pass if that row is for `sync-comm-a`.

**Assertion 2** — Checking `names.contains("sync-comm-a")` and `names.contains("sync-comm-d")`
verifies that two concepts from _different expected clusters_ were persisted. That is useful, but
it does not verify that they were assigned to _different_ communities. The entire purpose of
community detection is to _group_ related concepts. The test never asks: "are sync-comm-a and
sync-comm-b in the same community?" or "are sync-comm-a and sync-comm-d in different communities?"

**Missing structural assertions**:
```rust
// Concepts that co-occur heavily must share a community.
let id_a = get_community_id(&conn, "sync-comm-a").expect("a");
let id_b = get_community_id(&conn, "sync-comm-b").expect("b");
assert_eq!(id_a, id_b, "sync-comm-a and sync-comm-b must be in the same community");

// Concepts from the isolated pair must be in a DIFFERENT community.
let id_d = get_community_id(&conn, "sync-comm-d").expect("d");
assert_ne!(id_a, id_d, "sync-comm-a and sync-comm-d must be in different communities");
```

Without these assertions, this test cannot catch the most likely regression: a Louvain
modularity collapse where all concepts land in a single partition.

**Verdict**: **REWRITE** — The integration path is irreplaceable. The assertions must be
strengthened to verify community _structure_, not just community _presence_.

---

### Test 2 — `build_context_includes_community_id_on_related_concepts`

**FR coverage**: FR-13.6 — _"BuildContext includes CommunityId on related concepts"_
**Setup path**: Direct SQL `insert_concept` + `insert_edge` → `detect_and_persist` → `BuildContext`
via `ToolRegistry::try_invoke`

#### What it proves

This test verifies the **positive case** of FR-13.6: when community data exists, `BuildContext`
must populate `CommunityId` on the root concept and on every `DirectRelation`. The degradation
tests in `community_build_context_degradation.rs` cover the inverse (null when no community data;
null when concept is an orphan). The positive case is not covered elsewhere. That makes this test
load-bearing.

The triangle topology (alpha–beta–gamma with all three edges present) is a good choice: Louvain
should deterministically place all three in the same community.

#### Problems

**Problem 1 — Misplaced file.** This test does not invoke `Sync`. It calls `detect_and_persist`
directly (bypassing the tool layer) and then calls `BuildContext`. It has nothing to do with the
sync pipeline and does not belong in `community_sync.rs`. It belongs in
`community_build_context_basics.rs` or similar.

**Problem 2 — Weak assertion: `persisted > 0`** Same sign-of-life problem as Test 1.

**Problem 3 — Non-null ≠ correct.** The core assertion loop:
```rust
for rel in direct {
    assert!(!rel["CommunityId"].is_null(), ...);
}
```
This checks that a value is present. It does not check that the value is _meaningful_. In a
triangle topology, `ctx-alpha`, `ctx-beta`, and `ctx-gamma` must all be assigned the **same**
community ID. A bug that assigns each concept its own unique singleton community would populate
CommunityId with a non-null value for every relation and still pass.

**Missing structural assertion**:
```rust
let alpha_id = data["CommunityId"].as_i64().expect("alpha community id");
for rel in direct {
    let rel_id = rel["CommunityId"].as_i64()
        .unwrap_or_else(|| panic!("{} has no CommunityId", rel["Name"]));
    assert_eq!(
        rel_id, alpha_id,
        "FR-13.6: {} must share community with ctx-alpha (triangle topology)",
        rel["Name"]
    );
}
```

**Problem 4 — Depth 1 only.** `depth: 1` is hardcoded. The test does not verify that
concepts at depth 2 also have their `CommunityId` populated. This is a coverage gap for
`expand_related_concepts`.

**Problem 5 — Overlap with siblings test.** `community_build_context_siblings.rs::build_context_returns_community_siblings`
also calls `BuildContext` with community data set up and inspects `DirectRelations`. While the
focus differs (CommunitySiblings + NormalizedOverlap vs. CommunityId presence), the test
infrastructure is duplicated. If this test is moved into the build_context suite, it can share
setup helpers.

**Verdict**: **MERGE** — Move this test into `community_build_context_siblings.rs` or a new
`community_build_context_basics.rs`. Add the community ID consistency assertion. Mark it as the
positive-case FR-13.6 coverage.

---

## Structural Issues (File Level)

| Issue | Severity | Assessment |
|-------|----------|------------|
| File named `community_sync.rs` but half its content is a BuildContext test | Medium | Violates single-responsibility for test files; future contributors adding sync tests or build_context tests may miss this file or add to the wrong one |
| Both tests acquire `TEST_MUTEX` (global process singleton) | Low | Known limitation, documented. Forced serial execution is a design constraint of the current Config architecture, not a test quality failure |
| `assert!(rows > 0, ...)` pattern in both tests | High | Insufficient — covers existence, not correctness |
| Test 1 uses ToolRegistry path; Test 2 uses `detect_and_persist` directly | Medium | The inconsistency is architecturally interesting: they cover different depths of the stack. This is defensible as long as it is intentional |
| Neither test asserts community grouping (same-community or different-community) | High | The fundamental output of Louvain is a _partition_. Tests that don't assert partition structure cannot catch the most common modularity bugs |

---

## Recommendations

1. **REWRITE `sync_produces_community_assignments`** (RTM: FR-13.4)
   - Retain the ToolRegistry pipeline setup — it is uniquely valuable.
   - Add `get_community_id` assertions: a+b in same community, d+e in same community,
     a and d in different communities.
   - Consider also asserting `distinct_community_count(&conn) >= 2`.

2. **MERGE `build_context_includes_community_id_on_related_concepts`** (RTM: FR-13.6)
   - Move to `community_build_context_basics.rs` (create if needed) or append to
     `community_build_context_siblings.rs`.
   - Add community ID consistency assertion: all triangle members share the same ID.
   - Add `depth: 2` variant if expanded relations also carry CommunityId (verify against source).

3. After MERGE, `community_sync.rs` will contain only 1 test. If a second sync-specific
   integration test is warranted (e.g., re-running Sync replaces stale community data —
   complementing `run_and_persist_second_run_replaces_existing` in the Louvain persistence suite
   but via the tool path), this file is the right home for it.

4. **Standardize assertion depth** across the community test suite: every test that validates
   community detection output should assert _at minimum_: (a) row count > 0, (b) expected
   community count, (c) at least one expected same-community pair, (d) at least one expected
   different-community pair where topology makes this deterministic.

---

## Load-Bearing Score: 2/2

Both tests cover integration paths not exercised elsewhere. Neither is removable without
creating coverage gaps. Both require rewrites to make their assertions commensurate with what
"load-bearing" actually means: the ability to _catch regressions_, not merely _confirm execution_.

| Test | Load-bearing | Assertion quality | Action required |
|------|:---:|:---:|:---:|
| `sync_produces_community_assignments` | ✅ | ⚠️ Weak | Rewrite assertions |
| `build_context_includes_community_id_on_related_concepts` | ✅ | ⚠️ Weak | Move + strengthen |

---
## SWE Remediation

- REWRITE `sync_produces_community_assignments` (FR-13.4): replaced weak sign-of-life assertions (`rows > 0`, `names.contains`) with structural partition assertions — `distinct_community_count >= 2`, same-community checks (`id_a == id_b`, `id_d == id_e`), different-community check (`id_a != id_d`). Removed unused `detect_and_persist` import. Retained the ToolRegistry pipeline path (uniquely valuable).
- MERGE `build_context_includes_community_id_on_related_concepts` (FR-13.6): moved from `community_sync.rs` to `community_build_context_siblings.rs` — the test exercises `detect_and_persist` + `BuildContext`, not the Sync tool. Strengthened from `!is_null()` checks to community ID consistency assertion: all triangle members must share the same community ID via `assert_eq!(rel_id, alpha_community_id)`.
- `community_sync.rs`: 1 test (was 2). `community_build_context_siblings.rs`: 3 tests (was 2).
- `cargo test --test integration_tests -p maenifold-core -- community_sync`: 1 passed, 0 failed
- `cargo test --test integration_tests -p maenifold-core -- community_build_context_siblings`: 3 passed, 0 failed
- Commit: e19bb95
