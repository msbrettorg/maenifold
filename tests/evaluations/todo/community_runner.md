# Test Evaluation: community_runner.rs

**Traceability**: T-12.7 (NFR-1 oracle parity), T-W1-8 (NFR-2 coverage)  
**FR references**: NFR-1, NFR-2  
**Evaluator mode**: 🔘 Gray — Skeptical Inquiry  
**Date**: 2025-07-15

---

## Summary

`community_runner.rs` maps the C# `CommunityDetectionRunnerTests.cs` acceptance criteria
(AC-4 through AC-7) onto the Rust `detect_and_persist` function. The file contains **4
tests** across **113 lines** (excluding the support module import).

**Critical finding**: The mapping from a C# `Runner` class (with explicit locks and a
`RunBestEffort` pattern) to a synchronous Rust function (`detect_and_persist`) is
conceptually valid, but it has produced tests that are semantically mislabelled and
structurally redundant. Two of the four tests assert conditions that are:

1. Trivially guaranteed by the synchronous Rust execution model (no async, no internal
   buffering, no separate writer thread), or  
2. Already fully covered by `community_louvain_persistence.rs` and
   `community_louvain_basics.rs` with more precise assertions.

Across the full community test suite, `assert_eq!(count, 2)` on the
`triangle_plus_isolated_pair` topology appears **at least 7 times** in different test
files. `community_row_count == 5` for the same topology appears at least **4 times**.
Both assertions are present in this file. That is not defense-in-depth; it is noise that
must be maintained under refactoring.

The one genuinely unique and load-bearing test (`runner_concurrent_calls_do_not_panic_or_corrupt`)
carries a weak assertion that the 5-second `busy_timeout` renders unnecessary. The one
genuinely unique *boundary condition* (empty DB → count 0) is buried inside a
two-scenario test function that conflates it with a duplicated data-path check.

**Net load-bearing score: 2 / 4.**

---

## Verdicts

| # | Test name | Load-bearing? | Unique? | Assertions correct? | Name clear? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `runner_community_data_immediately_available_after_run` | Weak | ✗ | Partial | ✗ | Yes | **MERGE** |
| 2 | `runner_community_data_persists_across_connections` | ✗ | ✗ | Partial | ✗ | Yes | **REMOVE** |
| 3 | `runner_concurrent_calls_do_not_panic_or_corrupt` | ✓ | ✓ | ✗ | ✓ | Yes | **REWRITE** |
| 4 | `runner_detect_and_persist_returns_valid_community_count` | Partial | Partial | Partial | ✗ | Yes | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `runner_community_data_immediately_available_after_run`

**Mapped AC**: AC-4 — "Community data is immediately available in the DB after
`detect_and_persist` returns."

**What it does**: Inserts `triangle_plus_isolated_pair`, calls `detect_and_persist`,
opens a new `Connection`, asserts `community_row_count == 5` and return value `== 2`.

**Problems**:

**Semantic mislabelling.** "Immediately available" implies eventual consistency — a
concern in async systems, systems with write-behind caches, or systems where a background
writer thread may not have flushed yet. `detect_and_persist` is a **synchronous Rust
function** that calls `transaction.commit()` before returning. There is no async path,
no background flush, no internal buffer. The word "immediately" adds zero specification
and misleads future readers into thinking there was an async concern here worth testing.
The test does not — and cannot — distinguish "synchronous return = data visible" from any
other visibility model because there is only one model here.

**Duplication.** The assertions `count == 2` and `community_row_count == 5` on the
`triangle_plus_isolated_pair` topology are also made, on a post-`detect_and_persist` new
connection, in:
- `community_louvain_persistence.rs::run_and_persist_persists_to_database` (count==2,
  rows==5, plus modularity and resolution checks — *strictly stronger*)
- `community_louvain_basics.rs::run_louvain_triangle_plus_isolated_pair_two_communities`
  (count==2 plus community ID membership assertions — *strictly stronger*)
- `community_coverage_basics.rs` line 66 (rows==5 with same topology)

This test is a subset of `run_and_persist_persists_to_database` in every assertion, while
providing a weaker claim about the subject under test.

**Missing assertions.** The support module provides `get_modularity()`, `get_resolution()`,
`get_community_id()`, and `distinct_community_count()` — none are called here. If this
test is meant to verify the post-run DB state, it under-asserts.

**Verdict: MERGE** into `runner_detect_and_persist_returns_valid_community_count` (after
that test is rewritten to separate the empty-DB and data scenarios). The "immediately
available" framing should be dropped; the assertion it carries belongs with the empty-DB
boundary test as confirmation that a populated run actually writes rows.

---

### Test 2 — `runner_community_data_persists_across_connections`

**Mapped AC**: AC-5 — "Community data persists across new DB connections (durable write)."

