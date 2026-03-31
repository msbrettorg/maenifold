---
id: dynamic_model_loading
status: done
---

# Test Evaluation: dynamic_model_loading

**File**: `crates/maenifold-core/tests/dynamic_model_loading.rs`
**Evaluated by**: Red Team — Gray (Skeptical Inquiry)
**Traceability**: T-9.6 · FR-7 · RTM-050 · RTM-051 · RTM-053 · RTM-055 · RTM-056
**Production code under test**: `vector::load_model()`, `vector::generate_embedding()`,
`vector::embedding_dimensions()`, `vector::model_filename()`, `vector::cleanup()`
in `crates/maenifold-core/src/vector/`

---

## Summary

This file contains **four integration tests** for the dynamic model loading subsystem introduced in
T-9.6. All four share a global `TEST_MUTEX` (parking_lot) to serialize execution against the
singleton `model_state()` global, and delegate setup/teardown to the `TestContext` RAII guard in
`dynamic_model/support.rs`.

The production discovery path in `find_model_path()` searches three locations in order:
1. `{exe_dir}/assets/models` — the "shadow" directory TestContext controls
2. Cargo.toml ancestor walk-up → `ancestor/assets/models` — hits the **real packaged model**
3. `Config::current().assets_path/models` — isolated tmp root

TestContext's design correctly interferes with paths (1) and (2): it renames the real
`all-MiniLM-L6-v2.onnx` via `hide_packaged_model()` to block the walk-up probe, and stages a
shadow `assets/models` dir at the exe-adjacent path for positive tests. This is non-trivial
infrastructure and is the right approach.

The four tests collectively cover:
- Happy-path discovery (test 1)
- Dynamic dimension extraction from ONNX tensor metadata (test 2)
- Graceful degradation when no model is present (test 3)
- Multi-candidate selection by file size (test 4)

**Critical observation**: Tests 1 and 2 have significant setup overlap — both install the real
packaged model under a shadow name and call `load_model()`. They diverge only in how they express
the expected embedding dimension (hardcoded vs. derived), and test 1 additionally exercises
`model_filename()`. This overlap is containable but represents a maintainability cost.

**Critical observation**: Test 3 never calls `load_model()` with the model hidden. It verifies
"after `cleanup()`, the API returns safe defaults" rather than "when the model file is absent,
`load_model()` returns `Err`". The RTM-055 requirement covers both the discovery failure path
*and* graceful API state — only the latter is tested.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `t9_6_model_discovery_finds_shadow_onnx_file` | ⚠️ Partial | ⚠️ Partial | ⚠️ Fragile | ✅ Yes | ⚠️ Yes | **REWRITE** |
| 2 | `t9_6_embedding_dimensions_match_model_output_tensor` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⬛ No | **KEEP** |
| 3 | `t9_6_no_model_generate_embedding_returns_err_and_dimensions_none` | ⚠️ Partial | ✅ Yes | ⚠️ Incomplete | ✅ Yes | ⬛ No | **REWRITE** |
| 4 | `t9_6_multiple_onnx_files_select_largest` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Marginally | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `t9_6_model_discovery_finds_shadow_onnx_file`

**RTM coverage claimed**: RTM-050 (exe-adjacent discovery), RTM-051 (runtime dims from tensor),
RTM-053 (fallback tokenizer assets loaded alongside model)

**What it tests**: Installs a real model (copy of `all-MiniLM-L6-v2.onnx`) as `test-model.onnx`
in the shadow dir, calls `load_model()`, then asserts `model_filename`, `embedding_dimensions`,
and `generate_embedding` output length.

**Load-bearing?** Partial. The `model_filename()` assertion — verifying that the discovered
filename is reported correctly — is **unique to this test**. No other test in this file checks
`model_filename()`. The embedding_dimensions and generate_embedding checks are duplicated with
better rigor in test 2.

**Unique?** Partial. The filename assertion is unique. The dimensions/embedding assertions are
redundant with test 2.

**Correct assertions?**
- `assert_eq!(vector::model_filename().as_deref(), Some("test-model.onnx"))` — ✅ correct and
  unique
- `assert_eq!(vector::embedding_dimensions(), Some(384))` — ⚠️ **fragile**. This hardcodes 384,
  which is coincidentally correct because `all-MiniLM-L6-v2.onnx` is a 384-dim model. But the
  RTM-051 requirement explicitly states dims must come from ONNX tensor metadata rather than a
  hardcoded constant. A test asserting a hardcoded value does *not* verify the dynamic discovery
  mechanism — it would pass even if the production code returned `Some(384)` unconditionally. If
  the packaged model is ever swapped for a 768-dim variant the assertion will break with a
  confusing failure.
