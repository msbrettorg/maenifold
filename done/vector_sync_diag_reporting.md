---
id: vector_sync_diag_reporting
status: done
pipeline: tdd
---

# Test Evaluation: `vector_sync_diag_reporting.rs`

> **Reviewer perspective:** 🔘 Gray — Skeptical Inquiry
> **Source file:** `crates/maenifold-core/tests/vector_sync_diag_reporting.rs`
> **Traceability:** T-9.6 | T-20.1 | RTM FR-7, FR-8 | FR-25.2 (archived diagnostics req) | T-QUAL-002.6b
> **Production code under test:**
> - `src/sync/vector_diagnostics.rs` → `VectorSyncDiagnostics`, `build_vector_failure_report`, `build_vector_unavailable_report`
> - `src/sync/vector_embeddings.rs` → `generate_concept_embeddings`
> **Peer files:** `vector_sync_diag_unavailable.rs` (4 tests — tool-layer unavailable path), `vector_tools_batch.rs` (uses `generate_concept_embeddings` success path)
> **Date:** 2025-07-25

---

## Summary

This file holds 5 unit/integration tests covering the **diagnostic reporting contract** for
vector sync: the guarantee that `VectorSyncDiagnostics` correctly counts failures and
skips, that both report-builder functions emit well-formed structured output, that sensitive
data (file paths, exception strings) does not leak into that output, and that detail
sampling is capped and deduplicated.

Before assigning verdicts, the following questions were answered against the production code:

1. **Is there overlap with peer test files?** `vector_sync_diag_unavailable.rs` covers
   `build_vector_unavailable_report` indirectly via the full-stack tool integration layer
   (with `MAENIFOLD_DISABLE_VECTOR=true`). None of those tests call the report builders
   directly at the unit level. The unit coverage here is not duplicated. `vector_tools_batch.rs`
   exercises `generate_concept_embeddings` in a *success* path against a real `vec_concepts`
   table; the failure path (no table) is unique to Test 2 in this file.

2. **Are the five tests sufficient to define the report format contract?** No. Both report
   builders emit a `"- STATUS: …"` line and `build_vector_failure_report` emits a
   `"- FAILURES:"` section header that no test in this file asserts. Any regression
   removing those lines would pass undetected.

3. **Are any naming or traceability problems present?** Two tests carry the same `fr25_2_ac1_`
   prefix despite testing different functions. The `fr25_2` prefix also embeds a reference
   to an archived requirement (FR-25.2, the original diagnostics AC) that conflicts with the
   active FR-25 in the current RTM (Azure Deployment Manifests, RTM-137–140), creating
   future reader confusion.

4. **Does Test 2 have an environment-dependent flakiness risk?** Yes. If
   `MAENIFOLD_DISABLE_VECTOR=true` is set in CI, `generate_concept_embeddings` takes the
   skip path instead of the failure path and the assertions on `concept_embedding_failures()`
   would fail with a confusing mismatch.

**Bottom line:** All five tests guard unique, load-bearing behaviors not covered elsewhere.
The file is net-positive. However, two tests share a naming collision, three tests have
assertion gaps that would allow real regressions to pass silently, and one test has
environment-dependent fragility. No test should be removed. Three warrant rewrites;
one warrants a targeted addition; the fifth is the strongest test in the file.

---

## Verdicts

