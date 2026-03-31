---
id: memory_tools_edit_sections
status: done
---

# Test Evaluation: memory_tools_edit_sections

**File:** `crates/maenifold-core/tests/memory_tools_edit_sections.rs`
**Test count:** 4
**Traceability:** T-12.6 / RTM NFR-1 (inherited via shared `support.rs`; no per-test tags in this file)
**Evaluated:** 2026-07-15

---

## Summary

This file contains 4 tests for the `replace_section` operation of `edit_memory`. It is the **second** file in the suite covering this operation — `memory_tools_replace_sections.rs` already exists and covers 5 overlapping tests; `memory_tools_editing_parity.rs` adds 4 more for T-15.3/T-15.4 parity behaviours. Critically, 3 of 4 tests here are **near-duplicates** of tests in `replace_sections.rs`, exercising identical code paths through structurally similar fixtures. The 4th test — replacing the last section in a multi-section document — is the only integration-level test across all three files that covers the EOF terminus boundary, and is therefore genuinely load-bearing.

Two systemic problems affect the whole file:

1. **Naming convention conflict.** All 4 functions use an `edit_memory_` prefix, while `replace_sections.rs` uses `replace_section_`. The mismatch makes grep-based navigation misleading and implies these tests were written in a separate context without reconciling against the existing file.

2. **Traceability gap.** The `// T-12.6: RTM NFR-1` comment lives in `support.rs` on `TEST_MUTEX`, not in this file. No individual test carries a `// T-*` tag. Tests cannot be independently traced to a requirement without inspecting the shared module.

---

## Verdicts

| # | Test Function | Verdict | Reason |
|---|---|---|---|
| 1 | `edit_memory_replace_section_replaces_existing_section` | **MERGE** | Near-duplicate of `replace_section_when_section_exists_replaces_content` in `replace_sections.rs`. Both replace a middle section in a 3-section document and verify new content present, old content absent, siblings untouched. Different fixture text; identical code path. The `starts_with("Updated memory FILE:")` format-string assertion is the same fragile proxy flagged in the `edit_ops` evaluation. Merge the assertion style (remove format-string check; assert URI returned) into the canonical test; delete this copy. |
| 2 | `edit_memory_replace_section_throws_when_section_not_found` | **MERGE** | Near-duplicate of `replace_section_when_section_does_not_exist_throws_invalid_operation` in `replace_sections.rs`. **However,** this version asserts *both* `"Section not found"` AND the section name in the error message; the canonical version only asserts the section name. The stronger assertion is the correct one — it should be migrated to the canonical test before this copy is removed. |
| 3 | `edit_memory_replace_section_works_with_different_heading_levels` | **MERGE** | Claims to test "different heading levels" but provides marginal unique coverage. `replace_sections.rs` already has `replace_section_with_h1_heading_replaces_correctly` (all-H1 doc) and `replace_section_with_h3_heading_replaces_correctly` (H2 parent + H3 children). This test targets an H2 in a mixed H1/H2/H3 document. The implementation's section parser treats all `#`-prefixed lines uniformly — no distinct branch for level combinations. The fixture's new content has no heading (the T-15.4 "no heading in replacement" path), already covered by `t15_4_new_content_without_heading_preserves_old_heading` at the unit level. Merge into `replace_sections.rs` if the mixed-level fixture is deemed worth preserving as documentation; otherwise remove. |
| 4 | `edit_memory_replace_section_replaces_last_section` | **KEEP** | Load-bearing. The only integration test across all three files that replaces the **last** section in a multi-section document. The implementation initialises `section_end = existing_content.len()` and only overrides it when a *next* heading is encountered after the target — the EOF path is structurally distinct from the middle-section path. All 5 tests in `replace_sections.rs` target middle sections; all `editing_parity.rs` replace_section tests also target middle sections. The unit test `t15_3_exact_heading_still_works` in `editing.rs` has a degenerate single-section doc (trivially "last"), but not the two-section case where section boundaries interact. This test catches a regression that the others would miss. |

---

## Detailed Observations

### 1. Duplicate coverage with `memory_tools_replace_sections.rs`

