---
id: repair_concepts_creation
status: done
---

# Test Evaluation: repair_concepts_creation
**File:** `crates/maenifold-core/tests/repair_concepts_creation.rs`
**Test count:** 5
**Evaluated:** 2026-05-30
**Traceability:** T-10.10 (FR-17, RTM-094–097), T-3.2 (FR-6)

---

## Summary

This file tests the `createWikiLinks: true` mode of `RepairConcepts` — referred to as "Mode B" in the T-10.10 sprint context — which wraps plain-text concept mentions in WikiLink syntax rather than replacing existing WikiLink variants. All five tests use isolated temp directories, a shared mutex for sequencing, and `minSemanticSimilarity: 0.0` to bypass the semantic validation gate; those are correct setup choices.

Three tests (1–3) are labeled `mode_b_*` and trace to FR-17. Two tests (4–5) are labeled `create_wikilinks_*` and trace to FR-6. They exercise the same code path but originate from different sprint RTM entries, creating a visible naming split that carries no semantic meaning at the test level.

**Strengths:** All five tests assert against on-disk file content rather than relying purely on output-string matching. The code-block exclusion test (test 2) and the double-wrap prevention test (test 3) are particularly precise. The word-boundary test (test 4) verifies a correctness invariant that would be trivial to regress.

**Weaknesses:** Test 5 has an incomplete assertion set — it verifies preservation of unrelated WikiLinks but does not verify the count or positions of newly created WikiLinks, making it a one-sided guard. RTM-095 (one of four FR-17 RTM entries) has no test coverage in this file or anywhere else in the suite. Tests 4 and 5 use `create_wikilinks_*` naming while tests 1–3 use `mode_b_*` — the inconsistency is minor but adds cognitive friction when scanning the file.

---

## Verdicts

| Test Function | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable | Verdict |
|---|---|---|---|---|---|---|
| `repair_concepts_mode_b_wraps_plain_text_mentions_with_canonical_wikilinks` | ✅ | ✅ | ✅ | ✅ | Partial | **KEEP** |
| `repair_concepts_mode_b_skips_code_blocks_and_inline_code` | ✅ | ✅ | ✅ | ✅ | No | **KEEP** |
| `repair_concepts_mode_b_does_not_double_wrap_existing_wikilinks` | ✅ | ✅ | ✅ | ✅ | No | **KEEP** |
| `repair_concepts_create_wikilinks_mode_respects_word_boundaries` | ✅ | ✅ | ✅ | ⚠️ | Partial | **KEEP** (rename) |
| `repair_concepts_create_wikilinks_preserves_existing_wikilinks` | ✅ | ✅ | ⚠️ | ⚠️ | No | **REWRITE** |

---

### Test 1 — `repair_concepts_mode_b_wraps_plain_text_mentions_with_canonical_wikilinks`
**RTM:** T-10.10 / FR-17 / RTM-094

**Load-bearing?** Yes. This is the canonical Mode B happy-path test and the only test that verifies the basic contract: a plain-text mention of a variant ("tools") is replaced with a WikiLink to the canonical concept ("[[tool]]"). Removing it would leave the core behavior untested at this layer.

**Unique?** Yes. No other test in the suite exercises this bare minimum path.

**Correct assertions?**
- `output.contains("✓ Modified: repair/mode-b-wrap")` — output-label check, necessary to confirm the tool reported activity.
- `output.contains("Total replacements: 1")` — count assertion on the output report. Load-bearing: prevents silent double-counting.
- `updated.contains("Using [[tool]] in prose...")` — file-content check. Strong.
- `!updated.contains("Using tools in prose...")` — negative file-content check. Strong.
All four assertions are appropriate and together form a complete verification triangle (reported + counted + mutated + original gone).

**Good name?** Yes. Verbose but precise: mode, input condition, expected output. Consistent with tests 2 and 3.

**Parameterizable?** Partially. The setup structure (`write_memory_fixture`, `invoke_repair_concepts`, `read_memory_fixture`) is identical to tests 2 and 3. An `#[rstest]` table over (fixture_content, canonical, variant, expected_prose_result) could unify tests 1–3 while preserving distinct assertions for code-block behavior. However, the different assertion shapes make parameterization awkward without sacrificing clarity. Acceptable as-is.

**Verdict: KEEP**

---

### Test 2 — `repair_concepts_mode_b_skips_code_blocks_and_inline_code`
**RTM:** T-10.10 / FR-17 / RTM-096

**Load-bearing?** Yes. Code-block and inline-code exclusion is a correctness invariant: if Mode B wraps concept mentions inside backtick or fenced-code spans, it corrupts technical content. This is the only test guarding that path.

