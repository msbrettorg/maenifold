---
id: T-80.75
title: "Test Evaluation: graph_analyzer_visualize_behavior.rs"
status: done
pipeline: tdd
---

# Test Evaluation: graph_analyzer_visualize_behavior.rs

**File**: `crates/maenifold-core/tests/graph_analyzer_visualize_behavior.rs`
**Traceability**: T-1.6 / NFR-1
**Evaluator color**: 🔘 Gray — Skeptical Inquiry
**Sibling file**: `graph_analyzer_visualize_validation.rs` (7 tests, reviewed for overlap)

---

## Summary

This file contains **4 tests** covering the happy-path and parametric behavior of the `visualize`
tool. All tests share identical setup via `seed_graph`, use the same target concept (`concept-a`),
and call `visualize` from the `graph_analyzer_support` harness. The validation cases (invalid
input, missing concept, boundary values) live in the sibling file.

The file is well-scoped in intent. The problems are almost entirely about **assertion
strength** and **test granularity**: two tests are doing effectively the same work on the same
data, one assertion is logically weaker than the data guarantees, and the normalization test
fails to assert the one thing it purports to prove.

**Assumptions questioned during review:**

- *"OR is fine because at least one neighbor is enough."* — No. The seed guarantees both
  `concept-b` and `concept-c` are depth-1 neighbors of `concept-a`. An AND assertion is
  warranted and tighter.
- *"Depth 2 should be ≥ depth 1."* — The seed deterministically produces **strictly more**
  edges at depth 2. Using `>=` permits a regression where depth 2 collapses to the same
  output as depth 1 without failing.
- *"Three variants each produce `graph TD`, therefore normalization works."* — Each passing
  independently does not prove they produce **identical** output. That is the normalization
  claim.

---

## Verdicts

| # | Test name | Load-bearing? | Unique? | Assertions correct? | Good name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `visualize_with_valid_concept_returns_mermaid_diagram` | ✅ | ⚠️ | ⚠️ | ✅ | No | **REWRITE** |
| 2 | `visualize_includes_co_occurrence_counts` | ⚠️ | ✅ | ✅ | ✅ | No | **MERGE → 1** |
| 3 | `visualize_respects_depth_parameter` | ✅ | ✅ | ❌ | ✅ | No | **REWRITE** |
| 4 | `visualize_normalizes_concept_names` | ✅ | ✅ | ❌ | ✅ | Yes | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `visualize_with_valid_concept_returns_mermaid_diagram`

**Purpose**: Primary happy-path. Valid concept, connected graph, valid parameters → valid
Mermaid output.

**Problems:**

1. **`!result.is_empty()` is redundant.** If `starts_with("graph TD")` passes, the string
   cannot be empty. The assertion adds zero signal and creates the impression of coverage
   it does not provide.

2. **`contains("concept_b") || contains("concept_c")` is logically weak.** The seed
   inserts two edges from `concept-a` at depth 1: `concept-a → concept-b` (weight 2) and
   `concept-a → concept-c` (weight 2). Both neighbors are **guaranteed** to appear at
   depth 1. The `||` means the test would pass even if one neighbor were silently omitted —
   a real regression in graph traversal.  
   **Should be:** `contains("concept_b") && contains("concept_c")`

3. **Overlap with `visualize_generates_valid_mermaid_node_ids` in sibling file.** That
   test also asserts `starts_with("graph TD")` and `-->` edge syntax, and goes further with
   a regex. The basic structural assertions here are subsumed.

4. **Missing co-occurrence assertion.** Test 2 exists solely to add the `-->|2|` check.
   This assertion belongs here (or the two tests should be merged here).

**Evidence quality**: Medium. Passes for the right reasons but would not catch several
real regressions.

---

### Test 2 — `visualize_includes_co_occurrence_counts`

**Purpose**: Verify that edge labels include the seeded co-occurrence count (`-->|2|`).

**Problems:**

1. **Single-assertion test on identical setup to Test 1.** The test uses the same
   `isolated_config`, `seed_graph`, and `visualize` call as Test 1 with the same parameters.
   The only difference is one additional assertion.  
   Two separate test functions, two database setups, two `visualize` calls, one mutex
   acquisition — for one extra `assert!`. This is costly for zero structural benefit.

2. **The assertion is correct and unique.** `contains("-->|2|")` verifies the seeded weight
   value appears in output. The sibling file's regex (`\w+ -->\|\d+\| \w+`) only confirms
   *some* digit is present; this test confirms the *actual value* 2. That is a meaningful
   distinction worth preserving.

3. **Should be merged into Test 1** (or a single richer happy-path test), not live as a
   standalone function.

**Evidence quality**: The assertion itself is strong. The separation is the waste.

---

### Test 3 — `visualize_respects_depth_parameter`

**Purpose**: Verify that `depth=2` produces more output than `depth=1`.

**Problems:**

1. **`>=` is the wrong comparator.** The seed graph is deterministic and asymmetric at
   depth 2:
   - depth 1 from `concept-a`: 2 edges (a→b, a→c) → 3 lines (`graph TD` + 2 edge lines)
   - depth 2 from `concept-a`: 4 edges (a→b, a→c, b→d, c→e) → 5 lines

   The `>=` operator permits a regression where depth 2 produces *identical* line count to
   depth 1 without failing. The data guarantees `>` is always true. Using `>=` is not
   conservative safety — it is a false claim of verification.  
   **Should be:** `assert!(depth_two_lines > depth_one_lines, ...)`

