---
id: proximity_weight_sections
status: done
---

# Test Evaluation: proximity_weight_sections

**File:** `crates/maenifold-core/tests/proximity_weight_sections.rs`
**Test count:** 10
**Evaluated:** 2026-03-14
**Traceability:** T-4.3 / RTM FR-23.1, FR-23.2
**Reviewer stance:** Gray (skeptical inquiry) — demand evidence, challenge assumptions.

---

## Summary

The strongest of the three evaluated files. The 10 tests divide cleanly into two layers: 5 pure-function unit tests of `extract_wikilinks_with_sections` and 5 integration tests that run the full sync pipeline and query the database for resulting edge weights. Every test asserts a **distinct behavioral contract**. All 10 are load-bearing. The primary criticisms are architectural (integration tests depend on global config state + a file-lock serialization workaround) and structural (pure-function tests would benefit from parameterization).

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `extract_wikilinks_with_sections_tags_multi_section_documents` | KEEP | Unique: baseline case — multiple H2 sections receive distinct 1-based indices. Foundational. |
| `extract_wikilinks_with_sections_uses_section_zero_for_flat_docs` | KEEP | Unique: flat docs (no H2) stay in section 0. Non-trivial — section 0 has special weight semantics downstream. |
| `extract_wikilinks_with_sections_keeps_preamble_in_section_zero` | KEEP | Unique: preamble content BEFORE the first H2 retains section 0 when the document also has named sections. Distinct from the flat-doc case. |
| `extract_wikilinks_with_sections_deduplicates_per_section_not_globally` | KEEP | Unique: concept appearing in multiple sections produces N entries (one per section), not 1 entry. This is a subtle and load-bearing behavior. |
| `extract_wikilinks_with_sections_reads_wikilinks_inside_h2_headings` | KEEP | Unique: WikiLinks **inside the H2 heading line itself** are captured and assigned to that section. Non-obvious edge case worth pinning. |
| `sync_same_section_pairs_store_weight_one` | KEEP | Unique: same-section co-occurrence → weight `1.0`. This is one half of the core proximity-weight contract. |
| `sync_cross_section_pairs_store_weight_half` | KEEP | Unique: cross-section co-occurrence → weight `0.5`. The other half of the core contract. |
| `sync_flat_docs_store_half_weight_for_all_pairs` | KEEP | Unique: flat-doc pairs (all in section 0) → weight `0.5`. This is NOT redundant with `sync_cross_section_pairs` — it tests that section 0 is treated as a "weak" section rather than a "strong" same-section match. |
| `sync_prefers_named_section_overlap_over_preamble_overlap` | KEEP | Unique: concept appearing in BOTH preamble (section 0) AND a named section — verifies that the highest-weight section overlap wins, producing `1.0` not `0.5`. Non-obvious behavior that could easily regress. |
| `sync_serializes_source_files_as_weight_dictionary` | KEEP | Unique: the `source_files` database column is a JSON dictionary `{uri: weight}`, not a list or a scalar. This pins the serialization contract which callers (e.g., `BuildContext`) depend on. |

---

## Deeper Concerns

### 1. Integration tests depend on a file-lock serialization workaround

Tests 6–10 call `invoke_sync()` which mutates global `Config::override_root()` state. Parallelism is controlled by a file-level lock (`maenifold-rs-proximity-weight-tests.lock`) rather than by test isolation. This is a design smell: the global config state leaks across tests, and if a test panics while holding the lock, subsequent tests will spin for 30 seconds before panicking. Consider:

- Moving toward an explicit `TestEnv`-scoped `ToolRegistry` instance rather than a global
- Or using `#[serial]` attribute from the `serial_test` crate to express the constraint more clearly than a DIY file-lock

### 2. `invoke_sync()` is a black-box — failures are hard to diagnose

Tests 6–10 assert `sync.contains("Sync complete:")`, then query the database. If sync fails silently or the assert string changes, the test continues and fails at the DB assertion with a confusing message. Consider asserting the sync output more specifically (sync count, any errors) before relying on DB state.

### 3. Tests 1–5 are parameterizable

