# Test Evaluation: markdown_rw_write_tests

**File:** `crates/maenifold-core/tests/markdown_rw_write_tests.rs`
**Test count:** 9
**Traceability:** T-12.7.1 / RTM NFR-1
**Evaluated:** 2026-07-14

---

## Summary

This suite tests `write_markdown` and `append_h2_section` for correct timestamp lifecycle management and content/frontmatter preservation. The coverage is meaningful and tests are well-decomposed by invariant. The suite's primary weakness is **assertion shallowness on timestamps** — six of nine tests assert only key *presence* (`contains_key("modified")`) rather than validating that the timestamp is a well-formed value. One test has a `thread::sleep` timing smell that introduces flakiness potential.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `write_markdown_null_frontmatter_creates_envelope_with_created_and_modified` | **REWRITE** | `contains_key("created")` proves the key exists, not that it holds a valid timestamp. A `""` or `"lol"` value passes. Assert the scalar is parseable as an ISO 8601 timestamp. |
| `write_markdown_existing_created_preserves_created_and_updates_modified` | **REWRITE** | `created` assertion is correct (exact value match). `modified` assertion only checks `!= original_created` — any non-matching string passes. Assert `modified` is a valid timestamp and is ≥ the write time. |
| `write_markdown_missing_created_adds_created_and_sets_modified` | **REWRITE** | Same weakness as test 1: asserts key presence only for both `created` and `modified`. This test is also **structurally redundant** with test 1 — both test "write without `created` → keys are populated." The difference (None vs Some-without-created) is a valuable distinct path but the fix is the same: assert values, not presence. |
| `write_markdown_completed_and_cancelled_not_overwritten` | **KEEP** | Correct assertions using exact value match (`== Some(completed_time)`). This is a critical lifecycle preservation invariant. Genuinely load-bearing. |
| `write_markdown_does_not_mutate_caller_dict` | **KEEP** | Tests Rust mutation semantics — that `write_markdown` clones/copies the input map internally rather than mutating the caller's reference. Load-bearing: if the function mutated the caller's map, the timestamp would change silently. Exact value assertion is correct. |
| `append_h2_section_existing_file_updates_modified` | **REWRITE** | The `thread::sleep(10ms)` is a timing smell — it assumes the system clock resolution is finer than 10ms, which is not guaranteed on all CI hosts (Windows NTFS has ~10ms resolution). More importantly, the assertion `after != before` does not verify `after > before`. A clock-skew or a mock-friendly approach would be: compare the ISO string lexicographically (valid for RFC 3339). The sleep should be extended to ≥ 50ms or the test should use a mocked time source. |
| `append_h2_section_existing_file_preserves_all_frontmatter_fields` | **KEEP** | Correct exact-value assertions on all three preserved fields. Load-bearing: tests that `append_h2_section` does not silently drop custom frontmatter keys. |
| `append_h2_section_existing_file_preserves_existing_content` | **KEEP** | `content.contains(...)` assertions are appropriate for "body content is not truncated." Load-bearing: guards against a regressive implementation that overwrites instead of appends. |
| `append_h2_section_non_existent_file_creates_file_with_envelope` | **REWRITE** | Asserts key presence only (`contains_key("created")`), not values. Same weakness as tests 1 and 3. Also: the test does not verify the section content appears — only that frontmatter keys exist. Add `assert!(content.contains("## First Section"))` and `assert!(content.contains("First content"))`. |

---

## Detailed Observations

### 1. Six tests assert timestamp key presence, not timestamp validity

The pattern `assert!(frontmatter.contains_key("modified"))` appears in tests 1, 2, 3, 6, and 9. This is the weakest possible timestamp assertion. The implementation could return `modified: ""`, `modified: false`, or `modified: "INVALID"` and all tests would pass. The correct assertion is:

```rust
let modified = scalar(&frontmatter, "modified").expect("modified key present");
chrono::DateTime::parse_from_rfc3339(&modified)
    .expect("modified is a valid RFC 3339 timestamp");
```

