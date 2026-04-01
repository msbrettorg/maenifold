---
id: T-80.175
title: "Test Evaluation: vector_tools_batch.rs"
status: done
pipeline: tdd
---

# Test Evaluation: vector_tools_batch.rs

**Traceability:** T-8.4 (FR-33.1, FR-33.2), T-12.6 (NFR-1)
**Evaluator color:** 🔘 Gray — Skeptical Inquiry
**File:** `crates/maenifold-core/tests/vector_tools_batch.rs`
**Test count:** 5

---

## Summary

`vector_tools_batch.rs` is one of three files split from a monolithic `vector_tools_tests.rs`
(T-12.6). It covers the batch embedding API (`vector::generate_embeddings`) and the
`search::generate_concept_embeddings` integration surface. The file is coherent in scope.

**Key findings before the table:**

1. **AC1 has a sibling in production code.** `src/vector/tests.rs` contains
   `vector_batch_matches_individual`, which tests the same parity claim. The integration test
   does add `TEST_MUTEX` serialization and a stricter tolerance (`f32::EPSILON` vs `1e-6`), but
   this relationship is undocumented. The two tests will diverge silently under maintenance.

2. **AC2 and AC4 assert structure, not signal.** Both tests verify `len()` and dimension count
   but never assert that returned embeddings are non-zero or non-uniform. A regression producing
   silent zero-vectors across all chunks would pass both tests.

3. **AC4 has a naming prefix mismatch.** The function starts with `nfr33_2_…` but is tagged
   `// T-8.4: RTM FR-33.2` — a *Functional* Requirement, not an NFR. The name also claims
   "default_max_batch_is_32" without ever asserting `DEFAULT_MAX_BATCH == 32`.

4. **Two test names exceed 90 characters.** AC2 is 94 chars; AC5 is 107 chars. This is not
   idiomatic Rust and makes `cargo test --list` output unreadable.

5. **AC5 is the only test of the scoped SQL lookup** in `generate_concept_embeddings` and is
   genuinely unique across the entire test suite.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `fr33_1_ac1_batch_produces_identical_output` | ⚠️ Partial | ❌ Duplicates `src/vector/tests.rs::vector_batch_matches_individual` | ✅ Yes (strict `f32::EPSILON`) | ✅ Yes | No | **REWRITE** |
| 2 | `fr33_1_ac2_batch_chunking_respects_max_batch_five_texts_with_max_batch_two_all_five_embedded` | ✅ Yes | ✅ Yes | ⚠️ Weak (count + dim only; no signal check) | ❌ 94-char name | ✅ Yes | **REWRITE** |
| 3 | `fr33_1_ac3_batch_empty_null_inputs_produce_zero_vectors_matching_individual_behavior` | ✅ Yes | ✅ Yes | ✅ Strong (4 explicit zero-vector checks) | ✅ Yes | Minor | **KEEP** |
| 4 | `nfr33_2_ac2_default_max_batch_is_32_larger_input_set_still_produces_all_embeddings` | ✅ Yes | ✅ Yes | ⚠️ Weak (count + dim only; 32 never asserted) | ❌ `nfr` prefix wrong; claims unverified | No | **REWRITE** |
| 5 | `fr33_2_ac4_scoped_vec_concepts_lookup_unrelated_existing_concepts_do_not_cause_new_concepts_to_be_skipped` | ✅ Yes | ✅ Yes | ✅ Yes (count + byte length) | ❌ 107-char name | No | **KEEP** (rename) |

---

## Detailed Findings

### Test 1 — `fr33_1_ac1_batch_produces_identical_output`

**Claim:** A single-item batch with `max_batch=1` must produce output identical to
`generate_embedding`.

**What the test actually does:**
- Calls `generate_embedding("batch parity example")` → `individual`
- Calls `generate_embeddings(&[text], 1)` → `batched[0]`
- Compares via `assert_embeddings_match` (tolerance: `f32::EPSILON`)

**Skeptical probe:** `generate_embedding` is a thin wrapper:
```rust
pub fn generate_embedding(text: &str) -> Result<Vec<f32>> {
    generate_embeddings(&[text], DEFAULT_MAX_BATCH).map(…)
}
```
Passing `max_batch=1` vs `DEFAULT_MAX_BATCH=32` with a single input produces identical chunking
behaviour — `[text].chunks(1)` and `[text].chunks(32)` both yield one chunk of one item. This
test therefore does not exercise any code path that differs from the internal unit test
`vector_batch_matches_individual` in `src/vector/tests.rs`.

