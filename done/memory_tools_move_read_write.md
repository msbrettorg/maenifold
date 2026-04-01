---
id: T-80.111
title: "Test Evaluation: memory_tools_move_read_write.rs"
status: done
pipeline: tdd
---

# Test Evaluation: memory_tools_move_read_write.rs

**Traceability**: T-12.6 · NFR-1 · NFR-2  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**File**: `crates/maenifold-core/tests/memory_tools_move_read_write.rs`  
**Tests evaluated**: 6

---

## Summary

This file claims to test three distinct tool surface areas — `MoveMemory`, `ReadMemory`, and
`WriteMemory` — but it is doing so with an incoherent scope. The file name implies a single
cohesive concern (move + read + write together), yet the six tests are really three orthogonal
mini-suites that happen to share a support harness. That grouping is a smell, not a design.

Of the six tests, **four are genuinely load-bearing** with no full duplicates elsewhere. One
(`move_memory_with_simple_destination_preserves_existing_behavior`) substantially overlaps with
`memory_lifecycle_validation.rs::move_memory_valid_destination_succeeds` and adds only marginal
value. All six lack individual traceability comments; only the `TEST_MUTEX` declaration carries
the `T-12.6` tag, which gets inherited by every test in the file without discrimination.

The assertions are generally correct but incomplete: no test verifies that the **source URI
becomes inaccessible after a move**, two tests contain redundant guard logic in their assertion
predicates, and `read_memory_handles_empty_frontmatter` uses a misleading name for a condition
that is not "empty frontmatter" but rather "no frontmatter at all."

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `move_memory_with_path_based_destination_preserves_markdown_extension` | ✅ | ⚠️ partial | ⚠️ incomplete | ❌ misleading | ✅ | **REWRITE** |
| 2 | `move_memory_with_simple_destination_preserves_existing_behavior` | ⚠️ marginal | ❌ overlaps | ✅ sufficient | ❌ vague | ✅ | **MERGE** |
| 3 | `read_memory_handles_missing_title_in_frontmatter` | ✅ | ✅ | ✅ | ✅ | ✅ | **KEEP** |
| 4 | `read_memory_handles_empty_frontmatter` | ✅ | ✅ | ⚠️ incomplete | ❌ wrong | ✅ | **REWRITE** |
| 5 | `write_memory_does_not_write_embedding_fields` | ✅ | ✅ | ✅ | ✅ | ⚠️ limited | **KEEP** |
| 6 | `write_memory_with_existing_h1_does_not_create_duplicate_heading` | ✅ | ✅ | ⚠️ redundant predicate | ✅ | ✅ | **KEEP** |

---

## Per-Test Analysis

---

### Test 1 — `move_memory_with_path_based_destination_preserves_markdown_extension`

**Verdict: REWRITE**

**What it actually tests**: That `MoveMemory` with a slash-containing destination (e.g.,
`"regression-test/Moved Extension Test"`) succeeds, creates the subdirectory, and produces a
readable file at the new URI.

**What the name claims it tests**: That the `.md` file extension is preserved through a move.

These are not the same thing. The test never inspects the filesystem path of the moved file,
never calls `raw_path_for_uri` to confirm the extension, and the URI format (`memory://`) does
not carry extensions. The `.md` claim is entirely unverified.

**Overlap risk**: `memory_path_nested.rs::move_memory_creates_nested_destination_directories`
covers path-based destinations with deeper nesting AND verifies that the subdirectory exists on
disk. Test 1 covers a 2-level path but adds no assertion that the directory was created, making
it weaker than the existing nested-path test for the aspect it shares.

**Missing assertions**:
- The source URI (`uri`) is never re-read to confirm it returns `ERROR:` after the move. A move
  that copies without deleting the source would pass all current assertions.
- The raw file extension is never checked (contradicts the name).

**Missing traceability**: No `// T-*:` comment on the test function itself.

**Recommended replacement name**: `move_memory_path_based_destination_moves_content_to_subdirectory`

---

### Test 2 — `move_memory_with_simple_destination_preserves_existing_behavior`

**Verdict: MERGE into `memory_lifecycle_validation.rs::move_memory_valid_destination_succeeds`**

**What it tests**: Simple flat rename of a memory file — write, move to a single-word
destination, check URI, read back content.