- `assert_eq!(embedding.len(), 384)` — same fragility as above.

**Good name?** Yes. The "shadow" qualifier communicates the isolation mechanism.

**Parameterizable?** Marginally. The model name (`test-model.onnx`) is a trivial string; the
interesting variant would be testing a model under a subdirectory path to cover `model_dir`
resolution for tokenizer loading — not covered anywhere in this file.

**Skeptical probes:**
- If `load_model()` is idempotent (it is: early-returns when `guard.is_some()`), and if a prior
  test leaked state through a mutex or panic, this test could pass without actually loading the
  shadow model. The `vector::cleanup()` call in `TestContext::new()` mitigates this, but the
  mutex is a `parking_lot::Mutex` — it does NOT poison on panic, so a panicking prior test could
  leave `model_state` as `Some(...)` (if the crash happened after load but before cleanup). The
  `cleanup()` in `new()` is the correct defense, and it is present.
- The test does not assert that `is_initialized()` returns true after `load_model()` succeeds.
  This is not strictly necessary but would be a zero-cost robustness addition.

**Verdict: REWRITE**
Retain the `model_filename()` assertion (unique, load-bearing). Replace the hardcoded `384`
with `model_output_dimensions(&ctx.shadow_models_dir().join("test-model.onnx"))` following test
2's pattern. This makes the test both correct per RTM-051 and resilient to model swaps. The
embedding length assertion should be derived from the same value.

---

### Test 2 — `t9_6_embedding_dimensions_match_model_output_tensor`

**RTM coverage claimed**: RTM-051 (runtime dims must match ONNX output tensor metadata, not
a hardcoded constant)

**What it tests**: Installs the real model as `output-dims.onnx`, derives `expected_dims` using
`model_output_dimensions()` from support (which independently re-reads the ONNX session outputs),
then asserts that `vector::embedding_dimensions()` and `generate_embedding().len()` match.

**Load-bearing?** Yes. This is the **only test** that directly validates the dynamic dimension
discovery mechanism described in RTM-051. It catches bugs in `discover_embedding_dimensions()`
and `select_best_session_output()` that a hardcoded-value test cannot. Remove it and the
dimension discovery logic has no behavioral coverage independent of the static model assumption.

**Unique?** Yes. The use of `model_output_dimensions()` as the oracle is the defining feature.
No other test derives expected dims from the model itself.

**Correct assertions?** Yes. Both `Some(expected_dims)` and `embedding.len() == expected_dims`
follow from the derivation. The `model_output_dimensions()` helper in `support.rs` implements
a parallel output-selection algorithm (`preferred_names` fallback chain) that differs slightly
from production `select_best_session_output()` — this is acceptable test-oracle divergence and
actually provides stronger validation than calling the same function.

**Good name?** Yes. "output tensor" is precise and surfaces the RTM-051 requirement clearly.

**Parameterizable?** No. The test exercises a single model. Parameterizing over multiple
models with different output tensor topologies (e.g., a 768-dim model, a 3D-output model that
requires dimension selection heuristics) would provide substantially deeper coverage of
`select_best_session_output()`, but this requires models not currently in the assets directory.
Not actionable as-is.

**Skeptical probes:**
- `model_output_dimensions()` in support is called on the installed shadow model *before*
  `load_model()`. This is the correct ordering — oracle computed, then production code runs.
  No timing issue.
- The support `model_output_dimensions()` function uses a different preferred-name list
  (`["sentence_embedding", "embedding", "pooled_output", "last_hidden_state"]`) compared to
  the production `select_best_session_output()`. If the production list diverges further, the
  test oracle could select a different output than production while both succeed silently. This
  is a latent correctness gap worth documenting, though both lists agree for `all-MiniLM-L6-v2`.

**Verdict: KEEP**

---

### Test 3 — `t9_6_no_model_generate_embedding_returns_err_and_dimensions_none`

**RTM coverage claimed**: RTM-055 (no model → APIs degrade gracefully, no panic, no stale dims)

**What it tests**: After `TestContext::new()` (which calls `vector::cleanup()`), hides the
packaged model, calls `cleanup()` again explicitly, then asserts `embedding_dimensions()` and
`model_filename()` are `None`, calls `generate_embedding()`, and reasserts both remain `None`.