**What it does**: Inserts `triangle_plus_isolated_pair`, calls `detect_and_persist`,
opens `conn2` (a brand-new connection), asserts `community_row_count == 5`.

**Problems**:

**Strict subset of Test 1.** Compare the two test bodies side by side:
- Test 1: drops initial conn → `detect_and_persist` → opens new conn → `rows == 5` AND `count == 2`.
- Test 2: drops initial conn → `detect_and_persist` → opens new conn → `rows == 5`.

Test 2 is Test 1 minus the return-value assertion. It adds no new signal. The "brand-new
connection" framing in Test 2 is identical in mechanism to the new `conn` opened in
Test 1. Both already falsify any hypothesis that the data only exists in a cached
connection scope.

**Also covered by `community_louvain_persistence.rs`.** `run_and_persist_persists_to_database`
opens a fresh connection after `detect_and_persist` and asserts rows==5, count==2,
modularity is finite, and resolution==1.0. This is a strict superset of Test 2.

**Verdict: REMOVE.** This test is dead weight. Removing it reduces the maintenance
surface and the noise floor of the community test suite without losing any signal.

---

### Test 3 — `runner_concurrent_calls_do_not_panic_or_corrupt`

**Mapped AC**: AC-6 — "Concurrent calls to `detect_and_persist` must not panic or corrupt data."

**What it does**: Spawns two threads, each calling `detect_and_persist` on the same
`db_path`. Asserts `r1.is_ok() || r2.is_ok()` (at least one succeeds) and
`community_row_count == 5` afterwards.

**This is the only concurrency test for `detect_and_persist` in the entire test suite.
It is load-bearing.**

**Problems**:

**The OR assertion is too weak.** The assertion `r1.is_ok() || r2.is_ok()` tolerates
one thread **always** failing. Given the setup:

- `open_conn` sets `PRAGMA busy_timeout=5000` (5 seconds).
- `detect_and_persist` on a 5-node, 4-edge graph takes microseconds.
- SQLite WAL mode (or default journal mode) with a 5-second timeout will virtually never
  produce a `SQLITE_BUSY` error in this scenario.

The busy_timeout exists precisely so that *both* calls succeed under contention. The
test should assert `r1.is_ok() && r2.is_ok()`. The current OR assertion would pass even
if the implementation serialized all calls and returned an error for every second caller
— as long as it always returned `Ok` for the first. It masks regressions where
`detect_and_persist` starts returning `Err` under any concurrent access.

**`TEST_MUTEX` and concurrency are architecturally in tension.** The test holds
`TEST_MUTEX` for its entire duration to serialize against other community tests (correct
— the Config singleton must be protected). But this means the spawned threads cannot
compete with *other* tests, only with each other. The test measures intra-test
concurrency in isolation, which is fine — but worth noting that the global serialization
exists for a different reason (the Config singleton), not because `detect_and_persist`
itself is unsafe to run concurrently.

**Nondeterminism.** Thread scheduling is non-deterministic. The test could occasionally
pass with only one thread completing, masking a scenario where concurrent access reliably
produces an error. The OR assertion makes this worse.

**Verdict: REWRITE.** Strengthen the assertion to `&&`. Add an explicit comment
acknowledging that `busy_timeout=5000ms` guarantees both complete for this dataset size,
so the OR guard is unnecessary and misleading. Optionally add a sleep or a heavier
topology if CI flakiness is a concern.

---

### Test 4 — `runner_detect_and_persist_returns_valid_community_count`

**Mapped AC**: AC-7 — "`detect_and_persist` returns a valid community count (>= 0) on
any initialized DB."

**What it does**: Calls `detect_and_persist` on an empty DB (asserts `count == 0`), then
inserts `triangle_plus_isolated_pair` and calls again (asserts `count == 2`).

**The empty-DB baseline (count == 0) is genuinely unique.** No other test in the
community suite, `community_louvain_persistence.rs`, or `community_louvain_basics.rs`
calls `detect_and_persist` on a freshly initialized empty graph and checks the return
value. This is a real boundary condition: an empty graph must not panic, must return 0,
and must not corrupt the schema. This part is load-bearing and should be preserved.

**The populated case (count == 2) is the most-duplicated assertion in the suite.**
Across all community test files, `assert_eq!(count, 2)` or equivalent on
`triangle_plus_isolated_pair` appears at least 7 times. This test repeats it as a
"second phase" of the same function, which:

1. Does not add new evidence (the empty-DB run already verified the function works; the
   second run just re-verifies a known topology).
2. Conflates two distinct concerns into one test function body — empty-graph behavior and
   normal-graph behavior are separate scenarios that deserve separate names and separate
   failure messages.

**Name problem.** The AC says "valid community count (>= 0)" but the test asserts exact
equality (`== 0`, `== 2`). The name implies a range check; the implementation is an exact
check. There's nothing wrong with exact checks — but the name should reflect what the
test actually verifies.

