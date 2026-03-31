---
id: markdown_rw_read_tests
status: done
---

# Test Evaluation: markdown_rw_read_tests
**File:** `crates/maenifold-core/tests/markdown_rw_read_tests.rs`
**Test count:** 8
**Evaluated:** 2026-07-10
**Traceability:** T-14.2 (RTM FR-10), T-14.3 (RTM FR-10), T-15.17 (RTM NFR-1), T-15.18 (RTM NFR-1)

---

## Summary

All eight tests are load-bearing, each covering a distinct behavioral case in `read_markdown` or `append_h2_section`. Assertions are behavior/value-oriented throughout (return type structure, exact content strings, error propagation). The BOM regression guard (`read_markdown_without_bom_unchanged`) and the valid-frontmatter sanity test (`read_markdown_valid_frontmatter_still_parses_correctly`) could be questioned as redundant with the primary tests, but both provide meaningful safety nets against over-correction during maintenance. The most interesting structural observation: tests 1–4 (frontmatter parsing) and tests 7–8 (BOM) are internally parameterizable without coverage loss.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `read_markdown_malformed_frontmatter_returns_empty_frontmatter` | **KEEP** | Load-bearing error path: asserts `frontmatter.is_none()` and that body content is still returned despite YAML parse failure. If graceful degradation were removed, the function would either panic or return an error — this test is the only guard for that specific failure mode. |
| `read_markdown_crlf_frontmatter_parses_correctly` | **KEEP** | Load-bearing cross-platform correctness: the CRLF `\r\n` delimiter case is a distinct code path from LF `\n`. Asserts both parsed title and verbatim content (including `\r\n`). Without this, a CRLF regression in the delimiter scanner would be invisible. |
| `read_markdown_closing_delimiter_at_eof_parses_frontmatter` | **KEEP** | Load-bearing edge case: `---\ntitle: EOF Note\n---` (no trailing newline after closing `---`) is a valid format that many editors produce. Asserts `content.is_empty()`, which is the correct semantic answer. Fails if the parser requires `\n` after the closing delimiter. |
| `read_markdown_valid_frontmatter_still_parses_correctly` | **KEEP** | Regression guard for the happy path after adding error-handling for malformed/CRLF/EOF variants. Asserts two distinct scalar fields (`title`, `status`) and exact body content. Skeptical challenge: *would removing this test allow a regression to ship?* Yes — a bug introduced while handling malformed YAML that accidentally corrupted the valid-YAML branch would not be caught by test 1 alone. |
| `append_h2_section_propagates_permission_error` | **KEEP** | Load-bearing error propagation (`#[cfg(unix)]`): asserts `result.is_err()` and that the error message references permission denial. If `append_h2_section` silently swallowed IO errors, this would be the only test to catch it. The assertion `err_msg.contains("ermission denied")` is deliberately case-tolerant (covers OS message variance) — this is correct, not sloppy. |
| `append_h2_section_not_found_defaults_to_empty` | **KEEP** | Load-bearing behavior: verifies create-on-write semantics for nonexistent files. Asserts both the section heading and body text are present in the created file. Unique: no other test in this file exercises the file-not-found path for `append_h2_section`. |
| `read_markdown_strips_utf8_bom` | **KEEP** | Load-bearing for T-15.18 (C# oracle parity): a BOM-prefixed file would have broken frontmatter parsing before this fix. Asserts parsed title AND body content. If the `strip_prefix('\u{FEFF}')` line were removed, this test catches the regression within one commit. |
| `read_markdown_without_bom_unchanged` | **KEEP** | Regression guard: ensures the BOM-stripping `strip_prefix` does not accidentally strip the first character of non-BOM files. Skeptical challenge: *could this ever fail given `str::strip_prefix`'s contract?* In theory, no — but the test documents intent and guards against future refactors that change the stripping strategy (e.g., a regex-based approach that could over-match). Costs nothing to keep. |

---

## Recommendations

1. **Consider parameterizing tests 1–4** (frontmatter parsing suite): All four tests write a fixture, call `read_markdown`, and assert on `(frontmatter, content)`. A table-driven harness with rows for `(fixture_content, expected_frontmatter_keys, expected_content)` would make it trivial to add new edge cases (e.g., frontmatter with Unicode characters, nested YAML structures) without copy-paste boilerplate.

2. **Consider parameterizing tests 7–8** (BOM handling): Two rows — `(with_bom, without_bom)` — with the same assertion pattern. Low priority since the file only has two cases, but worth noting the structure is identical.

3. **Add a negative test for `append_h2_section_propagates_permission_error` on non-Unix platforms**: Currently `#[cfg(unix)]` gates this test, meaning Windows CI has no coverage of `append_h2_section` error propagation. At minimum, document why no Windows equivalent exists or use a different error-induction strategy (e.g., write to a read-only in-memory path).

4. **Strengthen `read_markdown_closing_delimiter_at_eof_parses_frontmatter`**: Currently only asserts `content.is_empty()`. Consider adding `assert_eq!(frontmatter.get("title"), Some("EOF Note"))` using the `scalar` helper — the test already imports it. The title assertion would make it a complete behavioral check.

---

## Load-Bearing Score: 8/8

> All eight tests protect against distinct, real failure modes. No test is ceremonial or testing only labels/format. The regression guards (tests 4 and 8) earn their place by protecting against maintenance-induced over-corrections.

---
## SWE Remediation
- All 8 tests: KEEP — no changes required, all load-bearing
- R1 (parameterize tests 1–4): Skipped — P3+ "Consider" level, medium refactor effort
- R2 (parameterize tests 7–8): Skipped — P3+ "Consider" + explicitly "Low priority"
- R3 (non-Unix permission test): Skipped — P3+ "Consider", platform-specific
- R4 (strengthen EOF delimiter test): Already addressed — `assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("EOF Note"))` is present at line 63
- Tests verified: `cargo test --test integration_tests -p maenifold-core -- markdown_rw_read_tests` → 8 passed, 0 failed
- Commit: 43f16a5

---
## Red-team Audit
- ✅ KEEP `read_markdown_malformed_frontmatter_returns_empty_frontmatter`: present, unchanged, passes
- ✅ KEEP `read_markdown_crlf_frontmatter_parses_correctly`: present, unchanged, passes
- ✅ KEEP `read_markdown_closing_delimiter_at_eof_parses_frontmatter`: present, unchanged, passes
- ✅ KEEP `read_markdown_valid_frontmatter_still_parses_correctly`: present, unchanged, passes
- ✅ KEEP `append_h2_section_propagates_permission_error`: present, unchanged, passes
- ✅ KEEP `append_h2_section_not_found_defaults_to_empty`: present, unchanged, passes
- ✅ KEEP `read_markdown_strips_utf8_bom`: present, unchanged, passes
- ✅ KEEP `read_markdown_without_bom_unchanged`: present, unchanged, passes
- ✅ R4 "Already addressed": confirmed — line 63 contains `assert_eq!(scalar(&frontmatter, "title").as_deref(), Some("EOF Note"))`, matching the eval recommendation exactly
- ✅ R1–R3 skipped: appropriate — all are P3+ "Consider"-level recommendations, not mandatory
- ✅ No unrelated changes: commit 43f16a5 only appended SWE remediation to eval spec; `git diff` on `crates/` is empty
- ✅ Tests verified independently: 8 passed, 0 failed, 910 filtered out
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_read`: 8 passed, 0 failed (909 filtered out)
- All 8 test functions confirmed:
  - `read_markdown_malformed_frontmatter_returns_empty_frontmatter` — ok
  - `read_markdown_crlf_frontmatter_parses_correctly` — ok
  - `read_markdown_closing_delimiter_at_eof_parses_frontmatter` — ok
  - `read_markdown_valid_frontmatter_still_parses_correctly` — ok
  - `append_h2_section_propagates_permission_error` — ok
  - `append_h2_section_not_found_defaults_to_empty` — ok
  - `read_markdown_strips_utf8_bom` — ok
  - `read_markdown_without_bom_unchanged` — ok
- Traceability: T-14.2, T-14.3 (RTM FR-10), T-15.17, T-15.18 (RTM NFR-1)
- Verdict: **PASS**