**Unique?** Yes. No other test in the suite uses a fixture that mixes prose, inline code, and a fenced code block with the same target term.

**Correct assertions?**
- `output.contains("Total replacements: 1")` — verifies exactly one replacement occurred (the prose mention, not the code mentions). This is the key assertion: a bug that also replaces inside code blocks would report "3" or more.
- `updated.contains("[[tool]] should be wrapped in prose.")` — prose result confirmed.
- `updated.contains("Inline code: \`tools\`")` — inline code untouched. Strong.
- `updated.contains("```text\ntools\n```")` — fenced block untouched. Strong and precise (verbatim whitespace check).
All five assertions are correct and together cover both positive and negative sides of the code-block exclusion contract.

**Good name?** Yes. The name correctly calls out both exclusion cases (code blocks + inline code).

**Parameterizable?** No. The multi-region fixture (prose + inline + fenced) is specific to this test's purpose and cannot be simplified without losing coverage.

**Verdict: KEEP**

---

### Test 3 — `repair_concepts_mode_b_does_not_double_wrap_existing_wikilinks`
**RTM:** T-10.10 / FR-17 / RTM-097

**Load-bearing?** Yes. Double-wrapping (`[[[tool]]]`) is a document-corruption bug. Without this test, a naive implementation that applies word-boundary replacement to all content including existing WikiLink text would pass tests 1 and 2 but silently corrupt the graph.

**Unique?** Yes. This is the only test in the file — and likely the suite — where the fixture contains an existing WikiLink for the *same canonical concept* that Mode B is also being asked to create.

**Correct assertions?**
- `output.contains("Total replacements: 1")` — critical. Prevents silent double-replacement.
- `!updated.contains("[[[") && !updated.contains("]]]]")` — structural guard against triple-bracket corruption. Somewhat weak as a standalone check (a different corruption pattern could escape), but kept as a belt alongside the strong assertion below.
- `updated.matches("[[tool]]").count() == 2` — count-based: one preserved, one added. This is the primary load-bearing assertion.
- `updated.contains("Existing [[tool]] should stay linked while [[tool]] in prose may be added.")` — exact-string check on the full transformed sentence. The strongest assertion in the file; it verifies both position and preservation of the original WikiLink simultaneously.
The four assertions form a layered check. No redundancy that should be removed; the structural guard and exact-string check complement each other.

**Good name?** Yes. "Does not double wrap" precisely describes the regression being prevented.

**Parameterizable?** No. The fixture shape (existing WikiLink + nearby plain-text variant) is specific to this anti-regression scenario.

**Verdict: KEEP**

---

### Test 4 — `repair_concepts_create_wikilinks_mode_respects_word_boundaries`
**RTM:** T-3.2 / FR-6

**Load-bearing?** Yes. Word-boundary enforcement prevents "tool" from matching inside "toolbox" or "toolkit". Without this, Mode B would corrupt any compound term containing the variant as a substring. This is a class of bug that would be immediately user-visible.

**Unique?** Yes. No other test in the file or `repair_concepts_batch.rs` uses a fixture specifically designed to distinguish whole-word matches from substring matches.

**Correct assertions?**
- `updated.matches("[[software-tool]]").count() == 2` — count assertion confirms exactly the two whole-word matches ("The tool" and "the tool") are wrapped. Correct: the fixture contains two standalone `tool` matches and two substring occurrences (`toolbox`, `toolkit`).
- `updated.contains("toolbox") && updated.contains("toolkit")` — confirms substrings survive intact. Correct.
- `output.contains("✓ Modified: repair/word-boundary")` — output-label check; confirms the tool acted on the file.
All three assertions are correct. One question: the fixture lacks YAML frontmatter, unlike every other fixture in this file. The implementation handles frontmatter-less `.md` files correctly (scans all content as body), but the inconsistency is confusing and creates a silent assumption that could break if the implementation ever begins requiring frontmatter.

**Good name?** Partially. The prefix `create_wikilinks_mode` is inconsistent with tests 1–3 which use `mode_b_`. Both refer to the same code path (`createWikiLinks: true`). Recommend renaming to `repair_concepts_mode_b_respects_word_boundaries` to align the naming convention across the file.

**Parameterizable?** Partially. The word-boundary test shares the `invoke_repair_concepts` + `read_memory_fixture` invocation pattern with tests 1–3, but the distinct fixture shape and count-based assertion makes parameterization marginal.

**Verdict: KEEP** (rename; add frontmatter to fixture for consistency)

---

### Test 5 — `repair_concepts_create_wikilinks_preserves_existing_wikilinks`
**RTM:** T-3.2 / FR-6

