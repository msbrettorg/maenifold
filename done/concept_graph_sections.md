---
id: T-80.35
title: "Test Evaluation: concept_graph_sections.rs"
status: done
pipeline: tdd
---

# Test Evaluation: concept_graph_sections.rs

**Evaluator**: Red Team (Gray — Skeptical Inquiry)  
**Traceability**: T-1.6 · NFR-1 · NFR-2  
**File**: `crates/maenifold-core/tests/concept_graph_sections.rs`  
**Oracle Ports**: `ConceptGraphTests.cs::BuildContextExtractsSectionWithConceptTest`, `ConceptGraphTests.cs::ExtractSectionWithConceptEdgeCasesTest`  
**Function Under Test**: `search::extract_section_with_concept` → `graph::preview::extract_section_with_concept`  
**Tests Evaluated**: 2  
**Current Status**: Both passing (`cargo test -- concept_graph_sections`)

---

## Summary

This file covers section-aware content preview extraction — the behavior that ensures `BuildContext` returns a preview anchored to the document section where the queried concept appears, rather than blindly returning the document's opening content.

The [[test-quality]] signal here is **mixed**. Both tests are structurally sound and trace correctly to T-1.6 / NFR-1. However, on close adversarial inspection, **Test 1's assertions are epistemically hollow**: they pass whether `extract_section_with_concept` works correctly *or* is completely broken. Test 2 is the real load-bearing gate for this function, but contains assertion laxity and an unnecessarily borrowed mutex that signal cargo-cult copy-paste from the integration test above it.

The core question Gray must ask: *What does this test actually prevent from breaking?* For Test 1, the honest answer is: **very little**. For Test 2: **most of the unit behavior**, but with weaknesses.

---

## Verdicts

| # | Test Name | Assertions | Load-Bearing? | Unique? | Correct? | Good Name? | Parameterizable? | Verdict |
|---|-----------|-----------|---------------|---------|----------|------------|-----------------|---------|
| 1 | `build_context_extracts_section_with_concept_test` | 3 | Yes (integration path) | Yes | **No — false positive** | Yes | No | **REWRITE** |
| 2 | `extract_section_with_concept_edge_cases_test` | 10 (8 scenarios) | Yes (only unit coverage) | Yes | Mostly, with 2 weaknesses | Partial | **Yes — strongly** | **REWRITE** |

---

## Detailed Findings

### Test 1: `build_context_extracts_section_with_concept_test`

**What it claims to verify**: When `BuildContext` is called with `includeContent=true`, the returned `ContentPreview` contains the *section* where the concept appears, not the document's intro section.

**Fixture setup**: Creates two files with `[[authentication]]` in later sections (a "## Security" section and a "## Overview" section), syncs the graph, calls `build_context_json`, then checks preview values.

**The problem — Assertion 1 (positive check):**

```rust
assert!(
    preview_values
        .iter()
        .any(|preview| { preview.contains("## Security") || preview.contains("JWT tokens") }),
    ...
);
```

The document bodies are:
- *Authentication System Overview*: contains "## Security" and "JWT tokens" in-body
- *JWT Implementation*: contains "JWT tokens" in-body

If `extract_section_with_concept` is broken and returns the **full document body** as a fallback, both strings still appear in the preview. This assertion **cannot distinguish correct section extraction from a broken fallback**.

**The problem — Assertion 2 (negative check):**

```rust
assert!(
    preview_values
        .iter()
        .all(|preview| !preview.starts_with("## Introduction")),
    ...
);
```

The fallback path in `extract_section_with_concept` returns the full body starting at the H1 title (`# Authentication System Overview`), **not** `## Introduction`. So the broken fallback starts with `# Authentication...`, which does not start with `## Introduction`. This assertion also **passes with broken extraction**.

**Net effect**: This integration test would pass even if `load_content_preview_map` ignored the concept entirely and returned the full document body for every preview. The test name promises section extraction behavior but the assertions do not enforce it.

**Correct assertions would be:**

```rust
// Positive: the section header appears at the START of the preview
assert!(
    preview_values
        .iter()
        .any(|preview| preview.starts_with("## Security")),
    "preview should START at the Security section: {context}"
);
// Negative: the intro section should not appear AT ALL in the extracted preview
assert!(
    preview_values
        .iter()
        .all(|preview| !preview.contains("## Introduction")),
    "no preview should include the intro section content: {context}"
);
```

---

### Test 2: `extract_section_with_concept_edge_cases_test`

**What it covers (8 scenarios in one function):**

| Scenario | Input | Assertion |
|----------|-------|-----------|
| 1 | Multi-section, concept in later section | `contains("## Security")` + `contains("[[authentication]]")` |
| 2 | Multi-section, concept in first section | `contains("## Intro")` |
| 3 | H1-only doc, concept mentioned | `contains("[[jwt]]")` |
| 4 | Concept not found | `contains("Title")` (fallback) |
| 5 | Plain-text match (no brackets) | `contains("## Usage")` + `contains("authentication")` |
| 6 | Long section, truncation | `result6.len() <= 200` with `max_length=100` |
| 7 | Empty string | `is_empty()` |
| 8 | Whitespace-only | `trim().is_empty()` |

**Weakness — Scenario 1 assertion:**

```rust
assert!(result1.contains("## Security"), ...);
```

