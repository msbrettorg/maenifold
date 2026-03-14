# Test Evaluation: proximity_weight_decay.rs

> **Traceability**: T-4.3 · FR-23.4 · NFR-1 · NFR-2  
> **File under review**: `crates/maenifold-core/tests/proximity_weight_decay.rs`  
> **Evaluator persona**: 🔘 Gray — Skeptical Inquiry  
> **Related concepts**: [[test-quality]] · [[maenifold-rs]] · [[proximity-weight]] · [[decay-weight]] · [[build-context]]

---

## Summary

Five tests cover two distinct surface areas: the `compute_edge_decay_weight` public API (Tests 1–2) and the `BuildContext` tool's proximity-to-DecayWeight pipeline (Tests 3–5). The suite has one severe structural defect — Test 1 contains dead setup code driven by a fundamental misunderstanding of the function under test — and one redundancy — Tests 3 and 4 share identical setup with mathematically equivalent assertions when `co_occurrence_count = 1`.

### Critical Discovery: `compute_edge_decay_weight` does not read files

`compute_edge_decay_weight(source_files: &str)` computes the **arithmetic mean of proximity weights** present in the JSON string it receives. It does not open any file, check any timestamp, or apply temporal decay. It calls `parse_source_file_weights` → averages the `f64` values in the resulting `BTreeMap`. Test 1 creates a real file on disk with a precise `created` timestamp — that file is never read. Seven lines of setup are completely inert. The name "for_fresh_file" describes behavior the function does not have.

The _actual_ temporal decay path lives in `graph/support.rs → relation_weighted_score` (called by `BuildContext`), where each URI's proximity is multiplied by the time-decay score from `load_file_decay_weight`. **This path is not tested anywhere in this file.** Tests 3–5 use URIs that are absent from the `file_content` table, so `load_file_decay_weight` always returns the neutral fallback `1.0` — the decay multiply never fires.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `compute_edge_decay_weight_returns_proximity_for_fresh_file` | Partially | Yes | ❌ No | ❌ No | Yes | **REWRITE** |
| 2 | `compute_edge_decay_weight_rejects_corrupt_source_files_json` | Yes | Yes | ⚠️ Narrow | ✅ Yes | Yes | **KEEP** |
| 3 | `build_context_uses_proximity_weight_in_decay_weight` | Partially | ❌ No | ✅ Yes | ✅ Yes | Yes | **MERGE** with T4 |
| 4 | `build_context_ranks_stronger_proximity_above_weaker_proximity` | Partially | ❌ No | ✅ Yes | ✅ Yes | Yes | **MERGE** with T3 |
| 5 | `build_context_treats_empty_source_files_as_zero_decay_weight` | Yes | Yes | ✅ Yes | ✅ Yes | No | **KEEP** |

---

## Findings by Test

### Test 1 — `compute_edge_decay_weight_returns_proximity_for_fresh_file` · REWRITE

**What the test does:** Creates a real file at `projects/prox-decay-test.md` with a current-UTC `created` frontmatter field. Calls `compute_edge_decay_weight` with a single-entry JSON `{uri: 0.5}`. Asserts the result is (a) positive, (b) ≤ 0.5 + ε, (c) > 0.4.

**What the function actually does:** Parses the JSON string, sums the values, divides by count. For a single entry with value `0.5`, the result is exactly `0.5`. Always. With no regard to the file on disk.

**Problems:**

1. **Dead setup code.** The `TempDir`, `Config::override_root`, `fs::create_dir_all`, and `fs::write` calls contribute nothing. The function under test never opens a file. Any reviewer trying to understand the decay formula will be actively misled.

2. **Trivially satisfied assertions.** Given the function returns exactly `0.5` for this input:  
   - `weight > 0.0` → `0.5 > 0.0` — passes for any positive proximity  
   - `weight <= 0.5 + 1e-6` → exact equality within epsilon — passes trivially  
   - `weight > 0.4` — a 20% tolerance around an exact value; could hide a regressed implementation returning 0.45  
   None of these assertions would catch a broken averaging implementation (e.g., returning `proximity * 2.0`).

3. **Name is a factual lie.** "for_fresh_file" asserts a temporal property the function does not evaluate.

4. **No test of averaging.** The only test for this function uses exactly one URI. The `sum / len` path is trivially equivalent to identity for `N=1`. A two-URI test would actually exercise the arithmetic.

