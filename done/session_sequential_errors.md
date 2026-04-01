---
id: T-80.151
title: "Test Evaluation: session_sequential_errors"
status: done
pipeline: tdd
---

# Test Evaluation: session_sequential_errors

> **Traceability**: `T-12.7 / RTM NFR-1` (file header), `T-14.10 / RTM FR-16` (inline on test 4)
> **Evaluator lens**: 🔘 Gray — Skeptical Inquiry
> **Sibling files examined for overlap**: `session_sequential_lifecycle.rs`, `session_sequential_flow.rs`, `session_sequential_status.rs`

---

## Summary

`session_sequential_errors.rs` contains **5 tests** across 159 lines. The central problem is a **file identity crisis**: the filename declares this is an *error* test file, yet 2 of 5 tests (40%) are happy-path lifecycle tests with no error assertions whatsoever. This is not a minor quibble — it actively misrepresents coverage, makes the test suite harder to navigate, and raises the question of what invariants each file is actually responsible for.

**The 3 genuine error tests** (tests 3–5) are correctly placed, cover distinct error paths, and have adequate assertions. They should stay.

**The 2 misplaced tests** (tests 1–2) warrant scrutiny:

- **Test 1** (`complete_session_lifecycle_…`) is the most thorough test in the entire file — 13+ assertions covering frontmatter fields, thought count, content structure, and conclusion rendering. It belongs in `session_sequential_lifecycle.rs`. It adds real coverage beyond what lifecycle already provides (`analysisType` frontmatter, `completed` key, 3-thought flow, `thoughts` field rendering, conclusion WikiLink persistence), so it earns its keep *once relocated*.

- **Test 2** (`new_session_extracts_concepts_correctly`) is the weakest test in the file: 3 assertions, 2 of which are trivial string checks. It overlaps substantially with `lifecycle::new_session_with_concepts_creates_session_file`. Its only unique claim is verifying that the `thoughts` param content appears in the session file — a single-assertion fact buried under a misleadingly broad test name. It should be rewritten as a focused `thoughts_field_renders_in_session_content` test, or merged into lifecycle with a proper assertion added there.