**Load-bearing?** Partial. The test verifies that `generate_embedding()` returns `Err` when no
model is loaded and does not panic. That is valuable and aligns with the "no panic" half of
RTM-055. However:

The test **never calls `load_model()`** after hiding the model. RTM-055 states "when no *.onnx
file exists anywhere in assets/models, embedding APIs should degrade gracefully." The phrase
"when no *.onnx file exists" implies the load attempt itself should be clean — yet that path is
not exercised. The test only validates the already-cleaned state (`cleanup()` was called before
`generate_embedding()`), not the discovery-failure path.

Concretely: if `find_model_path()` were changed to panic instead of returning `Err` when no
model is found, this test would **still pass** — because `load_model()` is never called. The
test relies entirely on the pre-cleaned `None` state established by `cleanup()`.

**Unique?** Yes. This is the only test exercising the absent-model degradation path at all.

**Correct assertions?**
- Pre-call assertions (`embedding_dimensions() == None`, `model_filename() == None`) — ✅ but
  these are redundant: `cleanup()` guarantees `None` state; these assertions test the test
  infrastructure, not the production code.
- `result.is_err()` — ✅ correct for the "model not loaded" path.
- Post-call assertions (dims and filename still `None`) — ✅ verifies no side-effect mutation
  from a failed `generate_embedding()` call. This is the most interesting assertion in the test.

**Good name?** Yes. The name captures both observable outcomes.

**Skeptical probes:**
- `ctx.hide_packaged_model()` renames `all-MiniLM-L6-v2.onnx` to
  `all-MiniLM-L6-v2.onnx.dynamic-model-tests`. This blocks the Cargo walk-up discovery probe.
  This is correct and necessary — without it, a `load_model()` call inside this test would
  succeed via the walk-up path, defeating the test's purpose. But since no `load_model()` call
  is made, the hide is only a defense-in-depth guard against future lazy-loading refactors.
  It provides no behavioral coverage today.
- The shadow dir is empty (no `.onnx` installed), so path (1) in `find_model_path()` returns
  `None`. Path (2) is blocked by hide. Path (3) (Config tmp) is also empty. If `load_model()`
  were called here, it would correctly return `Err`. But it isn't called.
- The explicit second `vector::cleanup()` call in the test body is redundant. `TestContext::new()`
  already called `cleanup()`, and `hide_packaged_model()` does not load anything. The redundancy
  does no harm but adds noise.

**Verdict: REWRITE**
Add an explicit `vector::load_model()` call after hiding the model and assert it returns `Err`.
This covers the discovery-failure path and makes the test truly validate RTM-055's "when no
*.onnx file exists" scenario. Remove the redundant second `cleanup()` call. The revised test
would be:
```
ctx.hide_packaged_model();
let load_result = vector::load_model();
assert!(load_result.is_err(), "load_model() must return Err when no model exists");
// existing assertions follow unchanged
```

---

### Test 4 — `t9_6_multiple_onnx_files_select_largest`

**RTM coverage claimed**: RTM-056 (when multiple *.onnx files exist, select the largest)

**What it tests**: Installs the real model as `large.onnx`, then writes a 100-byte dummy
`small.onnx` (invalid ONNX content, just `b'x'` bytes). Calls `load_model()`, asserts
`large.onnx` was selected and produces valid 384-dim embeddings.

**Load-bearing?** Yes. This is the only test exercising the multi-candidate selection logic in
`discover_model()`. Remove it and RTM-056 has zero coverage.

**Unique?** Yes.

**Correct assertions?**
- `assert_eq!(vector::model_filename().as_deref(), Some("large.onnx"))` — ✅ the discriminating
  assertion. Confirms the largest file by name. This would catch a regression to smallest-first
  ordering.
- `assert_eq!(vector::embedding_dimensions(), Some(384))` — ⚠️ same hardcoded-384 fragility as
  test 1, but less critical here. The test's purpose is selection, not dimension dynamics. Still,
  it's an assumption that should be made explicit.
- No assertion that `small.onnx` was not loaded — this is implicitly covered by the filename
  check.

**Good name?** Yes.

**Parameterizable?** Marginally. The test uses two candidates. Edge cases not covered:
- Equal-size files: `discover_model()` uses `.then_with(|| left.1.cmp(&right.1))` as a tiebreak
  (lexicographic by path). This tiebreak behavior has no test.