**Recommendation — Rewrite as:**
```rust
// Tests the arithmetic mean of two proximity weights.
fn compute_edge_decay_weight_averages_multiple_proximity_weights() {
    let result = compute_edge_decay_weight(
        r#"{"memory://a": 1.0, "memory://b": 0.5}"#
    ).expect("valid json");
    assert_approx_eq(result, 0.75);  // (1.0 + 0.5) / 2
}

// Tests single-entry passthrough.
fn compute_edge_decay_weight_single_entry_returns_that_weight() {
    let result = compute_edge_decay_weight(
        r#"{"memory://x": 0.5}"#
    ).expect("valid json");
    assert_approx_eq(result, 0.5);
}
```
No `TestEnv`, no disk I/O, exact assertions. The `acquire_test_lock` call is also unnecessary for a pure function test — remove it.

---

### Test 2 — `compute_edge_decay_weight_rejects_corrupt_source_files_json` · KEEP

**Assessment:** Correct and load-bearing. It exercises the `serde_json::from_str` error path that propagates via `?` in `parse_source_file_weights`. No other test in this file exercises the error arm.

**Concern — Coverage gap:** `parse_source_file_weights` also handles valid JSON that is neither object nor array (e.g., `"null"`, `"42"`, `"true"`) by returning `Ok(BTreeMap::new())`. For those inputs `compute_edge_decay_weight` returns `Ok(0.0)`, not `Err`. The test name says "corrupt" but the silent `Ok(0.0)` behaviour for semantically wrong types is equally surprising and untested. Add:
```rust
fn compute_edge_decay_weight_returns_zero_for_json_non_object() {
    assert_approx_eq(compute_edge_decay_weight("null").expect("null"), 0.0);
    assert_approx_eq(compute_edge_decay_weight("42").expect("number"), 0.0);
    assert_approx_eq(compute_edge_decay_weight("true").expect("bool"), 0.0);
}
```

**Lock call:** `acquire_test_lock` is called but this test is entirely pure (no global state, no disk, no DB). The lock is unnecessary overhead — and misleads future readers into thinking global state is mutated.

---

### Tests 3 & 4 — `build_context_uses_proximity_weight_in_decay_weight` / `build_context_ranks_stronger_proximity_above_weaker_proximity` · MERGE

**Why they overlap:** Both tests:
- Insert the same concept triple (query, strong at proximity 1.0, weak at proximity 0.5)
- Use a URI absent from `file_content` (so `load_file_decay_weight` always returns `1.0`)
- Assert that the strong concept ranks above the weak concept

The only difference is the output field inspected: `DecayWeight` vs `WeightedScore`.

**Why the assertions are numerically equivalent here:** `WeightedScore = co_occurrence_count × DecayWeight`. Both edges have `co_occurrence_count = 1`. Therefore `WeightedScore ≡ DecayWeight` in this test. A regressed implementation where `WeightedScore` diverged from `co_occurrence_count * DecayWeight` would not be caught — both tests would still pass or fail together.

**The missing assertion — temporal decay never fires:** Because no row exists in `file_content` for `"memory://test/prox-buildcontext"` or `"memory://test/prox-ranking"`, `load_file_decay_weight` returns the neutral fallback `1.0`. The `file_weight * decay_weight` multiply is `proximity * 1.0 = proximity`. These tests cannot distinguish a correct temporal-decay implementation from one that ignores temporal decay entirely. This is a significant coverage gap given the stated purpose (FR-23.4 — proximity weight decay).

**Recommendation — Merge into one test with clear documentation, and add a temporal-decay variant:**
```rust
fn build_context_decayweight_and_weightedscore_reflect_proximity_ordering() {
    // ... identical setup ...
    assert!(strong["DecayWeight"] > weak["DecayWeight"], "...{payload}");
    assert!(strong["WeightedScore"] > weak["WeightedScore"], "...{payload}");
    // co_occurrence_count = 1, so WeightedScore == DecayWeight; if they ever diverge, update.
}

fn build_context_decayweight_reduced_for_old_file() {
    // Insert a file_content row with created = 365 days ago, last_accessed = 365 days ago
    // Assert DecayWeight < proximity (1.0) — i.e. temporal decay fires
}
```

---

### Test 5 — `build_context_treats_empty_source_files_as_zero_decay_weight` · KEEP

**Assessment:** Clean, correct, and load-bearing. The boundary condition (`source_files = "{}"`) exercises the empty-object branch in `relation_weighted_score` where `document_count = 0` triggers the `Ok((0.0, 0.0))` early return. The assertion uses `assert_approx_eq` for exact equality, which is correct.