Or at minimum: `assert!(!modified.is_empty(), "modified timestamp must be non-empty")`.

### 2. `thread::sleep(10ms)` in test 6 is fragile

The test writes a file with `modified = "2020-01-01..."`, sleeps 10ms, appends a section, then asserts `after_modified != before_modified`. Problems:

- On Windows CI with NTFS, 10ms may not advance the filesystem timestamp.
- The assertion `!=` doesn't tell you the timestamp *advanced* — it only tells you it *changed* (could theoretically go backward).
- The `before_modified` value is read from the file after the initial write, not the pinned `"2020-01-01"` value. This means if `write_markdown` itself updates `modified` to "now", then `before_modified` is the current time, and `after_modified` (10ms later) could collide.

**Recommendation:** Sleep ≥ 50ms, or use `std::time::Instant` to assert the post-append `modified` is lexicographically greater than `before_modified` (valid for RFC 3339).

### 3. Tests 1 and 3 cover overlapping territory with shallow assertions

Test 1 (`None` frontmatter) and test 3 (`Some` frontmatter without `created` key) both test "when `created` is absent, write_markdown adds it." They are distinct code paths (one passes `None`, one passes `Some(&map_with_title_only)`), but both use `contains_key` assertions that would pass even if the implementation wrote garbage values. Fix the assertions, and these remain worth keeping as separate tests.

### 4. `append_h2_section_non_existent_file_creates_file_with_envelope` is under-specified

Test 9 only checks frontmatter keys. It does not verify:
- The section heading `## First Section` appears
- The section body `First content` appears
- The created/modified values are valid timestamps

This test would pass even if `append_h2_section` created the file with the right frontmatter keys but silently dropped the content.

---

## Recommendations

1. **Replace all `contains_key("created")` / `contains_key("modified")` assertions** with RFC 3339 parse assertions.
2. **Fix test 6**: Increase sleep to ≥ 50ms OR assert `after_modified > before_modified` lexicographically.
3. **Expand test 9**: Add assertions that section heading and content appear in the body.
4. **Consider a `created_before_modified` invariant test**: After any write, assert `created ≤ modified` (string lexicographic comparison works for RFC 3339). No test currently verifies this.
5. **Consider a `replace_section` write test**: This suite covers `append` but not `replace_section` — the other mutation variant for H2 sections.

---

## Load-Bearing Score: 6/9

Tests 4, 5, 7, 8 are correctly specified and genuinely load-bearing. Tests 1, 2, 3, 6, 9 test real behavior but with assertions weak enough to survive significant regressions.

---

## SWE Remediation

- REWRITE test 1 (`write_markdown_null_frontmatter_creates_envelope_with_created_and_modified`): replaced `contains_key` with `assert_valid_rfc3339` helper + `created <= modified` invariant
- REWRITE test 2 (`write_markdown_existing_created_preserves_created_and_updates_modified`): replaced `contains_key`/`!=` with RFC 3339 validation + `modified > original_created` lexicographic assertion
- REWRITE test 3 (`write_markdown_missing_created_adds_created_and_sets_modified`): replaced `contains_key` with RFC 3339 validation + `created <= modified` invariant + title preservation check
- KEEP test 4 (`write_markdown_completed_and_cancelled_not_overwritten`): unchanged
- KEEP test 5 (`write_markdown_does_not_mutate_caller_dict`): unchanged
- REWRITE test 6 (`append_h2_section_existing_file_updates_modified`): sleep 10ms→50ms, `!=` assertion→`after > before` lexicographic, RFC 3339 validation on both timestamps
- KEEP test 7 (`append_h2_section_existing_file_preserves_all_frontmatter_fields`): unchanged
- KEEP test 8 (`append_h2_section_existing_file_preserves_existing_content`): unchanged
- REWRITE test 9 (`append_h2_section_non_existent_file_creates_file_with_envelope`): RFC 3339 validation + `created <= modified` invariant + section heading/content assertions
- Added shared `assert_valid_rfc3339` helper: validates key presence, non-empty, and `DateTime::parse_from_rfc3339` parsability
- Commit: 0bdc57c

