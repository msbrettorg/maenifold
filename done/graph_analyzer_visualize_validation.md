---
id: graph_analyzer_visualize_validation
status: done
pipeline: tdd
---

# Test Evaluation: graph_analyzer_visualize_validation
**File:** `crates/maenifold-core/tests/graph_analyzer_visualize_validation.rs`
**Test count:** 7
**Evaluated:** 2026-03-13
**Traceability:** T-1.6 · FR-21 (NFR-1 oracle parity) · NFR-2 (test coverage)

---

## Summary

**4 of 7 tests are currently FAILING** — not flaky, not slow, not intermittent: they **panic** unconditionally on every run. The root cause is a design mismatch between the test harness and the implementation: the support helper `visualize()` calls `.expect("visualize")` on a `Result<String>`, but `analyzer::visualize` returns `Err(MaenifoldError::Message(...))` for all input validation failures. These tests never reach their `assert!` statements. They are dead weight in the test suite.

The 3 passing tests are genuinely load-bearing and structurally sound, with one weak assertion worth tightening.

**Pass rate: 3/7 (43%)**

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `visualize_with_empty_concept_name_returns_error` | **REWRITE** | Panics — `.expect()` in support harness hits `Err`; never reaches `assert!`. Conceptually load-bearing; fix harness or production error path. |
| `visualize_with_null_concept_name_returns_error` | **MERGE** | Panics — same root cause as above. Worse: null and empty string follow **identical code paths** (`as_str()` returns `None` → `unwrap_or_default()` → `""`) producing identical errors. Not a distinct scenario; merge into the empty-string test as a parameterized case. |
| `visualize_with_non_existent_concept_returns_not_found_message` | **REWRITE** | Passes, but the sync-guidance assertion (`result.to_ascii_lowercase().contains("sync")`) is incidental — any future message containing the word "sync" in any context would pass. Tighten to the actual message format. |
| `visualize_with_invalid_depth_returns_error` | **REWRITE** | Panics — same `.expect()` root cause. Additionally, the inline loop stops on first panic; subsequent values `[-1, 6, 10]` after `0` are never tested in a single run. Split into distinct sub-cases or fix the harness. |
| `visualize_with_invalid_max_nodes_returns_error` | **REWRITE** | Panics — same root cause as depth test. Same loop-stops-on-first-panic problem. |
| `visualize_with_isolated_concept_returns_empty_graph` | **REWRITE** | Passes, but the OR assertion `result.contains("No connections found") \|\| result.contains("isolated_concept")` is unnecessarily weak. Both conditions are true in the actual output; the OR permits regressions where one branch silently disappears. Assert both. |
| `visualize_generates_valid_mermaid_node_ids` | **KEEP** | Passes. Load-bearing: the only test that validates Mermaid structural syntax via regex. Edge regex and per-line check are reasonable. Minor: per-line `contains("-->") \|\| contains('[')` would accept any line containing `[` for unrelated reasons, but the risk is low. |

---

## Findings

### Critical: 4 Tests Are Non-Functional (Panic, Never Assert)

The support helper is:

```rust
pub(crate) fn visualize(cfg: &Config, payload: serde_json::Value) -> String {
    let context = markdown_context(cfg);
    search::visualize(&context, &payload).expect("visualize")  // ← panics on Err
}
```

`analyzer::visualize` returns `Err(MaenifoldError::Message(...))` for all validation failures:

```
thread '...' panicked at tests/graph_analyzer_support/mod.rs:91:43:
visualize: Message("conceptName is required")
```

The test assertions (`assert!(result.starts_with("ERROR"), ...)`) are **unreachable**. These tests have never caught a validation regression — they would fail for the wrong reason regardless of what the validation logic does.

**Fix option A (preferred — minimal scope):** Update the support helper to convert errors:
```rust
pub(crate) fn visualize(cfg: &Config, payload: serde_json::Value) -> String {
    let context = markdown_context(cfg);
    search::visualize(&context, &payload)
        .unwrap_or_else(|e| format!("ERROR: {e}"))
}
```

**Fix option B (aligns with other tools):** Change `analyzer::visualize` to return
`Ok(format!("ERROR: ..."))` for validation failures, consistent with how `memory/access.rs` handles input validation (`OutputFormat::Markdown => format!("ERROR: {msg}")`). This ensures MCP callers also see well-formed error strings rather than tool-call failures.

---

### Design Issue: Null and Empty String Are Identical Code Paths

`visualize_with_null_concept_name_returns_error` and `visualize_with_empty_concept_name_returns_error` exercise the **same line of production code**:

```rust
let concept_name = payload
    .get("conceptName")
    .and_then(Value::as_str)   // Null → None; missing → None; empty string → Some("")
    .unwrap_or_default()       // None → ""
    .trim();
if concept_name.is_empty() { ... }
```

`Value::Null.as_str()` returns `None`. `"".as_str()` returns `Some("")`. Both become `""` after `unwrap_or_default()`. Both reach the same `is_empty()` branch. There is no distinct null-handling path. A separate null test provides zero additional coverage of any production logic and should be merged.

---

### Weak Assertion: Non-Existent Concept "Sync" Check

