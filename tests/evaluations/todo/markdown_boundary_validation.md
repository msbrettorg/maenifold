# Test Evaluation: markdown_boundary_validation

**File:** `crates/maenifold-core/tests/markdown_boundary_validation.rs`
**Test count:** 10
**Evaluated:** 2025-07-15
**Traceability:** T-12.7, T-W1-2, NFR-1, NFR-2, FR-28.9

## Summary

All 10 tests guard the same security invariant — URI validation rejects malformed or malicious inputs — but do so against 10 different entry points. The assertion quality is sound (`assert_invalid_argument` checks both error variant and message fragment). The tests are individually defensible from a security standpoint but are structurally redundant: 7 test path traversal, 2 test missing-prefix, 1 tests null URI. This file should become a single table-driven security sweep.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `fr28_9_ac1_read_markdown_traversal_uri_throws_invalid_argument` | **KEEP** | Load-bearing: `read_markdown` is a primary entry point. Path traversal on read is the canonical attack. Name is verbose but traceable to FR-28.9-AC1. If validation is ever refactored out of this function, this test is the canary. |
| `fr28_9_ac2_write_markdown_no_memory_prefix_throws_invalid_argument` | **KEEP** | Load-bearing: unique coverage of "missing scheme" vs "traversal" — different validation branch. Verifies the prefix check fires before any filesystem access. |
| `fr28_9_ac3_read_markdown_empty_uri_throws_invalid_argument` | **KEEP** | Unique: tests the null/empty guard specifically. Not redundant with traversal — an empty string might slip past a traversal check that looks for `..` sequences. Message fragment `"null or empty"` is specific enough to be meaningful. |
| `fr28_9_ac4_file_exists_traversal_uri_throws_invalid_argument` | **MERGE** | `file_exists` is used internally; traversal protection here is only needed if `file_exists` is reachable from a tool invocation without prior URI validation. Merge into a parameterized traversal sweep (see Recommendations). The function still warrants a traversal test, but not as a standalone 20-line test. |
| `fr28_9_ac5_delete_file_traversal_uri_throws_invalid_argument` | **KEEP** | Load-bearing independently: `delete_file` is a destructive operation. If traversal validation is ever stripped from this path (e.g., during a refactor that centralizes validation), the consequences are severe. Worth the explicit standalone test. |
| `fr28_9_ac6_get_memory_files_traversal_folder_uri_throws_invalid_argument` | **MERGE** | The `folder_uri` argument is optional and passes through the same `uri_to_path` validation gate as all other operations. This test does not probe a distinct code path. Merge into parameterized suite. |
| `fr28_9_ac7_write_markdown_null_uri_throws_invalid_argument` | **REWRITE** | This test calls `write_markdown_boundary(None, ...)` — a test-support adapter function defined in `support.rs`, **not** the production `markdown::write_markdown`. It tests that the test adapter correctly converts `None` to `MaenifoldError::InvalidArgument`. This is testing test infrastructure, not the production null-URI guard. Either: (a) add a null-URI test directly against the production API (if Rust's type system doesn't prevent it), or (b) acknowledge that `Option<&str>` callers are already forced to handle `None` by the compiler and remove this test. Currently it provides false confidence. |
| `fr28_9_ac8_update_markdown_traversal_uri_throws_invalid_argument` | **KEEP** | Load-bearing: `update_markdown` is a write operation with checksum logic. Traversal here bypasses the checksum check entirely. The risk profile is different from a read traversal — an attacker could overwrite arbitrary files. Keep as standalone. |
| `fr28_9_ac9_append_h2_section_no_prefix_throws_invalid_argument` | **KEEP** | Unique coverage: tests the missing-prefix guard specifically on `append_h2_section`. Combined with `ac2`, these confirm the prefix check is applied uniformly to all write-type operations, not just `write_markdown`. |
| `fr28_9_ac10_get_memory_directories_traversal_folder_uri_throws_invalid_argument` | **MERGE** | `get_memory_directories` is a support wrapper that calls `get_memory_files` internally (see `support.rs`). Traversal is blocked at the `get_memory_files` level (already covered by `ac6`). This test is redundant with `ac6` once the call chain is understood. Merge into parameterized suite. |

## Recommendations

1. **Parameterize the traversal sweep.** Seven tests all check `assert_invalid_argument(err, "path traversal")` against different functions. Replace with a macro or `#[test_case]`-style table:
   ```rust
   // (operation_name, closure_that_invokes_it)
   let cases = [
       ("read_markdown",            || markdown::read_markdown("memory://../escape", &cfg)),
       ("file_exists",              || markdown::file_exists("memory://../escape", &cfg)),
       ("delete_file",              || markdown::delete_file("memory://../escape", &cfg)),
       ("get_memory_files",         || markdown::get_memory_files(&cfg, Some("memory://../escape"))),
       ("update_markdown",          || markdown::update_markdown("memory://../escape", None, "", None, &cfg)),
       ("get_memory_directories",   || get_memory_directories(&cfg, Some("memory://../escape"))),
   ];
   for (name, op) in cases {
       let err = op().expect_err(name);
       assert_invalid_argument(err, "path traversal");
   }
   ```
   This is 6 asserted behaviors in ~10 lines instead of 6 × 20 = 120 lines, and adding a new operation to the security sweep is a one-liner.

2. **Fix the null-URI test (ac7).** Either test the production API directly or delete it. Asserting that a test helper converts `None` to an error is not a meaningful security test.

3. **Verify validation is centralized or per-function.** The tests assume (but never prove) that each operation has its own traversal guard. If `uri_to_path` is called by all of them, a single test of `uri_to_path` + one integration check per operation would be more honest. If traversal guards are *not* centralized (each function does its own check), the current per-function tests are exactly right — but add a comment saying so.

4. **Add a test for Unicode normalization traversal.** The traversal check likely looks for `..` sequences. Does it catch `..%2F`, `%2e%2e`, or Unicode-normalized equivalents? No test currently probes this. Add at minimum one test for URL-encoded traversal: `memory://%2e%2e/escape`.

5. **Consider naming convention.** The `fr28_9_acN_` prefix embeds an RTM reference in the test name, which is valuable for traceability but makes test output noisy. Consider a companion test-name comment convention instead so the function names are human-readable: `fn write_traversal_uri_is_rejected()`.

## Load-Bearing Score: 7/10

Tests ac1, ac2, ac3, ac5, ac8, ac9 are load-bearing against distinct operations or distinct validation branches. Test ac7 tests a test adapter, not production. Tests ac4, ac6, ac10 overlap with each other or with tested callee functions.

---
## SWE Remediation
- MERGE ac4+ac6+ac10: collapsed into `fr28_9_traversal_sweep_secondary_operations` — single test covers file_exists, get_memory_files, get_memory_directories traversal rejection (Rec 1)
- REWRITE ac7: replaced test-adapter call (`write_markdown_boundary(None, ...)`) with production `markdown::write_markdown("", ...)` — now validates empty-URI guard on the real API (Rec 2)
- ADD `fr28_9_url_encoded_traversal_is_rejected`: confirms `%2e%2e` URL-encoded traversal is caught by path-level dot-sequence validation (Rec 4). Discovered: URL-encoded traversal returns `MaenifoldError::Message` (path-level check) not `InvalidArgument` (URI-level check) — two-layer defense confirmed.
- ADD comment documenting `uri_to_path` centralized validation gate (Rec 3)
- KEEP ac1, ac2, ac3, ac5, ac8, ac9: unchanged
- Test count: 10 → 9 (3 merged into 1, 1 rewritten, 1 new)
- All 9 pass
- Commit: e885675