All five pure-function tests call `extract_wikilinks_with_sections(content)` and inspect the result. They follow the same structure: provide content, assert section assignments. While the cases are genuinely distinct, a single `rstest`-parameterized test would reduce boilerplate while keeping the coverage identical.

### 4. No test covers empty content or content with only a title

`extract_wikilinks_with_sections("")` and `extract_wikilinks_with_sections("# Title\n")` (H1 only, no concepts) are untested. The function should return an empty vec; pinning this prevents accidental panic on edge input.

### 5. No test covers H3+ headings

The implementation only splits on `## ` (H2). What happens with `### SubSection`? The current tests don't verify that H3 content is absorbed into the current H2 section's body. This is a gap if the corpus contains multi-level headings.

---

## Recommendations

1. **ADD** test: `extract_wikilinks_with_sections_ignores_h3_headings` — verify that H3 content is attributed to the surrounding H2 section, not its own section.
2. **ADD** test: `extract_wikilinks_with_sections_empty_content_returns_empty_vec` — pin the empty-input edge case.
3. **CONSIDER** replacing the DIY file-lock in `support.rs` with `serial_test::serial` attribute — the intent is clearer and the 30-second timeout is surprising behavior.
4. **CONSIDER** parameterizing tests 1–5 with `rstest` — same coverage, less boilerplate.
5. **ADD** a sync integration test that verifies weight behavior when the **same pair appears across multiple files** — the current tests all use a single source file per pair.

---

## Load-Bearing Score: 10/10

All 10 tests are genuinely load-bearing. Each covers a distinct and non-obvious behavioral contract. Removing any single test would leave a real regression path open.

---
## SWE Remediation
- KEEP all 10 original tests: unchanged per verdicts
- ADD `extract_wikilinks_with_sections_ignores_h3_headings` (R1): verifies H3 content is attributed to surrounding H2 section, not its own section
- ADD `extract_wikilinks_with_sections_empty_content_returns_empty_vec` (R2): pins empty-input and title-only edge cases returning empty vec
- ADD `sync_same_pair_across_multiple_files_aggregates_source_weights` (R5): verifies same concept pair across two files produces source_files dict with both URIs and correct per-file weights (1.0 same-section, 0.5 cross-section)
- SKIPPED R3 (serial_test crate): P3+ CONSIDER — would add a dependency for marginal clarity improvement
- SKIPPED R4 (rstest parameterization): P3+ CONSIDER — would add a dependency; existing tests are clear and load-bearing as-is
- Test count: 10 → 13 (all passing)
- Commit: 3a48cf1

---
## Red-team Audit
- ✅ KEEP (10 original tests): all 10 byte-identical to pre-commit state (diff of first 210 lines: zero delta). No modifications, no reorders.
- ✅ ADD `extract_wikilinks_with_sections_ignores_h3_headings` (R1): verifies H3 content absorbed into parent H2 section index, not promoted to its own. Asserts 3 concepts in section 1 and 1 concept in section 2. Matches eval spec exactly.
- ✅ ADD `extract_wikilinks_with_sections_empty_content_returns_empty_vec` (R2): covers both empty string AND title-only (`# Title\n`) edge cases in a single test. Both were called out in eval Deeper Concern §4.
- ✅ ADD `sync_same_pair_across_multiple_files_aggregates_source_weights` (R5): creates two files with alpha/beta pair — same-section (1.0) and cross-section (0.5) — asserts `source_files` JSON dict contains both URIs with correct per-file weights. Matches eval recommendation exactly.
- ✅ SKIP R3 (serial_test crate): correctly classified as P3+ CONSIDER. No dependency added.
- ✅ SKIP R4 (rstest parameterization): correctly classified as P3+ CONSIDER. No dependency added.
- ✅ Traceability: all 3 new tests tagged `T-4.3: RTM FR-23.1` (pure-function) or `T-4.3: RTM FR-23.2` (integration), with eval recommendation number (R1/R2/R5).
- ✅ Commit scope: 3a48cf1 touches exactly 1 file, 99 lines appended. No unrelated changes.
- ✅ All 13 tests pass (`cargo test -p maenifold-core --test integration_tests proximity_weight_sections -- --test-threads=1`): 13 passed, 0 failed.
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- proximity_weight_sections`: 13 passed, 0 failed
- Verdict: PASS
