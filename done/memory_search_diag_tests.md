---
id: T-80.105
title: "Test Evaluation: `memory_search_diag_tests.rs`"
status: done
pipeline: tdd
---

# Test Evaluation: `memory_search_diag_tests.rs`

> **Reviewer perspective:** 🔘 Gray — Skeptical Inquiry
> **Source file:** `crates/maenifold-core/tests/memory_search_diag_tests.rs`
> **Traceability:** T-QUAL-002.6a | RTM FR-25.2 (historical diagnostics req) | NFR-2 (Test Coverage)
> **Production code under test:**
> - `src/decay/metadata.rs` → `get_decay_weight_for_file`, `get_decay_weight_for_file_with_connection`
> - `src/search/display.rs` → `get_file_display_info`
> - `src/decay/diagnostics.rs` → `emit_degraded_decay_diagnostic`, `redact_diagnostic_path`
> **Platform gate:** `#![cfg(unix)]` (all tests; required by `capture_stderr` raw-FD machinery)
> **Peer files:** `degraded_diag_assets.rs` (4 tests), `degraded_diag_graph.rs` (3 tests)
> **Date:** 2025-07-25

---

## Summary

This file holds 5 integration tests that validate the **degraded-diagnostic contract** for
the `MemorySearchTools.Fusion` subsystem: the guarantee that, when a decay or display-info
lookup fails, the tool (a) returns a safe neutral/fallback value rather than crashing, and
(b) emits a structured `[DEGRADED][MemorySearchTools.Fusion][Method]` line to stderr so
operators can detect the condition.

Before rendering verdicts, the following questions were answered against the production code:

1. **What does each test uniquely exercise?** Traced every test's expected code path
   through `decay/metadata.rs` and `search/display.rs` to confirm no other test file
   covers the same failure branch. Only `sync_perf_indexing.rs` also calls
   `get_decay_weight_for_file` but never tests stderr output or degraded signals.
2. **Are the assertions specific enough to catch real regressions?** Each assertion was
   mapped to the code site it guards. Several are too loose or test implementation internals.
3. **Is the shared infrastructure sound?** The `TEST_MUTEX` / `capture_stderr` machinery was
   inspected for correctness and known failure modes (detailed below).
4. **Are there coverage gaps or naming collisions?** Found one AC-label collision (two
   distinct tests both labeled `ac1`) and one coverage gap (no test for
   `get_decay_weight_for_file` path redaction despite a parallel code path existing).

**Bottom line:** All five tests cover load-bearing scenarios not reached by any other test
file. Two are ready to keep as-is. Two need rewrites to excise fragile implementation-detail
assertions or consolidate redundant concerns. One needs a targeted fix for a naming
collision. The `capture_stderr` infrastructure (also used in peer files) has a
panic-safety gap that is a systemic risk.

---

## Verdicts

| # | Test Name | Verdict | Load‑Bearing | Unique | Correct Assertions | Good Name | Parameterizable |
|---|-----------|:-------:|:---:|:---:|:---:|:---:|:---:|
| 1 | `fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal` | **KEEP** (rename) | ✅ | ✅ | ⚠️ fragile text coupling | ⚠️ ac1 collision | ⚠️ MERGE candidate |
| 2 | `fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal` | **MERGE** into T1 | ✅ | ✅ | ⚠️ `starts_with` too loose | ⚠️ ac1 collision | ✅ |
| 3 | `fr25_2_ac2_success_path_behavior_remains_unchanged` | **REWRITE** (split) | ⚠️ partial | ⚠️ partial | ⚠️ weight bounds trivial | ✅ | ✅ |
| 4 | `fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic` | **REWRITE** (assertion) | ✅ | ✅ | ❌ internals leaked | ✅ | ❌ |
| 5 | `fr25_2_ac4_path_diagnostics_redacts_absolute_paths` | **KEEP** | ✅ | ✅ | ✅ | ✅ | ⚠️ coverage gap |

---

## Per-Test Analysis

---

### Test 1 — `fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal`

#### What it tests

`get_decay_weight_for_file` called with a `memory://` URI that has no backing file on disk
and no row in `file_content`. Expected code path:

1. DB connection succeeds (DB is initialized by `isolated_env`).
2. `SELECT … WHERE file_path = ?1` returns `Ok(None)` — URI is not indexed.
3. Falls to `get_decay_weight_from_disk` → `get_decay_weight_from_disk_with_status`.
4. `read_created_date_for_identifier` → `uri_to_path` succeeds syntactically but the file
   does not exist on disk → `fs::read_to_string` returns `Err`.
5. `emit_degraded_decay_diagnostic("GetDecayWeightFromDisk", "Falling back to neutral decay
   weight", …)` fires; returns `1.0`.

#### Assertion quality

| Assertion | Guards | Verdict |
|-----------|--------|---------|
| `weight == 1.0` | Neutral fallback value not silently changed | ✅ precise |
| `stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightFromDisk]")` | Struct tag not removed or renamed | ⚠️ brittle — any rename of `GetDecayWeightFromDisk` breaks the test without touching behavior |
| `stderr.contains("Falling back to neutral decay weight")` | Message not silently removed | ⚠️ same fragility — message text is an implementation detail |

Neither message assertion is anchored to an observable contract. They would fail on harmless
refactors of the internal diagnostic string. However, removing them entirely would leave no
guard on the diagnostic contract at all. The right fix is not to delete them but to capture
the exact expected string in a constant shared with production code.

#### Naming / traceability

- **Naming collision**: Both this test and Test 2 carry the `ac1` prefix. The label implies
  a single acceptance criterion exercises both behaviors, but they cover distinct functions.
  One must be renamed (e.g., `ac1a` / `ac1b`) or the two tests merged.
- **FR number ambiguity**: `fr25_2` embedded in the function name refers to an *archived*
  historical requirement. In the current PRD/RTM, FR-25 is Azure Deployment Manifests
  (RTM-137–140). Future readers will be confused. The name should be rewritten or a comment
  should clarify it references the completed diagnostics requirement, not the active FR-25.

**Verdict: KEEP — rename to `ac1a` or MERGE with Test 2. No logic change needed.**

---

### Test 2 — `fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal`

#### What it tests

`get_file_display_info` called with the same non-existent `memory://` URI. Code path:

1. `read_markdown` fails → `Err`.
2. `emit_stderr_line("[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo] Falling back to
   filename/snippet for 'non-existent-file'")`.
3. Returns `("non-existent-file", "[DEGRADED] Snippet unavailable")`.

#### Assertion quality

| Assertion | Guards | Verdict |
|-----------|--------|---------|
| `title == "non-existent-file"` | Fallback title strips `memory://` prefix and returns last path segment | ✅ |
| `snippet.starts_with("[DEGRADED] Snippet unavailable")` | Degraded snippet sentinel not removed | ⚠️ `starts_with` is loose — passes even if suffix garbage is appended |
| `stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetFileDisplayInfo]")` | Struct tag present | ⚠️ same text-coupling fragility as Test 1 |
| `stderr.contains("Falling back to filename/snippet")` | Message phrase present | ⚠️ same fragility |

The snippet assertion should be `assert_eq!(snippet, "[DEGRADED] Snippet unavailable")` —
the production code returns that exact string, so an exact match is both possible and
stronger.

#### Merge case

Tests 1 and 2 have identical structure: setup empty DB + non-existent URI → capture stderr
→ assert weight/title + snippet + two stderr substrings. They diverge only in which
function under test and which expected strings are checked. A parameterized approach
(or a single combined test) would eliminate 30 lines of near-duplicate setup and express
the contract more clearly: *both* functions degrade gracefully when a file does not exist.

**Verdict: MERGE with Test 1 into `fr25_2_ac1_fallback_on_missing_file` (or rename to
`ac1b`). Tighten snippet assertion to exact equality.**

---

### Test 3 — `fr25_2_ac2_success_path_behavior_remains_unchanged`

#### What it tests

Calls **both** `get_file_display_info` and `get_decay_weight_for_file` on a valid note
within a single `capture_stderr` closure, asserting positive behavior and that no DEGRADED
signal fires.

#### Why this is structurally problematic

1. **Dual-function bundling**: A single test exercises two independent code paths. If one
   fails, you must read the test carefully to determine which function caused it. Violates
   the one-concept-per-test principle.

2. **Convoluted capture**: The nested tuple `(((title, snippet), weight), stderr)` is
   unusually difficult to read. It exists only because two calls had to be stuffed into one
   closure.