| This file | Canonical equivalent | Difference |
|---|---|---|
| `edit_memory_replace_section_replaces_existing_section` | `replace_section_when_section_exists_replaces_content` | Fixture text varies; code path identical |
| `edit_memory_replace_section_throws_when_section_not_found` | `replace_section_when_section_does_not_exist_throws_invalid_operation` | This version has stronger error message assertion |
| `edit_memory_replace_section_works_with_different_heading_levels` | `replace_section_with_h1_heading_replaces_correctly` + `replace_section_with_h3_heading_replaces_correctly` | Mixed-level doc; no distinct code path |
| `edit_memory_replace_section_replaces_last_section` | *(no equivalent)* | **Unique** — last-section EOF boundary |

### 2. Format-string assertions are fragile proxies (tests 1, 3, 4)

Tests 1, 3, and 4 all open with:
```rust
assert!(
    edit_result.starts_with("Updated memory FILE:"),
    "unexpected: {edit_result}"
);
```
This asserts the output message format rather than a behavioral guarantee. It:
- **Survives** content corruption bugs (mutation wrong, message fine → false pass)
- **Breaks** on cosmetic message rewording (message renamed → false fail)

The right assertion is `assert!(edit_result.contains(&uri))`, which verifies the round-trip identity of the edited file, not the shape of a success string. The same issue was flagged in the `edit_ops` evaluation. Test 2 correctly avoids this pattern (it uses `expect_err`, so the format string doesn't apply).

### 3. Naming convention conflict creates navigation debt

This file's `edit_memory_replace_section_*` prefix diverges from `replace_sections.rs`'s `replace_section_*` prefix. Both files test the same `replace_section` operation via the same `edit_memory()` function. A developer `grep`-ing for `replace_section` tests will find `replace_sections.rs` but not this file unless they also grep for `edit_memory_replace_section`. The inconsistency is a maintenance tax.

### 4. No per-test traceability tags

The `// T-12.6: RTM NFR-1` comment appears in `support.rs` on the mutex declaration — it does not appear in this file. Individual tests lack `// T-*` annotations. If tests 1–3 are merged out, the remaining test 4 should receive an explicit `// T-12.6: RTM NFR-1` tag to maintain the traceability chain required by the PRD.

### 5. Test 2 has the strongest error assertion in the suite

`replace_section_when_section_does_not_exist_throws_invalid_operation` in `replace_sections.rs` only asserts:
```rust
assert!(err.to_string().contains("Methodology"));
```
This passes if the word "Methodology" appears anywhere in the error — including in a generic stack trace. Test 2 in this file correctly asserts both:
```rust
assert!(err.to_string().contains("Section not found"));
assert!(err.to_string().contains("Nonexistent Section"));
```
This is the right pattern and should replace the weaker version.

---

## Recommendations

1. **Migrate test 4 (`replaces_last_section`) to `memory_tools_replace_sections.rs`** and add `// T-12.6: RTM NFR-1` tag. Rename to `replace_section_replaces_last_section` for naming consistency. Delete this file.

2. **Absorb the stronger error assertion from test 2** into `replace_section_when_section_does_not_exist_throws_invalid_operation` in `replace_sections.rs`. Upgrade it to assert `"Section not found"` in addition to the section name.

3. **Remove tests 1 and 3** without replacement — `replace_sections.rs` already covers the same code paths. Do not carry fixture variations as independent tests unless they exercise distinct branches.

4. **Remove `starts_with("Updated memory FILE:")` assertions** from all replace_section integration tests (this file and `replace_sections.rs`). Replace with `assert!(edit_result.contains(&uri))` — a behavioral guarantee, not a format-string check.

5. **Add `// T-12.6: RTM NFR-1` to each test** in `replace_sections.rs` after migration, maintaining the RTM chain that would otherwise be broken when this file disappears.

---

## Load-Bearing Score: 1/4

Only test 4 (`replaces_last_section`) covers a boundary not tested elsewhere in the integration suite. Tests 1, 2, and 3 are duplicates that add maintenance surface without adding coverage. The duplication is especially costly because it creates two competing naming conventions for the same operation, making the test suite harder to navigate and extend.

---

## SWE Remediation

- MERGE test 1 (`edit_memory_replace_section_replaces_existing_section`): removed — duplicate of `replace_section_when_section_exists_replaces_content`
- MERGE test 2 (`edit_memory_replace_section_throws_when_section_not_found`): removed — stronger `"Section not found"` assertion absorbed into canonical `replace_section_when_section_does_not_exist_throws_invalid_operation`
- MERGE test 3 (`edit_memory_replace_section_works_with_different_heading_levels`): removed — covered by H1/H3 tests in `replace_sections.rs`
- KEEP test 4 (`edit_memory_replace_section_replaces_last_section`): migrated to `memory_tools_replace_sections.rs` as `replace_section_replaces_last_section` with `// T-12.6: RTM NFR-1` tag
- Replaced fragile `starts_with("Updated memory FILE:")` with `contains(&uri)` across all 6 tests in `replace_sections.rs` (P0 format-string fix)
- Added `// T-12.6: RTM NFR-1` traceability tags to all tests in `replace_sections.rs`
- Deleted `memory_tools_edit_sections.rs` and removed `mod` from `main.rs`
- All 6 tests pass: `cargo test --test integration_tests -p maenifold-core -- memory_tools_replace_sections`
- Commit: b24c547

---
## Red-team Audit

### Checklist

- ✅ MERGE test 1 (`edit_memory_replace_section_replaces_existing_section`): Deleted. Canonical `replace_section_when_section_exists_replaces_content` in `replace_sections.rs` covers the same middle-section code path. No unique coverage lost.
- ✅ MERGE test 2 (`edit_memory_replace_section_throws_when_section_not_found`): Deleted. The stronger `"Section not found"` assertion was absorbed into canonical `replace_section_when_section_does_not_exist_throws_invalid_operation` — confirmed in diff: `assert!(err.to_string().contains("Section not found"));` added at line 68 of the new file. Original assertion on section name `"Methodology"` preserved. Both aspects of the eval recommendation satisfied.
- ✅ MERGE test 3 (`edit_memory_replace_section_works_with_different_heading_levels`): Deleted. Mixed-level fixture exercised no distinct code branch beyond existing H1 and H3 tests. No unique coverage lost.
- ✅ KEEP test 4 (`edit_memory_replace_section_replaces_last_section`): Migrated to `memory_tools_replace_sections.rs` as `replace_section_replaces_last_section`. Renamed to `replace_section_` prefix per naming convention. EOF terminus boundary coverage preserved. Traceability comment `// T-12.6: RTM NFR-1 — migrated from memory_tools_edit_sections.rs` documents provenance.
- ✅ P0 format-string fix: All 5 `starts_with("Updated memory FILE:")` assertions replaced with `contains(&uri)` across the 5 applicable tests (test 2 uses `expect_err`, not applicable). Zero instances of the fragile pattern remain in the file (grep confirmed).
- ✅ Traceability tags: All 6 tests carry `// T-12.6: RTM NFR-1`. The first test already had the tag pre-commit; 5 new tags added in this commit.
- ✅ File deletion: `memory_tools_edit_sections.rs` removed. `mod memory_tools_edit_sections;` removed from `main.rs`. No dangling references (only the migration provenance comment in the last test, which is informational).
- ✅ No unrelated changes: Commit touches exactly 3 files — `main.rs` (1-line mod removal), `memory_tools_edit_sections.rs` (141 lines deleted), `memory_tools_replace_sections.rs` (57 lines net change). Scope is clean.
- ✅ Tests pass: `cargo test --test integration_tests -p maenifold-core -- memory_tools_replace_sections` → 6 passed, 0 failed.

### Verdict: PASS

All eval-specified actions executed correctly. Every MERGE absorbed the right coverage, the KEEP test migrated with proper naming/traceability, the P0 fragile-assertion fix applied to all applicable tests, and no unrelated changes introduced.

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_tools_replace_sections`: 6 passed, 0 failed
- Verdict: PASS
