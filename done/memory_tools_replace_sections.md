---
id: T-80.112
title: "Test Evaluation: memory_tools_replace_sections.rs"
status: done
pipeline: tdd
---

# Test Evaluation: memory_tools_replace_sections.rs

**Traceability**: T-12.6 | NFR-2 (test coverage) | NFR-1 (C# oracle parity)
**Evaluator**: Red Team / Gray (Skeptical Inquiry)
**Date**: 2025-07-12
**Subject file**: `crates/maenifold-core/tests/memory_tools_replace_sections.rs`
**Tests evaluated**: 5
**Competitor files examined**:
- `crates/maenifold-core/tests/memory_tools_edit_sections.rs` (4 tests, overlapping coverage)
- `crates/maenifold-core/tests/memory_tools_editing_parity.rs` (4 tests, T-15.x parity)
- `crates/maenifold-core/src/memory/editing.rs` → `editing_parity_tests` module (8 inline unit tests)

---

## Summary

The file was added as a dedicated home for `replace_section` integration tests. On skeptical examination, **three of the five tests do not clear the load-bearing bar** when measured against the full test ecosystem. Two tests are strict subsets of tests already present in `memory_tools_edit_sections.rs`, with one of them carrying a misleading name and a weaker assertion than its duplicate. The remaining three tests earn their keep by exercising angles no other test touches: replacing an H1-only document's middle section, targeting an H3 subsection inside a mixed-depth document, and proving that multi-paragraph old content is **completely** removed rather than partially overwritten.

The file has no per-test `// T-*: RTM *` annotations — only the shared `support.rs` module carries the `T-12.6: RTM NFR-1` tag. This makes traceability difficult for future audits.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Quality | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `replace_section_when_section_exists_replaces_content` | ❌ No | ❌ Duplicate | ✅ Yes | ✅ Good | ✅ Yes (with T3, T4) | **REMOVE** |
| 2 | `replace_section_when_section_does_not_exist_throws_invalid_operation` | ❌ No | ❌ Duplicate | ⚠️ Weak | ❌ Misleading | N/A | **REWRITE** |
| 3 | `replace_section_with_h1_heading_replaces_correctly` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Good | ✅ Yes (with T1, T4) | **KEEP** |
| 4 | `replace_section_with_h3_heading_replaces_correctly` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Good | ✅ Yes (with T1, T3) | **KEEP** |
| 5 | `replace_section_old_content_completely_removed_not_appended` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Excellent | ❌ No (intentionally narrow) | **KEEP** |

---

## Detailed Findings

### Test 1: `replace_section_when_section_exists_replaces_content`

**What it does**: Writes a file with H2 sections and an `# Introduction`, calls `replace_section` on the middle `## Methodology` section, asserts new content present / old content absent / surrounding sections intact, with WikiLink-level assertions.

**Duplicate**: `edit_memory_replace_section_replaces_existing_section` in `memory_tools_edit_sections.rs` covers the identical code path (write → edit → read, H2 middle section, success message, old/new content, sibling sections intact).

**What this test adds over the duplicate**: WikiLink-level granularity — it checks `[[improved-approach]]` and `[[better-techniques]]` are present and `[[documentation-structure]]` in the intro is preserved. This is cosmetic: it tests string containment on the same bytes the duplicate already exercises.

**Verdict**: REMOVE. The only difference is WikiLink assertions that test no additional execution path. If WikiLink preservation assurance is the goal, add one targeted assertion to the existing test rather than keep a full duplicate.

---

### Test 2: `replace_section_when_section_does_not_exist_throws_invalid_operation`

**What it does**: Attempts `replace_section` for a section name (`Methodology`) not present in the file, asserts an error is returned.

**Duplicate**: `edit_memory_replace_section_throws_when_section_not_found` in `memory_tools_edit_sections.rs` covers the same error path.

**Problems beyond duplication**:

1. **Misleading name**: The test is named `throws_invalid_operation`, implying it verifies a specific error category or code. No such verification exists. The implementation returns `MaenifoldError::Message("Section not found: ...")` — the word "invalid_operation" never appears. The name creates a false expectation for future readers.

2. **Weaker assertion than its duplicate**: The existing test in `edit_sections.rs` asserts BOTH `err.to_string().contains("Section not found")` AND `err.to_string().contains("Nonexistent Section")`. This test asserts only `err.to_string().contains("Methodology")` — the section name alone. A future implementation could return any error message containing the word "Methodology" and this test would still pass, even if the error category changed silently.

**Verdict**: REWRITE if kept standalone. The name must reflect what is actually being tested. Assertions must be strengthened to match or exceed the companion test: verify the "Section not found" phrase AND the section name. Alternatively, remove and rely on the stronger existing test.

---

### Test 3: `replace_section_with_h1_heading_replaces_correctly`

**What it does**: Builds an all-H1 document (`# First Section`, `# Second Section`, `# Third Section`), replaces the middle section, verifies content replacement and sibling preservation.

**Unique angle**: `edit_memory_replace_section_works_with_different_heading_levels` in `edit_sections.rs` uses a mixed-level document (H1 + H2 + H3) and replaces the **H2** section — it does **not** target an H1 section for replacement. No other integration test exercises replacing an H1 in a homogeneous H1-only document. This matters because the `replace_section` implementation's section-boundary detection (`line_start` tracking) behaves differently when the terminating heading has the same level as the target.

**Assertion quality**: All 5 assertions are logically necessary and independently falsifiable. WikiLink checks (`[[delta]]`, `[[beta]]`, `[[alpha]]`, `[[gamma]]`) correctly distinguish replaced vs. preserved concepts.

**Verdict**: KEEP. Unique code path. Add `// T-12.6: RTM NFR-1` traceability comment.

---

### Test 4: `replace_section_with_h3_heading_replaces_correctly`

**What it does**: Builds a document with an H2 parent section and two H3 subsections (`### Subsection A`, `### Subsection B`), replaces `### Subsection A`, verifies the sibling H3 subsection and parent H2 are intact.

**Unique angle**: `edit_memory_replace_section_works_with_different_heading_levels` in `edit_sections.rs` contains an `### Level 3 Heading` section but **does not replace it** — it replaces the H2 section and only checks the H3 survives intact. No other test targets an H3 for replacement in a mixed-level document. The H3 boundary case is meaningfully distinct: the section end is detected when the parser encounters `### Subsection B` (same level), not a higher-level heading.

**Assertion quality**: All 5 assertions are correct. The negative assertion `!read_result.contains("[[old-concept]]")` is essential and correctly targeted.

**Verdict**: KEEP. Unique code path. Add `// T-12.6: RTM NFR-1` traceability comment.

---

### Test 5: `replace_section_old_content_completely_removed_not_appended`

**What it does**: Sets up a target section with three multi-paragraph lines each carrying a distinct WikiLink (`[[deprecated-approach]]`, `[[legacy-implementation]]`, `[[outdated-pattern]]`). Replaces with a single-paragraph section. Asserts every old paragraph and every old WikiLink is absent; the replacement and surrounding sections are intact.

**Unique angle**: No other test in the codebase verifies that a multi-paragraph section body is **entirely** purged. The 7 negative assertions form a semantic regression contract: if the implementation degrades to appending instead of replacing, or truncates only the first paragraph, at least one assertion will catch it. The test name explicitly encodes the semantic invariant being protected ("not_appended"), making the intent self-documenting.

**Assertion quality**: 10 total assertions (3 positive, 7 negative). All are independently meaningful. This is the highest assertion-to-test-surface ratio in the file and the most deliberately constructed test.

**Verdict**: KEEP. High-value regression guard with no equivalent elsewhere.

---

## Recommendations

### Priority 1 — Remove Test 1

`replace_section_when_section_exists_replaces_content` is a full duplicate of `edit_memory_replace_section_replaces_existing_section`. Add the single WikiLink-preservation assertion (checking that WikiLinks in untouched sections survive) to the existing test in `memory_tools_edit_sections.rs`. Then delete this test.

### Priority 2 — Rewrite or Remove Test 2

The name `throws_invalid_operation` is a lie. The actual behavior is "returns an error containing the section name." Either:
- **Remove** and rely on the stronger `edit_memory_replace_section_throws_when_section_not_found` (which already asserts both `"Section not found"` and the section name).
- **Rewrite** with name `replace_section_error_message_names_the_missing_section`, asserting both `"Section not found"` and `"Methodology"` with a `// T-12.6` tag.

### Priority 3 — Add Traceability to Tests 3, 4, 5

All three kept tests lack per-test `// T-12.6: RTM NFR-1` annotations. Add them immediately above each `#[test]` attribute to make the traceability scannable by automated tools.

### Priority 4 — Consider Parameterization of Tests 3 and 4

Tests 1, 3, and 4 test the same logical operation at different heading levels. If the `rstest` crate is ever added to the test harness, these are natural candidates for a `#[rstest]` table: `(H1, "# heading")`, `(H2, "## heading")`, `(H3, "### heading")`. This would reduce duplication further.

### Priority 5 — Consider Moving Test 5 to edit_sections.rs

`replace_section_old_content_completely_removed_not_appended` belongs conceptually with the other replace_section integration tests in `memory_tools_edit_sections.rs`. The current file split (edit_sections + replace_sections) is not clearly motivated by any functional boundary — it appears to be an artifact of how tests were added over sprints. Consolidation would improve navigability.

---

## Load-Bearing Score: 3/5

Three tests exercise code paths or semantic invariants not covered by any other test in the suite. Two tests are either strict duplicates or degraded versions of existing tests. The file as a whole passes its regression budget only marginally.

| Test | Load-Bearing |
|------|:---:|
| `replace_section_when_section_exists_replaces_content` | ❌ |
| `replace_section_when_section_does_not_exist_throws_invalid_operation` | ❌ |
| `replace_section_with_h1_heading_replaces_correctly` | ✅ |
| `replace_section_with_h3_heading_replaces_correctly` | ✅ |
| `replace_section_old_content_completely_removed_not_appended` | ✅ |

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_tools_replace_sections`: 5 passed, 0 failed
- Verdict: PASS