**Minor note:** The `co_occurrence_count` argument to `insert_edge_raw_source_files` is `1.0` — this value has no effect on the assertion (DecayWeight is computed before it's multiplied by count). The setup could document this more clearly.

---

## Coverage Gaps (Missing Tests)

| Gap | Severity | Suggested Test Name |
|-----|----------|---------------------|
| `compute_edge_decay_weight` multi-URI averaging | High | `compute_edge_decay_weight_averages_multiple_proximity_weights` |
| Temporal decay firing in `BuildContext` (old file in DB) | High | `build_context_decayweight_reduced_for_old_file` |
| `compute_edge_decay_weight` with valid non-object JSON | Medium | `compute_edge_decay_weight_returns_zero_for_json_non_object` |
| `compute_edge_decay_weight` single-URI exact-value assertion | Medium | `compute_edge_decay_weight_single_entry_returns_that_weight` |
| `BuildContext` with multi-URI edge (averaging) | Medium | `build_context_decayweight_averages_multiple_source_files` |
| `BuildContext` with `source_files` as legacy comma-separated string | Low | `build_context_handles_legacy_comma_separated_source_files` |

The most consequential gap is the **absence of any test that verifies temporal decay actually fires** in the `BuildContext` pipeline. Every test in this file uses URIs with no `file_content` row, so the `load_file_decay_weight` fallback (`1.0`) is returned unconditionally. This means the entire time-decay multiply in `graph/support.rs:27–48` is not exercised here at all — despite this being a "proximity weight decay" test file.

---

## SWE Remediation

- REWRITE T1 `compute_edge_decay_weight_returns_proximity_for_fresh_file`: Removed all dead disk/file setup, `TestEnv`, `acquire_test_lock`, `chrono::Utc`, `std::fs` imports. Replaced with two focused pure-function tests:
  - `compute_edge_decay_weight_averages_multiple_proximity_weights`: tests (1.0 + 0.5) / 2 == 0.75 with exact assertion
  - `compute_edge_decay_weight_single_entry_returns_that_weight`: tests single-entry passthrough == 0.5 with exact assertion
- REMOVE `acquire_test_lock` from T2: pure function test needs no global lock
- MERGE T3+T4 into `build_context_decayweight_and_weightedscore_reflect_proximity_ordering`: single test asserts both DecayWeight and WeightedScore ordering, with comment documenting co_occurrence_count=1 equivalence
- KEEP T2 `rejects_corrupt_source_files_json`: unchanged (error path)
- KEEP T5 `treats_empty_source_files_as_zero_decay_weight`: unchanged (boundary)
- ADD `compute_edge_decay_weight_returns_zero_for_json_non_object`: pins silent Ok(0.0) for null/42/true inputs (P1 coverage gap)
- ADD `build_context_decayweight_reduced_for_old_file`: inserts file_content row with 2024-01-01 timestamps, verifies DecayWeight < 1.0 — exercises temporal decay multiply that was previously untested (P0 coverage gap)
- ADD `insert_file_content` helper to `proximity_weight/support.rs`
- 7 tests, all passing
- Commit: 7bf6f6c

---

## Recommendations

1. **REWRITE Test 1** — remove all disk/file setup; test `compute_edge_decay_weight` as the pure arithmetic-mean function it is; use exact assertions.

2. **MERGE Tests 3 & 4** — one test with both field assertions, documented explanation of why `WeightedScore == DecayWeight` when count is 1.

3. **ADD temporal decay firing test** — insert a `file_content` row with `created`/`last_accessed` ~365 days ago, assert `DecayWeight < proximity`. This is the single highest-value missing test.

4. **Remove unnecessary `acquire_test_lock` from Test 2** — pure functions do not need the global process lock; its presence misleads.

5. **Tighten Test 1 assertions post-rewrite** — replace range bounds with `assert_approx_eq` to the exact expected mean value.

6. **Add `"null"` / `"42"` / `"true"` coverage for Test 2** — silent `Ok(0.0)` on valid-but-wrong-type JSON is a behavior worth pinning.

---

## Load-Bearing Score: 3/5

| Test | Load-Bearing as Written | Post-Restructure |
|------|:---:|:---:|
| T1 `returns_proximity_for_fresh_file` | ❌ — misleading setup, trivial assertions | ✅ after rewrite |
| T2 `rejects_corrupt_source_files_json` | ✅ — only error-path test | ✅ |
| T3 `uses_proximity_weight_in_decay_weight` | ⚠️ — duplicates T4 numerically | ✅ after merge |
| T4 `ranks_stronger_proximity_above_weaker_proximity` | ⚠️ — duplicates T3 numerically | ✅ after merge |
| T5 `treats_empty_source_files_as_zero_decay_weight` | ✅ — clean boundary test | ✅ |

**3/5 load-bearing as written. 5/5 after the recommended rewrite, merge, and temporal-decay addition.**

The suite covers the zero-value boundary (T5) and the error path (T2) reliably. It fails to cover the function's core arithmetic (T1 due to dead setup), and the temporal-decay multiply that justifies the file's existence as a "decay" test suite (gap across T3–T4). The highest-leverage action is adding one test that actually makes old-file decay observable.
