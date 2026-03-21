# Test Evaluation: benchmark_boundary_output.rs

**Traceability**: T-12.7 | NFR-2 (Test Coverage)
**RTM Coverage**: RTM-031, RTM-035, RTM-036, RTM-037 | TC-5.1, TC-5.5, TC-5.6
**Evaluator stance**: 🔘 Gray — skeptical inquiry. Every assertion is interrogated for what it actually proves vs. what it assumes.

---

## Summary

`crates/maenifold-core/tests/benchmark_boundary_output.rs` contains **3 boundary tests** for the `RunFullBenchmark` tool. The file was introduced by T-12.7 as a focused split of benchmark output assertions from a larger boundary suite.

The three tests cover three mutually exclusive tool states: empty-database invocation, seeded-data invocation, and `learn=true` help mode. This state partitioning is structurally sound. However, every test has at least one assertion defect that could allow a genuine regression to pass silently. Two of three tests have **misleading names or hedging assertion logic** that signals uncertainty about what the output actually looks like.

**What this suite does well**: isolation (via `isolated_env()` + mutex guard), fixture richness in test 2, and state coverage across the tool's three operating modes.

**What this suite does poorly**: the assertions are predominantly shallow string-presence checks that validate output structure but not output correctness. No test verifies that timing values are numeric, that statistics follow expected formats with real numbers, or that the benchmark tool's quantitative output is semantically meaningful.

---

## Verdicts

| # | Test Name | RTM | Load-Bearing? | Unique? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|-----|--------------|---------|---------------------|----------------|-----------------|---------|
| 1 | `benchmark_empty_db_contains_section_headers_or_insufficient_data` | RTM-031, RTM-037 | ✅ Yes | ✅ Yes | ⚠️ Partial — no negative gate on timing output; brittle `GRPH-009` literal | ❌ No — name says "or", logic uses AND | ✅ Low value (iterations) | **REWRITE** |
| 2 | `benchmark_with_data_contains_timing_statistics` | RTM-035 | ✅ Yes | ✅ Yes | ⚠️ Partial — no numeric validation; `"ms"` check is nearly vacuous; `includeDeepTraversal: true` asserts nothing | ✅ Acceptable | ✅ Yes (seed threshold) | **REWRITE** |
| 3 | `benchmark_learn_true_returns_help_documentation` | RTM-036 | ✅ Yes | ✅ Yes | ⚠️ Partial — OR hedging; fragile negative assertion pattern | ✅ Clear | ⬜ Minimal value | **KEEP** (with minor fixes) |

---

## Findings Per Test

### Test 1 — `benchmark_empty_db_contains_section_headers_or_insufficient_data`

```rust
// RTM-031 / RTM-037 / TC-5.1
```

#### What it claims to test
That an empty-database invocation either shows section headers OR surfaces an insufficient-data message.

#### What it actually tests
That **all four strings** are simultaneously present in the output. The "or" in the name is a false description of the logic — every `assert!` is a conjunction, not a disjunction.

#### Specific assertion problems

**Problem 1 — Misleading name (cosmetic but consequential)**
The test name says `…_or_insufficient_data`, implying that sometimes headers appear and sometimes the message appears. The test body shows this is wrong — it asserts both simultaneously. When someone reads the name to understand what the tool does on an empty DB, they get an incorrect mental model.

**Problem 2 — Brittle presentation string as assertion anchor**
```rust
assert!(
    output.contains("GRPH-009: CTE vs N+1 Pattern Benchmark"),
    ...
);
```
`"GRPH-009"` is a tool-internal label. If the benchmark tool renumbers its sections, renames this benchmark, or changes the label format (`GRPH-9:` vs `GRPH-009:`), this test fails due to cosmetic output changes — not a behavioral regression. The assertion validates the exact wording of a section header, not whether graph benchmarking was attempted.

**Problem 3 — No negative assertion for timing output**
The test verifies that "Insufficient test data" appears but never verifies that timing statistics do NOT appear. If a regression caused the empty-DB path to emit partial timing data alongside the insufficiency message, this test would still pass.

**Problem 4 — `iterations: 1` is not documented**
The parameter is set to 1, but no assertion checks iteration-count-related behavior. This is either unnecessary parameter noise or an underspecified intention. If the test means to probe minimum-iteration handling, an assertion should reflect it.

---

### Test 2 — `benchmark_with_data_contains_timing_statistics`