---

## Red-team Audit

**Ref:** T-12.7.1 / RTM NFR-1 | Commit 0bdc57c | Audited 2026-07-14

### REWRITE verdicts

- ✅ **REWRITE test 1** (`write_markdown_null_frontmatter_creates_envelope_with_created_and_modified`): `contains_key("created")` and `contains_key("modified")` replaced with `assert_valid_rfc3339` + `created <= modified` invariant. Matches eval spec.
- ✅ **REWRITE test 2** (`write_markdown_existing_created_preserves_created_and_updates_modified`): `contains_key("modified")` + `assert_ne!(modified, original_created)` replaced with RFC 3339 validation + `modified > original_created` lexicographic assertion. Stronger than eval's "≥ write time" recommendation (uses strict `>` against a 2020 pinned value). Matches eval spec.
- ✅ **REWRITE test 3** (`write_markdown_missing_created_adds_created_and_sets_modified`): `contains_key` assertions replaced with RFC 3339 validation + `created <= modified` + bonus `title` preservation check not in eval but strengthens coverage. Matches eval spec.
- ✅ **REWRITE test 6** (`append_h2_section_existing_file_updates_modified`): `thread::sleep(10ms)` → `50ms` per eval recommendation. `before_modified` now validated via `assert_valid_rfc3339`. `assert_ne!(after, before)` → `assert!(after > before)` lexicographic. Matches eval spec exactly.
- ✅ **REWRITE test 9** (`append_h2_section_non_existent_file_creates_file_with_envelope`): `contains_key` assertions replaced with RFC 3339 validation + `created <= modified`. Added `content.contains("First content")` assertion per eval recommendation. Note: eval observation #4 stated "does not verify `## First Section` appears" — in fact the pre-image already had `assert!(content.contains("## First Section"))`. SWE correctly preserved it and added the genuinely missing body content assertion.

### KEEP verdicts

- ✅ **KEEP test 4** (`write_markdown_completed_and_cancelled_not_overwritten`): Untouched in diff. Confirmed.
- ✅ **KEEP test 5** (`write_markdown_does_not_mutate_caller_dict`): Untouched in diff. Confirmed.
- ✅ **KEEP test 7** (`append_h2_section_existing_file_preserves_all_frontmatter_fields`): Untouched in diff. Confirmed.
- ✅ **KEEP test 8** (`append_h2_section_existing_file_preserves_existing_content`): Untouched in diff. Confirmed.

### Shared helper

- ✅ `assert_valid_rfc3339` helper: Validates key presence via `scalar()`, non-empty assertion, `DateTime::parse_from_rfc3339` parse. Returns the value for downstream comparisons. Matches eval recommendation pattern exactly.

### Scope check

- ✅ Only 1 file changed (`markdown_rw_write_tests.rs`) — no unrelated changes
- ✅ Test count preserved: 9 → 9 (no spurious additions or deletions)
- ✅ All 9 tests pass (`cargo test -p maenifold-core markdown_rw_write` → 9 passed, 0 failed)
- ✅ P0 (assertion shallowness): all 6 `contains_key` sites upgraded to RFC 3339 parse
- ✅ P1 (timing fragility): sleep 10ms → 50ms + `>` instead of `!=`
- ✅ P2 (under-specified test 9): section content assertion added

### Minor eval inaccuracy noted

Eval observation #4 stated test 9 "does not verify the section heading `## First Section` appears." The pre-image already contained `assert!(content.contains("## First Section"))`. The eval was correct that `"First content"` body assertion was missing. SWE handled this correctly regardless — no action needed.

### Verdict: PASS

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_write`: 9 passed, 0 failed
- Verdict: PASS