**Verdict: REWRITE.** Split into two focused tests:
- `runner_returns_zero_on_empty_db` — keep the empty-DB phase; this is the unique,
  load-bearing assertion.
- Remove the data-path phase entirely (it duplicates `louvain_basics` and
  `louvain_persistence`), or merge it into the post-rewrite version of Test 1.

---

## Cross-Cutting Observations

### The `seed` parameter is never exercised

`detect_and_persist` has signature `(db_path: &Path, gamma: f64, seed: Option<u64>)`.
All 4 tests pass `seed = None`. The seed parameter exists specifically to enable
deterministic Louvain runs for reproducible testing. No test in this file (or in
`community_louvain_basics.rs` — also all `None`) exercises a fixed seed and then asserts
output stability. The determinism test in `community_louvain_basics.rs` calls the
function twice with `None` and relies on algorithmic convergence, not seed-pinning.
This is a gap, especially as the code comment cites T-15.10 for the seed feature.

### Support helpers go unused

`community/support.rs` exports `get_community_id()`, `get_modularity()`,
`get_resolution()`, and `distinct_community_count()`. None are called by
`community_runner.rs`. The file's post-run assertions are limited to row counts and return
values — no verification of which concepts landed in which communities, what modularity
was stored, or that resolution was correctly round-tripped. These checks exist in
`community_louvain_persistence.rs` and `community_louvain_basics.rs` but not here.

### "Runner" is a C# concept without a Rust equivalent

The file header acknowledges: *"The Rust implementation exposes `detect_and_persist`
rather than a Runner class."* This is a correct observation, but it raises a question
the file does not fully answer: **what justifies a separate `community_runner.rs` test
file?** The C# Runner had distinct concerns (lock-based `RunBestEffort`, explicit
`Runner.IsRunning` state, async coordination). The Rust function is synchronous and
stateless. The "runner" behaviors that were meaningful in C# are either trivially true in
Rust (immediate visibility) or belong in the concurrency test (Test 3) and the boundary
test (Test 4's empty-DB case). After removing Test 2 and rewriting Tests 1, 3, and 4,
this file could reasonably be consolidated into `community_louvain_persistence.rs` with
two additional test functions.

---

## Recommendations

**Priority 1 — Remove Test 2.**  
`runner_community_data_persists_across_connections` is a strict subset of Test 1 and a
strict subset of `run_and_persist_persists_to_database`. It can be deleted with zero
coverage loss.

**Priority 2 — Rewrite Test 3 assertion.**  
Change `r1.is_ok() || r2.is_ok()` to `r1.is_ok() && r2.is_ok()`. Add a comment citing
`busy_timeout=5000ms` as the reason both calls are expected to succeed. This turns a
masked regression-risk into a real guard.

**Priority 3 — Rewrite Test 4 (split it).**  
Extract `runner_returns_zero_on_empty_db` as a standalone test with only the empty-DB
phase. Drop the populated-graph phase; it contributes nothing not already covered
elsewhere. Rename to reflect the actual assertion (exact check, not range check).

**Priority 4 — Merge Test 1 or drop it.**  
After Test 4 is rewritten to cover the empty-DB baseline, Test 1's unique content (count
returned == 2, rows immediately queryable) is fully covered by `run_and_persist_persists_to_database`
with stronger assertions. Either merge into Test 4's data phase or remove. If retained,
drop the "immediately available" framing and add at least `get_modularity()` and
`get_community_id()` assertions to justify its existence as a distinct test.

**Priority 5 — Add a seed-pinning test.**  
A test that calls `detect_and_persist(db_path, 1.0, Some(42))` twice and asserts
identical community assignments covers the deterministic-Louvain contract that T-15.10
introduced. This is currently unverified anywhere in the suite.

---

## Load-Bearing Score: 2 / 4

| Test | Load-bearing? | Justification |
|------|:---:|---------------|
| `runner_community_data_immediately_available_after_run` | ✗ | Fully duplicated by `community_louvain_persistence.rs::run_and_persist_persists_to_database` with stronger assertions |
| `runner_community_data_persists_across_connections` | ✗ | Strict subset of Test 1; zero unique signal |
| `runner_concurrent_calls_do_not_panic_or_corrupt` | ✓ | Only concurrency test for `detect_and_persist` in the suite |
| `runner_detect_and_persist_returns_valid_community_count` | Partial (½) | Empty-DB phase is unique and load-bearing; populated phase is the 7th duplicate of `count == 2` |

**Effective unique signal: 1.5 / 4 tests.**  
If Test 3 is rewritten with a `&&` assertion, that rises to 2 / 4 with genuine value.
The file as written costs 4 test slots and delivers the equivalent of ~1.5.