| # | Test Name | Verdict | Load‑Bearing | Unique | Correct Assertions | Good Name | Parameterizable |
|---|-----------|:-------:|:---:|:---:|:---:|:---:|:---:|
| 1 | `fr25_2_ac1_vector_sync_failures_are_counted_and_surfaced_in_diagnostics_output` | **REWRITE** | ✅ | ✅ | ⚠️ missing `STATUS` + `FAILURES` header | ❌ `ac1` collision | ⚠️ counts only |
| 2 | `fr25_2_ac1_generate_concept_embeddings_failure_is_captured_with_reason` | **REWRITE** | ✅ | ✅ | ⚠️ reason count unchecked; env-var fragile | ❌ `ac1` collision | ✅ |
| 3 | `fr25_2_ac5_vector_unavailable_reason_codes_are_normalized_and_do_not_leak_exception_or_paths` | **REWRITE** | ✅ | ✅ | ⚠️ only UNKNOWN path; counter lines absent | ⚠️ name implies full coverage | ✅ |
| 4 | `fr25_2_ac6_vector_diagnostics_clearly_separate_skipped_and_failed_counters` | **REWRITE** | ✅ | ✅ | ⚠️ reason lines and `STATUS` not checked | ✅ | ⚠️ dual-function |
| 5 | `fr7_t20_1_concept_embedding_failure_details_are_sampled_and_rendered` | **KEEP** (+enhance) | ✅ | ✅ | ⚠️ dedup + `DETAIL` prefix unchecked | ✅ | ⚠️ cap constant |

---

## Per-Test Analysis

---

### Test 1 — `fr25_2_ac1_vector_sync_failures_are_counted_and_surfaced_in_diagnostics_output`

#### What it tests

Constructs a `VectorSyncDiagnostics` in isolation, records 1 `CONCEPT_LOOKUP_FAILED` and
2 `FILE_EMBEDDING_FAILED` failures, then calls `build_vector_failure_report` and inspects
the returned lines. The three counter accessors (`concept_embedding_failures`,
`file_embedding_failures`, `total_failures`) are also directly asserted.

#### Load-bearing?

**Yes.** This is the only test that directly exercises `build_vector_failure_report` at the
unit level with a mixed failure set (concept + file). The peer integration tests in
`vector_sync_diag_unavailable.rs` never invoke the failure report path.

#### Unique?

**Yes.** No other test file calls `build_vector_failure_report` with recorded reason codes
and validates the rendered line format.

#### Assertion quality

| Assertion | What it guards | Verdict |
|-----------|----------------|---------|
| `concept_embedding_failures() == 1` | Counter increment for concept path | ✅ |
| `file_embedding_failures() == 2` | Counter increment for file path | ✅ |
| `total_failures() == 3` | Additive total not capped or wrong | ✅ |
| `lines.iter().any(line == "Vector sync diagnostics:")` | Header line present | ✅ |
| `lines.iter().any(line == "- COUNTERS_FAILED: concepts=1, memory=2, total=3")` | Failure counter rendering | ✅ |
| `lines.iter().any(line == "- COUNTERS_SKIPPED: concepts=0, memory=0, total=0")` | Zero-skipped rendering | ✅ |
| `lines.iter().any(line == "- REASON FILE_EMBEDDING_FAILED: 2")` | Per-reason count rendering | ✅ |
| `lines.iter().any(line == "- REASON CONCEPT_LOOKUP_FAILED: 1")` | Per-reason count rendering | ✅ |
| `!output.contains("SqliteException")` | No exception leakage | ✅ |

**Missing assertions:**

- `"- STATUS: failed"` — `build_vector_failure_report` unconditionally emits this line as
  the second element. No test in this file asserts it. A regression that changes `"failed"`
  to `"error"` or removes the line entirely passes undetected.
- `"- FAILURES:"` section header — the function emits this label line before individual
  reason lines when the reason map is non-empty. The test asserts the reason lines directly
  but not the header that groups them. If the header were removed, the output structure
  breaks but all existing assertions still pass.

#### Naming

The `fr25_2_ac1_` prefix collides with Test 2. Both tests exercise entirely different
entry points (`build_vector_failure_report` vs. `generate_concept_embeddings`). The shared
prefix implies a single acceptance criterion when there are at least two. The `fr25_2`
component refers to an archived historical requirement; the active FR-25 in the current
RTM is Azure Deployment Manifests (RTM-137–140), creating a misleading cross-reference.