**Overlap**: `memory_lifecycle_validation.rs::move_memory_valid_destination_succeeds` (tagged
`T-15.5`) tests exactly this scenario with an explicit regression comment explaining why it
exists. The only difference is that test 2 verifies content (`"Testing [[refactoring]]
functionality."` survives) while the lifecycle test does not. That incremental value belongs
as a single added `assert!` in the existing test, not as a standalone test in a different file.

**Name problem**: "preserves existing behavior" is meaningless as a test name. What behavior?
For whom? Compared to what baseline? This phrasing signals the test was written to prevent
accidental deletion ("don't remove this, it preserves behavior") rather than to document a
specific contract.

**Missing assertions**: Source URI accessibility not verified post-move.

**Recommended action**: Delete this test. Strengthen `move_memory_valid_destination_succeeds`
with a content-survival assertion.

---

### Test 3 — `read_memory_handles_missing_title_in_frontmatter`

**Verdict: KEEP**

**What it tests**: When a `.md` file has valid frontmatter (`type`, `status`) but no `title`
key, `ReadMemory` should fall back to the filename slug as the display heading.

**Correctness**: All four assertions are load-bearing and non-redundant:
- No error returned ✅
- `# no-title-test` heading derived from filename ✅
- `URI: memory://memory-tools-tests/no-title-test` present ✅
- Content body preserved ✅

**Uniqueness**: `markdown_rw_read_tests.rs::read_markdown_malformed_frontmatter_returns_empty_frontmatter`
covers corrupted YAML, not absent title. `frontmatter_sessions.rs::fr24_6_ac4_*` covers
timestamp display, not heading fallback. This test is genuinely distinct.

**Missing traceability**: No `// T-*:` comment. Should reference T-12.6 or the relevant
sprint task for `ReadMemory` title-fallback behavior.

**Minor note**: The WikiLink `[[metadata-handling]]` in the test content is incidental fixture
data, not part of what's being asserted. Harmless but slightly noisy.

---

### Test 4 — `read_memory_handles_empty_frontmatter`

**Verdict: REWRITE**

**The name is factually wrong.** The file written is:
```
# Test Without Frontmatter\n\nThis is a [[yaml-frontmatter]] without any frontmatter.\n
```
There are no `---` delimiters. This is a file **with no frontmatter**, not a file with
**empty frontmatter**. Empty frontmatter would be `---\n---\n# Body`. These are distinct parse
paths. The test may accidentally be correct in its behavior but it documents the wrong contract.

**Why this matters**: If a future developer adds special handling for the empty-frontmatter
case (`---\n---`), this test will not catch regressions in the "truly empty YAML block" path
because it never exercises that input.

**Inconsistency with Test 3**: Test 3 asserts `read_result.contains("URI: memory://...")` as
part of the output contract. Test 4 does not check for the URI line. These tests cover
structurally identical output paths; the assertion asymmetry is unexplained.

**Missing assertions**:
- URI line not checked (inconsistent with Test 3)
- Does not verify that no `ERROR:` appears in a content-containing file (negative check)

**Recommended replacement name**: `read_memory_handles_file_with_no_frontmatter`

**Recommended fix**: Add the missing URI assertion. Separately add a true
`read_memory_handles_empty_frontmatter_delimiters` test that writes `"---\n---\n# Body\n"`.

---

### Test 5 — `write_memory_does_not_write_embedding_fields`

**Verdict: KEEP**

**What it tests**: That `WriteMemory` does not persist internal computed fields
(`embedding_base64`, `embedding_model`, `embedding_date`) into the markdown file's frontmatter
on disk.

**Why it's load-bearing**: Embedding fields are computed at index time and should only live
in the database. If they leaked into the `.md` file, round-trip reads would expose them to
users and corrupt the canonical source. No other test in the corpus was found that asserts
these fields are absent from file output.

**Correctness**: The three negative assertions are precise and non-redundant. Reading the
raw file via `raw_path_for_uri` is the right tool — it bypasses the read pipeline and confirms
what is actually persisted.

**Minor concern**: The test only checks three specific field names. If a fourth internal field
(e.g., `embedding_version`) were introduced and accidentally written, this test would not catch
it. A complementary approach would be to assert the full frontmatter key set allowlist, though
that is a higher maintenance burden.

