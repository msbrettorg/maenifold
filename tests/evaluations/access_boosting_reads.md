# Test Evaluation: access_boosting_reads

**File:** `crates/maenifold-core/tests/access_boosting_reads.rs`
**Evaluator role:** 🔘 Gray — Skeptical Inquiry
**Traceability claimed:** `T-GRAPH-DECAY-002.1-3` / `RTM NFR-7.6.1` / `T-12.0`
**Tests evaluated:** 3
**Date:** 2025-07-17

---

## Summary

This file tests a single, well-bounded contract: calling `ReadMemory` refreshes the
`last_accessed` timestamp in the `file_content` database table. That contract is the
foundation of access-boosting inside the [[decay-weighting]] system — files read recently
are down-weighted less aggressively by the decay scorer.

All three tests are load-bearing and distinct from one another. No test is purely
redundant. However, the suite has meaningful gaps (no negative-path coverage), one
structural performance concern (a hard 1.1-second sleep), and a traceability problem
that warrants immediate attention: the IDs cited in the source comments do not resolve
to any entry in `RTM.md` or `TODO.md`.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `read_memory_updates_last_accessed_on_every_read` | ✅ | ✅ | ✅ (minor redundancy) | ✅ | Low value | **KEEP** |
| 2 | `read_memory_updates_last_accessed_on_subsequent_reads` | ✅ | ✅ | ✅ (sleep is justified) | ✅ | Medium — impl-dependent | **KEEP + ANNOTATE** |
| 3 | `read_memory_by_title_updates_last_accessed` | ✅ | ✅ | ✅ | ✅ | Low value | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `read_memory_updates_last_accessed_on_every_read`

**What it does:** Seeds `last_accessed` to 2 days ago, calls `ReadMemory` by URI,
asserts the timestamp advances.

**Load-bearing?** Yes. This is the baseline invariant: any successful `ReadMemory`
call must refresh `last_accessed`. If the [[access-boosting]] write is removed or
regressed, this test fails immediately.

**Unique?** Yes. No other test in this file covers the first-read baseline from a
stale-seed state.

**Correct assertions?**

- `!read_result.starts_with("ERROR:")` — reasonable guard; confirms the read succeeded
  before making timestamp claims.
- `after_read_last_accessed.is_some()` — technically redundant: if it were `None`, the
  `>` comparison on `Option<DateTime>` would evaluate `None > Some(...)` = `false` and
  the next assert would also fire. The redundancy is *intentional* — it produces a
  sharper failure message. Acceptable.
- `after_read_last_accessed > initial_last_accessed` — the core assertion. Relies on
  `Option<T>: PartialOrd` where `Some(a) > Some(b)` iff `a > b`. Correct.

**Skeptical challenge:** What if `update_last_accessed` silently fails? The production
code calls it as `.ok()`, discarding the `Result`. The test *would* catch this because
the timestamp would not advance — but only because the test re-reads from the DB
directly. This is fragile if the helper `get_last_accessed` were ever changed to not
query the same row. Acceptable risk, but worth noting.

**Parameterizable?** The 2-day seed value is hardcoded. No material gain from
parameterizing — the test's purpose is the direction of change (advances), not the
magnitude.

**Verdict: KEEP**

---

### Test 2 — `read_memory_updates_last_accessed_on_subsequent_reads`

**What it does:** Seeds to 2 days ago, performs two sequential reads separated by a
`READ_SLEEP_MS = 1_100` ms sleep, asserts the second timestamp is strictly greater than
the first.

**Load-bearing?** Yes. This is *distinct* from Test 1 in an important way: Test 1
proves the timestamp advances from a stale seed. Test 2 proves that *each successive
read* refreshes the timestamp, not just the first one. These are independent behavioral
claims.

**Unique?** Yes. The sequential-read + temporal-ordering dimension is not covered by
any other test in this file.

**Correct assertions?**
- Both read results are checked for non-error — correct.
- `second_read_timestamp > first_read_timestamp` — correct.

**Critical observation — the sleep is load-bearing per the implementation:**
`update_last_accessed` formats as:
```rust
chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
```
This is **second-precision only**. Two reads within the same wall-clock second would
produce identical timestamps. The `>` assertion would then fail (`equal` is not
`greater`). The 1.1-second sleep exists to guarantee the second read falls in a
different second. This is *correct* but means:

1. The test takes ≥ 1.1 seconds unconditionally. In isolation this is acceptable; in a
   large suite with many such tests it compounds.
2. If the implementation is ever changed to millisecond or nanosecond precision, the
   sleep could safely drop to ~10 ms — but no one will remember to update this.
3. The constant `READ_SLEEP_MS` is defined in the shared `support.rs`, which is good
   hygiene. A comment in support.rs explaining *why* 1.1s is necessary (second-precision
   DB format) would prevent future confusion.

**Parameterizable?** The test cannot be meaningfully shortened without changing the
implementation's timestamp precision. An annotation (`#[cfg_attr(not(feature =
"slow-tests"), ignore)]` or a `// NOTE: slow by design` comment) would communicate
intent to future maintainers.

**Verdict: KEEP** — with a recommendation to annotate it as a slow test and add a
comment explaining the precision dependency.

---

### Test 3 — `read_memory_by_title_updates_last_accessed`

**What it does:** Creates a file, reads it *by human title* (not by URI), asserts
`last_accessed` advances.

**Load-bearing?** Yes. The `read_memory` code takes a distinct code path for
non-URI identifiers:
```rust
let uri = if identifier.starts_with("memory://") {
    identifier.to_string()
} else {
    match find_file_as_uri(identifier, &context.config) { ... }
};
```
`update_last_accessed` is called after this branch merges. This test verifies the
title-resolution path does not short-circuit before the access update. Without this
test, a future refactor could hoist `update_last_accessed` inside the `starts_with
("memory://")` branch and silently break title-based access tracking.