**Verdict: REWRITE** — fix naming collision (`ac1a`), add `"- STATUS: failed"` and
`"- FAILURES:"` assertions, add a comment clarifying the `fr25_2` archival status.

---

### Test 2 — `fr25_2_ac1_generate_concept_embeddings_failure_is_captured_with_reason`

#### What it tests

Opens an in-memory SQLite connection (no `vec_concepts` table), passes it to
`generate_concept_embeddings` with `vector_ready=true`, and asserts the function returns 0
and populates diagnostics with `CONCEPT_LOOKUP_FAILED`. This is an integration test of the
function's SQL-error handling branch: `connection.prepare(…)` will fail because
`vec_concepts` does not exist in a fresh in-memory database, triggering
`record_concept_failures(diagnostics, CONCEPT_LOOKUP_FAILED, …)` before returning 0.

#### Load-bearing?

**Yes.** `vector_tools_batch.rs` calls `generate_concept_embeddings` in a success path
against a properly seeded database. The failure path (SQL error → diagnostic population →
early return of 0) is only tested here.

#### Unique?

**Yes.** No other test exercises the `CONCEPT_LOOKUP_FAILED` path through an actual DB call.
Test 1 records that reason code manually; this test earns it through real function behavior.

#### Assertion quality

| Assertion | What it guards | Verdict |
|-----------|----------------|---------|
| `generated == 0` | Early return on SQL failure | ✅ |
| `concept_embedding_failures() == 1` | Failure counted exactly once | ✅ |
| `total_failures() == 1` | No phantom additional failures recorded | ✅ |
| `failure_reasons().contains_key(CONCEPT_LOOKUP_FAILED)` | Reason code recorded | ⚠️ weak |

**Missing assertion — reason count:**

`contains_key(…)` verifies the reason code was *ever* recorded but does not check the
count. `failure_reasons().get(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED)` should
equal `Some(1)`. Without this check, a bug that double-counts the reason (e.g., recording
it in both `record_concept_failures` and an outer loop) would pass: `concept_embedding_failures() == 1`
bounds the failure count but the reason map count is independent.

**Environment-variable fragility:**

`generate_concept_embeddings` checks `vector_sync_disabled_by_config()` (reads
`MAENIFOLD_DISABLE_VECTOR` env var) before the SQL path. If `MAENIFOLD_DISABLE_VECTOR=true`
is set in the CI environment or via a leaked `EnvVarReset` from a parallel test, the
function skips the SQL entirely: `diagnostics.record_concept_skipped(1)` is called and the
function returns 0. The assertion `concept_embedding_failures() == 1` then fails with a
confusing `0 != 1` error that looks like a logic bug rather than a test isolation gap.
The test should either use `EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "false")` or
explicitly assert `concept_embedding_skipped() == 0` to make the failure mode obvious.

**Verdict: REWRITE** — rename to `ac1b` (or `generate_concept_embeddings_lookup_failure_is_captured`),
add `failure_reasons().get(CONCEPT_LOOKUP_FAILED) == Some(&1)`, guard against
`MAENIFOLD_DISABLE_VECTOR`.

---

### Test 3 — `fr25_2_ac5_vector_unavailable_reason_codes_are_normalized_and_do_not_leak_exception_or_paths`

#### What it tests

Calls `build_vector_unavailable_report` with a reason string containing a raw exception
class and an absolute file path. Asserts the output normalizes to
`VECTOR_UNAVAILABLE_UNKNOWN` and neither the exception text nor the path appears in the
output. Also records 1 skipped concept and 1 skipped file before calling the report builder,
but never asserts the counter lines.

#### Load-bearing?

**Yes.** The security/privacy contract tested here — that operator-visible diagnostic
output cannot contain raw exception strings or filesystem paths from user machines — is
critical and not asserted anywhere else at this granularity. The peer tests in
`vector_sync_diag_unavailable.rs` use `DISABLED_BY_CONFIG` (a known-good code) and never
inject adversarial input strings.