**Structural concern on tests 3 & 4**: Both test "provide a sessionId, get an ERROR: back" — they differ only in *which* kind of bad sessionId (nonexistent vs. malformed). The test bodies are structurally identical. These are prime candidates for table-driven / parameterized testing if the codebase ever adopts a parameterization pattern.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `complete_session_lifecycle_create_continue_complete_succeeds` | ✅ Yes | ✅ Partial¹ | ✅ Yes (13 assertions) | ⚠️ No² | ❌ No | **MOVE** → `session_sequential_lifecycle.rs` |
| 2 | `new_session_extracts_concepts_correctly` | ⚠️ Marginal | ⚠️ Marginal³ | ❌ No (3 weak assertions) | ❌ No⁴ | ❌ No | **REWRITE** → rename + add assertions, or MERGE into lifecycle |
| 3 | `continue_session_with_nonexistent_session_id_returns_error` | ✅ Yes | ✅ Yes | ⚠️ Fragile⁵ | ✅ Yes | ✅ With test 4 | **KEEP** |
| 4 | `continue_session_with_invalid_session_id_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ With test 3 | **KEEP** |
| 5 | `session_with_branch_from_thought_without_branch_id_returns_error` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No (distinct setup) | **KEEP** |

**Notes:**

¹ Test 1 is *partially* unique. `lifecycle::continue_session_with_valid_session_id_appends_thought` covers 2-thought create+complete, but test 1 adds: `analysisType` frontmatter persistence, `completed` timestamp key presence, `thoughtCount: "3"` (3-thought flow), `thoughts` field rendering as a content section, and conclusion WikiLink preservation. These are real, load-bearing coverage gaps if test 1 is dropped without being rehomed.

² The name `complete_session_lifecycle_create_continue_complete_succeeds` reads as a prose transcript of what the test *does*, not what property it *verifies*. A cleaner name for the lifecycle file: `session_lifecycle_with_analysis_type_persists_all_frontmatter_fields`.

³ Test 2's unique claim is that the `thoughts` parameter content appears in the session file. `lifecycle::new_session_with_concepts_creates_session_file` does not verify the `thoughts` field at all. However, the current test only weakly checks this with `content.contains("Thoughts:")` and `content.contains("AI")` — it does not verify that the thoughts *WikiLinks* are extracted or that the section is structurally correct.

⁴ `new_session_extracts_concepts_correctly` — "extracts_concepts_correctly" implies it's testing the concept extraction pipeline. It is not. It's testing that the `thoughts` param's free text and WikiLinks appear in rendered content. A honest name: `new_session_renders_thoughts_field_in_content`.

⁵ Test 3's assertion `result.contains("missing")` is a substring check against an error message. If the error message changes (e.g., to "not found" or "does not exist"), the test silently passes while the message changes. Consider asserting a more stable error code or the full expected message pattern.

---

## Recommendations

### R-1 (High): Move test 1 to `session_sequential_lifecycle.rs`
Test 1 is a well-written lifecycle test with strong assertions. It does not belong in an errors file. Moving it (without modification) closes genuine coverage gaps in `lifecycle` around `analysisType`, the `completed` timestamp key, `thoughtCount`, and conclusion WikiLink rendering.

**Rename on arrival**: `session_lifecycle_with_analysis_type_persists_all_frontmatter_fields`

### R-2 (Medium): Rewrite test 2 — focus and strengthen it
Replace `new_session_extracts_concepts_correctly` with `new_session_renders_thoughts_field_in_session_content`, scoped to verifying exactly that the `thoughts` parameter content renders correctly. Strengthen assertions:
- Assert the `Thoughts:` section heading exists
- Assert WikiLinks from `thoughts` appear in the content (not just substring of the raw concept name)
- Assert WikiLinks from `response` and `thoughts` appear separately (they're different sections)
Move to `session_sequential_lifecycle.rs` alongside the existing concept-related lifecycle tests.

### R-3 (Low): Parameterize tests 3 & 4
Tests 3 and 4 share identical structure: create harness → invoke with bad sessionId → assert ERROR: + specific message. If the codebase introduces a test parameterization pattern (e.g., a macro or data-driven table), these are the ideal candidates:

```
// Proposed parameterized form:
// ("session-9999999999999-12345", "missing"),
// ("session-not-a-timestamp",    "Invalid sessionId format"),
```

Until then, they are acceptable as separate tests.

### R-4 (Low): Harden test 3's error message assertion
`result.contains("missing")` is fragile. Pin it to the actual error message or at minimum a longer substring, e.g. `result.contains("session is missing")` or whatever the canonical error string is. This prevents silent drift.

### R-5 (Meta): Establish a file contract for `session_sequential_errors.rs`
After relocating tests 1 and 2, the remaining 3 tests clearly own: *"SequentialThinking returns `ERROR:` when given bad session identity inputs."* Document this in a top-of-file comment so future contributors don't add happy-path tests here.

---

## Load-Bearing Score: 4/5

Four of five tests cover unique, non-redundant behavior that would leave real gaps if removed:
- **Test 1**: Would lose `analysisType`, `completed`, 3-thought lifecycle, and conclusion WikiLink coverage
- **Test 3**: Would lose "session not found" error path coverage
- **Test 4**: Would lose sessionId format validation coverage (T-14.10 / FR-16)
- **Test 5**: Would lose branch-without-branchId validation coverage

Test 2 is the exception: marginal uniqueness, weak assertions, wrong file. The behavior it weakly tests is arguably a subset of what a strengthened lifecycle test should verify explicitly.

---

## SWE Remediation

- MOVE test 1 (`complete_session_lifecycle_create_continue_complete_succeeds`): removed from errors file; already present in `session_sequential_lifecycle.rs` as `session_lifecycle_with_analysis_type_persists_all_frontmatter_fields` (prior eval commit `0ad5145`)
- REWRITE test 2 (`new_session_extracts_concepts_correctly`): removed from errors file; already present in `session_sequential_lifecycle.rs` as `new_session_renders_thoughts_field_in_session_content` with strengthened WikiLink assertions (prior eval commit `0ad5145`)
- KEEP test 3: hardened `result.contains("missing")` → `result.contains("missing. Start with thoughtNumber=0")` per R-4
- KEEP test 4: no changes
- KEEP test 5: no changes
- R-5: added file contract comment establishing error-path ownership for `session_sequential_errors.rs`
- Commit: 2c289b4

---

## Red-team Audit

| # | Eval Verdict | SWE Claim | Audit Result |
|---|-------------|-----------|--------------|
| 1 | **MOVE** → lifecycle, rename to `session_lifecycle_with_analysis_type_persists_all_frontmatter_fields` | Removed from errors file; present in lifecycle.rs at prior commit `0ad5145` | ✅ **Confirmed.** Diff at `0ad5145` adds the test verbatim to `session_sequential_lifecycle.rs` with correct new name and `(R-1)` provenance comment. Diff at `2c289b4` removes original from errors. All 13 original assertions preserved. Rename matches R-1 spec exactly. |
| 2 | **REWRITE** → lifecycle as `new_session_renders_thoughts_field_in_session_content`, strengthen assertions | Removed from errors file; present in lifecycle.rs at prior commit `0ad5145` with strengthened WikiLink assertions | ✅ **Confirmed.** Original had 3 weak assertions: `content.contains("machine-learning")`, `content.contains("Thoughts:")`, `content.contains("AI")`. Rewrite has 7 assertions: verifies `[[machine-learning]]`, `[[neural-networks]]`, `[[deep-learning]]` as WikiLinks in thought section, then `Thoughts:` heading, `[[AI]]`, `[[algorithms]]`, `[[data-science]]` as separate section WikiLinks. This satisfies R-2 bullet points (section heading, WikiLinks from `thoughts`, response/thoughts rendered separately). Rename matches R-2 spec. `(R-2)` provenance comment present. |
| 3 | **KEEP**, harden assertion per R-4 | Hardened `result.contains("missing")` → `result.contains("missing. Start with thoughtNumber=0")` | ✅ **Confirmed.** Diff shows exactly one line changed: `"missing"` → `"missing. Start with thoughtNumber=0"`. Longer substring pins the error message more tightly, satisfying R-4. |
| 4 | **KEEP**, no changes | No changes | ✅ **Confirmed.** No diff touches this test. |
| 5 | **KEEP**, no changes | No changes | ✅ **Confirmed.** No diff touches this test. |
| R-5 | File contract comment | Added file contract comment | ✅ **Confirmed.** 5-line comment block added at top of file: declares error-path ownership, lists valid test scope, and directs happy-path tests to lifecycle file. |

### Coverage Verification

- **No unique coverage lost**: Test 1's full assertion set (13 assertions covering `analysisType`, `completed` key, `thoughtCount: "3"`, 3-thought flow, conclusion `[[modular-design]]` WikiLink) is present verbatim in lifecycle's `session_lifecycle_with_analysis_type_persists_all_frontmatter_fields`.
- **Test 2 strictly improved**: Original had `contains("machine-learning")` (bare text match, not WikiLink); rewrite asserts `contains("[[machine-learning]]")` (WikiLink syntax). Original checked `contains("AI")` (2-char substring, could match noise); rewrite checks `contains("[[AI]]")`. New assertions for `[[algorithms]]` and `[[data-science]]` add coverage the original never had.
- **No unrelated changes**: Commit `2c289b4` touches exactly 1 file (`session_sequential_errors.rs`), and commit `0ad5145` touches exactly 1 file (`session_sequential_lifecycle.rs`). No production code modified.
- **All 10 tests green**: errors (3/3 pass) + lifecycle (7/7 pass) = 10 passing tests across both files.

### Observations (Informational, non-blocking)

- **R-3 (parameterize tests 3 & 4)**: Not addressed. Eval rated this Low and noted "acceptable as separate tests" until a parameterization pattern exists. No concern.
- **Two-commit strategy**: The SWE split work across `0ad5145` (lifecycle file: additions + strengthening) and `2c289b4` (errors file: removals + hardening). This is clean — the lifecycle additions were committed *before* the errors removals, ensuring no intermediate state where coverage was lost.

### Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- session_sequential_errors`: 3 passed, 0 failed
- Verdict: PASS
