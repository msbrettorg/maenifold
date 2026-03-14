# Test Evaluation: `vector_tools_runtime.rs`

> **Traceability**: T-12.6 · T-12.7 · T-8.4 · FR-33.1 · NFR-1 · NFR-2  
> **Evaluator**: Red Team — Gray (Skeptical Inquiry)  
> **File**: `crates/maenifold-core/tests/vector_tools_runtime.rs`  
> **Sibling files examined**: `vector_tools_batch.rs`, `vector_tools_semantic.rs`  
> **Prior evaluation cross-reference**: `crates/maenifold-core/tests/evaluations/access_boosting_search.md` (recommends `vector_tools_runtime.rs` as canonical T-8.4 home)

---

## Summary

This file is the **post-split survivor** of a larger vector test suite, trimmed under T-12.7
("acceptance hygiene trims smoke-only runtime vector tests that overclaim behavior"). It
contains exactly **two `#[test]` functions**:

1. `cleanup_properly_disposes_resources` — ONNX model lifecycle (load → cleanup → reload)
2. `embedding_quality_differs_between_onnx_and_fallback` — embedding non-degeneracy probe

Both tests use a global serialization mutex (`TEST_MUTEX`) and call `vector::cleanup()` as a
defensive preamble. The pair is small by design — the file's job is to hold the runtime
initialization/disposal contract for the ONNX model layer, distinct from batch parity
(`vector_tools_batch.rs`) and semantic validation (`vector_tools_semantic.rs`).

**Critical finding**: Test 2's name is **actively false**. It claims to compare ONNX output
against a fallback, but the test body invokes no fallback path, inserts no fallback vector,
and performs no comparison. The test does exactly one thing: confirms the ONNX embedding for a
single sentence is neither all-zeros nor all-uniform — a necessary but extremely weak quality
gate. The discrepancy between name and behavior is the highest-risk defect in this file
because it signals a property that does not exist, misleading anyone who depends on it for
coverage confidence.

**Secondary finding**: Test 1's name omits the most operationally interesting assertion — that
the model **reinitializes cleanly** after a `cleanup()` call. The reload step is the behavior
that matters for graceful-restart and multi-test isolation scenarios; it should be surfaced in
the name.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `cleanup_properly_disposes_resources` | ✅ Yes | ✅ Yes | ⚠️ Partial | ⚠️ Partial | ❌ No | **REWRITE** |
| 2 | `embedding_quality_differs_between_onnx_and_fallback` | ⚠️ Partial | ✅ Yes | ❌ No | ❌ No | ✅ Yes | **REWRITE** |

---

## Per-Test Analysis

---

### Test 1 — `cleanup_properly_disposes_resources`

**Tag**: T-8.4 / FR-33.1  
**Location**: line 11

#### What it does

```
cleanup()                 // defensive pre-clean (no assertion)
load_model()              // explicit initialization
assert is_initialized()   // verify load succeeded
cleanup()                 // exercise disposal
assert !is_initialized()  // verify state cleared
load_model()              // exercise reload-after-cleanup
assert is_initialized()   // verify reinit succeeded
```

#### Load-bearing?

**Yes.** The `cleanup()` → `is_initialized()` state machine governs model lifetime across the
entire system. In multi-test runs the reload path is exercised by test ordering; in production
it gates graceful restart. This test is the only one in the codebase that explicitly sequences
all three transitions: `load → clear → reload`.

#### Unique?

**Yes.** No sibling file tests `cleanup()` as a first-class subject. `vector_tools_batch.rs`
calls `cleanup()` defensively as a preamble but never asserts on `is_initialized()` or the
post-cleanup state.

#### Correct assertions?

**Partial.** The assertions are logically sound but have two gaps:

1. **No pre-state assertion before the first `cleanup()`.** The defensive call may be a no-op
   (model not loaded) or may clear a model (model left loaded by a previous test). Neither
   state is verified. This is acceptable as a defensive measure, but it means the test gives
   no signal about teardown from a live-model state.

2. **The reload is asserted structurally (`is_initialized() == true`) but not functionally.**
   After `cleanup()` + `load_model()`, there is no check that the reloaded model produces
   the same output as the pre-cleanup model, or even that embedding dimensions are preserved.
   For the use case of verifying graceful restart, a dimension or embedding-parity assertion
   on a trivial sample would strengthen the claim.