```rust
// RTM-035 / TC-5.5
```

#### What it claims to test
That with sufficient data, the benchmark output contains timing statistics.

#### What it actually tests
That certain section-header strings appear and that the substring `"ms"` exists somewhere in the output.

#### Specific assertion problems

**Problem 1 — Implicit and undocumented threshold assumption**
```rust
seed_file_content_bulk(&conn, 110);
```
Why 110? There is a data sufficiency threshold (evidenced by test 1's "Insufficient test data" message). The number 110 exceeds whatever that threshold is — but by how much? If the threshold is 100, 110 clears it by 10. If the threshold ever changes to 120, this test silently tests the wrong state (it would hit the insufficiency path while the test expects the timing path). The constant should either be named (e.g., `BENCHMARK_DATA_THRESHOLD + 10`) or asserted explicitly.

**Problem 2 — `includeDeepTraversal: true` adds execution cost with zero assertion value**
```rust
let output = invoke_benchmark(json!({
    "iterations": 1,
    "includeDeepTraversal": true
}));
```
No assertion in this test is conditioned on deep traversal output. The flag makes the test slower without testing any additional behavior. Either add assertions for deep traversal output, or remove the flag.

**Problem 3 — `output.contains("ms")` is nearly vacuous**
`"ms"` is a two-character substring. It would match "milliseconds", but also "forms", "comes", "arms", "symptoms" — any word containing those letters in sequence. This assertion is essentially checking that the output is non-empty with the right character distribution. A regression that emits "benchmark error: no data in forms" would pass. The correct assertion is a pattern like `\d+(\.\d+)?\s*ms` to verify that a numeric timing value with a unit is present.

**Problem 4 — Statistical format not validated numerically**
```rust
assert!(output.contains("Hybrid Average:"), ...);
assert!(output.contains("Hybrid Min/Max/Median:"), ...);
```
These verify that label headers are present, but not that values follow. A regression that outputs:
```
Hybrid Average: [error parsing results]
Hybrid Min/Max/Median: —
```
would pass all assertions. The test validates output structure but not output meaning.

**Problem 5 — Graph benchmark section gap**
Test 1 asserts `"GRPH-009: CTE vs N+1 Pattern Benchmark"` but test 2 does not. If the graph benchmark section disappears from seeded runs, no test in this file catches it. The responsibility split between test 1 (empty) and test 2 (seeded) creates a structural blind spot for graph benchmark presence in the happy path.

**Problem 6 — Magic numbers in graph seed**
```rust
seed_graph_data(&conn, 3, 8);
```
No comment explains that 3 = hub count and 8 = edges per hub. If these values interact with graph traversal thresholds (e.g., minimum edge count for CTE benchmark activation), the test is sensitive to those thresholds in undocumented ways.

---

### Test 3 — `benchmark_learn_true_returns_help_documentation`

```rust
// RTM-036 / TC-5.6
```

#### What it claims to test
That `learn=true` returns documentation rather than running the benchmark.

#### What it actually tests
That certain documentation strings appear and that a timestamp-like pattern doesn't appear outside code blocks.

#### Specific assertion problems

**Problem 1 — OR hedging signals uncertain output contract**
```rust
assert!(
    output.contains("iterations") || output.contains("Iterations"),
    ...
);
assert!(
    output.contains("includeDeepTraversal") || output.contains("deep traversal"),
    ...
);
```
Both OR assertions hedge between two cases. This is a code smell: the test writer didn't know which form the output would take. A well-specified tool has a stable output format. These ORs should be resolved: either the output uses `"iterations"` (lowercase) or `"Iterations"` (title case) — pin it and use `assert!(output.contains("the-known-form"))`.

**Problem 2 — Fragile negative assertion**
```rust
let outside_code_blocks: String = output.split("```").step_by(2).collect();
assert!(
    !outside_code_blocks.contains("Started: 20"),
    "learn=true should not produce live benchmark output: {output}"
);
```
The intent is correct: verify no live benchmark output leaked outside documentation. However:
- The pattern `"Started: 20"` is a guess at timestamp format (e.g., "Started: 2026-..."). If the timestamp format changes (e.g., "Benchmark started at 2026-..."), this check silently stops detecting the regression.
- `step_by(2)` collects splits at even indices — but this only correctly separates inside/outside backtick blocks if backtick delimiters are balanced. If the help text has an unbalanced backtick (even in a raw string or escaped context), the inside/outside split is inverted.
- A more robust check: verify that strings unique to live output (e.g., `"Hybrid Average:"` or `\d+\.\d+ ms`) are absent from the full output when `learn=true`, rather than trying to parse code block boundaries.

**Problem 3 — Help completeness not validated**
The test checks that the tool name and two parameter names appear in help output. It does not check that the help describes what the tool does, what the output format is, or includes any usage examples. A degenerate `learn=true` implementation that emits only:
```
RunFullBenchmark
iterations
includeDeepTraversal
```
would pass all three assertions.

---

## Recommendations

### Priority 1 — Correct the misleading test name (Test 1)
Rename `benchmark_empty_db_contains_section_headers_or_insufficient_data` to `benchmark_empty_db_shows_insufficient_data_and_section_headers`. The "or" implies disjunction; the logic is conjunction. This is a low-effort fix with high documentation value.

### Priority 2 — Resolve OR hedging in Test 3
Pick one canonical casing for `"iterations"` and `"includeDeepTraversal"` in help output, pin that form, and remove the OR. If the output format is intentionally case-flexible, use `to_lowercase()` + single `contains()`.

### Priority 3 — Replace `output.contains("ms")` with a pattern check (Test 2)
```rust
// Before (vacuous):
assert!(output.contains("ms"), ...);

// After (meaningful):
let has_timing = regex::Regex::new(r"\d+(\.\d+)?\s*ms").unwrap().is_match(&output);
assert!(has_timing, "expected numeric timing values with ms unit: {output}");
```
Or use a simpler check like `output.contains("0.")` or assert that the statistical labels are followed by digits. The current check proves almost nothing.

### Priority 4 — Document the seed threshold in Test 2
```rust
// Before:
seed_file_content_bulk(&conn, 110);

// After:
const BENCHMARK_THRESHOLD: usize = 100; // tool requires >100 files to produce timing output
seed_file_content_bulk(&conn, BENCHMARK_THRESHOLD + 10);
```
This makes the threshold explicit and ensures that a threshold change causes a visible constant update rather than a silent test state mismatch.

### Priority 5 — Add a negative assertion to Test 1
After verifying "Insufficient test data" appears, add:
```rust
assert!(
    !output.contains("Hybrid Average:"),
    "empty db should not produce timing statistics: {output}"
);
```
This ensures the two test states (empty vs. seeded) are genuinely exclusive.

### Priority 6 — Remove `includeDeepTraversal: true` or add assertions for it (Test 2)
If deep traversal behavior is tested elsewhere, remove the flag. If it's not tested anywhere, add assertions for the output it produces. Dead parameters that add execution cost without assertion value are noise.

### Priority 7 — Replace the `"Started: 20"` negative check with a domain-specific one (Test 3)
```rust
// Before (fragile timestamp guess):
assert!(!outside_code_blocks.contains("Started: 20"), ...);

// After (domain-specific):
assert!(
    !output.contains("Hybrid Average:") && !output.contains("Sync Min/Max/Median:"),
    "learn=true should not produce live benchmark statistics: {output}"
);
```

### Longer term — Parameterize Test 2 for threshold boundary behavior
The test currently only probes one data configuration. Consider adding a companion test `benchmark_at_threshold_boundary_shows_insufficient_data` that seeds exactly `BENCHMARK_THRESHOLD - 1` files to confirm the boundary is correct. This would also make the threshold constant visible.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. They cover the three primary operating states of `RunFullBenchmark` and no two tests are redundant with each other. Removing any one of them would leave a complete tool state untested.

However, the **effective assertion strength** is significantly lower than the count of 3 suggests:
- Test 1 validates 4 strings in output but misses the inverse (no spurious timing data on empty DB)
- Test 2 validates section headers and an essentially meaningless `"ms"` substring, missing numeric validation entirely
- Test 3 validates documentation presence but hedges on format and uses a fragile negative check

**Honest assessment**: these tests would not catch a regression that produces section headers with corrupted statistics, numeric values replaced by error strings, or help text with wrong parameter names (as long as "iterations" and "includeDeepTraversal" still appear somewhere). The tests protect against crashes and catastrophic output omissions, but not against quiet data corruption in benchmark output.

*Traceability: T-12.7 (benchmark boundary split) | NFR-2 (each tool SHALL have tests at all 3 layers)*