**Load-bearing?** Partially. The test's strength is the count assertion on `[[existing-concept]]` — it verifies that Mode B does not corrupt WikiLinks for a *different* concept. This is a scenario not covered by test 3 (which uses the same concept). Removing this test would leave a real gap.

**Unique?** Yes. The other tests all use the same canonical concept for existing and new WikiLinks. This is the only test where the existing WikiLinks use a *different* concept than the one being created.

**Correct assertions?**
- `assert_eq!(updated.matches("[[existing-concept]]").count(), original.matches("[[existing-concept]]").count())` — strong: counts 3 occurrences and verifies all survive. This is the primary value of the test.
- `updated.contains("[[build-tool]]")` — **weak.** The fixture has two standalone "tool" mentions (line 1: "with tool text" and line 3: "tool should be converted"). The test only asserts `contains`, not `count == 2`. A bug that creates only one WikiLink instead of two would pass this assertion. The correct assertion is `updated.matches("[[build-tool]]").count() == 2`.
- `output.contains("build-tool")` — very weak output-label check that would be satisfied by a log line containing the string "build-tool" anywhere. Adds little value beyond confirming the tool ran without panic.

The missing count assertion on `[[build-tool]]` is the concrete gap. It means the test is guarding preservation (existing WikiLinks) but not creation completeness (new WikiLinks). That makes it one-directional: it could pass while Mode B silently misses one of the two replacement targets.

**Good name?** Partially. The suffix "preserves_existing_wikilinks" accurately describes what the test checks, but "existing_wikilinks" is vague — it could mean any WikiLinks, when the test is specifically verifying WikiLinks *for an unrelated concept*. A more precise name: `repair_concepts_mode_b_preserves_unrelated_wikilinks_while_creating_new_ones`. The `create_wikilinks_*` prefix also breaks the naming convention established by tests 1–3.

**Verdict: REWRITE**

Rewrite the assertion set:
```rust
// Replace weak `contains` with an exact count:
assert_eq!(
    updated.matches("[[build-tool]]").count(), 2,
    "both standalone 'tool' mentions should be wrapped: {updated}"
);
// Keep the strong preservation assertion as-is.
assert_eq!(
    updated.matches("[[existing-concept]]").count(),
    original.matches("[[existing-concept]]").count(),
    "existing wikilinks for unrelated concepts should be preserved: {updated}"
);
// Add an exact-string check for at least one specific transformed line:
assert!(
    updated.contains("Already [[existing-concept]] with [[build-tool]] text."),
    "unrelated wikilink should stay intact while plain-text variant is wrapped: {updated}"
);
// Drop the trivial output.contains("build-tool") check — it adds no value.
```

---

## Recommendations

1. **RTM-095 COVERAGE GAP.** FR-17 spans RTM-094 through RTM-097. Tests 1–3 cover 094, 096, and 097. RTM-095 has no test in this file and no coverage elsewhere in the suite (`grep -rn "RTM-095"` returns zero results). The RTM entry for 095 is not individually documented in CHANGELOG.md (only the range is listed). This must be investigated: determine what behavior RTM-095 was intended to verify (likely Mode B dry-run, or Mode B with multiple files), and either add a test or document explicitly that it is covered by another test with a `// T-10.10: RTM FR-17 / RTM-095` comment.

2. **REWRITE test 5** as detailed above. Replace `updated.contains("[[build-tool]]")` with `updated.matches("[[build-tool]]").count() == 2`. Add an exact-string assertion for the transformed line. Drop the low-value `output.contains("build-tool")` check.

3. **RENAME tests 4 and 5** from the `create_wikilinks_*` prefix to `mode_b_*` to match the convention of tests 1–3. The current split creates two naming conventions for the same code path within a single 231-line file.

4. **ADD frontmatter to test 4's fixture.** All other fixtures in this file include YAML frontmatter (`title`, `created`, `modified`). The word-boundary test fixture is raw body-only markdown. Even if the implementation handles this correctly today, the inconsistency is a latent trap: if the implementation ever begins skipping files without frontmatter, this test silently becomes a false-positive (the tool would skip the file, the assertions would pass because the original content remains, and the count of `[[software-tool]]` would be 0 — which is not 2, so the count assertion would catch it; but the output assertion would fail with a confusing message). Add frontmatter to be consistent and safe.

5. **Consider Mode B dry-run coverage.** None of the five tests invoke Mode B with `dryRun: true`. The `repair_concepts_replacement.rs` suite covers dry-run for Mode A, but Mode B + dry-run is a distinct state (no file written, but output should report "Would modify" with a count). If RTM-095 turns out to be the Mode B dry-run case, this is the natural home for it.