#### Good name?

**Partial.** `cleanup_properly_disposes_resources` describes the cleanup direction but omits the
reload-after-cleanup path, which is the behaviorally novel and operationally important part of
the test. A reader following the name alone would not realize the test also verifies
reinitialization.

#### Verdict: REWRITE (minor)

The test covers the right behavior. The rewrite should:

- Split into two focused tests:
  - `cleanup_disposes_initialized_model` — load → assert initialized → cleanup → assert not initialized
  - `model_reinitializes_cleanly_after_cleanup` — cleanup → load → cleanup → load → assert initialized (+ optional: embedding dimensions unchanged)
- Optionally add a `vector::is_initialized()` assertion *before* the first `cleanup()` to
  verify the pre-state is known (eliminating the ambiguity of the defensive call).

---

### Test 2 — `embedding_quality_differs_between_onnx_and_fallback`

**Tag**: T-8.4 / FR-33.1  
**Location**: line 33

#### What it does

```
cleanup()                                              // drops model state
generate_embedding("graph semantic search quality")    // lazy re-init + infer
assert any(|v| v.abs() > EPSILON)                     // not all zeros
assert any(|v| (v - first).abs() > EPSILON)           // not all uniform
```

#### Load-bearing?

**Partial.** The non-degeneracy check (non-zero, non-uniform) is a meaningful sanity gate — a
broken ONNX session returning a zero vector would silently poison all downstream semantic
ranking. However, the test asserts the *minimum* possible quality bar. It would pass even if
the model returned a vector where 383 of 384 dimensions were zero and one was non-zero.

#### Unique?

**Yes.** No sibling test explicitly checks that a freshly-generated ONNX embedding contains
real signal rather than degenerate output. The `vector_tools_batch.rs` tests check dimension
counts and batch parity but not value quality.

#### Correct assertions?

**No.**

**Primary defect — the name does not match the test body.** The name
`embedding_quality_differs_between_onnx_and_fallback` asserts:

- That there are *two* code paths being compared (ONNX and fallback)
- That the test *measures a difference* between them

Neither is true. The test body exercises a single path (`generate_embedding` → ONNX inference
after lazy `load_model()`), checks two heuristics on a single embedding, and never references
the `zero_embedding_vector()` fallback or any alternative code path. The name is not a
documentation imprecision — it is a factual claim about the test's behavior that is wrong.

The risk of this mismatch: any reviewer auditing T-8.4 coverage who reads this test name will
believe the fallback/degradation path is covered. It is not.

**Secondary defect — assertions are necessary but not sufficient.** "Not all zeros" and "not
all uniform" exclude the two most obvious failure modes but leave open a large space of
near-degenerate output. The test would pass if the model returned `[0.001, 0.0, 0.0, …]`.
A stronger gate:

- Check that `embedding.len() == vector::EMBEDDING_DIMENSIONS` (ONNX produced the expected
  dimensionality)
- Check that two semantically different inputs produce embeddings with cosine similarity < 1.0
  (confirming the model differentiates content — the *actual* quality claim implied by the name)

**Tertiary defect — lazy re-init is implicit, not stated.** The `cleanup()` call before
`generate_embedding()` exercises the lazy initialization path (since `generate_embedding`
internally calls `load_model()` when the model is absent). This is meaningful behavior to
verify, but it's invisible from the test name and not asserted via `is_initialized()` at any
point.

#### Good name?

**No.** The name is actively misleading. It claims a comparative test (ONNX vs. fallback) that
does not exist. At minimum the name must change; ideally the test is rewritten to either
*actually perform the comparison* the name implies, or *accurately describe* what it does.

#### Verdict: REWRITE

Suggested replacement:

```rust
// T-8.4: RTM FR-33.1 — ONNX embedding via lazy load is non-degenerate:
// correct dimensionality, non-zero, and non-uniform.
#[test]
fn onnx_embedding_is_non_degenerate_after_lazy_model_load() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();
    assert!(!vector::is_initialized(), "pre-condition: model cleared");

    let embedding = vector::generate_embedding("graph semantic search quality").expect("embedding");

    assert!(vector::is_initialized(), "generate_embedding should trigger lazy load");
    assert_eq!(
        embedding.len(),
        vector::EMBEDDING_DIMENSIONS,
        "embedding must match declared ONNX output dimension"
    );
    assert!(
        embedding.iter().any(|v| v.abs() > f32::EPSILON),
        "real ONNX embedding must not be all zeros"
    );
    let first = embedding[0];
    assert!(
        embedding.iter().any(|v| (v - first).abs() > f32::EPSILON),
        "real ONNX embedding must not be all-uniform"
    );
}

// T-8.4: RTM FR-33.1 — ONNX model differentiates semantically distinct inputs.
// Verifies the claim that was implied (but absent) from the old fallback-comparison test.
#[test]
fn onnx_embeddings_differentiate_semantically_distinct_inputs() {
    let _guard = TEST_MUTEX.lock();
    vector::cleanup();

    let a = vector::generate_embedding("knowledge graph").expect("embedding a");
    let b = vector::generate_embedding("invoice payment processing").expect("embedding b");

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let cosine = if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        1.0
    };
    assert!(
        cosine < 0.99,
        "semantically distinct inputs must produce meaningfully different embeddings (cosine={cosine:.4})"
    );
}
```

The second new test would be the test that the current name implies but the body never delivers.

---

## Recommendations

1. **REWRITE Test 1** (`cleanup_properly_disposes_resources`) into two focused tests:
   `cleanup_disposes_initialized_model` and `model_reinitializes_cleanly_after_cleanup`.
   Each test should assert only one lifecycle transition. Add a pre-state
   `is_initialized()` assertion so the test's entry condition is explicit, not assumed.

2. **REWRITE Test 2** (`embedding_quality_differs_between_onnx_and_fallback`) with a name
   that accurately describes behavior. The minimum fix is renaming. The correct fix is
   splitting into:
   - `onnx_embedding_is_non_degenerate_after_lazy_model_load` — current behavior, renamed
     and strengthened with a dimension assertion and explicit `is_initialized()` bookends.
   - `onnx_embeddings_differentiate_semantically_distinct_inputs` — the comparison test the
     original name implied but never delivered. This is the actual quality gate the name was
     advertising.

3. **Document the absent fallback comparison.** Once the `access_boosting_search.md`
   recommendation (REWRITE of `search_memories_semantic_mode_does_not_update_last_accessed`)
   is executed, the misplaced T-8.4 probe there will be removed. At that point
   `vector_tools_runtime.rs` becomes the **sole** source of T-8.4 runtime coverage.
   The file should reflect complete coverage of the contract: non-degenerate output, correct
   dimensions, semantic differentiation, model lifecycle, and lazy re-initialization.

4. **Do not add fallback coverage for a fallback that does not exist.** The `zero_embedding_vector()` path is an internal error-recovery mechanism (returned on empty input or
   decode failure), not a "fallback inference mode." The original test name implies a runtime
   fallback mode — that concept does not exist. Do not create a test for it; remove the
   concept from the test vocabulary.

5. **Consider a `#[cfg(feature = "onnx_runtime")]` guard or `#[ignore]` annotation.** Both
   tests require the ONNX model to be present on disk and the `ort` runtime to initialize
   successfully. In CI environments where the model is absent, these tests fail at `load_model`
   with an opaque error. Either gate them explicitly or document the prerequisite.

---

## Load-Bearing Score: 1.5 / 2

| Test | Load-Bearing Score | Reasoning |
|------|--------------------|-----------|
| `cleanup_properly_disposes_resources` | **1.0 / 1.0** | Only test exercising model disposal and reinitialization. Removing it eliminates T-8.4 lifecycle coverage entirely. |
| `embedding_quality_differs_between_onnx_and_fallback` | **0.5 / 1.0** | The non-degeneracy invariant is real and worth testing. Half score because the test is too weak to constitute a meaningful quality gate and its name overclaims coverage of a code path that does not exist. |

**Overall: 1.5 / 2** — Both tests survive the cut, but neither can be shipped as-is without
misleading future readers and leaving coverage gaps that are invisible unless the test body is
read in full.