#### Unique?

**Yes** for the adversarial path. **Partially overlapping** for the `VECTOR_UNAVAILABLE_UNKNOWN`
reason code: no other test exercises the UNKNOWN normalization. However, the positive
normalization case (`DISABLED_BY_CONFIG`) is *not* tested here at the unit level — only
indirectly through the integration tests in `vector_sync_diag_unavailable.rs`. That gap
means a regression that swapped the constant values of `DISABLED_BY_CONFIG` and `UNKNOWN`
would not be caught at the unit level.

#### Assertion quality

| Assertion | What it guards | Verdict |
|-----------|----------------|---------|
| `lines.iter().any(line == "- REASON_CODE VECTOR_UNAVAILABLE_UNKNOWN")` | Unknown-reason normalization | ✅ |
| `!output.contains("SqliteException")` | Exception string scrubbed | ✅ security |
| `!output.contains("/Users/brett/private/secrets.db")` | File path scrubbed | ✅ security |

**Missing assertions:**

- `"- STATUS: unavailable"` — `build_vector_unavailable_report` emits this on every call;
  no assertion verifies it.
- `"- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"` — the test goes to the trouble of
  recording skipped counts but never asserts the report reflects them. If the report builder
  dropped the `COUNTERS_SKIPPED` line, this test would pass despite the regression.
- **DISABLED_BY_CONFIG normalization** — the companion case: when the reason is exactly
  `VectorUnavailableReasonCodes::DISABLED_BY_CONFIG`, the normalized output should be
  `VECTOR_DISABLED_BY_CONFIG`, not `VECTOR_UNAVAILABLE_UNKNOWN`. The name implies testing
  "reason codes" (plural) but only one path is covered.

#### Parameterization opportunity

The test is a strong candidate for parameterization: a table of `(input_reason, expected_output_code)` pairs would cover `DISABLED_BY_CONFIG`, an entirely unknown string, an empty string, a lowercase variant, and the adversarial exception+path string in a single consolidated test.

**Verdict: REWRITE** — split into `ac5a_unknown_reason_is_normalized` (with counter
assertions + STATUS) and `ac5b_disabled_by_config_reason_is_normalized`, or parameterize.
Add `STATUS` and `COUNTERS_SKIPPED` assertions to the adversarial case.

---

### Test 4 — `fr25_2_ac6_vector_diagnostics_clearly_separate_skipped_and_failed_counters`

#### What it tests

Records 1 concept failure + 1 file failure + 3 concept skipped + 1 file skipped, then calls
**both** `build_vector_unavailable_report` and `build_vector_failure_report`, asserting that
both render the same `COUNTERS_FAILED` and `COUNTERS_SKIPPED` lines correctly.

#### Load-bearing?

**Yes.** This is the only test that verifies counter *separation* (failed ≠ skipped,
neither bleeds into the other) across both report builders simultaneously. The companion
tests in `vector_sync_diag_unavailable.rs` use an all-zero failure set; the counter
separation property requires a mixed state.

#### Unique?

**Yes.** No other test exercises both report builders against the same diagnostics instance
with non-zero values in all four counter fields.

#### Structural problem — dual-function bundling

The test exercises two independent code paths (the two report builders) inside one `#[test]`
function. If an assertion fails, the developer must determine which report builder produced
the unexpected output. The two report builders have different responsibilities:
`build_vector_failure_report` is called when sync *fails*; `build_vector_unavailable_report`
is called when the vector extension is *absent*. Conflating their validation into one test
obscures which contract is violated.

#### Assertion quality