**Missing traceability**: No `// T-*:` comment.

---

### Test 6 — `write_memory_with_existing_h1_does_not_create_duplicate_heading`

**Verdict: KEEP**

**What it tests**: When content passed to `WriteMemory` already contains an H1 (`# Existing
Heading`), the tool should not inject an additional H1 (e.g., from the `title` field), resulting
in exactly one H1 in the raw output.

**Why it's load-bearing**: Duplicate H1 injection is a real class of bug in tools that
auto-prepend headings. No other test in the corpus asserts this invariant.

**Assertion predicate smell**: The H1 counting filter is:
```rust
trimmed.starts_with("# ") && !trimmed.starts_with("## ")
```
The second condition (`!trimmed.starts_with("## ")`) is **logically redundant**. A string
starting with `## ` (hash-hash-space) does not start with `# ` (hash-space), because the
second character differs. This reads as cargo-cult defensive coding; it suggests the author
was uncertain about the semantics. It should be removed to prevent future readers from
inferring that the two conditions are meaningfully independent.

**Correct assertions otherwise**: `h1_count == 1` plus `raw.contains("# Existing Heading")`
together guarantee both count and identity of the surviving heading. The final `read_result`
check confirms round-trip readability.

**Missing**: Test does not assert that `# Duplicate H1 Test` (the title argument) does NOT
appear as a standalone H1 line. Currently inferred by the count check, but would be more
explicit.

**Missing traceability**: No `// T-*:` comment.

---

## Recommendations

### Immediate (before Sprint 16 begins)

1. **Rename Test 4** to `read_memory_handles_file_with_no_frontmatter` and add the missing
   URI assertion for consistency with Test 3. One-line fix, zero behavioral change.

2. **Add `// T-12.6: RTM NFR-1` traceability comments** to Tests 3, 4, 5, and 6. The tag
   currently lives only on the `TEST_MUTEX` declaration and is not attributable to individual
   tests without reading the support module.

### Near-term (within current sprint)

3. **MERGE Test 2** into `memory_lifecycle_validation.rs::move_memory_valid_destination_succeeds`
   by adding a single content-survival assertion there. Delete Test 2 from this file.

4. **REWRITE Test 1** with a corrected name and two additional assertions:
   - `raw_path_for_uri` to verify the moved file has `.md` extension on disk, OR drop the
     extension claim and rename honestly.
   - Re-read the original `uri` after the move and assert it returns an error (source gone).

### Structural

5. **Split this file** into three files aligned with the tool under test:
   - `memory_tools_move_ops.rs` — all `MoveMemory` tests
   - `memory_tools_read_ops.rs` — all `ReadMemory` edge cases
   - `memory_tools_write_invariants.rs` — all `WriteMemory` output invariants

   The current file name `memory_tools_move_read_write.rs` is a collection bin, not a test
   module with a coherent contract. Grouping by tool makes it obvious when a tool lacks
   coverage and makes CI output failures easier to triage.

6. **Add a true empty-frontmatter test**: Write a file with `---\n---\n# Body\n` and verify
   `ReadMemory` handles it identically to the no-frontmatter case. Currently untested.

7. **Add source-URI-post-move assertion** to at least one `MoveMemory` test. A move that
   duplicates rather than moves would pass every test in this file today.

---

## Load-Bearing Score: 5/6

| Test | Load-Bearing |
|------|:---:|
| `move_memory_with_path_based_destination_preserves_markdown_extension` | ✅ Yes |
| `move_memory_with_simple_destination_preserves_existing_behavior` | ⚠️ Marginal (overlaps lifecycle validation) |
| `read_memory_handles_missing_title_in_frontmatter` | ✅ Yes |
| `read_memory_handles_empty_frontmatter` | ✅ Yes (wrong name; right behavior) |
| `write_memory_does_not_write_embedding_fields` | ✅ Yes |
| `write_memory_with_existing_h1_does_not_create_duplicate_heading` | ✅ Yes |

**5 of 6 tests are load-bearing.** None should be outright deleted; Test 2 should be merged.
The file has real value but is undermined by imprecise names, missing source-gone assertions,
and a traceability gap that makes it impossible to determine which sprint requirement each test
is guarding without reading the support module header.

