# Test Evaluation: session_assumption_crud
**File:** `crates/maenifold-core/tests/session_assumption_crud.rs`
**Test count:** 6
**Evaluated:** 2026-03-14
**Traceability:** T-12.7, RTM NFR-1

---

## Summary

The file is named "CRUD" but contains no Delete test — the D is structurally absent. The six tests that do exist cover distinct behaviors (create with full payload, two input-validation guards, read round-trip, update with status change, update with partial payload), and all six are load-bearing in the sense that each exercises a code path not covered by the others in this file. However, two tests have correctness gaps that would allow real bugs to ship silently: `read_assumption_returns_correct_content` checks that `**Created**:` and `**Modified**:` labels exist but never validates their values, and `update_assumption_persists_validation_plan` fails to assert that the `status` field was not accidentally reset during a partial update. The create test contains a redundant file-count side-effect assertion that is fully superseded by the URI-based frontmatter read below it, adding maintenance weight without adding coverage. The two input-validation tests (`requires_concepts`, `rejects_concepts_with_brackets`) are candidates for merging into a parameterized helper.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `append_assumption_creates_file_with_correct_structure` | REWRITE | Load-bearing. The frontmatter round-trip and WikiLink assertions are correct. But the `files_before + 1` file-count check is redundant once the URI-based `harness.read()` confirms the file exists and is parseable. More critically, `assert!(result.contains("💡 Next: Consider running Sync()"))` asserts display copy verbatim — one rewording in the tool response silently breaks this test while the behavior stays correct. Remove the file-count side effect and loosen the hint assertion. |
| `append_assumption_requires_concepts` | MERGE | Load-bearing validation guard. Tests `concepts: []` (empty array). Does **not** test completely absent `concepts` field. Structurally identical to `append_assumption_rejects_concepts_with_brackets` — both invoke append with a malformed concepts value and assert on an `ERROR:` prefix. Merge into a parameterized validation table with `rejects_concepts_with_brackets`. |
| `append_assumption_rejects_concepts_with_brackets` | MERGE | Load-bearing validation guard. Tests that pre-wrapped `[[concept]]` strings are rejected. Paired directly with `requires_concepts` — same action, different malformed input, same assertion shape. Combine both into one parameterized test (or a shared helper called twice) to eliminate duplication without losing coverage. |
| `read_assumption_returns_correct_content` | REWRITE | Load-bearing. The assertions on `**Validation Plan**`, assumption text, and `[[test-concept]]` WikiLink are correct. But `assert!(result.contains("**Created**:"))` and `assert!(result.contains("**Modified**:"))` only confirm the label is printed — they would pass if the value were empty, `None`, or a formatting error. Neither assertion checks that Created equals Modified on a fresh assumption (they should), nor that the values are valid ISO 8601 timestamps. Validate the actual date values or use `harness.read()` to inspect frontmatter directly. |
| `update_assumption_changes_status` | KEEP | Load-bearing. Verifies the update action changes status from `active` to `validated`, that the update response confirms the new status, and that the change persists through a read round-trip. Also verifies `notes` content appears in the read output. Flag: no test for an invalid status value (e.g. `"banana"`) — that gap belongs in `session_assumption_misc.rs` but is not present there either. |
| `update_assumption_persists_validation_plan` | REWRITE | Load-bearing scenario (partial update without status). But contains a critical assertion gap: after the update, the test only verifies `**Validation Plan**` in the read result. It never asserts that `**Status**` remained `"active"`. If the update handler accidentally resets status to a default, this test passes silently. Add `assert!(read_result.contains("**Status**: active"))` before the validation plan check. |

---

## Recommendations

1. **REWRITE `append_assumption_creates_file_with_correct_structure`**: Drop the `files_after == files_before + 1` assertion — file creation is already proven by the `harness.read(&uri)` call that would panic on a missing file. Replace `result.contains("💡 Next: Consider running Sync()")` with something semantically stable, such as `!result.contains("ERROR")` or a check that the returned URI is a valid `memory://assumptions/` path. The frontmatter and WikiLink assertions are correct; keep them.

2. **MERGE `append_assumption_requires_concepts` + `append_assumption_rejects_concepts_with_brackets`**: Extract an `assert_append_validation_error(concepts_value, expected_error_prefix)` helper. Call it with `json!([])` and `json!(["[[dialogue]]"])`. This reduces duplication to zero while maintaining both coverage points.

3. **REWRITE `read_assumption_returns_correct_content`**: Replace the weak label-presence assertions with value validation. Two options: (a) use `harness.read(&uri)` to inspect frontmatter and assert `created_at` is a non-empty string in ISO format; or (b) call `invoke_markdown` for read in `OutputFormat::Json` and parse the structured fields. At minimum, assert `Created == Modified` on a freshly-appended assumption (no update has occurred).

4. **REWRITE `update_assumption_persists_validation_plan`**: Add `assert!(read_result.contains("**Status**: active"))` after `update_result.starts_with(...)`. A partial update that silently resets status would currently pass this test undetected.

5. **Address the missing D**: This file is named `session_assumption_crud.rs` but has no test for a delete or "deactivate" operation. If AssumptionLedger intentionally has no delete action, rename the file `session_assumption_create_read_update.rs` and document the omission. If a soft-delete or archive operation exists or is planned, add a test here.

6. **Flag the absent negative-path tests for `read` and `update`**: No test in this file passes an invalid or nonexistent URI to `read` or `update`. This is covered in `session_assumption_misc.rs` (`read_non_existent_assumption_returns_error`, `update_non_existent_assumption_returns_error`) — confirm the coverage is intentionally split and document the division.

---

## Load-Bearing Score: 5/6

`append_assumption_requires_concepts` is the only test whose coverage is exactly duplicated by a sibling in this same file (`rejects_concepts_with_brackets`). Both test input validation for the concepts field with the same assertion shape. Merging them loses zero coverage. All other five tests exercise distinct code paths (full create pipeline, bracket-format guard, read action, status-change update, partial-field update) and cannot be removed without creating a coverage gap.
