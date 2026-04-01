---
id: T-80.15
title: "Test Evaluation: benchmark_boundary_controls.rs"
status: done
pipeline: tdd
---

# Test Evaluation: benchmark_boundary_controls.rs

**Traceability**: T-12.7 · RTM-032 · RTM-033 · RTM-034 · NFR-1  
**Evaluator color**: 🔘 Gray — Skeptical Inquiry  
**Scope**: `crates/maenifold-core/tests/benchmark_boundary_controls.rs` (4 `#[test]` functions)  
**Companion files read**: `benchmark_boundary/support.rs`, `src/benchmark/mod.rs`, `src/benchmark/graph.rs`, `src/benchmark/render.rs`, `benchmark_boundary_output.rs`

---

## Summary

The file tests **control-path behavior** of the `RunFullBenchmark` tool: parameter echo in the header, the `includeDeepTraversal` gate, and input validation. It was split from the broader output boundary suite (T-12.7) to maintain a focused concern.

All four tests are **load-bearing** — they each guard a distinct behavioral contract. However, two have correctness problems and one is a merge candidate. The most significant defect is in the deep-traversal test: the positive-case branch asserts only that a skip message is *absent*, never that traversal actually *ran*. A silent regression (e.g., traversal exits early due to empty concept results) would pass this test undetected.

A second gap exists outside this file: the upper-bound validation (`iterations > 100 → error`) has **no test anywhere** in the test suite.

---

## Verdicts

| # | Test Name | RTM | Load-Bearing? | Unique? | Assertions Correct? | Name Clear? | Parameterizable? | Verdict |
|---|-----------|-----|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `benchmark_iterations_parameter_reflected_in_output` | RTM-032 / TC-5.2 | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠ Merge candidate | **MERGE** with #2 |
| 2 | `benchmark_max_test_files_reflected_in_output` | RTM-033 / TC-5.3 | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠ Merge candidate | **MERGE** with #1 |
| 3 | `benchmark_exclude_deep_traversal_skips_complex_section_content` | RTM-034 / TC-5.4 | ✅ Yes | ✅ Yes | ⚠ Partial | ❌ Misleading | ✅ No | **REWRITE** |
| 4 | `benchmark_iterations_zero_returns_error` | RTM-032 / TC-5.2 | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠ Add companion | **KEEP** |

---

## Detailed Findings

### Test 1 — `benchmark_iterations_parameter_reflected_in_output`

**What it does**: Invokes benchmark with `iterations: 1` and again with `iterations: 3`, asserts both values appear in the output header string.

**Evidence checked**: The source format string at `benchmark/mod.rs:83` is:
```rust
"Iterations: {iterations}, Max Test Files: {max_test_files}\n"
```
The assertion targets `"Iterations: 1"` and `"Iterations: 3"` — both are correct substrings.

**Strengths**:
- Protects against header label rename without output change.
- The dual-value check (1 and 3) proves the number is not hardcoded in the header.

**Weaknesses**:
- `contains()` on a substring is order-insensitive to the rest of the line. If the label became `"Run Iterations: 1"` the test would still pass.
- Two invocations of `isolated_env()` + `invoke_benchmark()` for a check that is structurally identical to test 2. Same fixture, same pattern, different parameter name.

**RTM note**: RTM-032 is also claimed by test 4 (the error path). Two structurally different behaviors mapped to the same RTM entry is a traceability ambiguity.

---

### Test 2 — `benchmark_max_test_files_reflected_in_output`

**What it does**: Same pattern as test 1 but for `maxTestFiles: 42` and `maxTestFiles: 999`.

**Evidence checked**: Same format string. Assertions target `"Max Test Files: 42"` and `"Max Test Files: 999"` — both correct.

**Strengths**:
- Independently guards the `maxTestFiles` parameter echo.
- Values 42 and 999 are not adjacent boundary values — they test the middle of a range.

**Weaknesses**:
- `maxTestFiles` has no upper-bound validation in source (only `.max(0)` to clamp negatives). This test cannot catch a validation regression that doesn't exist. The test is purely an echo/reflection test.
- Pattern is structurally identical to test 1. Both could be a single test:
  ```rust
  fn benchmark_header_parameters_reflected_in_output() { ... }
  ```
  This would reduce fixture overhead (two fewer `TEST_MUTEX.lock()` + `isolated_env()` calls) and clarify that both assertions belong to the same behavioral contract: "the header reflects whatever parameters were passed."

