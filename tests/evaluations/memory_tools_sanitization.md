# Test Evaluation: memory_tools_sanitization
**File:** `crates/maenifold-core/tests/memory_tools_sanitization.rs`
**Test count:** 8
**Evaluated:** 2026-07-10
**Traceability:** T-12.6 (RTM NFR-1), T-14.8 (RTM FR-14)

---

## Summary

Seven of eight tests are load-bearing security and boundary guards that would allow real bugs to ship if removed. The exception is `sanitize_user_input_preserves_legitimate_percent_signs`, which has a **critically wrong name** — its title contains no percent sign, so it tests nothing about percent preservation. The four URL-encoding traversal tests (tests 1–4) cover distinct attack vectors against the same `sanitize_user_input` → `slugify` pipeline and could be collapsed into a single parameterized test without coverage loss.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `sanitize_user_input_blocks_url_encoded_path_traversal` | **KEEP** | Load-bearing: exercises the `urlencoding::decode` → `..` strip → `/` strip pipeline end-to-end. Asserts the decoded URI contains "testmalicious" (not just absence of `..`), proving the slug is correct. Unique: URL-encoded slash variant. |
| `sanitize_user_input_blocks_double_url_encoded_path_traversal` | **KEEP** | Load-bearing: double-encoded `%252e%252e%252f` is a real bypass technique. Correctly verifies one-pass decode is applied (second `%2e` is not re-decoded into `.`). Assertions are minimal (only `!result.contains("..")`) — **consider adding a URI value assertion** to prove what slug was actually produced. |
| `sanitize_user_input_blocks_url_encoded_dot_dot` | **KEEP** | Unique: tests `%2e%2e` without a slash component — distinct from test 1. If the slash strip were removed from the implementation, test 1 would still pass while test 3 catches `..` reassembly. Both are needed. |
| `sanitize_user_input_blocks_mixed_encoded_and_plain_path_traversal` | **KEEP** | Unique: `%2e./test` mixes encoded and literal characters. The decode step turns `%2e` → `.`, producing `../test`, which then hits the `..` stripper. Catches a distinct partial-encoding bypass vector. |
| `sanitize_user_input_preserves_legitimate_percent_signs` | **REWRITE** | **Naming lie**: the title `"Task 50 Complete"` contains zero percent signs. The test name claims to guard against over-stripping `%`, but the fixture data cannot exercise that path. This test is actually a basic write/read roundtrip for a clean title — valuable as a regression guard, but it should either (a) use a title like `"Task 50% Complete"` and assert `%` is preserved in content (not slug), or (b) be renamed `write_memory_roundtrip_plain_title`. Currently it provides false confidence that percent sign preservation is tested. |
| `write_memory_truncates_overlong_slug_but_preserves_title_metadata` | **KEEP** | Load-bearing for T-14.8: verifies `.chars().take(255)` truncation AND that the original 256-char title survives in frontmatter. Uniquely reads the raw file byte-for-byte. If either the truncation or frontmatter preservation regressed, only this test would catch it. |
| `write_memory_keeps_normal_slug_unchanged` | **KEEP** | Regression guard: proves `take(255)` on a 12-char slug doesn't corrupt it. Without this, a bug that always truncates to 255 chars would not be caught by the boundary test alone. |
| `write_memory_keeps_exact_255_char_slug_unchanged` | **KEEP** | Load-bearing boundary test for T-14.8: if the implementation used `< 255` instead of `<= 255` (or `take(254)` instead of `take(255)`), this is the only test that fails. True fence-post guard. |

---

## Recommendations

1. **Fix or rewrite `sanitize_user_input_preserves_legitimate_percent_signs`** (high priority): Change the title to `"Task 50% Complete"` and assert the `%` character is preserved in the *content* body (it is never URL-decoded in the body, only in the title/slug pipeline). The slug `task-50-complete` would still be slug-safe. This turns a mislabeled no-op into a real regression guard against over-aggressive URL decoding.

2. **Strengthen `sanitize_user_input_blocks_double_url_encoded_path_traversal`**: Add an assertion on the actual URI produced (e.g., `assert!(uri.contains("test"), "slug should contain 'test'")`). The current `!result.contains("..")` assertion does not distinguish between "correctly sanitized" and "sanitized in a way that corrupts the title completely."

3. **Consider parameterizing tests 1–4** (low urgency): The four URL-encoding tests share an identical structure — `isolated_env()`, `write_memory(...)`, `assert!(!result.contains(".."))`, optional read-back. A `#[test_case]`-style table would reduce duplication while preserving the distinct attack vectors. Each row: `(title_input, expected_slug_fragment)`.

4. **Consider parameterizing tests 6–8** (low urgency): Three tests for slug length `[256, 12, 255]` with expected truncated lengths `[255, 12, 255]` could collapse into one table-driven test, making the truncation boundary logic explicit.

---

## Load-Bearing Score: 7/8

> Test 5 (`preserves_legitimate_percent_signs`) does not load-bear the claim in its name. It provides some write/read roundtrip value but zero value for the percentage-sign preservation story it advertises.
