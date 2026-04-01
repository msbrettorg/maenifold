---
id: T-80.148
title: "Test Evaluation: session_assumption_validate_update"
status: done
pipeline: tdd
---

# Test Evaluation: session_assumption_validate_update

**File:** `crates/maenifold-core/tests/session_assumption_validate_update.rs`
**Test count:** 21
**Evaluated:** 2025-07-15
**Traceability:** T-12.7 / NFR-1 / FR-17.9

---

## Summary

Pure-function validation tests — no I/O, no config, fast. The coverage is mechanically complete: every valid and invalid input is exercised. However, the file contains **two structurally identical fan-out clusters** (tests 6–9 and 14–17) that each probe the same code path with four distinct enum values. Individually every test is technically load-bearing (removing "refined" from the match arm would only be caught by its dedicated test), but the repetition is a maintenance liability and obscures signal-to-noise in test output. Tests 20 and 21 could be merged; test 19 is the weakest link.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `validate_update_input_null_uri_returns_error` | **KEEP** | Boundary: `None` URI → early-return path; distinct from empty-string path |
| `validate_update_input_empty_uri_returns_error` | **KEEP** | Boundary: `Some("")` hits `trim().is_empty()` branch; different code path than `None` |
| `validate_update_input_whitespace_uri_returns_error` | **KEEP** | Boundary: `Some("   ")` is the only test confirming the `.trim()` call matters; removing the trim would only break this test |
| `validate_update_input_valid_uri_null_status_returns_null` | **KEEP** | Happy path: valid URI + absent status must not produce false positive |
| `validate_update_input_valid_uri_empty_status_returns_null` | **KEEP** | Confirms that `Some("")` status is treated as absent — different from `None`, exercises the `.filter(!empty)` guard |
| `validate_update_input_valid_uri_status_active_returns_null` | **MERGE** | Delegates to `validate_assumption_status`; one delegation test is enough. Merge into a single parameterized `#[test]` (see Recommendations) |
| `validate_update_input_valid_uri_status_validated_returns_null` | **MERGE** | Same code path as active; only differs in enum literal |
| `validate_update_input_valid_uri_status_invalidated_returns_null` | **MERGE** | Same code path as active; only differs in enum literal |
| `validate_update_input_valid_uri_status_refined_returns_null` | **MERGE** | Same code path as active; only differs in enum literal |
| `validate_update_input_valid_uri_invalid_status_returns_error` | **MERGE** | Identical error scenario already covered by `validate_status_invalid_status_returns_error`; reduce to checking delegation propagates an error at all, then rely on the dedicated status tests |
| `validate_read_input_null_uri_returns_error` | **KEEP** | `validate_assumption_read_input` is a separate function; tests 1–3 cannot substitute |
| `validate_read_input_empty_uri_returns_error` | **KEEP** | Boundary: `Some("")` vs `None` is meaningful for the same reasons as the update tests |
| `validate_read_input_valid_uri_returns_null` | **KEEP** | Happy path for read; absence of false positive required |
| `validate_status_active_returns_null` | **MERGE** | Part of a 4-variant fan-out; all 4 are parameterizable (see Recommendations) |
| `validate_status_validated_returns_null` | **MERGE** | Same code path as active |
| `validate_status_invalidated_returns_null` | **MERGE** | Same code path as active |
| `validate_status_refined_returns_null` | **MERGE** | Same code path as active |
| `validate_status_active_mixed_case_returns_null` | **KEEP** | Case-insensitivity is a named contract (`eq_ignore_ascii_case`); this is the only test that proves it works |
| `validate_status_validated_upper_case_returns_null` | **REMOVE** | Strictly redundant with `validate_status_active_mixed_case_returns_null`. Both exercise the same `eq_ignore_ascii_case` branch; one is sufficient to prove case-insensitivity. "VALIDATED" upper-case adds no information `"Active"` mixed-case didn't already give |
| `validate_status_invalid_status_returns_error` | **MERGE** | Subsumed by the next test: `expect("error")` in test 21 implicitly asserts `is_some()`; merge the `contains("Invalid status")` assertion into test 21 |
| `validate_status_invalid_status_error_message_contains_all_valid_values` | **REWRITE** | Best test in the file — but asserts `contains("active")` etc. on a known-constant string. The real contract is that the error is *actionable*. Rewrite to also check that the message is prefixed `"ERROR:"` (C# parity) and consider table-driven approach covering both unknown-status and near-miss values |

---

## Observations

### Cluster 1 — update + valid status (tests 6–9)
These four tests confirm that `validate_assumption_update_input` delegates status validation correctly. You only need **one** delegation test (any valid status value) to establish the delegation contract. The other three values are already covered exhaustively by the dedicated `validate_assumption_status` tests (14–17). Keeping all four here means a deleted match arm gets caught twice — which is harmless, but the noise-to-signal tradeoff is poor when running `cargo test -- --test-output immediate`.

### Cluster 2 — validate_status happy-path (tests 14–17)
Each of these IS individually load-bearing: if someone removed `"refined"` from the slice `["active", "validated", "invalidated", "refined"]`, only test 17 would catch it. That said, Rust's `rstest` crate (or a simple macro/loop) could express this as one parameterized test with 4 cases, making the intent crystal-clear and reducing 4 failure lines to 1.

### Tests 20 + 21
Both use `"bogus"` input. Test 20 checks `is_some()` and `contains("Invalid status")`; test 21 calls `.expect("error")` (which also requires `is_some()`) and then checks for all four valid values. The `.expect()` in test 21 already covers the assertion in test 20. **Test 20 is fully subsumed.**

### Naming
Several test names encode return type in a Rust-specific way (`_returns_null` for `Option::None`). Prefer names that describe *behavior*: e.g. `validate_status_rejects_unknown_value` over `validate_status_invalid_status_returns_error`.

---

## Recommendations

1. **Collapse tests 6–9 into one delegation test** — pick one valid status value, rename to `validate_update_input_delegates_status_validation_to_status_validator`, and delete the other three.

2. **Parameterize tests 14–17** — use `rstest` or a `for` loop in a single `#[test]`:
   ```rust
   #[test]
   fn validate_status_accepts_all_valid_values() {
       for status in ["active", "validated", "invalidated", "refined"] {
           assert!(validate_assumption_status(status).is_none(), "expected None for {status}");
       }
   }
   ```
   This preserves individual bug-catching at the cost of a single test name, and the loop message gives per-value context on failure.

3. **Delete test 19** (`validate_status_validated_upper_case_returns_null`) — covered by test 18.

4. **Merge tests 20 + 21** into one test that chains all assertions. Name it `validate_status_rejects_unknown_value_with_actionable_error_listing_all_valid_values`.

5. **Add one missing edge case**: what does `validate_assumption_status("")` (empty string) return? Looking at the implementation — `""` trimmed to `""` will not match any valid status and will return the error. This is untested.

---

## Load-Bearing Score: 19/21

Tests 19 and 20 are load-bearing in theory but are fully subsumed by tests 18 and 21 respectively. Removing them would not allow any realistic bug to ship undetected. The remaining 19 are all genuinely load-bearing against specific regressions in the implementation.

---

## SWE Remediation

- KEEP tests 1–5, 11–13, 18: untouched (URI boundary, happy-path, case-insensitivity)
- MERGE tests 6–9 → `validate_update_input_delegates_status_validation_to_status_validator`: single delegation probe with "active" (Rec #1)
- MERGE test 10 → `validate_update_input_propagates_status_validation_error`: reduced to `is_some()` delegation check
- MERGE tests 14–17 → `validate_status_accepts_all_valid_values`: for-loop parameterized with per-value failure message (Rec #2)
- REMOVE test 19 (`validate_status_validated_upper_case_returns_null`): subsumed by test 18 (Rec #3)
- MERGE+REWRITE tests 20+21 → `validate_status_rejects_unknown_value_with_actionable_error_listing_all_valid_values`: chained assertions for ERROR: prefix, "Invalid status", and all four valid values (Rec #4)
- ADD `validate_status_rejects_empty_string`: edge case for empty-string status (Rec #5)
- Test count: 21 → 14, all passing, 0 coverage loss
- Commit: 314d883

---

## Red-team Audit

| Item | Eval Spec | Code | Status |
|---|---|---|---|
| KEEP tests 1–3 (URI boundary) | Unchanged | `validate_update_input_null_uri_returns_error`, `…empty_uri…`, `…whitespace_uri…` present, unmodified | ✅ |
| KEEP tests 4–5 (happy path absent status) | Unchanged | `validate_update_input_valid_uri_null_status_returns_null`, `…empty_status…` present, unmodified | ✅ |
| MERGE tests 6–9 → single delegation test (Rec #1) | Collapse to one test using "active" | `validate_update_input_delegates_status_validation_to_status_validator` uses `Some("active")`, asserts `.is_none()` with failure message; original 4 tests deleted | ✅ |
| MERGE test 10 → delegation error propagation | Reduce to `is_some()` delegation check | `validate_update_input_propagates_status_validation_error` asserts `result.is_some()` only; `contains("Invalid status")` assertion correctly moved to the rewritten test 20+21 | ✅ |
| KEEP tests 11–13 (read_input) | Unchanged | `validate_read_input_null_uri_returns_error`, `…empty_uri…`, `…valid_uri…` present, unmodified | ✅ |
| MERGE tests 14–17 → parameterized (Rec #2) | for-loop over 4 enum values with per-value failure message | `validate_status_accepts_all_valid_values` loops `["active", "validated", "invalidated", "refined"]` with `"expected None for '{status}'"` message | ✅ |
| KEEP test 18 (case-insensitivity) | Unchanged | `validate_status_active_mixed_case_returns_null` present, unmodified | ✅ |
| REMOVE test 19 (Rec #3) | Delete; subsumed by test 18 | Function deleted; tombstone comment explains subsumption rationale | ✅ |
| MERGE+REWRITE tests 20+21 (Rec #4) | Chain all assertions: `ERROR:` prefix (C# parity), `"Invalid status"`, all 4 valid values | `validate_status_rejects_unknown_value_with_actionable_error_listing_all_valid_values` checks `starts_with("ERROR:")`, `contains("Invalid status")`, and loops 4 values; all with diagnostic failure messages | ✅ |
| ADD empty-string edge case (Rec #5) | New test for `validate_assumption_status("")` | `validate_status_rejects_empty_string` asserts `result.is_some()` with failure message | ✅ |
| Test count | 21 → 14 | 14 `#[test]` annotations, `cargo test` reports 14 passed, 0 failed | ✅ |
| Traceability | T-12.7, FR-17.9, NFR-1 | File header `T-12.7: RTM NFR-1`; inline comments reference `FR-17.9`, `NFR-1`; commit message references `T-12.7: RTM NFR-1 / FR-17.9` | ✅ |
| No unrelated changes | Only this test file + eval spec in commit | `git show --stat 314d883` confirms 2 files changed | ✅ |

- P0: REWRITE test 21 — `ERROR:` prefix assertion added for C# parity ✅
- P1: MERGE clusters collapsed; REMOVE redundant test deleted ✅
- P2: Empty-string edge case added; naming improved on rewritten tests ✅
- No unrelated changes introduced ✅

**Verdict: PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_assumption_validate_update`: 14 passed, 0 failed
- Verdict: PASS