- Three or more candidates.
- A candidate with `metadata.len() == 0` — `discover_model()` filters these out with the
  `metadata.len() > 0` check. Not exercised.

**Skeptical probes:**
- `write_dummy_onnx("small.onnx", 100)` creates a 100-byte file with content `vec![b'x'; 100]`.
  This is not a valid ONNX protobuf. However, `discover_model()` does NOT validate ONNX content
  — it only checks file extension, `is_file()`, and `len() > 0`. So `small.onnx` enters the
  candidate list. Since `large.onnx` (the real model, several MB) sorts first by size, the test
  correctly never attempts to parse `small.onnx`. If the sort were reversed by a bug, the test
  would fail at `Session::builder().commit_from_file("small.onnx")` with an ONNX parse error —
  which is an acceptable failure mode that surfaces clearly.
- The `metadata.len() > 0` guard means a zero-byte file would be excluded from candidates.
  This edge case is untested but low-risk.

**Verdict: KEEP**

---

## Recommendations

1. **[P1] Rewrite test 1** to derive expected dimensions from `model_output_dimensions()` instead
   of hardcoding 384. Retain the `model_filename()` assertion — it is load-bearing and unique.
   This eliminates the RTM-051 contradiction where the test asserting "dims come from the model
   tensor" hardcodes a value independently of the tensor.

2. **[P1] Rewrite test 3** to explicitly call `vector::load_model()` after `hide_packaged_model()`
   and assert it returns `Err`. RTM-055 covers the discovery failure path, not just the
   already-cleaned state. The current test does not exercise `find_model_path()`'s failure branch.

3. **[P3] Remove the redundant `vector::cleanup()` call** in test 3's body. `TestContext::new()`
   already calls it; the second call tests nothing and adds noise.

4. **[P3] Consider adding `is_initialized()` assertion** to test 1 (and rewritten test 3) as a
   zero-cost signal of successful load vs. pre-existing state.

5. **[P4] Document the tiebreak behavior** of `discover_model()` (lexicographic by path when
   sizes are equal) with either a comment in test 4 or a dedicated small test. RTM-056 is silent
   on tiebreak, so the behavior is implementation-defined — but currently untested.

6. **[P4] Note the oracle divergence** in test 2: `model_output_dimensions()` in support.rs uses
   a slightly different preferred-name list than `select_best_session_output()` in production.
   Both agree for `all-MiniLM-L6-v2`, but a model with an unusual output tensor name could cause
   the oracle and production to disagree silently. A comment is sufficient.

---

## Load-Bearing Score: 3/4

One test (test 1) is only partially load-bearing: its unique assertion (`model_filename()`) is
load-bearing, but the majority of its assertions duplicate test 2 with a fragile hardcoded value.
The three remaining tests each cover a distinct RTM item with no duplication. After the
recommended rewrites, the score rises to **4/4**.

| Test | Unique RTM Coverage | Load-Bearing |
|------|--------------------:|:------------:|
| `t9_6_model_discovery_finds_shadow_onnx_file` | `model_filename()` return value only | ⚠️ Partial |
| `t9_6_embedding_dimensions_match_model_output_tensor` | RTM-051 dynamic dim extraction | ✅ Yes |
| `t9_6_no_model_generate_embedding_returns_err_and_dimensions_none` | RTM-055 API state only (discovery failure path missing) | ⚠️ Partial |
| `t9_6_multiple_onnx_files_select_largest` | RTM-056 size-based selection | ✅ Yes |

---
## SWE Remediation
- REWRITE test 1 (`t9_6_model_discovery_finds_shadow_onnx_file`): replaced hardcoded `384` with `model_output_dimensions()` derivation per P1/RTM-051; added `is_initialized()` assertion per P3
- KEEP test 2 (`t9_6_embedding_dimensions_match_model_output_tensor`): unchanged
- REWRITE test 3 (`t9_6_no_model_generate_embedding_returns_err_and_dimensions_none`): added explicit `vector::load_model()` call after `hide_packaged_model()` to exercise discovery-failure path per P1/RTM-055; removed redundant `cleanup()` per P3; added `!is_initialized()` assertion per P3
- KEEP test 4 (`t9_6_multiple_onnx_files_select_largest`): unchanged
- All 4 tests pass: `cargo test --test integration_tests -p maenifold-core -- dynamic_model_loading`
- Commit: c5392f8