| Assertion | What it guards | Verdict |
|-----------|----------------|---------|
| `unavailable_lines` — `COUNTERS_FAILED: concepts=1, memory=1, total=2` | Failure counters in unavailable report | ✅ |
| `unavailable_lines` — `COUNTERS_SKIPPED: concepts=3, memory=1, total=4` | Skip counters in unavailable report | ✅ |
| `failure_lines` — `COUNTERS_FAILED: concepts=1, memory=1, total=2` | Failure counters in failure report | ✅ |
| `failure_lines` — `COUNTERS_SKIPPED: concepts=3, memory=1, total=4` | Skip counters in failure report | ✅ |

**Missing assertions:**

- `"- STATUS: failed"` in `failure_lines` — not checked.
- `"- STATUS: unavailable"` in `unavailable_lines` — not checked.
- `"- REASON CONCEPT_EMBEDDING_FAILED: 1"` and `"- REASON FILE_EMBEDDING_FAILED: 1"` in
  `failure_lines` — the failure report emits these; they are entirely absent from the
  assertions. A bug that dropped the reason section after recording failures would go
  undetected here.
- `"- REASON_CODE VECTOR_DISABLED_BY_CONFIG"` in `unavailable_lines` — the test passes a
  known reason code but does not assert it appears in output.

**Verdict: REWRITE** — split into `ac6a_failure_report_counters_are_separated` and
`ac6b_unavailable_report_counters_are_separated`. Add `STATUS`, `REASON_CODE`, and reason
line assertions in each variant.

---

### Test 5 — `fr7_t20_1_concept_embedding_failure_details_are_sampled_and_rendered`

#### What it tests

Records 5 concept failures with detail via `record_concept_failure_with_detail`. Uses:
- 3 unique detail strings (the cap)
- 1 duplicate of an already-recorded string
- 1 unique string that would be the 4th distinct entry (over the cap)

Calls `build_vector_failure_report` and asserts: total failure count is 5, the reason
header line is present, the detail section header is present, the 3 admitted detail strings
appear in the output, and the 4th would-be entry is absent.

#### Load-bearing?

**Yes.** This is the only test for the T-20.1 feature:
`record_concept_failure_with_detail`, `sanitize_failure_detail`, and the
`MAX_CONCEPT_FAILURE_DETAILS` cap. `build_vector_failure_report`'s `FAILURE_DETAILS` section
is entirely untested outside this file.

#### Unique?

**Yes.** No duplication detected in any peer file.

#### Assertion quality

| Assertion | What it guards | Verdict |
|-----------|----------------|---------|
| `concept_embedding_failures() == 5` | All calls counted, not just distinct | ✅ |
| `lines.iter().any(line == "- REASON CONCEPT_EMBEDDING_FAILED: 5")` | Reason count in failure report | ✅ |
| `lines.iter().any(line == "- FAILURE_DETAILS CONCEPT_EMBEDDING_FAILED:")` | Section header present | ✅ |
| `output.contains("tokenizer encode failed: missing [UNK] token in tokenizer.json")` | Newline normalization via `sanitize_failure_detail` | ✅ subtle |
| `output.contains("onnx runtime error: invalid input_ids tensor shape")` | First admitted detail present | ✅ |
| `output.contains("embedding model not found: …")` | Third admitted detail present | ✅ |
| `!output.contains("extra sampled detail should be dropped by the cap")` | Cap enforced | ✅ |

**Minor gaps:**

- **Explicit deduplication check:** The test records `"onnx runtime error: invalid input_ids
  tensor shape"` twice but never explicitly asserts that only 3 detail lines (not 4) appear
  in the output. The cap assertion is satisfied by the count cap *and* by deduplication, but
  these are distinct code paths in `record_concept_failure_detail`. A bug that disabled
  deduplication but kept the count cap would still pass all existing assertions. Add:
  `assert_eq!(diagnostics.concept_failure_details().get(CONCEPT_EMBEDDING_FAILED).map(|v| v.len()), Some(3))`.