3. **Trivial weight bounds**: `assert!(weight > 0.0)` and `assert!(weight <= 1.0)` are
   satisfied by almost any non-pathological decay implementation. A freshly written file has
   `created = Utc::now()`, so the decay weight should be very close to `1.0` (full
   freshness). The assertion could and should narrow this: `assert!(weight > 0.90,
   "freshly created file should have near-maximal decay weight, got {weight}")`.

4. **Partial load-bearing**: The positive assertion for title/snippet content is likely
   duplicated in other tests (e.g., `search_tools_tests.rs`, `decay_benchmark_search.md`).
   The unique value is the negative stderr assertion `!stderr.contains("[DEGRADED]…")` —
   this ensures the success path does not accidentally emit degraded signals after a
   refactor.

#### Verdict: REWRITE — split into two tests

```rust
// REWRITE INTO:
fn fr25_2_ac2a_display_info_success_path_returns_title_and_snippet() { … }
fn fr25_2_ac2b_decay_weight_success_path_returns_bounded_weight_no_degraded_signal() { … }
```

In the decay variant: tighten `weight > 0.90` (or `(0.9..=1.0).contains(&weight)`), add
the negative stderr assertion specifically for decay. In the display variant: keep the
snippet content assertion and the negative stderr assertion for `GetFileDisplayInfo`.

---

### Test 4 — `fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic`

#### What it tests

Inserts a row into `file_content` with `created = "not-a-valid-iso-timestamp"` and no
`last_accessed`. Calls `get_decay_weight_for_file`. Expected code path:

1. Connection opens. `SELECT created, last_accessed, status` returns the malformed row.
2. `last_accessed = None` → skips first branch.
3. `parse_datetime("not-a-valid-iso-timestamp")` returns `None`.
4. `emit_degraded_decay_diagnostic("GetDecayWeightForFile(reuse-conn)", "Malformed
   file_content.created timestamp; using current UTC as degraded fallback", …)`.
5. Falls to `get_decay_weight_from_disk_with_status` → file doesn't exist on disk →
   `emit_degraded_decay_diagnostic("GetDecayWeightFromDisk", …)` → returns `1.0`.

This test is **load-bearing**: it is the only test that exercises the malformed-timestamp
branch deep inside `get_decay_weight_for_file_with_connection`. No other test file reaches
this code path.

#### Critical assertion flaw

```rust
assert!(
    stderr.contains("[DEGRADED][MemorySearchTools.Fusion][GetDecayWeightForFile(reuse-conn)]")
);
```

**This asserts an internal implementation detail.** The string `(reuse-conn)` is a
parenthetical note in the diagnostic label that describes *how* the connection is obtained
(reused from the caller rather than opened fresh). It encodes the refactoring history of
the function, not an observable behavioral contract. Any of these legitimate refactors
would break this test without changing behavior:

- Renaming the diagnostic method string to `GetDecayWeightForFileDb` for clarity.
- Removing the `(reuse-conn)` annotation during the FR-22 `DbBackend` migration.
- Merging `get_decay_weight_for_file_with_connection` into the trait method.

The test should assert the *message* (`"Malformed file_content.created timestamp"`) and the
*component tag* (`"[MemorySearchTools.Fusion]"`) but **not** the method sub-label
`(reuse-conn)`. If the function name truly matters for operator diagnostics, that string
should be a named constant shared between production code and the assertion.

**Verdict: REWRITE — replace the fragile `(reuse-conn)` string with a constant or drop the
sub-label from the assertion. Keep the malformed-timestamp message assertion.**

---

### Test 5 — `fr25_2_ac4_path_diagnostics_redacts_absolute_paths`

#### What it tests

Calls `get_file_display_info` with an absolute filesystem path (outside `memory_path`).
Verifies that:
- The diagnostic emits the **basename** (with extension) rather than the full absolute path.
- The full absolute path is **absent** from stderr.
- Title is derived from the basename (without extension).
- Snippet is the DEGRADED sentinel.

Uses a `uuid::Uuid` in the filename to eliminate any risk of a stale path appearing in
captured stderr from an unrelated log line.

#### Assertion quality

All four assertions are well-targeted:

| Assertion | Guards |
|-----------|--------|
| `title == file_stem` | `fallback_display_title` uses `file_stem()` for non-`memory://` paths |
| `snippet.starts_with("[DEGRADED] Snippet unavailable")` | Degraded sentinel present |
| `stderr.contains(&format!("for '{file_name}'"))` | Basename (with extension) emitted |
| `!stderr.contains(absolute_path)` | **Privacy/security**: no path leak |

The negative assertion on the absolute path is the most important one here — it guards a
security property (no sensitive filesystem path leakage into operator logs). This test is
irreplaceable for that reason alone.

#### Coverage gap

`get_decay_weight_for_file` also emits diagnostics via `redact_diagnostic_path` (in
`decay/diagnostics.rs`). That function has the **identical** path-redaction logic as
`redact_display_info_path` in `search/display.rs`. Neither this file nor any other test
file verifies that `get_decay_weight_for_file` redacts absolute paths in its diagnostic
output. Consider adding `ac4b` to cover that parallel path:

```rust
fn fr25_2_ac4b_decay_diagnostics_redacts_absolute_paths() { … }
```

**Verdict: KEEP. Add a companion test for the decay path.**

---

## Infrastructure Analysis

### `capture_stderr` — No Panic Guard

```rust
let result = operation();                         // ← panic here leaks the fd redirect
std::io::stderr().flush().expect("flush stderr"); // ← and this
```

If `operation()` panics (or if any downstream `expect` / `unwrap` fires), stderr is
**permanently redirected** for the remainder of the test binary run. Subsequent test output
disappears from the console and fills the pipe until the pipe buffer blocks, causing the
binary to hang silently. This is a systemic risk shared with `degraded_diag_assets.rs` and
`degraded_diag_graph.rs`.

**Fix:** Wrap the fd restore in a RAII guard:

```rust
struct StderrRestore { saved_fd: i32, stderr_fd: i32 }
impl Drop for StderrRestore {
    fn drop(&mut self) {
        unsafe { dup2(self.saved_fd, self.stderr_fd); close(self.saved_fd); }
    }
}
```

### `TEST_MUTEX` Scope

The global mutex correctly serializes all five tests within this file against each other.
However, tests from **other binaries** running concurrently in `cargo test --workspace` can
still emit to fd 2 during a `capture_stderr` window. This is inherent to the design, not
a bug specific to this file, but worth noting for flakiness investigations.

---

## Recommendations

1. **Fix the `ac1` naming collision** (Tests 1 & 2): rename to `ac1a` / `ac1b` or merge
   into a single `ac1_fallback_on_missing_file` test that asserts both functions degrade.

2. **Tighten the snippet assertion** (Test 2): change `starts_with` to `assert_eq!` with
   the exact production string `"[DEGRADED] Snippet unavailable"`.

3. **Split Test 3** into `ac2a` (display info) and `ac2b` (decay weight) and strengthen the
   weight bound from `> 0.0` to `> 0.90` for freshly created files.

4. **Remove `(reuse-conn)` from Test 4's assertion** or extract it to a constant. The
   method sub-label is an implementation detail; the message content is the behavioral
   contract. This is the highest-priority fix because FR-22 `DbBackend` migration will
   rename this code path.

5. **Add `ac4b`** (companion to Test 5) covering `get_decay_weight_for_file` path redaction
   via `decay/diagnostics.rs::redact_diagnostic_path`.

6. **Add RAII fd-restore to `capture_stderr`** to prevent silent test-binary hangs on
   unexpected panics.

7. **Clarify the `fr25_2` / `RTM FR-25.2` traceability** in comments: add a note that this
   refers to the *archived* diagnostics requirement, not the active FR-25 (Azure Deployment
   Manifests, RTM-137–140).

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing? | Reasoning |
|------|:---:|---------|
| `ac1_decay_fallback` | ✅ | Only test that exercises `GetDecayWeightFromDisk` degraded signal |
| `ac1_snippet_fallback` | ✅ | Only test that exercises `GetFileDisplayInfo` degraded signal |
| `ac2_success_path` | ⚠️ | Negative stderr assertion is unique; positive behavior likely duplicated elsewhere |
| `ac3_malformed_created` | ✅ | Only test for malformed-timestamp DB branch in `get_decay_weight_for_file_with_connection` |
| `ac4_path_redaction` | ✅ | Only test for the privacy/security path-redaction property in `GetFileDisplayInfo` |