**Unique?** Yes. The other two tests read by URI. The identifier type matters here.

**Correct assertions?**
- Correct. No timing ambiguity (seeded 2 days ago, one read from ~now, unambiguous).
- No explicit check that the right file's timestamp was updated — but it's correct by
  construction: `get_last_accessed` queries by the URI returned from write, and
  `read_memory_output` uses the title that maps to exactly that URI.

**Parameterizable?** No material benefit.

**Verdict: KEEP**

---

## Coverage Gaps

The following scenarios are not tested and represent behavioral blind spots:

### Gap 1 — ReadMemory on non-existent identifier (HIGH priority)
When `read_memory` returns `"ERROR: Memory file not found: ..."`, no call to
`update_last_accessed` should occur (there is nothing to update). This is currently
unverified. If a future refactor moved the `update_last_accessed` call earlier in the
function — before the existence check — it would write a timestamp for a ghost URI.
A simple negative test would close this gap.

```
fn read_memory_on_missing_uri_does_not_create_last_accessed_row()
```

### Gap 2 — ReadMemory with `learn=true` short-circuits before access update (MEDIUM)
When `learn_requested(payload)` is true, `read_memory` returns early. The
`update_last_accessed` is never reached. This is correct behavior, but it is not
tested. A test asserting `last_accessed` remains unchanged after a learn-mode call
would prevent regression if the short-circuit is ever reordered.

### Gap 3 — No assertion that `update_last_accessed` DB failure is surfaced (LOW)
The production code uses `.ok()` to silently swallow DB errors from
`update_last_accessed`. This means a broken DB connection would not fail the read —
it would silently skip the access record. The tests indirectly cover the happy path,
but there is no test that stubs a broken DB and verifies the read still succeeds
(graceful degradation). This is low priority but worth noting.

---

## Structural Observations

### Traceability: Phantom IDs (BLOCKER-class finding)

All three tests cite `T-GRAPH-DECAY-002.1-3` and `RTM NFR-7.6.1`. Neither of these
identifiers exists in `RTM.md` or `TODO.md`:

- `NFR-7` in `RTM.md` is titled **"Agent Isolation"** and covers agent crash handling,
  permission policy, and timeouts. It has no sub-items `7.6` or `7.6.1`.
- `T-GRAPH-DECAY-002.1-3` is not present in `TODO.md` under any section.
- `T-12.0` (module-level comment) is also absent from `TODO.md`.

These are phantom references. The tests are real and load-bearing, but they have no
traceable parent requirement. This violates the project's traceability contract and
means the RTM cannot be used to determine whether this behavior is validated.

**Recommendation:** Create a real TODO entry (e.g., `T-GRAPH-DECAY-002`) and a real
RTM row (e.g., under `NFR-7` or a new `NFR-8: Decay & Access Boosting`) and update
the source comments to match.

### `TEST_MUTEX` — serial execution by design

All three tests acquire `TEST_MUTEX` (a global `parking_lot::Mutex<()>`). This
ensures tests do not run in parallel, which is necessary because `Config::override_root`
and `Config::set_database_path` modify global state. Tests are correctly serialized.
The cost: the suite cannot benefit from Rust's parallel test runner for this file.
This is an accepted architectural trade-off given the global `Config` singleton pattern.

### `READ_SLEEP_MS` constant location is correct

Defining `READ_SLEEP_MS = 1_100` in `support.rs` (shared module) rather than inline
is good hygiene. It signals that the value is intentional and shared. **However, the
constant lacks a comment explaining why 1.1 seconds is specifically required.** Future
readers may reduce it not knowing about the second-precision timestamp format.

---

## Recommendations

| Priority | Recommendation |
|----------|---------------|
| 🔴 **BLOCKER** | Resolve phantom traceability IDs. Add `T-GRAPH-DECAY-002` to TODO.md and a real RTM requirement row. Update source comments to match real IDs. |
| 🟠 **HIGH** | Add negative test: `read_memory_on_missing_uri_does_not_update_last_accessed`. Verifies the not-found early return does not corrupt the `file_content` table. |
| 🟡 **MEDIUM** | Annotate Test 2 (`subsequent_reads`) as a slow test. Add a `// NOTE: sleep required because update_last_accessed uses second-precision timestamps` comment in `support.rs` next to `READ_SLEEP_MS`. |
| 🟡 **MEDIUM** | Add negative test: `read_memory_learn_mode_does_not_update_last_accessed`. Verifies the `learn_requested` short-circuit path does not touch `last_accessed`. |
| 🟢 **LOW** | Consider raising `update_last_accessed` timestamp precision from `%Y-%m-%dT%H:%M:%SZ` (seconds) to `%Y-%m-%dT%H:%M:%S%.3fZ` (milliseconds). This would let Test 2's sleep drop from 1.1s to ~10ms and remove the precision-dependency coupling between test and implementation. |
| 🟢 **LOW** | Add a single `test_quality` tag or attribute group comment (`// Access Boosting: ReadMemory path`) to logically group these three tests for `cargo test -- access_boosting_reads` filtering. |

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. Each covers a distinct facet of the
[[access-boosting]] contract for `ReadMemory`:

- **Test 1** — baseline: first read from stale state triggers an update
- **Test 2** — idempotency of boost: each subsequent read also triggers an update
- **Test 3** — path coverage: title-based resolution also triggers an update

None can be removed without leaving a meaningful behavioral gap undetected. The suite
is lean and purposeful. Its primary weaknesses are the missing negative tests and the
phantom traceability IDs — not the tests themselves.