**Verdict basis**: Not REMOVE — the assertion is meaningful. But the duplication of structure with test 1 argues for MERGE.

---

### Test 3 — `benchmark_exclude_deep_traversal_skips_complex_section_content`

**What it does**: Seeds 110 files and 5 hub concepts (each with 10 edges). Runs benchmark with `includeDeepTraversal: false` and then again with `includeDeepTraversal: true`. Asserts the skip/no-skip behavior.

**Evidence checked**:  
- Source at `benchmark/graph.rs:80–88`:
  ```rust
  if !include_deep_traversal {
      output.push_str("Skipped: includeDeepTraversal=false\n\n");
      return Ok(output);
  }
  ```
- `render_complex_traversal_benchmark` always emits the section header before the gate check, so `output.contains("Complex Traversal Bottleneck Analysis")` is always true. ✓

**Strengths**:
- The `false` branch is fully covered: header present + skip message present. Both assertions are necessary.
- Seeds enough data (5 hubs × 10 edges each → `SUM(relationship_count) = 10 > 1`) to guarantee `top_graph_concepts(conn, 5, 1)` returns results if the traversal runs.

**Critical weakness — the positive case (`includeDeepTraversal: true`) is under-asserted**:

The test asserts only:
```rust
assert!(
    !output_included.contains("Skipped: includeDeepTraversal=false"),
    ...
);
```

This is a *negative* assertion. It proves the flag was not ignored in the false direction, but it does **not** prove traversal actually executed. If `top_graph_concepts` returned an empty list (e.g., due to a seeding bug or schema change), the benchmark would emit:
```
Complex Traversal Bottleneck Analysis
--------------------------------------
Insufficient graph data: no connected hub concepts available.
```
…and this test would still **pass**, because "Skipped: includeDeepTraversal=false" is absent. The false negative would be invisible.

**Missing assertion** (concrete recommendation):
```rust
assert!(
    output_included.contains("Hub concepts:"),
    "expected traversal to run and emit hub concepts: {output_included}"
);
assert!(
    output_included.contains("Average time:"),
    "expected traversal timing output: {output_included}"
);
```

**Name problem**: "skips complex section *content*" describes only the skip (false) path. The test tests both paths. A more accurate name would be `benchmark_deep_traversal_flag_controls_execution`.

---

### Test 4 — `benchmark_iterations_zero_returns_error`

**What it does**: Sends `iterations: 0`, expects `is_err()`, checks error message contains `"iterations must be greater than zero"`.

**Evidence checked**:  
Source at `benchmark/mod.rs:45–50`:
```rust
if iterations <= 0 {
    return Err(MaenifoldError::InvalidArgument(
        "iterations must be greater than zero".to_string(),
    ));
}
```
Assertion is correct. The error type is `MaenifoldError::InvalidArgument`; the test uses `format!("{}", result.unwrap_err())` — which invokes `Display`. Confirmed this produces the expected string.