- **`"- DETAIL"` prefix format:** The rendered detail lines use the format
  `"- DETAIL {text}"`. The test checks `output.contains(detail_text)` (the text portion),
  but not that the prefix is `"- DETAIL "` rather than `"  - "` or similar. If the prefix
  format changed, callers parsing the report would break but this test would pass. A single
  `lines.iter().any(|line| line.starts_with("- DETAIL tokenizer encode"))` would catch it.

**Naming:** `fr7_t20_1_concept_embedding_failure_details_are_sampled_and_rendered` is
the clearest and most accurate name in the file. It references FR-7 and the specific task
T-20.1, and the description matches the behavior exactly.

**Verdict: KEEP** — add the explicit deduplication count assertion and one `"- DETAIL"`
prefix format check. No structural changes required.

---

## Recommendations

1. **Resolve the `ac1` naming collision** (Tests 1 & 2): rename Test 1 to
   `fr25_2_ac1a_failure_report_counters_and_reason_codes_are_rendered` and Test 2 to
   `fr25_2_ac1b_generate_concept_embeddings_lookup_failure_is_captured`. The collision
   implies a single acceptance criterion covers both behaviors when they are distinct.

2. **Add `"- STATUS: failed"` assertion** to Test 1, and `"- STATUS: unavailable"` to
   Tests 3 and 4. These lines are unconditionally emitted; their absence from all
   assertions is the largest single assertion gap in this file.

3. **Add `"- FAILURES:"` header assertion** to Test 1. The section header is structurally
   important for any consumer parsing the report.

4. **Add failure-reason count assertion** to Test 2:
   ```rust
   assert_eq!(
       diagnostics.failure_reasons().get(VectorFailureReasonCodes::CONCEPT_LOOKUP_FAILED),
       Some(&1)
   );
   ```

5. **Guard `MAENIFOLD_DISABLE_VECTOR` in Test 2.** Either reset the env var explicitly or
   add `assert_eq!(diagnostics.concept_embedding_skipped(), 0)` to surface the failure
   mode clearly rather than producing a confusing counter mismatch.

6. **Add `DISABLED_BY_CONFIG` normalization case to Test 3** (as `ac5b` or a parameterized
   variant). The `normalize_vector_unavailable_reason` function has two branches; only the
   UNKNOWN branch is unit-tested here. The integration tests in `vector_sync_diag_unavailable.rs`
   cover it end-to-end but do not verify the normalization function directly.

7. **Split Test 4 into `ac6a` and `ac6b`**, then add reason-line and `REASON_CODE`
   assertions to each. Dual-function tests obscure which contract failed.

8. **Add explicit deduplication assertion to Test 5**:
   ```rust
   assert_eq!(
       diagnostics.concept_failure_details()
           .get(VectorFailureReasonCodes::CONCEPT_EMBEDDING_FAILED)
           .map(|v| v.len()),
       Some(3),
       "duplicate detail strings should be deduplicated, cap should hold at 3"
   );
   ```

9. **Add `"- DETAIL"` prefix format check to Test 5** — a single `any(|l| l.starts_with("- DETAIL "))`
   assertion guards the rendered line format without duplicating content checks.

10. **Add a comment on `fr25_2_` prefix** in all tests (or strip it from the function
    names): in the current RTM, FR-25 maps to Azure Deployment Manifests (RTM-137–140).
    The `fr25_2` infix originated from an archived diagnostic requirement and will mislead
    future contributors attempting to trace tests to the requirements matrix.

---

## Load-Bearing Score: 5/5

All five tests guard behaviors not exercised by any peer test file at the same level of
specificity. None should be removed. Quality scores vary significantly.