Actual output produced by the implementation:
```
CONCEPT 'non-existent-concept-xyz' not found. Run sync first.
```

Current assertions:
```rust
assert!(result.contains("not found"), ...);           // passes
assert!(result.to_ascii_lowercase().contains("sync"), ...);  // passes incidentally
```

The "sync" check is accidental: it matches because the message happens to contain the word "sync". Any unrelated output containing "sync" (e.g., a future diagnostic note) would pass. Tighten to pin the actual format:
```rust
assert!(result.contains("not found"), ...);
assert!(result.contains("Run sync first"), ...);
// or: assert_eq!(result, format!("CONCEPT 'non-existent-concept-xyz' not found. Run sync first."));
```

---

### Weak Assertion: Isolated Concept OR Logic

Actual output:
```
graph TD
    isolated_concept [No connections found]
```

Current assertion:
```rust
assert!(
    result.contains("No connections found") || result.contains("isolated_concept"),
    "expected isolated-node indicator: {result}"
);
```

Both conditions hold simultaneously. The OR allows the test to pass even if `"No connections found"` were silently removed from the output (as long as the node ID appears). Assert both:
```rust
assert!(result.contains("isolated_concept"), ...);
assert!(result.contains("No connections found"), ...);
```

---

### Loop-in-Test Masking: Depth and MaxNodes Boundary Cases

Both `visualize_with_invalid_depth_returns_error` and `visualize_with_invalid_max_nodes_returns_error` iterate over multiple invalid values in a single test body:

```rust
for invalid_depth in [0, -1, 6, 10] {
    let result = visualize(...);
    assert!(result.starts_with("ERROR"), ...);  // panics on first value; loop terminates
}
```

Even after fixing the panic, this pattern means a failure at value `0` silently prevents verification of `-1`, `6`, and `10`. In Rust's test model, there is no sub-test isolation within a single test function. Prefer explicit separate test functions or use a helper that collects failures rather than aborting.

---

### Observation: `seed_graph` Called Unnecessarily by Validation-Only Tests

Tests 1 and 2 (`empty_concept_name`, `null_concept_name`) call `seed_graph(&cfg)` but fail at the `concept_name.is_empty()` check before any database access occurs. The seeding is not incorrect, but it is wasted work — each call inserts 6 concepts and 4 edges. Removing `seed_graph` from these two tests would be a minor cleanup.

---

## Positive Observations

- The test file is correctly isolated from `graph_analyzer_tests.rs` (historical split per T-1.6 comment) — separation of validation from behavior tests is good structure.
- `visualize_generates_valid_mermaid_node_ids` uses a real regex to verify output format, not just a substring check. This is the right approach for format validation.
- `visualize_with_invalid_depth_returns_error` correctly includes both boundary-adjacent values (`0`, `6`) and far-out-of-range values (`-1`, `10`) — good boundary coverage intent.
- The `isolated_config()` + `seed_graph()` pattern enforces test isolation via `TempDir`. No shared database state leaks between tests.
- `TEST_MUTEX` is used consistently across all tests — correct for a test suite running against a shared process-level config.

---

## Recommendations

**Priority 1 (blocking — tests currently broken):**

1. Fix the `visualize()` support helper to map `Err` → `"ERROR: {e}"` string, OR align `analyzer::visualize` to return `Ok("ERROR: ...")` for validation failures. Either unblocks the 4 failing tests without rewriting assertions.
2. After fixing the harness, run all 7 tests to confirm they reach their assertions. Expect `visualize_with_null_concept_name_returns_error` to pass trivially — this confirms it is redundant with the empty-string test.

**Priority 2 (test quality):**

3. Merge `visualize_with_null_concept_name_returns_error` into `visualize_with_empty_concept_name_returns_error` — use a parameterized helper or table-driven pattern over `["", serde_json::Value::Null]`.
4. Replace OR assertion in `visualize_with_isolated_concept_returns_empty_graph` with independent `assert!` calls for both the node ID and "No connections found".
5. Pin the "sync" assertion in `visualize_with_non_existent_concept_returns_not_found_message` to the exact phrase `"Run sync first"`.

**Priority 3 (future improvement):**

6. Split the inline loops in depth/maxNodes tests into named test functions per invalid value so failures identify the specific broken boundary.
7. Remove `seed_graph` from the two empty/null conceptName tests — they never query the database.

---

## Load-Bearing Score: 3/7 currently green

| Test | Conceptually Necessary | Currently Functioning |
|---|---|---|
| `empty_concept_name` | ✅ | ❌ panics |
| `null_concept_name` | ⚠️ redundant with above | ❌ panics |
| `non_existent_concept` | ✅ | ✅ (weak assertions) |
| `invalid_depth` | ✅ | ❌ panics |
| `invalid_max_nodes` | ✅ | ❌ panics |
| `isolated_concept` | ✅ | ✅ (weak OR) |
| `valid_mermaid_node_ids` | ✅ | ✅ |

**6/7 tests are conceptually necessary.** Only the null test is redundant. **3/7 tests are currently functioning.** Fixing the harness and tightening weak assertions would bring this to 6/7 green with real, meaningful assertions.