6. **Parameterization is optional** but reasonable for tests 1–3. Their shared structure (`write_fixture → invoke → read → assert count + content`) could be collapsed into a `#[rstest]` table driving `(fixture_path, fixture_body, expected_count, expected_presence, expected_absence)`. The benefit is lower boilerplate; the cost is reduced assertion clarity per-case. Given the file is already small and tests are short, parameterization is not urgent.

---

## Load-Bearing Score: 5/5

All five tests catch behaviors that would not be detected elsewhere in the suite. However, test 5's load-bearing capacity for the *creation* side of its scenario is diminished by the missing count assertion — it guards preservation but only weakly guards completeness. The score would be 4.5/5 in a half-point scale: five tests that should exist, one that needs a stronger assertion set before it fully earns its keep.

---
## SWE Remediation
- KEEP tests 1–3: unchanged (RTM-094, RTM-096, RTM-097)
- KEEP test 4 (rename): `create_wikilinks_mode_respects_word_boundaries` → `mode_b_respects_word_boundaries`; added YAML frontmatter to fixture for consistency with tests 1–3
- REWRITE test 5 (rename + assertions): `create_wikilinks_preserves_existing_wikilinks` → `mode_b_preserves_unrelated_wikilinks_while_creating_new_ones`; replaced weak `contains("[[build-tool]]")` with `matches("[[build-tool]]").count() == 2`; added exact-string assertion for transformed line; dropped trivial `output.contains("build-tool")` check; added YAML frontmatter to fixture
- ADD test 6: `mode_b_dry_run_reports_without_modifying_files` — closes RTM-095 coverage gap; verifies Mode B + dryRun=true reports "Would modify" and "This was a DRY RUN" without mutating on-disk content
- Commit: 2e5af5d

---
## Red-team Audit

**Commit audited:** `2e5af5d` (76 insertions, 10 deletions — single file)
**Tests executed:** 6/6 passing (`cargo test -p maenifold-core repair_concepts_mode_b`)
**Traceability:** T-10.10 (FR-17, RTM-094–097), T-3.2 (FR-6)

### Item-by-item

- ✅ **KEEP tests 1–3** (RTM-094, RTM-096, RTM-097): Diff hunks begin at line 162; lines 7–161 (tests 1–3) have zero changes. Confirmed identical to pre-commit.

- ✅ **KEEP test 4 (rename)**: `create_wikilinks_mode_respects_word_boundaries` → `mode_b_respects_word_boundaries`. Name change verified in diff. YAML frontmatter (`title: Word Boundary`, `created`, `modified`) added to fixture, matching eval Recommendation #4. Assertion set unchanged — correct per KEEP verdict.

- ✅ **REWRITE test 5 (rename + assertions)**:
  - Renamed `create_wikilinks_preserves_existing_wikilinks` → `mode_b_preserves_unrelated_wikilinks_while_creating_new_ones` ✓
  - Weak `updated.contains("[[build-tool]]")` replaced with `assert_eq!(updated.matches("[[build-tool]]").count(), 2, ...)` ✓ — matches eval spec exactly
  - Strong `[[existing-concept]]` count preservation assertion retained with updated message ✓
  - Exact-string assertion added: `updated.contains("Already [[existing-concept]] with [[build-tool]] text.")` ✓ — matches eval spec
  - Trivial `output.contains("build-tool")` dropped (variable renamed to `_output`) ✓ — matches eval recommendation
  - YAML frontmatter added to fixture ✓

- ✅ **ADD test 6** (`mode_b_dry_run_reports_without_modifying_files`): Closes RTM-095 coverage gap identified in eval Recommendation #1. Asserts:
  - `output.contains("Would modify: repair/mode-b-dry-run")` — dry-run summary check ✓
  - `output.contains("This was a DRY RUN")` — dry-run marker check ✓
  - `assert_eq!(after, original, ...)` — on-disk immutability check ✓
  - RTM comment `// T-10.10: RTM FR-17 / RTM-095` present ✓
  - Uses YAML frontmatter consistent with all other fixtures ✓

### No unrelated changes

- Diff is confined to the single test file. No source code modified.
- No new dependencies added. No other files touched.
- Commit message accurately describes all changes with traceability references.

### P0–P2 coverage

- **P0 (REWRITE test 5):** Fully addressed. Both the count assertion gap and the missing exact-string check are remediated.
- **P1 (RTM-095 gap):** Fully addressed. New test 6 covers Mode B + dryRun=true.
- **P2 (rename consistency):** Fully addressed. Tests 4 and 5 renamed to `mode_b_*` prefix.
- **P2 (frontmatter consistency):** Fully addressed. Tests 4 and 5 fixtures now include YAML frontmatter.

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- repair_concepts_creation`: 6 passed, 0 failed
- Verdict: PASS