| Test | Load-Bearing | Rationale |
|------|:---:|---------|
| `ac1_vector_sync_failures_counted` (Test 1) | ✅ | Only unit-level test of `build_vector_failure_report` with mixed reason codes |
| `ac1_generate_concept_embeddings_failure` (Test 2) | ✅ | Only test of the SQL-error → `CONCEPT_LOOKUP_FAILED` path through the actual function |
| `ac5_unavailable_reason_normalization` (Test 3) | ✅ | Only test guarding the privacy/security contract (no exception or path leakage) |
| `ac6_skipped_failed_counter_separation` (Test 4) | ✅ | Only test with non-zero values in all four counter fields across both report builders |
| `fr7_t20_1_detail_sampling` (Test 5) | ✅ | Only test for detail cap, deduplication, newline normalization, and `FAILURE_DETAILS` section format |

The file scores **3/5 on assertion completeness** (Tests 1–4 each have at least one
meaningful gap; Test 5 has minor gaps only). Priority order for rewrites: Test 4 (dual
function + missing reason lines) → Test 3 (single-branch normalization coverage) → Test 2
(env-var fragility) → Test 1 (missing status/header lines) → Test 5 (minor enhancements).

---

## SWE Remediation

**Test count: 5 → 7** (Test 3 split into ac5a/ac5b, Test 4 split into ac6a/ac6b)

### Changes Applied

- **REWRITE Test 1 → `fr25_2_ac1a_failure_report_counters_and_reason_codes_are_rendered`:**
  Renamed from `ac1` to `ac1a` resolving naming collision with Test 2. Added `"- STATUS: failed"` and `"- FAILURES:"` section header assertions (Rec 2, 3 — P0). Added archival-status module-doc comment (Rec 10).

- **REWRITE Test 2 → `fr25_2_ac1b_generate_concept_embeddings_lookup_failure_is_captured`:**
  Renamed from `ac1` to `ac1b` resolving naming collision. Added `EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "false")` guard (Rec 5 — P0). Added `failure_reasons().get(CONCEPT_LOOKUP_FAILED) == Some(&1)` reason count assertion (Rec 4 — P0). Added `concept_embedding_skipped() == 0` guard assertion for clear env-var failure diagnostics.

- **REWRITE Test 3 → `fr25_2_ac5a_unknown_unavailable_reason_is_normalized_and_does_not_leak_sensitive_data`:**
  Added `"- STATUS: unavailable"` and `"- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"` assertions (Rec 2 — P0). Retained all security/privacy assertions.

- **NEW Test 3b → `fr25_2_ac5b_disabled_by_config_reason_is_normalized`:**
  Added companion DISABLED_BY_CONFIG normalization case (Rec 6 — P1). Asserts STATUS line and `"- REASON_CODE VECTOR_DISABLED_BY_CONFIG"` positive normalization.

- **REWRITE Test 4 → split into `fr25_2_ac6a_failure_report_counters_are_separated` and `fr25_2_ac6b_unavailable_report_counters_are_separated`:**
  Split dual-function test into two isolated tests (Rec 7 — P1). Each now asserts STATUS, counters, and reason/reason-code lines independently.

- **KEEP Test 5 (enhanced) → `fr7_t20_1_concept_embedding_failure_details_are_sampled_and_rendered`:**
  Added explicit deduplication count assertion: `concept_failure_details().get(...).map(|v| v.len()) == Some(3)` (Rec 8 — P1). Added `"- DETAIL"` prefix format check via `starts_with("- DETAIL tokenizer encode")` (Rec 9 — P2).

### Recommendations addressed

| # | Priority | Status |
|---|----------|--------|
| 1 | P0 | ✅ `ac1` naming collision resolved (ac1a/ac1b) |
| 2 | P0 | ✅ STATUS assertions added to Tests 1, 3, 4a, 4b |
| 3 | P0 | ✅ FAILURES header assertion added to Tests 1, 4a |
| 4 | P0 | ✅ Reason count assertion added to Test 2 |
| 5 | P0 | ✅ MAENIFOLD_DISABLE_VECTOR env-var guard in Test 2 |
| 6 | P1 | ✅ DISABLED_BY_CONFIG normalization case added (Test 3b) |
| 7 | P1 | ✅ Test 4 split into ac6a/ac6b with reason assertions |
| 8 | P1 | ✅ Deduplication count assertion in Test 5 |
| 9 | P2 | ✅ DETAIL prefix format check in Test 5 |
| 10 | P3 | ✅ Archival-status comment on `fr25_2_` prefix |

