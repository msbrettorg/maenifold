---
id: workflow_supervisor_registration
status: done
---

# Test Evaluation: workflow_supervisor_registration
**File:** `crates/maenifold-core/tests/workflow_supervisor_registration.rs`
**Test count:** 6
**Evaluated:** 2026-07-11
**Traceability:** T-W1-7 · T-12.7 · NFR-1 · FR-10.1 · FR-10.3 · FR-10.4

---

## Summary

Six tests covering the submachine registration path of the workflow supervisor: the invalid-session rejection, the happy-path content note, the response-persistence + step-freeze invariant, WikiLink validation enforcement, the double-registration guard, and the status-transition block while waiting. Every test has a distinct primary target; no two tests are pure duplicates. Three specific concerns deserve attention:

1. **Weak `assert_ne!` on phase** (tests 1 and 4): both use `assert_ne!(phase, "waiting")` to verify rejection, but that assertion passes for *any* non-waiting value, including a hypothetical error/poison phase introduced by a bug. The known pre-registration phase is `"active"` — an `assert_eq!(phase, "active")` would catch a wider failure class.

2. **Content-presence overlap** (tests 2 and 3): both read the session body after a successful registration. They assert on different strings but the structural duplication means a failing write would cause both to fail simultaneously, providing no additional diagnostic signal. Not a defect, but worth noting.

3. **Two sub-cases in one test** (test 6): `status_parameter_while_waiting_is_blocked` exercises "completed" *and* "cancelled" in sequence. Both execute successfully, but parametrizing them would isolate failure attribution. The sequential nature also means the second "cancel" attempt operates on state the first "complete" attempt already touched — low risk but obscures independence.

No test is spurious. Load-bearing score is 6/6.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `register_nonexistent_submachine_returns_error` | **KEEP** (harden assertion) | Only test for the invalid-session-ID rejection path (FR-10.1). Verifies ERROR returned and session ID echoed in the message — both load-bearing. The `assert_ne!(phase, "waiting")` third assertion is weak: substitute `assert_eq!(phase, "active")` to catch phase corruption on error paths. |
| `register_submachine_appends_note_to_session_content` | **KEEP** | Verifies the registration note surface: "Submachine Registered" label, thinking session ID, and type string "sequential" all present in the body. Distinct from test 3 — this checks the *registration annotation*, not the *caller's response text*. Both content-reading tests are justified. |
| `response_persisted_during_register` | **KEEP** | The most complete happy-path assertion: response text written, WikiLinks preserved verbatim, `currentStep` frozen at `step_before`, and phase set to "waiting". The `step_before` capture pattern is correct — it defensively asserts no advancement without hard-coding the step number. This is the canonical test for the step-freeze invariant. |
| `register_path_enforces_wikilink_validation_on_response` | **KEEP** (harden assertion) | Sole test for WikiLink validation on the registration path (FR-10.3). The `result.contains("[[WikiLink]]")` assertion couples to the exact error-message string — if that literal is ever refactored the test silently weakens. Consider extracting the expected string to a named constant. The `assert_ne!(phase, "waiting")` should again be `assert_eq!(phase, "active")`. |
| `double_registration_while_waiting_is_blocked` | **KEEP** | Strongest test in the file. Two-phase assertion: verifies `activeSubmachineSessionId` is set to `first_session_id` after the first call, then confirms it is *unchanged* after the second attempt and the "waiting on submachine" error is returned. This would catch a bug where the backend replaces the stored session ID. No coverage overlap with any other test. |
| `status_parameter_while_waiting_is_blocked` | **KEEP** (note parameterizable) | Exercises both "completed" and "cancelled" status transitions being blocked while a submachine is registered. Final state assertion (`status = "active"`, `phase = "waiting"`) is comprehensive. The two status variants are sequential sub-cases and could be separated into `completion_blocked_while_waiting` / `cancellation_blocked_while_waiting` to isolate failure attribution, but the current form is not incorrect. |

---

## Recommendations

### P1 — Harden the weak `assert_ne!` assertions (tests 1 and 4)

Both `register_nonexistent_submachine_returns_error` and `register_path_enforces_wikilink_validation_on_response` use:

```rust
assert_ne!(
    yaml_string(&read_frontmatter(...), "phase"),
    "waiting",
);
```

This passes for `"active"`, `""`, `"error"`, or any other non-waiting string, including states that could indicate a partially-applied bug. Since the workflow starts in `"active"` phase before the first registration attempt, the correct assertion is:

```rust
assert_eq!(
    yaml_string(&read_frontmatter(...), "phase"),
    "active",
);
```

### P2 — Extract the WikiLink error literal (test 4)

`result.contains("[[WikiLink]]")` embeds a raw string constant. If the error message is ever reworded, this silently degrades. Define a shared constant in `support.rs`:

```rust
pub(crate) const WIKILINK_ERROR_HINT: &str = "[[WikiLink]]";
```

### P3 — Consider splitting test 6 into two focused tests

`status_parameter_while_waiting_is_blocked` packs both `"completed"` and `"cancelled"` into one test body. The two transitions are independent policy decisions; if either one regresses, the combined test obscures which path broke. Splitting into `completion_blocked_while_waiting_is_blocked` and `cancellation_blocked_while_waiting_is_blocked` would improve diagnostic precision at the cost of one `isolated_env()` setup.

### P4 — Rename to reflect behaviour, not mechanism

`status_parameter_while_waiting_is_blocked` names the *input parameter* rather than the *behavioural invariant*. Compare to the clear `double_registration_while_waiting_is_blocked`. A rename to `status_transitions_blocked_while_waiting` or, if split per P3, to per-status names, would be consistent with the naming register of the rest of the file.

---

## Load-Bearing Score: 6/6

---
## SWE Remediation
- KEEP `register_nonexistent_submachine_returns_error`: replaced the weak non-waiting check with an exact phase-preservation assertion against the known pre-registration `running` phase.
- KEEP `register_path_enforces_wikilink_validation_on_response`: replaced the weak non-waiting check with the same exact phase-preservation assertion.
- KEEP `register_path_enforces_wikilink_validation_on_response`: extracted the WikiLink error hint to `support::WIKILINK_ERROR_HINT`.
- Verification: `cargo test --test integration_tests workflow_supervisor_registration` (run from `crates/maenifold-core`, because this repo consolidates integration tests into the `integration_tests` binary).
- Commit: bfbbbf6