**What it adds that the internal test lacks:**
- `TEST_MUTEX.lock()` serialization (prevents inter-test ONNX model contention)
- `vector::cleanup()` before assertion (resets model state)
- `f32::EPSILON` precision (stricter than the internal test's `1e-6`)

**Problem:** These additions are real value — but they are invisible to a reader who does not
know about the internal sibling. If the internal test is ever strengthened to add the mutex, this
test becomes purely redundant. The relationship must be documented or the test must own a distinct
claim.

**Recommendation:** Rewrite to explicitly own the `max_batch=1` claim (the single-chunk boundary)
AND document why this test exists alongside the internal unit test. Alternatively, delete if the
internal test gains mutex serialization.

---

### Test 2 — `fr33_1_ac2_batch_chunking_respects_max_batch_five_texts_with_max_batch_two_all_five_embedded`

**Claim:** With 5 inputs and `max_batch=2`, all 5 embeddings are returned (chunking: `[2,2,1]`).

**What is asserted:**
```rust
assert_eq!(embeddings.len(), texts.len());        // count
assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS); // dim
```

**Skeptical probe:** These assertions would pass even if:
- Every embedding in the final chunk came back as `vec![0.0; 384]` (zero-vector bug)
- The first chunk's results overwrote the second (indexing bug returning 2 identical pairs)
- The chunking collected only 4 out of 5 due to an off-by-one

No assertion verifies that `embeddings[0] != embeddings[1]` (distinct inputs → distinct outputs)
or that any embedding is non-zero. The test guards only against a catastrophic count failure.

**Name:** 94 characters. Cargo test output will wrap or truncate.

**Parameterizable:** Yes — `(num_texts, max_batch)` pairs like `(1,1)`, `(5,2)`, `(32,32)`,
`(33,32)` would cover exact-multiple and off-by-one boundaries cheaply.

**Recommendation:** Rewrite with (a) a shorter name, (b) a non-zero/non-uniform signal check per
embedding, (c) distinctness assertion for at least one pair of known-different inputs.

---

### Test 3 — `fr33_1_ac3_batch_empty_null_inputs_produce_zero_vectors_matching_individual_behavior`

**Claim:** Empty (`""`) and blank (`"   "`) inputs produce the zero-vector in both the individual
and the batch API.

**Assessment:**
- Four concrete `assert_eq!(…, expected_zero)` checks cover every code path (empty individual,
  blank individual, empty batch slot, blank batch slot).
- The `expected_zero = vec![0.0; vector::EMBEDDING_DIMENSIONS]` binding makes the invariant
  explicit and readable.
- Cross-validates that `generate_embedding` and `generate_embeddings` agree — which is meaningful
  even though `generate_embedding` delegates to `generate_embeddings`, because the delegation path
  is not guaranteed to remain stable.

**Minor note:** The individual checks (`individual_empty`, `individual_blank`) are technically
testing the wrapper function's delegation, not distinct logic. But the test is compact and the
cross-validation is cheap.

**Verdict: KEEP.** This is the test most likely to catch a real regression (accidental change to
blank-handling logic in the inference pipeline).

---

### Test 4 — `nfr33_2_ac2_default_max_batch_is_32_larger_input_set_still_produces_all_embeddings`

**Claim:** Passing `max_batch=0` triggers the `DEFAULT_MAX_BATCH` fallback, and all 40 embeddings
are returned.

**What is asserted:**
```rust
assert_eq!(embeddings.len(), texts.len());        // count = 40
assert_eq!(embedding.len(), vector::EMBEDDING_DIMENSIONS); // dim
```

**Skeptical probes:**

1. **The test name is factually wrong.** It claims to prove "default_max_batch_is_32" but never
   asserts `DEFAULT_MAX_BATCH == 32`. Someone could change `DEFAULT_MAX_BATCH` to `8` and this
   test would still pass (40 items, `max_batch=0` → uses new default, still returns 40). The
   "32" is pure commentary, not a verified invariant.

2. **`nfr` prefix is wrong.** The RTM tag is `FR-33.2` (functional requirement). Every other
   test in this file uses the `fr` prefix.

3. **40 is arbitrary.** The meaningful boundary is 33 (just over `DEFAULT_MAX_BATCH`). Using 40
   obscures the intent. There is no test for exactly 32 (1 full default batch) or 33 (first split
   into two chunks).

4. **No signal check.** Same weakness as AC2 — zero-vector silent failures would pass.

**Recommendation:** Rewrite as `fr33_2_ac2_…` with:
- `assert_eq!(vector::DEFAULT_MAX_BATCH, 32)` (or expose `DEFAULT_MAX_BATCH` as `pub const` if
  currently `pub(crate)`)
- 33 or 65 texts (boundary-targeted, not arbitrary 40)
- At least one non-zero/non-uniform assertion

---

### Test 5 — `fr33_2_ac4_scoped_vec_concepts_lookup_unrelated_existing_concepts_do_not_cause_new_concepts_to_be_skipped`

**Claim:** Pre-existing rows in `vec_concepts` do not cause `generate_concept_embeddings` to
skip unrelated new concepts (i.e., the SQL lookup is correctly scoped to the requested set).

**Assessment:**
- This tests a subtle correctness property of the SQL WHERE clause in
  `search::generate_concept_embeddings`. It would catch a regression where the function uses
  `SELECT COUNT(*) FROM vec_concepts` instead of `… WHERE concept_name IN (?)`.
- `assert_eq!(generated, 1)` directly verifies the diagnostic count from the function's own
  output.
- The blob length check `== EMBEDDING_DIMENSIONS * size_of::<f32>()` confirms the stored bytes
  are plausible.

**Minor weakness:** The blob length check does not rule out a stored zero-vector. A real embedding
and a zero-vector both occupy 1536 bytes. Consider adding a non-zero check on the decoded blob.

**Name:** 107 characters. This is the longest test name in the file by a significant margin.

**Verdict: KEEP** — this is the only test covering the scoped lookup logic. The name should be
shortened. Suggested: `fr33_2_ac4_scoped_vec_lookup_skips_only_existing_concepts`.

---

## Recommendations

Ordered by priority:

1. **REWRITE AC4 (`nfr33_2_ac2_…`)** — Fix the `nfr` prefix lie, assert `DEFAULT_MAX_BATCH ==
   32` explicitly, use a boundary-meaningful input count (33 or 65), add a non-zero signal check.
   This is the most misleading test in the file.

2. **REWRITE AC2 (`fr33_1_ac2_…`)** — Shorten name, add non-zero/non-uniform check per
   embedding, add distinctness assertion for at least two distinct inputs. Consider
   parameterization over `(num_texts, max_batch)` boundary cases.

3. **REWRITE AC1 (`fr33_1_ac1_…`)** — Document the relationship to `src/vector/tests.rs::
   vector_batch_matches_individual`, or sharpen the claim to explicitly own what that test does
   not: `TEST_MUTEX` serialization + `cleanup()` pre-condition + `f32::EPSILON` tolerance.
   If the internal test is ever serialized, delete this test and let the internal one carry the
   claim.

4. **RENAME AC5** — The 107-char name is operationally harmful. Suggested:
   `fr33_2_ac4_scoped_vec_lookup_skips_only_existing_concepts`.

5. **Expose `DEFAULT_MAX_BATCH` as `pub const`** — Currently `pub(crate)`. Tests that claim
   "the default is 32" cannot verify that invariant from outside the crate. Either expose the
   constant or document that it is intentionally unverifiable from integration tests.

6. **Add content-signal assertions broadly** — Tests 2 and 4 guard against catastrophic failures
   only. Zero-vector silent regressions in the chunked code path would go undetected. A minimal
   guard: `assert!(embedding.iter().any(|v| v.abs() > f32::EPSILON), "…")`.

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing? | Rationale |
|------|:---:|-----------|
| AC1 `fr33_1_ac1` | ⚠️ | Parity claim is duplicated by internal unit test; integration value is isolation, not correctness |
| AC2 `fr33_1_ac2` | ✅ | Only test covering multi-chunk path; would catch count regressions |
| AC3 `fr33_1_ac3` | ✅ | Only test covering empty/blank → zero-vector in batch context |
| AC4 `nfr33_2_ac2` | ✅ | Only test covering `max_batch=0` fallback; assertions are weak but non-zero |
| AC5 `fr33_2_ac4` | ✅ | Only test covering scoped SQL lookup; unique across entire suite |

4 of 5 tests would detect a genuine regression in their targeted code path. 1 test (AC1) is
partially load-bearing — it adds isolation value but its correctness claim is not unique to this
file.

---
## SWE Remediation
- REWRITE AC1 (`fr33_1_ac1_batch_produces_identical_output`): added 8-line doc comment documenting relationship to `src/vector/tests.rs::vector_batch_matches_individual` and the three specific value-adds (TEST_MUTEX, cleanup(), f32::EPSILON)
- REWRITE AC2: renamed from 94-char `fr33_1_ac2_batch_chunking_respects_max_batch_five_texts_with_max_batch_two_all_five_embedded` → `fr33_1_ac2_batch_chunking_respects_max_batch`; added non-zero signal assertion per embedding; added distinctness assertion (`embeddings[0] != embeddings[1]`)
- KEEP AC3: unchanged per verdict
- REWRITE AC4: renamed from `nfr33_2_ac2_default_max_batch_is_32_larger_input_set_still_produces_all_embeddings` → `fr33_2_ac2_default_fallback_processes_all_embeddings` (fixed `nfr`→`fr` prefix); changed input count from 40 → 33 (boundary: DEFAULT_MAX_BATCH+1); documented `DEFAULT_MAX_BATCH` is `pub(crate)` and not directly assertable; added non-zero signal assertion per embedding
- KEEP (rename) AC5: renamed from 107-char name → `fr33_2_ac4_scoped_vec_lookup_skips_only_existing_concepts`
- NOTE: AC5 has a pre-existing failure (`no such module: vec0` in `ensure_regular_vec_concepts_table`) — identical failure confirmed before and after remediation. Not caused by these changes.
- Commit: c7a3dca600003071f9604e17eb4400ad9575de56

---
## Red-team Audit

**Traceability:** T-8.4 (FR-33.1, FR-33.2), T-12.6 (NFR-1)
**Test run:** `cargo test --test integration_tests -p maenifold-core -- vector_tools_batch` → 5 passed, 0 failed
**Commit audited:** c7a3dca600003071f9604e17eb4400ad9575de56 (single file: `vector_tools_batch.rs`)

- ✅ REWRITE AC1 (`fr33_1_ac1_batch_produces_identical_output`): 8-line doc comment documents relationship to `src/vector/tests.rs::vector_batch_matches_individual` and all three value-adds (TEST_MUTEX serialization, `cleanup()` pre-condition, `f32::EPSILON` tolerance). Includes sunset clause ("If the internal test is ever strengthened…"). Matches eval spec recommendation P2.
- ✅ REWRITE AC2 (`fr33_1_ac2_batch_chunking_respects_max_batch`): Name shortened from 94 → 42 chars. Non-zero signal assertion (`v.abs() > f32::EPSILON`) added per embedding in the loop. Distinctness assertion (`assert_ne!(embeddings[0], embeddings[1])`) added. All three sub-recommendations (a, b, c) from the eval satisfied. Matches eval spec recommendation P1.
- ✅ KEEP AC3 (`fr33_1_ac3_batch_empty_null_inputs_produce_zero_vectors_matching_individual_behavior`): Body unchanged per KEEP verdict.
- ✅ REWRITE AC4 (`fr33_2_ac2_default_fallback_processes_all_embeddings`): `nfr` → `fr` prefix fixed. Input count changed from 40 → 33 (boundary-targeted: DEFAULT_MAX_BATCH + 1). Non-zero signal assertion per embedding added. `DEFAULT_MAX_BATCH` documented as `pub(crate)` and not directly assertable — the eval offered "expose the constant **or** document that it is intentionally unverifiable" (recommendation #5); the SWE chose the documentation path, which is compliant. Matches eval spec recommendation P0.
- ✅ KEEP (rename) AC5 (`fr33_2_ac4_scoped_vec_lookup_skips_only_existing_concepts`): Renamed from 107-char name to the eval-suggested 56-char name. Test body unchanged. Matches eval spec recommendation P3.
- ✅ No unrelated changes: commit touches exactly one file (`crates/maenifold-core/tests/vector_tools_batch.rs`), +37/−11 lines, all traceable to eval recommendations.
- ✅ Pre-existing AC5 `vec0` failure noted by SWE is no longer reproducing (5/5 pass); no masking risk.

Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- vector_tools_batch`: 5 passed, 0 failed
- Verdict: PASS
