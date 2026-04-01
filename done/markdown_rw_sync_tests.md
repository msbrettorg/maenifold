---
id: T-80.98
title: "Test Evaluation: markdown_rw_sync_tests"
status: done
pipeline: tdd
---

# Test Evaluation: markdown_rw_sync_tests
**File:** `crates/maenifold-core/tests/markdown_rw_sync_tests.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-12.7.1 | RTM: NFR-1 (C# Oracle Parity), NFR-2 (Test Coverage)

---

## Summary

The file contains 7 tests covering the sync pipeline for markdown files: frontmatter
parsing, wikilink extraction, concept counting, code-block/inline-code exclusions, and
edge cases (no frontmatter, empty file). All 7 tests pass. The broad intent is sound, but
two structural problems recur across the file and justify three REWRITEs and one REMOVE.

**Problem 1 — Brittle output-string assertions (T5, T6):**
`output.contains("2 concept mentions updated")` and `"0 concept mentions updated"` hard-code
message format and English text. A message phrasing change breaks these tests without any
behavioral regression. The DB helper `mention_count()` already exists in the support module
and should be used instead.

**Problem 2 — Kitchen-sink concerns in T1:**
`sync_basic_file_frontmatter_and_content_match_expected_processing` calls
`extract_wikilinks_with_sections` and `count_all_concept_occurrences` directly inside the
test body — in addition to calling `sync_one` and verifying DB state. These helpers are
already unit-tested in `proximity_weight_sections.rs` (5 tests) and `markdown_rw_extract_tests.rs`
respectively. The redundant helper assertions blur the failure signal: a `sync_one` bug and
a `count_all_concept_occurrences` bug produce identical test failures. The test name
"frontmatter_and_content_match_expected_processing" does not reflect the DB-side-effect
verification that is its most valuable assertion.

**Problem 3 — T2 is outclassed elsewhere:**
`sync_wikilinks_with_sections_section_indices_match_expected_extraction` tests
`extract_wikilinks_with_sections` with weaker `contains()` assertions. `proximity_weight_sections.rs`
provides 5 dedicated tests for the same function using exact-match via `BTreeMap` collection,
including the cross-section repetition and preamble-zero scenarios T2 covers. T2 adds no
uniquely exercised code path and its assertions are strictly weaker.

**Positive observations:**
- Naming convention is consistent and descriptive (`sync_<domain>_<expectation>`).
- Mutex isolation via `TEST_MUTEX` is correctly applied to tests that touch global state and
  omitted from pure-unit tests (T2, T3) — this is correct and intentional.
- T4 and T7 (fenced code block and inline code exclusions) are distinct, non-overlapping
  scenarios that warrant separate tests; both are correctly structured.
- The support module (`markdown_rw_support`) is well-factored with reusable DB helpers
  (`file_concepts`, `mention_count`). The issue is that they are not always used when they should be.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `sync_basic_file_frontmatter_and_content_match_expected_processing` | REWRITE | Kitchen sink: tests `extract_wikilinks_with_sections` + `count_all_concept_occurrences` inline, redundant with `proximity_weight_sections.rs` / `markdown_rw_extract_tests.rs`. Name misrepresents scope. Strip helper assertions; keep frontmatter read + DB sync verification. |
| `sync_wikilinks_with_sections_section_indices_match_expected_extraction` | REMOVE | Tests `extract_wikilinks_with_sections` with weaker `contains()` assertions. `proximity_weight_sections.rs` provides 5 exhaustive exact-match tests for the same function, including the preamble-zero and cross-section repetition scenarios exercised here. Zero unique coverage. |
| `sync_concept_counts_match_expected_occurrences` | KEEP | Sole test for `count_all_concept_occurrences` (multi-concept batch variant). Pure unit test, correct exact-match assertions, no overlap with `markdown_rw_extract_tests.rs` (which tests the single-concept `count_concept_occurrences`). |
| `sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks` | KEEP | Load-bearing exclusion parity test. Exercises the full sync pipeline; correctly asserts concept presence and absence in the DB. Minor gap: does not verify `mention_count` is exactly 2, but the presence/absence checks are sufficient for regression detection. |
| `sync_no_frontmatter_content_only_file_is_processed` | REWRITE | Brittle output-string assertion `output.contains("2 concept mentions updated")`. Replace with `assert_eq!(mention_count(&env.cfg, &uri), 2)` using the existing DB helper. The string-matching asserts message phrasing, not behavior. |
| `sync_empty_file_is_handled_gracefully` | REWRITE | Two issues: (1) `!checksum.is_empty()` is trivially true — SHA of empty string is always non-empty, so this assertion can never catch a regression; (2) brittle `output.contains("0 concept mentions updated")` should be replaced with `assert_eq!(mention_count(&env.cfg, &uri), 0)`. |
| `sync_inline_code_exclusion_ignores_wikilinks_in_backticks` | KEEP | Correctly tests a distinct exclusion path from T4 (backticks vs fenced blocks). Same minor gap as T4 on `mention_count`, acceptable for the exclusion-focused scope. |

---

## Per-Test Analysis

### T1 — `sync_basic_file_frontmatter_and_content_match_expected_processing`

**Verdict: REWRITE**

**What it does well:** The final assertions — `file_concepts`, `mention_count`, and the `sync_one` output — are genuinely load-bearing integration checks that belong here.

**What it does poorly:** Calls `extract_wikilinks_with_sections` and `count_all_concept_occurrences` as *intermediate assertion steps*, not as setup. This turns a sync integration test into a joint test of 3 independent components. Cross-referencing:
- `extract_wikilinks_with_sections` is tested by 5 tests in `proximity_weight_sections.rs` with stronger assertions.
- `count_all_concept_occurrences` is tested in T3 (same file) with cleaner isolation.

The name `frontmatter_and_content_match_expected_processing` implies it is testing reading; the most valuable part is verifying DB state after `sync_one`.

**Proposed rewrite:** Remove the `extract_wikilinks_with_sections` and `count_all_concept_occurrences` calls and their assertions. Rename to `sync_basic_file_db_state_matches_expected_after_sync`. Keep: `read_markdown` assertions (title, content, checksum), `sync_one` call, `file_concepts`, and `mention_count` verification.

---

### T2 — `sync_wikilinks_with_sections_section_indices_match_expected_extraction`

**Verdict: REMOVE**

**Evidence of overlap:** `proximity_weight_sections.rs` contains:
- `extract_wikilinks_with_sections_keeps_preamble_in_section_zero` — preamble at index 0 ✓
- `extract_wikilinks_with_sections_deduplicates_per_section_not_globally` — concept in multiple sections ✓  
- `extract_wikilinks_with_sections_tags_multi_section_documents` — multi-section indexing ✓

T2's scenario (preamble concept at 0, alpha at 1 and 2) is the composition of scenarios already tested individually in `proximity_weight_sections.rs`. It adds no new covered branch.

**Assertion quality gap:** T2 uses `concepts_with_sections.contains(&(...))` which does not detect phantom extra entries. `proximity_weight_sections.rs` collects into `BTreeMap` or `Vec` and asserts exact values.

**Note on naming:** "sync_wikilinks..." implies sync pipeline involvement; there is none — this is a pure unit call to `extract_wikilinks_with_sections`. The name reinforces that this test was placed in the wrong file.

---

### T3 — `sync_concept_counts_match_expected_occurrences`

**Verdict: KEEP**

`count_all_concept_occurrences` (multi-concept batch) is distinct from `count_concept_occurrences` (single). The former is not tested in `markdown_rw_extract_tests.rs`. Assertions are exact (`["alpha"] == 3`, `["beta"] == 2`). No filesystem or DB interaction — clean unit scope. Naming is accurate.

**Parameterization opportunity:** Could be table-driven with different (content, concepts, expected_counts) triples, but current scope is sufficient as a smoke test.

---

### T4 — `sync_code_block_exclusion_ignores_wikilinks_in_fenced_code_blocks`

**Verdict: KEEP**

Exercises the full sync pipeline with an exclusion scenario. Verifies DB output (not just parser output), which is the right level of validation for a file called `_sync_tests`. Exclusion assertions (`contains` / `!contains` on the `BTreeSet`) are clear and correct.

**Minor gap:** Does not assert `mention_count == 2`. Adding `assert_eq!(mention_count(&env.cfg, &uri), 2)` would make it airtight.

---

### T5 — `sync_no_frontmatter_content_only_file_is_processed`

**Verdict: REWRITE**

The `frontmatter.is_none()`, `content.contains("[[some-concept]]")`, `file_concepts` assertions, and `!checksum.is_empty()` are all sound. The single problem is:

```rust
assert!(output.contains("2 concept mentions updated"));
```

This asserts English message format. If the output string ever changes to `"Updated 2 mentions"` or `"2 mentions indexed"`, the test fails with no behavioral regression. The `mention_count()` DB helper already exists to assert the same fact structurally:

```rust
assert_eq!(mention_count(&env.cfg, &uri), 2);
```

Drop the output string assertion; use `mention_count`.

---

### T6 — `sync_empty_file_is_handled_gracefully`

**Verdict: REWRITE**

Two issues:

1. **Trivially-true assertion:** `assert!(!checksum.is_empty())`. The SHA-256/MD5 of an empty byte slice is always a non-empty hex string. This assertion can never fail — it provides zero regression protection and creates false confidence about checksum behavior.

2. **Brittle string assertion:** `output.contains("0 concept mentions updated")` — same problem as T5.

**Proposed rewrite:**
```rust
// Remove: assert!(!checksum.is_empty());         <- trivially true
// Remove: assert!(output.contains("0 concept mentions updated"));
assert_eq!(mention_count(&env.cfg, &uri), 0);    // <- structural
assert!(file_concepts(&env.cfg, &uri).is_empty()); // <- already present, keep
```

The implicit pass condition of "sync_one does not panic" is actually the most important assertion for an empty file and does not need to be stated explicitly — it is already enforced by the `expect("sync single file")` in the `sync_one` helper.

---

### T7 — `sync_inline_code_exclusion_ignores_wikilinks_in_backticks`

**Verdict: KEEP**

Inline code exclusion (single backtick spans) is a distinct code path from fenced code block exclusion (T4). Both tests are needed. Structure mirrors T4 correctly. Same minor gap: adding `assert_eq!(mention_count(&env.cfg, &uri), 2)` would close the one hole.

---

## Recommendations

**Priority 1 — Remove brittle output-string assertions (T5, T6):**
Replace `output.contains("N concept mentions updated")` with `assert_eq!(mention_count(...), N)`.
The support module already provides this helper. This is a one-line change per test with zero
behavior change but eliminates fragility against message phrasing changes.

**Priority 2 — Rewrite T1 to focus on sync DB-state:**
Strip the intermediate `extract_wikilinks_with_sections` and `count_all_concept_occurrences`
assertions. Let T3 and `proximity_weight_sections.rs` own those units. T1 becomes a clean
integration test: write fixture → sync → assert DB state.

**Priority 3 — Remove T2:**
The coverage is fully owned by `proximity_weight_sections.rs` with stronger assertions.
Removing T2 reduces the test surface to maintain with no reduction in regression detection.

**Priority 4 — Minor gap closure (T4, T7):**
Add `mention_count` assertion to T4 and T7 to verify the exclusion not only prevents
DB insertion but does not silently increment the count for excluded concepts.

**Parameterization (optional):**
T3 is the strongest candidate for table-driven parameterization (different content strings,
different concept/count combinations). T4 and T7 could be extended with additional exclusion
patterns (indented code blocks, tilde fences) but this is low priority given existing
adversarial coverage in `markdown_adversarial_code.rs`.

---

## Load-Bearing Score: 6/7

| Test | Load-Bearing? | Notes |
|---|---|---|
| T1 `sync_basic_...` | ✅ Yes | DB-state assertions are load-bearing; helper assertions are redundant noise |
| T2 `sync_wikilinks_with_sections_...` | ❌ No | Fully covered by `proximity_weight_sections.rs` with stronger assertions |
| T3 `sync_concept_counts_...` | ✅ Yes | Sole test for `count_all_concept_occurrences` |
| T4 `sync_code_block_exclusion_...` | ✅ Yes | Unique exclusion path, DB-state verified |
| T5 `sync_no_frontmatter_...` | ✅ Yes | Distinct no-frontmatter path; brittle but functionally load-bearing |
| T6 `sync_empty_file_...` | ✅ Yes | Important edge case; trivial assertion lowers signal quality |
| T7 `sync_inline_code_exclusion_...` | ✅ Yes | Distinct from T4; unique inline-code exclusion path |

---
## SWE Remediation
- REWRITE T1 (`sync_basic_file_frontmatter_and_content_match_expected_processing`): stripped redundant `extract_wikilinks_with_sections` + `count_all_concept_occurrences` inline assertions (covered by `proximity_weight_sections.rs` and T3). Renamed to `sync_basic_file_db_state_matches_expected_after_sync`. Kept read_markdown + sync_one + DB state verification per P2.
- REMOVE T2 (`sync_wikilinks_with_sections_section_indices_match_expected_extraction`): confirmed fully covered by `proximity_weight_sections.rs` with stronger exact-match assertions per P3.
- KEEP T3: no changes.
- KEEP T4: added `assert_eq!(mention_count(&env.cfg, &uri), 2)` per P4 gap closure.
- REWRITE T5 (`sync_no_frontmatter_content_only_file_is_processed`): replaced brittle `output.contains("2 concept mentions updated")` with `assert_eq!(mention_count(&env.cfg, &uri), 2)` per P1.
- REWRITE T6 (`sync_empty_file_is_handled_gracefully`): removed trivially-true `!checksum.is_empty()` and brittle `output.contains("0 concept mentions updated")`, added `assert_eq!(mention_count(&env.cfg, &uri), 0)` per P1.
- KEEP T7: added `assert_eq!(mention_count(&env.cfg, &uri), 2)` per P4 gap closure.
- Removed unused `BTreeMap` import and `extract_wikilinks_with_sections` import.
- Test count: 7 → 6 (T2 removed)
- All 6 tests pass.
- Commit: 6fbb3d2

---
## Red-team Audit
- ✅ REWRITE T1 (`sync_basic_file_db_state_matches_expected_after_sync`): matches eval spec. Redundant `extract_wikilinks_with_sections` + `count_all_concept_occurrences` calls and all associated assertions removed. Rename applied. `read_markdown` + `sync_one` + DB-state assertions (`file_concepts`, `mention_count`) retained. `BTreeSet::from` literal replaces the now-removed derived-from-concepts variable — semantically equivalent.
- ✅ REMOVE T2 (`sync_wikilinks_with_sections_section_indices_match_expected_extraction`): confirmed removed. Verified `proximity_weight_sections.rs` covers all T2 scenarios — `keeps_preamble_in_section_zero` (exact-match via `BTreeMap`) covers preamble-at-0, `deduplicates_per_section_not_globally` covers cross-section repetition, `tags_multi_section_documents` covers multi-section indexing. All with stronger assertions than T2's `contains()`. No unique coverage lost.
- ✅ KEEP T3: no changes. Correct.
- ✅ KEEP T4 + P4 gap closure: `assert_eq!(mention_count(&env.cfg, &uri), 2)` added per eval recommendation.
- ✅ REWRITE T5: brittle `output.contains("2 concept mentions updated")` replaced with `assert_eq!(mention_count(&env.cfg, &uri), 2)`. Output variable correctly prefixed with `_`. All other assertions preserved (frontmatter, content, checksum, file_concepts).
- ✅ REWRITE T6: trivially-true `!checksum.is_empty()` removed, checksum variable correctly changed to `_checksum`. Brittle output string assertion replaced with `assert_eq!(mention_count(&env.cfg, &uri), 0)`. `file_concepts(...).is_empty()` retained.
- ✅ KEEP T7 + P4 gap closure: `assert_eq!(mention_count(&env.cfg, &uri), 2)` added per eval recommendation.
- ✅ Import cleanup: `BTreeMap` and `extract_wikilinks_with_sections` removed — necessary consequence of T1 rewrite and T2 removal. No extraneous changes.
- ✅ Test count 7 → 6 confirmed. All 6 pass. No unrelated changes introduced (commit touches exactly 1 file).
- ✅ P0 (no P0 in this eval): N/A. P1 (brittle strings): addressed in T5, T6. P2 (T1 kitchen-sink): addressed. P3 (T2 removal): addressed. P4 (mention_count gaps): addressed in T4, T7.
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- markdown_rw_sync`: 6 passed, 0 failed
- Verdict: PASS
