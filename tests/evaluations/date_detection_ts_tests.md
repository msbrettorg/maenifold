# Test Evaluation: date_detection_ts_tests.rs

**File**: `crates/maenifold-core/tests/date_detection_ts_tests.rs`  
**Traceability**: T-DATE-001 | RTM FR-005, FR-006 (legacy C# oracle IDs — not present in current RTM.md; mapped to `thinking/formatting.rs` and `thinking/session.rs`)  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**Tests evaluated**: 4

---

## Summary

This file contains integration-level regression tests ported from the C# oracle, targeting two behavioral properties of the `SequentialThinking` tool:

1. **Inline thought timestamps** must include the `UTC` suffix when rendered as italicised markdown (FR-005).
2. **Frontmatter completion/cancellation timestamps** must be ISO 8601 (FR-006).

The tests exercise real tool invocations against a temp-directory harness, read the written files back, and assert on both content and frontmatter. The intent is sound. The execution has meaningful weaknesses.

**Critical finding**: Tests 3 and 4 share an ISO 8601 regex that does not assert a timezone designator — meaning a naive `2026-01-01T12:00:00` (no `Z`, no offset) would pass both tests, allowing a UTC regression to go undetected in the frontmatter layer. The production code is currently correct (`now_iso8601_roundtrip()` returns RFC 3339 with `Z`), but the tests do not guard it.

**Secondary finding**: The entire harness (`TestHarness`, `extract_session_id`, `session_uri`, `frontmatter_string`) is a private reimplementation of utilities that already exist in `session/support.rs` and `workflow_supervisor/support.rs`. This creates a silent maintenance trap: any change to the tool's output format or URI scheme must be patched in N copies.

**RTM gap**: The header comment cites `RTM FR-005` and `RTM FR-006`. Neither ID exists in the current `RTM.md` (which starts at FR-22). These are C# oracle legacy IDs. If these requirements were retired they should be reconciled; if they are implicit behavioral contracts they should be added to the RTM.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterisable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `sequential_thought_timestamp_includes_utc_suffix` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | No | **KEEP** |
| 2 | `sequential_thought_timestamp_does_not_contain_bare_timestamp_without_utc` | ⚠️ Marginal | ⚠️ Marginal | ✅ Yes | ✅ Yes | No | **MERGE → #1** |
| 3 | `completed_session_has_iso8601_completed_field_in_frontmatter` | ✅ Yes | ✅ Yes | ⚠️ Weak (no tz) | ✅ Yes | With #4 | **REWRITE** |
| 4 | `cancelled_session_has_iso8601_cancelled_field_in_frontmatter` | ✅ Yes | ✅ Yes | ⚠️ Weak (no tz) | ✅ Yes | With #3 | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `sequential_thought_timestamp_includes_utc_suffix`

**What it tests**: After invoking `SequentialThinking` with `nextThoughtNeeded: true`, the written markdown file contains an italicised timestamp matching `*YYYY-MM-DD HH:MM:SS UTC*`.

**Production path**: `thinking/formatting.rs` → `build_thought_section` → `Utc::now().format("%Y-%m-%d %H:%M:%S UTC")` wrapped in `*…*`.

**Is it load-bearing?** ✅ Yes. No other test in the suite asserts the UTC-suffix format of an in-file thought timestamp. Removing this test would leave a silent regression path open: changing `%Y-%m-%d %H:%M:%S UTC` to `%Y-%m-%d %H:%M:%S` would not be caught anywhere else.

**Is it unique?** ✅ Yes. The closest overlap (`session_sequential_errors.rs`, `complete_session_lifecycle…`) asserts content keywords (`Thought 0/3`, `Analyzing problem space`) but never inspects timestamp format.

**Correct assertions?** ✅ Yes. The regex `\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC\*` is precise and anchors on the wrapping asterisks. Reading from the file (not from the tool response string) correctly validates persisted state.

**Name?** ✅ Clear and descriptive.

**Parameterisable?** Not meaningfully — single path.

**Verdict: KEEP**

**Minor concerns**:
- The guard assertion `assert!(result.contains("Created session:"))` is noise; session creation failure would already panic at `extract_session_id`.
- `TestHarness`, `extract_session_id`, `session_uri`, and `frontmatter_string` are locally redefined. Use `session/support.rs` equivalents.

---

### Test 2 — `sequential_thought_timestamp_does_not_contain_bare_timestamp_without_utc`

**What it tests**: The same file does NOT contain an italicised timestamp matching `*YYYY-MM-DD HH:MM:SS*` (i.e., bare, no UTC suffix).

**Is it load-bearing?** ⚠️ Marginal. The regression this test guards against — format changing from `*…UTC*` to `*…*` — is already caught by Test 1: if UTC disappears, Test 1 fails. The negative assertion in Test 2 only provides independent value in a scenario where the content has *two* timestamps: one with UTC (satisfying Test 1) and a second bare one (caught only by Test 2). Given that `build_thought_section` emits exactly one timestamp, this scenario is structurally improbable.

**Is it unique?** ⚠️ Marginally. The negative assertion is distinct, but the coverage overlap with Test 1 is high.

**Correct assertions?** ✅ Yes. `r"\*\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\*"` correctly requires `*` immediately after the seconds digit — this would NOT match the UTC-bearing format, so there is no false-positive risk.

**Name?** ✅ Accurate, if verbose.

**Parameterisable?** No — but this test should be collapsed into Test 1.

**Verdict: MERGE into Test 1**

Add the negative assertion to `sequential_thought_timestamp_includes_utc_suffix`:
```rust
// positive: UTC suffix present
assert!(utc_timestamp_pattern.is_match(&content), "…");
// negative: bare timestamp absent
assert!(!bare_timestamp_pattern.is_match(&content), "…");
```
This eliminates a full round-trip tool invocation and halves the test count with zero coverage loss.

---

### Test 3 — `completed_session_has_iso8601_completed_field_in_frontmatter`

**What it tests**: After completing a session (`nextThoughtNeeded: false` + `conclusion`), the frontmatter contains `status: completed` and a `completed` field whose value matches an ISO 8601 prefix.

**Production path**: `thinking/session.rs` → `update_session_status("completed")` → `now_iso8601_roundtrip()` → `Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)` → value like `2026-01-01T12:00:00.123456789Z`.

**Is it load-bearing?** ✅ Yes. `session_sequential_errors.rs`'s lifecycle test asserts `frontmatter.contains_key("completed")` but never asserts format. Test 3 is the only test that verifies the value is ISO 8601. Removing it would allow `now_iso8601_roundtrip()` to be silently replaced with a human-readable string without CI catching it.

**Is it unique?** ✅ Yes — for format verification.

**Correct assertions?** ⚠️ Weak. The regex `r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"` anchors at the start and verifies the `T` separator and seconds, but it does **not** assert a timezone designator. A value of `2026-01-01T12:00:00` (no `Z`, no offset) would pass the test. Since FR-006 requires UTC persistence and `now_iso8601_roundtrip()` emits `Z`, the test should verify `Z` or the offset pattern.

```
// Current (insufficient):
r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"

// Recommended:
r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"
```

**Name?** ✅ Verbose but accurate.

**Parameterisable?** ✅ Yes — with Test 4. Both tests follow the pattern `create_session → terminal_action → verify_frontmatter`. They differ only in: (a) how termination is triggered, and (b) which frontmatter key (`completed` vs `cancelled`) and status value is asserted.

**Verdict: REWRITE** (fix the ISO 8601 regex to include timezone assertion; optionally MERGE with Test 4 via a shared helper)

---

### Test 4 — `cancelled_session_has_iso8601_cancelled_field_in_frontmatter`

**What it tests**: After cancelling a session (`cancel: true`), the frontmatter contains `status: cancelled` and a `cancelled` field whose value matches an ISO 8601 prefix.

**Production path**: Same as Test 3 but via `update_session_status("cancelled")`.

**Is it load-bearing?** ✅ Yes. There is no other test asserting the ISO 8601 format of the `cancelled` frontmatter field.

**Is it unique?** ✅ Yes — for the cancellation path specifically.

**Correct assertions?** ⚠️ Same weakness as Test 3: the regex `r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}"` does not verify a timezone designator.

**Name?** ✅ Clear.

**Parameterisable?** ✅ Yes — with Test 3.

**Verdict: REWRITE** (fix timezone regex; consider merging with Test 3 via parameterisation)

---

## Recommendations

### R-1 (High) — Fix ISO 8601 regex in Tests 3 and 4

The current regex does not assert UTC. FR-006 requires UTC persistence. The production code is correct today, but the test does not guard the requirement it claims to protect.

```rust
// Replace in both completed and cancelled tests:
let iso8601_utc_pattern = Regex::new(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"
).expect("regex");
assert!(
    iso8601_utc_pattern.is_match(&completed_value),
    "completed timestamp must be ISO 8601 with timezone. Got: {completed_value}"
);
```

### R-2 (Medium) — Merge Test 2 into Test 1

Test 2 duplicates the full session-creation setup and provides only a negative assertion that Test 1 already implicitly covers in the primary regression scenario. Collapse both assertions into a single test to eliminate redundant integration overhead.

### R-3 (Medium) — Eliminate private harness duplication

`TestHarness`, `extract_session_id`, `session_uri`, and `frontmatter_string` are each defined at least twice across the test suite (`date_detection_ts_tests.rs`, `session/support.rs`, `workflow_supervisor/support.rs`). This file should import from `session/support.rs` (or a shared `tests/common/` module) rather than redefining them.

### R-4 (Low) — Reconcile RTM IDs

The header comment cites `RTM FR-005` and `RTM FR-006`. These IDs do not exist in the current `RTM.md`. Either:
- Add `FR-005` and `FR-006` entries to `RTM.md` as behavioral contracts for thinking timestamp formats, or
- Update the comment to reference the current nearest RTM entries (e.g., the SequentialThinking NFR-1 parity requirements).

### R-5 (Low) — Remove redundant guard assertions

The `assert!(result.contains("Created session:"))` lines in Tests 1 and 2 are defensive guards, but `extract_session_id` already panics if the pattern is absent. Remove to reduce noise; let the panic message in `extract_session_id` serve as the failure signal.

---

## Load-Bearing Score: 3/4

| Test | Load-Bearing |
|------|:---:|
| `sequential_thought_timestamp_includes_utc_suffix` | ✅ |
| `sequential_thought_timestamp_does_not_contain_bare_timestamp_without_utc` | ⚠️ Marginal |
| `completed_session_has_iso8601_completed_field_in_frontmatter` | ✅ |
| `cancelled_session_has_iso8601_cancelled_field_in_frontmatter` | ✅ |

Three of four tests provide genuine, non-redundant regression protection. The fourth (Test 2) is a near-duplicate of Test 1 and should be absorbed into it. All four tests that are load-bearing contain a shared weakness in the ISO 8601 regex (Tests 3 and 4) — the requirements they claim to guard are not fully verified.
