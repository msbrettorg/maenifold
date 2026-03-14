# Test Evaluation: repair_concepts_replacement
**File:** `crates/maenifold-core/tests/repair_concepts_replacement.rs`
**Test count:** 8
**Evaluated:** 2026-03-14
**Traceability:** T-3.2 (FR-6, RTM-038 through RTM-046)

---

## Summary

This is the strongest of the three files. Seven of eight tests assert real, observable behavioral outcomes by inspecting on-disk file content rather than relying purely on output string labels. The dry-run test is the safety net for the most dangerous tool path. Two tests have narrower issues: one uses overly clever code-block parsing to assert a negative, and one checks absence of a UI label string to infer semantic bypass behavior — both are fragile to cosmetic renames.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `repair_concepts_mode_a_replaces_wikilink_variants_on_disk` | KEEP (rename) | Excellent load-bearing test. Verifies canonical replacement, variant removal, and non-target concept preservation all in one pass against real file content. The fixture name `repair/mode-a.md` and test name suffix `mode_a` are opaque — "mode A" has no defined meaning in the PRD or RTM. Rename to `repair_concepts_replaces_variant_and_preserves_unrelated_wikilinks`. |
| `repair_concepts_dry_run_returns_summary_without_modifying_files` | KEEP | Critical safety test. Verifies the dry-run contract at both the output level ("Would modify", "DRY RUN") and the filesystem level (`after == original`). The filesystem assertion is what makes this test load-bearing; without it, dry-run could silently write and the test would still pass. |
| `repair_concepts_folder_parameter_restricts_scanning` | KEEP | Load-bearing scope-restriction test. Uses two concrete fixture files and verifies both the positive case (inside modified) and the negative case (outside unchanged) against actual file content. This would catch any folder-scoping regression. |
| `repair_concepts_learn_true_returns_help_documentation` | REWRITE | The negative assertion uses `output.split("```").step_by(2).collect::<String>()` to strip code blocks before checking for "Scanning". This is test-internal parsing logic that is brittle to any change in code-fence formatting. The intent is simpler: `learn=true` should return help text and must not execute the repair pipeline. Express this directly: `!output.contains("Modified") && !output.contains("Scanning") && !output.contains("✓")`. The positive assertion `output.contains("RepairConcepts")` is reasonable but also fragile to case/punctuation changes in help text; prefer asserting a stable structural marker if one exists. |
| `repair_concepts_dry_run_with_invalid_folder_returns_error_output` | KEEP | Verifies error path for non-existent folder. Checks both `"ERROR"` and `"Directory not found"` — two levels of the error message chain. Load-bearing: this path is safety-critical since a silent no-op on an invalid folder would be dangerous. |
| `repair_concepts_dry_run_with_empty_variants_returns_error` | KEEP | Input validation guard. Uses `try_invoke` + `expect_err` correctly to assert that empty `conceptsToReplace` surfaces as an error, not a silent no-op. Note: `dryRun: true` in the payload is irrelevant to input validation (the error fires before the repair pipeline executes), but it is harmless. |
| `repair_concepts_excludes_git_directories_from_scanning` | KEEP | Load-bearing security/correctness test. `.git/` exclusion prevents accidental corruption of version-control internals. The assertion `output.contains("Files modified: 0")` is a format-label check, but the important assertion is `!output.contains(".git")`. Both together are adequate. Consider additionally asserting that the fixture file written to `.git/` is unchanged on disk (as `repair_concepts_dry_run_returns_summary_without_modifying_files` does) to make the test watertight without depending on output text. |
| `repair_concepts_semantic_bypass_with_zero_threshold_skips_validation_output` | REWRITE | The test asserts behavioral bypass via absence of the UI string `"SEMANTIC VALIDATION"`. This is fragile: if the string changes to lowercase, is split across lines, or is renamed, the test falsely passes while the behavior may have regressed. The test also only writes a file with `[[tool]]` (which IS the canonical target), so it does not exercise a genuinely semantically-dissimilar replacement — it cannot actually prove bypass occurred. A better approach: write a fixture with a concept that would fail semantic similarity if validation were active, invoke with `minSemanticSimilarity: 0.0`, and assert the file WAS modified (demonstrating bypass happened). |

---

## Recommendations

1. **RENAME `repair_concepts_mode_a_replaces_wikilink_variants_on_disk`** → `repair_concepts_replaces_variant_and_preserves_unrelated_wikilinks`. The "mode-a" label is vestigial from the C# oracle test naming scheme and carries no semantic meaning in this context.

2. **REWRITE `repair_concepts_learn_true_returns_help_documentation`**: Replace the `step_by(2)` code-block stripping with direct behavioral assertions: the output must contain some stable help-text marker (e.g., `"RepairConcepts"`) AND must not contain pipeline execution markers (`"✓ Modified"`, `"Scanning"`, `"Files modified"`). Avoid asserting against internal code-fence structure.

3. **STRENGTHEN `repair_concepts_excludes_git_directories_from_scanning`**: Add a `read_memory_fixture` call on the `.git/test.md` file and assert its content is unchanged. Currently, the test relies entirely on the output string not mentioning `.git` — a bug that silently modifies the file without mentioning it in output would escape.

4. **REWRITE `repair_concepts_semantic_bypass_with_zero_threshold_skips_validation_output`**: Replace UI-string-absence assertion with a file-content assertion. Seed a fixture where `minSemanticSimilarity: 0.5` would block replacement but `0.0` should permit it. Assert the file IS modified at `0.0` — this proves bypass by observed behavior rather than absence of a log string.

5. **Consider a parameterized approach** for the six tests that all share the same fixture (`REPAIR_FIXTURE`) and same target pair (`"tools"` → `"tool"`). A `#[rstest]` or table-driven structure would reduce boilerplate while making the behavioral dimension explicit. That said, the current clarity is acceptable given the tests are short and self-contained.

---

## Load-Bearing Score: 7/8

Seven tests reliably catch regressions via on-disk or error-type assertions. The semantic bypass test (test 8) is the exception: it cannot falsify a bypass regression because it checks UI text absence rather than an observable effect on file content.