---

## SWE Remediation

- REWRITE `move_memory_with_path_based_destination_preserves_markdown_extension` → `move_memory_path_based_destination_moves_content_to_subdirectory`: renamed for accuracy, added source-URI-post-move assertion to detect copy-without-delete bugs, added traceability comment
- MERGE `move_memory_with_simple_destination_preserves_existing_behavior` → `memory_lifecycle_validation.rs::move_memory_valid_destination_succeeds`: merged with content-survival assertion; deleted from this file
- KEEP `read_memory_handles_missing_title_in_frontmatter`: added `// T-12.6: RTM NFR-1` traceability comment
- REWRITE `read_memory_handles_empty_frontmatter` → `read_memory_handles_file_with_no_frontmatter`: renamed to match actual input (no frontmatter delimiters, not empty frontmatter), added missing URI assertion for consistency with Test 3, added traceability comment
- NEW `read_memory_handles_empty_frontmatter_delimiters`: true empty-frontmatter test (`---\n---\n# Body\n`) per recommendation 6
- KEEP `write_memory_does_not_write_embedding_fields`: added `// T-12.6: RTM NFR-1` traceability comment
- KEEP `write_memory_with_existing_h1_does_not_create_duplicate_heading`: added `// T-12.6: RTM NFR-1` traceability comment
- All 6 tests in `memory_tools_move_read_write` pass; both tests in `memory_lifecycle_validation` pass
- Commit: 91fb6bb

---

## Red-team Audit

| # | Remediation Claim | Verified | Notes |
|---|---|---|---|
| 1 | REWRITE `move_memory_path_based_destination_moves_content_to_subdirectory` | ✅ | Renamed accurately; source-URI-post-move assertion added (lines 43-47); traceability comment present (lines 11-13). Extension claim dropped per eval OR option. |
| 2 | MERGE `move_memory_with_simple_destination_preserves_existing_behavior` → lifecycle file | ✅ | Deleted from `memory_tools_move_read_write.rs`; new `move_memory_valid_destination_succeeds` created in `memory_lifecycle_validation.rs` with content-survival assertion. Note: eval referenced this as a pre-existing test to strengthen; it did not exist — SWE correctly created it, achieving the eval's intent. |
| 3 | KEEP `read_memory_handles_missing_title_in_frontmatter` + traceability | ✅ | `// T-12.6: RTM NFR-1` added; no other changes. |
| 4 | REWRITE `read_memory_handles_file_with_no_frontmatter` | ✅ | Renamed from `read_memory_handles_empty_frontmatter`; URI assertion added (line 94) for consistency with Test 3; traceability comment with explanatory distinction between no-frontmatter and empty-frontmatter (lines 74-76). |
| 5 | NEW `read_memory_handles_empty_frontmatter_delimiters` | ✅ | Input is `---\n---\n# Body\n...` per eval recommendation 6. Asserts no ERROR, URI present, content present. |
| 6 | KEEP `write_memory_does_not_write_embedding_fields` + traceability | ✅ | `// T-12.6: RTM NFR-1` added; no other changes. |
| 7 | KEEP `write_memory_with_existing_h1_does_not_create_duplicate_heading` + traceability | ✅ | `// T-12.6: RTM NFR-1` added; redundant `!trimmed.starts_with("## ")` predicate left in place — consistent with KEEP verdict. |

**Test execution confirmed**: 6/6 pass in `memory_tools_move_read_write`, 2/2 pass in `memory_lifecycle_validation`.

**Unrelated changes**: None detected. Commit touches only the two test files and the eval spec.

**Observations (non-blocking)**:
- The eval evaluator incorrectly stated `move_memory_valid_destination_succeeds` already existed in `memory_lifecycle_validation.rs`. It did not — the file previously contained only `move_memory_rejects_empty_and_whitespace_destinations` with a comment deferring valid-destination coverage to the other file. The SWE handled this correctly by creating the test.
- The new `move_memory_valid_destination_succeeds` in the lifecycle file does not include a source-URI-post-move assertion (unlike the rewritten Test 1). This is consistent with the eval, which only called for "content-survival" in the merge, and "source-gone" in the rewrite. Acceptable.

- Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_tools_move_read_write`: 6 passed, 0 failed
- Verdict: PASS