**Strengths**:
- Exact message assertion. This test would fail if someone changed the error string, which is the right behavior for oracle parity (RTM-032 implies the C# oracle returns this exact message).
- `invoke_benchmark_result` (the fallible variant) is used correctly.

**Coverage gap — upper bound is untested anywhere in the suite**:

Source at `benchmark/mod.rs:51–54`:
```rust
if iterations > 100 {
    return Err(MaenifoldError::InvalidArgument(
        "iterations must be between 1 and 100".to_string(),
    ));
}
```
A search across **all test files** confirms: **no test exercises `iterations: 101` or any value > 100**. This is a dead code path from a testing perspective. A refactor could raise the cap to 1000 (a DoS risk, per T-2.6 comment) and no test would catch it.

**RTM traceability ambiguity**: RTM-032/TC-5.2 covers both test 1 (echo) and test 4 (validation). These are distinct behaviors. RTM-032 should be split: one entry for "iterations echoed in header", one for "iterations=0 rejected". Currently two tests claim the same TC ID, making traceability matrix reads unreliable.

---

## Recommendations

**Priority 1 — Fix test 3's positive-case assertion (REWRITE)**

The deep traversal positive path is effectively untested. Add assertions that `Hub concepts:` and `Average time:` appear in `output_included`. Rename to `benchmark_deep_traversal_flag_controls_execution`.

```rust
// After the negative assertion:
assert!(
    output_included.contains("Hub concepts:"),
    "expected deep traversal to execute and report hub concepts: {output_included}"
);
assert!(
    output_included.contains("Average time:"),
    "expected traversal timing output: {output_included}"
);
```

**Priority 2 — Add missing upper-bound validation test**

No test covers `iterations > 100`. Add to this file (it belongs alongside test 4):

```rust
// RTM-032 / TC-5.2 — upper bound
#[test]
fn benchmark_iterations_above_maximum_returns_error() {
    let _guard = TEST_MUTEX.lock();
    let _env = isolated_env();

    let result = invoke_benchmark_result(json!({ "iterations": 101 }));
    assert!(result.is_err(), "iterations=101 should return an error");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("iterations must be between 1 and 100"),
        "unexpected error message: {err_msg}"
    );
}
```

**Priority 3 — Merge tests 1 and 2 (MERGE)**

Combine into a single `benchmark_header_parameters_reflected_in_output` test that asserts both `iterations` and `maxTestFiles` are echoed. This reduces fixture overhead and signals that both assertions belong to the same contract.

**Priority 4 — Fix RTM traceability ambiguity**

Tests 1 and 4 both claim RTM-032/TC-5.2. The RTM entry covers `iterations` validation and output reflection — two different contracts. Recommend splitting RTM-032 into:
- RTM-032a: iterations parameter echoed in output header (test 1)
- RTM-032b: iterations=0 rejected with error message (test 4)  
- RTM-032c: iterations>100 rejected with error message (new test from Priority 2)

**Priority 5 — Consider `maxTestFiles` validation gap**

`maxTestFiles` currently has no upper-bound validation (source clamps with `.max(0)` only). Either add validation to source (and test it) or document the intentional absence of an upper bound. The current test only verifies echo behavior — it cannot catch a missing validation because there is none to catch.

---

## Load-Bearing Score: 4/4

All four tests guard **distinct, real behavioral contracts** and would each catch a specific class of regression if removed:

| Test | What a deletion would miss |
|------|--------------------------|
| 1 | Silently renamed `iterations` header key |
| 2 | Silently renamed `maxTestFiles` header key |
| 3 | `includeDeepTraversal` gate removed or inverted (false-branch path) |
| 4 | `iterations <= 0` validation removed |

However, **effectiveness** is lower than the count implies:
- Test 3's positive branch is too weak to catch a traversal silent-failure.
- The upper-bound (`iterations > 100`) validation is entirely unguarded.

**Effective coverage score: 4.5/6** (4 tests covering real contracts + 0.5 credit for test 3's partial positive coverage − 1 point for the untested upper-bound path).

---

## SWE Remediation

- MERGE test 1 + test 2 → `benchmark_header_parameters_reflected_in_output`: combined into single test (same behavioral contract — header parameter echo). Reduces fixture overhead by 2 mutex/env cycles.
- REWRITE test 3 → `benchmark_deep_traversal_flag_controls_execution` (P1):
  - Added positive-path assertions: `"Hub concepts:"` and `"Average time:"` now required when `includeDeepTraversal: true`.
  - Fixed data seeding: replaced DB-only `seed_file_content_bulk` + `seed_graph_data` with real `.md` files on disk (via `write_memory_file`) + pre-sync. The benchmark's internal sync benchmark (`render_sync_benchmark`) calls `sync::sync()` which deletes file_content records not backed by real files — the original DB-only seeds were silently destroyed between the graph benchmark and complex traversal sections within a single benchmark run.
  - Renamed from `benchmark_exclude_deep_traversal_skips_complex_section_content` per eval recommendation.
- KEEP test 4 (`benchmark_iterations_zero_returns_error`): unchanged.
- ADD `benchmark_iterations_above_maximum_returns_error` (P2): covers the `iterations > 100` validation path that had zero test coverage.
- Commit: ea8d33d
