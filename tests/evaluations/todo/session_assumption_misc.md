# Test Evaluation: session_assumption_misc
**File:** `crates/maenifold-core/tests/session_assumption_misc.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-12.7, RTM NFR-1

---

## Summary

Six of the seven tests exercise real behavior and are worth keeping, but two rely on emoji-prefixed and markdown-bold string assertions that are one cosmetic rename away from going silent. One test (`update_non_existent_assumption_returns_error`) is redundant with `read_non_existent_assumption_returns_error` — both assert the identical error prefix against the identical hardcoded URI, covering the same code path for different operations that likely share the same `find_assumption` helper.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `update_assumption_rejects_invalid_status` | KEEP | Validates enum rejection at the boundary. Both assertions check real contract: error prefix + enumerated valid values. Format-sensitive but the error API surface IS the contract here. |
| `update_assumption_adds_timestamped_notes` | REWRITE | The `assert!(update_result.starts_with("✅ Assumption updated at"))` assertion tests emoji and exact prefix wording, not behavior. The round-trip persistence assertion (`## Update:` and note content) is load-bearing and correct. The update response check should be loosened to verify success semantics without hardcoding the emoji. |
| `read_non_existent_assumption_returns_error` | KEEP | Validates error handling for missing read. Hardcoded `2025/01/assumption-nonexistent` URI is intentionally nonexistent — this is fine. |
| `update_non_existent_assumption_returns_error` | MERGE with `read_non_existent_assumption_returns_error` | Identical hardcoded URI, identical error prefix (`ERROR: Assumption not found`), same underlying `find_assumption` lookup path. Unique only in the `action` field. Collapse into a single parameterized helper or table test covering both operations. |
| `invalid_action_returns_error` | KEEP | Tests rejection of an unrecognized action string. The assertion also verifies the valid options are listed in the error — acceptable API-contract checking. |
| `assumption_file_uses_correct_date_based_path` | KEEP | The strongest test in the file. Verifies filesystem structure (file actually created, count incremented) AND URI date path format. Behavioral assertions, not just string labels. |
| `update_assumption_changes_confidence` | REWRITE | `assert!(update_result.contains("**New Confidence**: high"))` asserts a markdown-bold fragment, not a semantic outcome. If the response drops the bold markers or renames the field label, this silently breaks. The read-back assertion (`**Confidence**: high`) has the same problem. Should assert confidence value was persisted by reading the frontmatter or a structured field, not by pattern-matching markdown bold syntax. |

---

## Recommendations

1. **REWRITE `update_assumption_adds_timestamped_notes`**: Replace `starts_with("✅ ...")` with `!result.contains("ERROR")` or match on a structured success indicator. The behavioral proof is already in the read-back; the update response check adds noise, not signal.

2. **REWRITE `update_assumption_changes_confidence`**: Use `OutputFormat::Json` for the read-back, parse the confidence field from structured output, and assert the value directly. Markdown bold syntax is an output formatting detail, not a behavioral contract.

3. **MERGE `read_non_existent_assumption_returns_error` + `update_non_existent_assumption_returns_error`**: Extract a `assert_missing_uri_returns_not_found(action, uri)` helper and call it for both `"read"` and `"update"`. Two identical error lookups for the same nonexistent URI add maintenance surface without adding coverage.

4. **Consider testing `append` with missing required fields**: No test in this file covers what happens when `concepts` or `assumption` is omitted from an append. The `session_assumption_validate_append.rs` file likely handles this — confirm coverage is not duplicated here.

---

## Load-Bearing Score: 6/7

`update_non_existent_assumption_returns_error` is the one test whose removal would not allow a new bug to ship undetected — `read_non_existent_assumption_returns_error` already exercises the same `find_assumption` failure path.