2. **Line count is an indirect proxy.** Empty lines, whitespace trimming, the header line —
   all are included in the count. Asserting on the number of *edges* (lines containing `-->`)
   or the presence of depth-2-specific concepts (`concept-d`, `concept-e`) would be more
   direct and less brittle to output formatting changes.

3. **No assertion on what depth 1 actually contains.** The test is purely relative
   (depth 2 ≥ depth 1). It would pass even if both outputs were `"graph TD\n"` (both 1
   non-empty line). A lower-bound assertion on depth 1 content would anchor the comparison.

**Evidence quality**: Low. The test structure is correct but the comparator is too weak to
catch the regression it is meant to catch.

---

### Test 4 — `visualize_normalizes_concept_names`

**Purpose**: Verify that `concept-a`, `CONCEPT-A`, and `Concept-A` all resolve to the same
concept.

**Problems:**

1. **The critical assertion is missing.** Each variant is only checked for
   `starts_with("graph TD")`. This proves each call *succeeded*, not that normalization
   *worked*. The normalization claim is that all three variants produce **identical output**.
   A test for normalization must assert equivalence:
   ```rust
   assert_eq!(lower, upper, "uppercase must normalize to same output");
   assert_eq!(lower, mixed, "mixed case must normalize to same output");
   ```
   As written, the test would pass even if uppercase returned a completely different (but
   non-empty) graph than lowercase.

2. **Three full `visualize` calls with three mutex guards** where one call + equality check
   would be equivalent and faster.

3. **Good candidate for parameterization** via a small loop over variants, asserting each
   equals the reference result. Would eliminate the three near-identical call blocks.

**Evidence quality**: Low. Passes for the wrong reason. Does not verify the stated behavior.

---

## Recommendations

### P0 — Fix incorrect assertion (test 3)

Change `>=` to `>` in `visualize_respects_depth_parameter`. The seed data guarantees this
always holds; `>=` is an incorrect weakening that masks regressions.

```rust
// Before
assert!(depth_two_lines >= depth_one_lines, ...);

// After
assert!(
    depth_two_lines > depth_one_lines,
    "depth 2 must produce strictly more lines than depth 1:\n{depth_one}\n---\n{depth_two}"
);
```

### P0 — Fix missing equivalence assertion (test 4)

Add equality assertions to `visualize_normalizes_concept_names`:

```rust
assert_eq!(
    lower, upper,
    "uppercase variant should produce identical output to lowercase"
);
assert_eq!(
    lower, mixed,
    "mixed-case variant should produce identical output to lowercase"
);
```

### P1 — Merge test 2 into test 1

Fold the `contains("-->|2|")` assertion into the happy-path test and delete the standalone
`visualize_includes_co_occurrence_counts` function. One database setup, one `visualize` call,
all structural assertions together.

### P1 — Strengthen test 1 neighbor assertion

Replace the `||` with `&&`:

```rust
// Before
assert!(
    result.contains("concept_b") || result.contains("concept_c"),
    ...
);

// After
assert!(
    result.contains("concept_b") && result.contains("concept_c"),
    "expected both depth-1 neighbors in diagram: {result}"
);
```

And drop the redundant `!result.is_empty()` check.

### P2 — Improve depth test specificity (test 3)

Assert on concepts known to appear at depth 2 but not depth 1:

```rust
assert!(
    depth_two.contains("concept_d") && depth_two.contains("concept_e"),
    "depth 2 should include depth-2 concepts d and e: {depth_two}"
);
assert!(
    !depth_one.contains("concept_d") && !depth_one.contains("concept_e"),
    "depth 1 should not include depth-2 concepts: {depth_one}"
);
```

This is more direct than counting lines and more resilient to output formatting changes.

### P3 — Parameterize test 4

Replace three repeated calls with a loop:

```rust
let reference = visualize(&cfg, json!({ "conceptName": "concept-a", ... }));
for variant in ["CONCEPT-A", "Concept-A", "concept-a"] {
    let result = visualize(&cfg, json!({ "conceptName": variant, ... }));
    assert_eq!(result, reference, "variant '{variant}' should normalize to same output");
}
```

---

## Load-Bearing Score: 3/4

| Test | Load-bearing? | Notes |
|------|:---:|-------|
| `visualize_with_valid_concept_returns_mermaid_diagram` | ✅ | Primary happy path; assertions need strengthening |
| `visualize_includes_co_occurrence_counts` | ⚠️ | Assertion is load-bearing; test as standalone function is not — merge |
| `visualize_respects_depth_parameter` | ✅ | Unique coverage of depth param; comparator is wrong |
| `visualize_normalizes_concept_names` | ✅ | Unique coverage of normalization; misses the actual claim |

**3 of 4 tests carry genuinely unique behavioral coverage** (tests 1, 3, 4). Test 2 carries a
unique assertion but does not justify its existence as a separate function. After the merges
and rewrites above, the file should contain **3 well-formed tests** with correct, tight
assertions — down from 4 loose ones.

The sibling file (`graph_analyzer_visualize_validation.rs`) is in better shape and does not
require changes as part of these recommendations.