Should be `starts_with("## Security")`. The `contains` check passes even if extraction is broken and returns the full document (which also contains "## Security" somewhere). Scenario 1 is the canonical happy-path test for section selection and deserves the strongest assertion.

**Weakness — Scenario 6 assertion:**

```rust
let result6 = search::extract_section_with_concept(&long_content, "test", 100);
assert!(
    result6.len() <= 200,
    "long sections should be truncated to a compact preview: {}",
    result6.len()
);
```

`max_length=100` with a tolerance bound of `<= 200` is a **2× margin**. The implementation adds at most 3 bytes (`...`) to the truncated output, so the real correct bound is `<= 103`. The 200-byte threshold would allow a partial regression (e.g., truncation at 150 chars) to pass silently. Should be `result6.len() <= 103` or `result6.chars().count() <= 103`.

**Cargo-culted mutex:**

```rust
fn extract_section_with_concept_edge_cases_test() {
    let _guard = TEST_MUTEX.lock();
    // ... pure function calls, no DB, no filesystem
}
```

This test calls `search::extract_section_with_concept` directly — a pure, stateless string transformation function with no database or filesystem access. Holding `TEST_MUTEX` is unnecessary. It serializes this test with all other concept-graph DB tests, adding latency (though small) and introducing a false coupling that implies shared state where none exists. A future maintainer may depend on this lock for protection that doesn't exist.

**Monolith test structure:**

All 8 independent scenarios fail as a block. If scenario 1 panics, scenarios 2–8 never run. This hides the *scope* of regressions. Each scenario tests a distinct code path (WikiLink match, plain-text match, first-section match, fallback, truncation, empty, whitespace) and should be independently observable.

---

## Recommendations

### Priority 1 — Fix Test 1's false-positive assertions (NFR-2 / test integrity)

Replace `contains` with `starts_with` for the positive section check, and replace `starts_with("## Introduction")` with `contains("## Introduction")` for the negative check:

```rust
// Positive: extracted preview must START at the Security section
assert!(
    preview_values
        .iter()
        .any(|preview| preview.starts_with("## Security")),
    "preview should START at the Security section heading: {context}"
);
// Negative: intro section content must be absent entirely
assert!(
    preview_values
        .iter()
        .all(|preview| !preview.contains("## Introduction")),
    "intro section should be excluded from section-targeted preview: {context}"
);
```

This is a correctness fix, not a style preference. Without it, Test 1 provides zero protection against regressions in `extract_section_with_concept`.

### Priority 2 — Tighten truncation bound in Test 2, Scenario 6

```rust
// Was: result6.len() <= 200
// Fix:
assert!(
    result6.chars().count() <= 103,  // max_length(100) + len("...")(3)
    "truncated preview must not exceed max_length + ellipsis: {} chars",
    result6.chars().count()
);
```

### Priority 3 — Remove unnecessary mutex from Test 2

Delete `let _guard = TEST_MUTEX.lock();` from `extract_section_with_concept_edge_cases_test`. It is a pure function with no shared state. Add a comment to document the absence is intentional:

```rust
fn extract_section_with_concept_edge_cases_test() {
    // No mutex needed — pure function, no DB or filesystem access.
```

### Priority 4 — Parameterize / split Test 2 (long-term)

The 8 scenarios in `extract_section_with_concept_edge_cases_test` are independent and should either be:

a) Split into 8 named tests (`extract_section_finds_later_section`, `extract_section_finds_first_section`, `extract_section_h1_only`, `extract_section_fallback_on_missing`, `extract_section_plain_text_match`, `extract_section_truncates_long`, `extract_section_empty_input`, `extract_section_whitespace_input`), or  
b) Driven by a parameterized table using rstest or similar.

**Benefit**: A regression in scenario 1 currently silently prevents scenarios 2–8 from running, hiding the true blast radius.

---

## Positive Observations

- **Fixture isolation**: `isolated_env()` with `TempDir` and `Config::reset_overrides()` in `Drop` is correct and safe. No shared state leaks between tests that hold the mutex.
- **Traceability**: Both tests carry `// T-1.6: RTM NFR-1` annotations that correctly map to the C# oracle tests being ported. This is exactly what NFR-1 compliance looks like.
- **Edge case breadth**: Test 2 covers 8 distinct scenarios (WikiLink match, plain-text match, first-section, later-section, H1-only, fallback, truncation, empty, whitespace). The *breadth* is good; the *precision* of individual assertions needs tightening.
- **Integration depth**: Test 1 correctly exercises the full `write_memory → sync_graph → build_context → ContentPreview` pipeline, which no other test in `concept_graph_context.rs` does for section-specific behavior.
- **Pure function exposure**: The fact that `search::extract_section_with_concept` is re-exported and directly callable in Test 2 is excellent API design — it allows pure unit testing without spinning up a database.

---

## Load-Bearing Score: 1.5/2

| Test | Score | Rationale |
|------|-------|-----------|
| `build_context_extracts_section_with_concept_test` | 0.5/1 | Structurally load-bearing (integration path, unique coverage) but assertions are false positives — it does not actually enforce section extraction correctness |
| `extract_section_with_concept_edge_cases_test` | 1/1 | Genuinely load-bearing for the unit behavior; 8 of 10 assertions are correct; weaknesses are local and fixable without restructuring |

**Overall**: The *intent* is sound. The *execution* has two verifiable assertion failures that make Test 1 essentially inert as a regression guard. These are rewrite-priority issues, not style concerns.
