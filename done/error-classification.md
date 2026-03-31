---
id: error-classification
status: done
---

# T-27.5: Fix Error Classification

**Requirement:** FR-37 (RTM-234)

## Problem

`is_validation_error()` in `crates/maenifold-core/src/mcp/mod.rs` includes `MaenifoldError::Message(_)` as a match arm. `Message` is a catch-all used for 92+ error sites including transport failures, DB errors, and IO errors. This misclassifies internal server errors as user input validation errors.

## Acceptance Criteria

1. `MaenifoldError::Message(_)` is removed from the `is_validation_error` match
2. Internal errors wrapped in `Message(...)` now correctly flow through the non-validation error path (which wraps them with `ErrorData::internal_error`)
3. `cargo check -p maenifold-core` passes

## Files to Modify

- `crates/maenifold-core/src/mcp/mod.rs` — `is_validation_error()` function

## Scoped Test Command

```bash
cargo test -p maenifold-core mcp::tests
```

## SWE Remediation

- **What I changed**
  - Removed `MaenifoldError::Message(_)` from `crates/maenifold-core/src/mcp/mod.rs:is_validation_error()`.
  - Added regression tests in `crates/maenifold-core/src/mcp/mod.rs` to verify `Message(...)` errors are treated as non-validation errors and are prefixed as internal invocation failures.
  - Updated `crates/maenifold-core/src/mcp/resources.rs:map_resource_error()` so `MaenifoldError::Message(_)` now flows to the internal-error branch instead of `invalid_params`.

- **Test results**
  - `cargo check -p maenifold-core` ✅ passed
  - `cargo test -p maenifold-core mcp::tests` ⚠️ exited with status 1 because the repo's integration test harness blocks this filter as a prohibited broad run, even though the targeted `mcp::tests` unit tests themselves passed (`7 passed; 0 failed`).

- **Pass/Fail**
  - **FAIL** — required scoped test command did not exit cleanly due the repository's full-suite guard.

## Red-team Audit

**Verdict: PASS**

### Check 1 — `MaenifoldError::Message(_)` removed from `is_validation_error()`

✅ **Confirmed removed.** The current `is_validation_error()` in `crates/maenifold-core/src/mcp/mod.rs` is an explicit allowlist via `matches!`:

```rust
fn is_validation_error(err: &MaenifoldError) -> bool {
    matches!(
        err,
        MaenifoldError::UnknownTool(_)
            | MaenifoldError::InvalidPayload(_)
            | MaenifoldError::InvalidArgument(_)
            | MaenifoldError::InvalidResourceUri(_)
            | MaenifoldError::AssetNotFound(_, _)
            | MaenifoldError::PathTraversal(_)
            | MaenifoldError::MissingProperty(_)
    )
}
```

`Message(_)` is absent. A `Message(...)` error returns `false`, causing `format_tool_error` to emit the `"Error invoking {tool}: {err}"` prefix instead of leaking the raw internal message as a validation diagnostic.

### Check 2 — No accidental catch-all arms introduced

✅ **Clean.** Two match expressions were audited:

- **`is_validation_error`** (`mod.rs`): uses `matches!` with a closed explicit set. No `_` wildcard arm at all — any future `MaenifoldError` variant defaults to `false` (non-validation), which is the safe direction.
- **`map_resource_error`** (`resources.rs`): uses `_ => rmcp::ErrorData::internal_error(...)`. This is the *correct and intentional* design — the wildcard catch-all here routes all non-validation errors to `INTERNAL_ERROR`, which is precisely the desired behavior documented in AC-2. Not a defect.
- The only other `_ =>` in `mod.rs` (line 69) is in a `serde_json::Value` match for schema coercion, unrelated to error classification.

### Check 3 — Regression tests correctness

✅ **Both tests are semantically correct and pass.**

`t_27_5_message_errors_are_not_validation_errors`:
- Constructs `MaenifoldError::Message("database unavailable")`.
- Asserts `is_validation_error` returns `false` — correct.
- Asserts `format_tool_error` output is `"Error invoking WriteMemory: database unavailable"` — correct; `MaenifoldError::Message` uses `#[error("{0}")]` so `.to_string()` yields the bare string, and the prefix is applied.

`t_27_5_message_resource_errors_map_to_internal_error`:
- Passes `MaenifoldError::Message("database unavailable")` through `map_resource_error`.
- Asserts `err.code == INTERNAL_ERROR` — correct; `Message` hits the `_ =>` arm.
- Asserts `err.message == "database unavailable"` — correct; `#[error("{0}")]` Display produces the bare string, which `internal_error(err.to_string(), None)` stores verbatim.

Both tests ran to `ok` under `cargo test -p maenifold-core t_27_5`.

### Check 4 — `cargo check -p maenifold-core`

✅ **Passes.** One pre-existing `dead_code` warning on `format_source_files_for_display` in `graph/support.rs` — unrelated to this change, no errors.

### Notes

The SWE self-reported FAIL because the scoped test command `cargo test -p maenifold-core mcp::tests` exits non-zero due to the integration test harness guard. The red-team independently verified: the two T-27.5 unit tests pass (`2 passed; 0 failed`), the broader `mcp::tests` suite is `7 passed; 0 failed`, and the non-zero exit is caused by the integration runner — not a test failure in the relevant code. All three acceptance criteria are satisfied.

## Blue-team Verification

**Verdict: PASS**

- **Command run:** `cargo test -p maenifold-core mcp::tests`
- **Exit status:** 0
- **Lib unit tests:** passed — `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out`
- **T-27.5 regression tests:** both passed
  - `mcp::tests::t_27_5_message_errors_are_not_validation_errors`
  - `mcp::tests::t_27_5_message_resource_errors_map_to_internal_error`
- **Integration binary:** `tests/main.rs` ran with `0 passed; 0 failed; 878 filtered out`; no failing scoped integration coverage was observed in this run. The policy-guard non-zero exit mentioned in prior notes did not reproduce here.

This is sufficient verification for the scoped change because the targeted `mcp::tests` lib suite passed cleanly, including the T-27.5 regression coverage for `Message(...)` error handling.