- Commit: 07fd5ab (bundled with update_assets_boundary remediation)

---

## Red-team Audit

**Traceability:** T-9.6 | T-20.1 | T-QUAL-002.6b | RTM FR-7, FR-8

### REWRITE Tests

- ✅ **REWRITE Test 1 → `ac1a`**: Naming collision resolved (`ac1` → `ac1a`). `"- STATUS: failed"` assertion added (Rec 2, P0). `"- FAILURES:"` section header assertion added (Rec 3, P0). Module-level archival comment on `fr25_2_` prefix present (Rec 10, P3). All original assertions retained. Matches eval spec.

- ✅ **REWRITE Test 2 → `ac1b`**: Naming collision resolved (`ac1` → `ac1b`). `EnvVarReset::set("MAENIFOLD_DISABLE_VECTOR", "false")` guard added (Rec 5, P0). `failure_reasons().get(CONCEPT_LOOKUP_FAILED) == Some(&1)` reason count assertion added (Rec 4, P0). `concept_embedding_skipped() == 0` guard assertion added for clear env-var failure diagnostics. All original assertions retained. Matches eval spec.

- ✅ **REWRITE Test 3 → `ac5a`**: `"- STATUS: unavailable"` assertion added (Rec 2, P0). `"- COUNTERS_SKIPPED: concepts=1, memory=1, total=2"` assertion added (closes counter gap). All three security/privacy assertions retained (exception scrub, path scrub, UNKNOWN normalization). Matches eval spec.

### NEW Tests

- ✅ **NEW Test 3b → `ac5b`**: DISABLED_BY_CONFIG normalization case added (Rec 6, P1). Asserts STATUS line and `"- REASON_CODE VECTOR_DISABLED_BY_CONFIG"` positive normalization. Companion coverage for the second branch of `normalize_vector_unavailable_reason`. Matches eval spec.

### SPLIT Tests

- ✅ **REWRITE Test 4 → split `ac6a`/`ac6b`** (Rec 7, P1):
  - `ac6a` (failure report): Asserts STATUS, both counter lines, FAILURES header, and both REASON lines (`CONCEPT_EMBEDDING_FAILED: 1`, `FILE_EMBEDDING_FAILED: 1`). Matches eval spec.
  - `ac6b` (unavailable report): Asserts STATUS, both counter lines, and `REASON_CODE VECTOR_DISABLED_BY_CONFIG`. Matches eval spec.
  - Original dual-function test fully replaced. No unique coverage lost.

### KEEP (enhanced)

- ✅ **KEEP Test 5 (enhanced)**: Deduplication count assertion added: `concept_failure_details().get(...).map(|v| v.len()) == Some(3)` (Rec 8, P1). `"- DETAIL"` prefix format check added via `starts_with("- DETAIL tokenizer encode")` (Rec 9, P2). All 7 original assertions retained. Matches eval spec.

### Cross-cutting

- ✅ Test count 5 → 7: confirmed (`running 7 tests`)
- ✅ All tests pass: confirmed (`test result: ok. 7 passed; 0 failed`)
- ✅ All 10 recommendations (P0–P3) addressed per remediation table
- ✅ No production code changes in commit (only test files)
- ✅ No unrelated changes introduced (commit modifies `vector_sync_diag_reporting.rs` + sibling `update_assets_boundary.rs` only)
- ✅ Module-level doc comment clarifies `fr25_2_` archival provenance (Rec 10)

### Verdict: PASS

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- vector_sync_diag_reporting`: 7 passed, 0 failed
- Verdict: PASS