Removing any of Tests 1, 2, 4, or 5 would leave a meaningful coverage gap. Test 3 could
be dropped if the negative stderr assertion were redistributed to the two replacement
`ac2a`/`ac2b` tests.

---

## SWE Remediation

- MERGE Tests 1+2 → `fr25_2_ac1_fallback_on_missing_file`: eliminated ac1 naming collision, consolidated both degraded paths into one test with shared setup (R1)
- Tightened snippet assertions from `starts_with` to exact `assert_eq!` in merged ac1 and KEEP ac4 (R2)
- REWRITE Test 3 → split into `fr25_2_ac2a_display_info_success_path_no_degraded_signal` and `fr25_2_ac2b_decay_weight_success_path_no_degraded_signal`; tightened decay weight bound from `> 0.0` to `> 0.90` (R3)
- REWRITE Test 4: removed fragile `(reuse-conn)` sub-label from assertion; now asserts `[MemorySearchTools.Fusion]` component tag + message content only (R4 — P0)
- KEEP Test 5: no logic change, tightened snippet assertion to exact equality
- ADD `fr25_2_ac4b_decay_diagnostics_redacts_absolute_paths`: new companion test covering `get_decay_weight_for_file` path redaction via `decay/diagnostics.rs::redact_diagnostic_path` (R5)
- Replaced `capture_stderr` with RAII `StderrGuard` matching `degraded_diag/support.rs` pattern to prevent silent test-binary hangs on panics (R6)
- Added traceability comment at file top clarifying `fr25_2` references the archived diagnostics requirement, not active FR-25 (R7)
- All 6 tests pass: `cargo test --test integration_tests -p maenifold-core -- memory_search_diag_tests`
- Commit: f44b688

---

## Red-team Audit

- ✅ MERGE Tests 1+2 → `fr25_2_ac1_fallback_on_missing_file`: ac1 naming collision eliminated, all 7 original assertions preserved, no unique coverage lost (R1)
- ✅ TIGHTEN snippet assertions: `starts_with` → exact `assert_eq!` in merged ac1 and KEEP ac4 (R2)
- ✅ REWRITE Test 3 → split into `ac2a` (display info) and `ac2b` (decay weight): one-concept-per-test achieved, weight bound tightened from `> 0.0` to `> 0.90`, negative stderr assertions correctly scoped per-function (R3)
- ✅ REWRITE Test 4 (P0): fragile `(reuse-conn)` sub-label removed; assertion now checks `[MemorySearchTools.Fusion]` component tag + message content only — behavioral contract retained, implementation detail excised (R4)
- ✅ KEEP Test 5: snippet assertion tightened to exact equality; all 4 assertions including the security-critical negative path assertion preserved
- ✅ ADD `fr25_2_ac4b_decay_diagnostics_redacts_absolute_paths`: mirrors ac4 structure, covers parallel path-redaction code path in `decay/diagnostics.rs::redact_diagnostic_path`, uses uuid filename isolation (R5)
- ✅ RAII `StderrGuard`: `Drop` impl restores stderr fd on panic, closing the systemic panic-safety gap in `capture_stderr` (R6)
- ✅ Traceability comment: file-level comment disambiguates `fr25_2` as archived diagnostics requirement vs active FR-25 (RTM-137–140) (R7)
- ✅ No unrelated changes: only eval-specified remediations plus improved assertion diagnostics messages
- ✅ Test count: 5 → 6 (merge 2→1, split 1→2, rewrite 1, keep 1, add 1) — arithmetic verified

Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_search_diag`: 6 passed, 0 failed
- Test inventory confirmed (5 → 6 arithmetic):
  - `fr25_2_ac1_fallback_on_missing_file` — MERGED from Tests 1+2 ✅
  - `fr25_2_ac2a_display_info_success_path_no_degraded_signal` — SPLIT from Test 3 ✅
  - `fr25_2_ac2b_decay_weight_success_path_no_degraded_signal` — SPLIT from Test 3 ✅
  - `fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic` — REWRITTEN Test 4 ✅
  - `fr25_2_ac4_path_diagnostics_redacts_absolute_paths` — KEPT Test 5 ✅
  - `fr25_2_ac4b_decay_diagnostics_redacts_absolute_paths` — ADDED new test ✅
- Verdict: **PASS**
