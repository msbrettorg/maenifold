---
id: dynamic_model_tokenizer
status: done
---

# Test Evaluation: dynamic_model_tokenizer.rs

**Traceability**: T-10.11 | FR-18 | RTM-098, RTM-099  
**Evaluator**: Gray (Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/dynamic_model_tokenizer.rs`  
**Tests evaluated**: 2  

---

## Summary

This file contains two integration tests for the [[dynamic model loading]] tokenizer path. Both tests operate against RTM-098 and RTM-099 from FR-18 (Sprint 9 RTM coverage backfill). They exercise the live ONNX model pipeline end-to-end, relying on `TestContext` for filesystem isolation and `TEST_MUTEX` for serialization against the shared global `MODEL_STATE`.

**File is load-bearing.** Removing it would leave two distinct FR-18 contract requirements (tokenizer.json precedence and `model_max_length` defaulting) with zero dedicated boundary coverage. The `vector/tests.rs` unit tests do not substitute — they test inference correctness, not tokenizer asset selection logic.

**Structural concerns are limited but real.** One test hardcodes a constant value in both its assertion and its name. One test bundles two behaviorally distinct scenarios into a single `#[test]` block. These are not cosmetic nits — they reduce diagnosability and create silent breakage risk if `DEFAULT_MAX_TOKENS` ever changes.

---

## Verdicts

| # | Test Name | Covers | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | Verdict |
|---|-----------|--------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `t10_11_tokenizer_json_is_preferred_over_vocab_txt` | RTM-098 | ✅ Yes | ✅ Yes | ⚠️ Mostly (key assertion implicit) | ✅ Yes | 🔘 Marginally | **KEEP** |
| 2 | `t10_11_max_tokens_reads_config_and_defaults_to_512` | RTM-099 | ✅ Yes | ⚠️ Partial (Scenario A overlaps T1) | ⚠️ Mostly (hardcodes constant; missing zero-value edge) | ❌ No | ✅ Yes | **REWRITE** |

---

## Per-Test Analysis

---

### `t10_11_tokenizer_json_is_preferred_over_vocab_txt`

**Covers**: RTM-098 — when both `tokenizer.json` and `vocab.txt` fallback assets exist, runtime loading MUST prefer `tokenizer.json`.

#### Is it load-bearing?

Yes. It is the **only test** that deliberately poisons `vocab.txt` with invalid content to force a failure if the tokenizer falls back to the wordpiece path. No other test file constructs this adversarial precondition. `dynamic_model_loading.rs` only installs valid fallback assets and never breaks vocab intentionally. Removing this test would leave RTM-098 unverified.

#### Is the assertion correct?

**Mostly, but the primary assertion is implicit, not explicit.**

The test proves tokenizer.json preference through a negative implication: it writes a vocab.txt that would cause `WordPiece::from_file` to fail, then asserts `load_model()` succeeds. The success itself is the signal. There is no assertion of the form `assert!(used_tokenizer_json)` or `assert!(!used_vocab_txt)`.

This is epistemically valid — if vocab.txt were preferred, `load_model()` would return `Err` and the `expect()` would panic. But the mechanism is invisible to a reader who doesn't trace through the source. A reader looking at the four explicit assertions:

```rust
assert_eq!(vector::model_filename().as_deref(), Some("preferred-tokenizer.onnx"));
assert_eq!(vector::max_tokens(), Some(96));
assert_eq!(vector::embedding_dimensions(), Some(384));
assert_eq!(vector::generate_embedding("tokenizer preference")...len(), 384);
```

...might not immediately understand that the critical proof is the absence of a panic, not any of these four values.

Secondary concern: `embedding_dimensions() == Some(384)` and `generate_embedding().len() == 384` hardcode the packaged model's output dimensionality. This is reasonable since the packaged model (`all-MiniLM-L6-v2.onnx`) is stable and `DEFAULT_EMBEDDING_DIMENSIONS` is 384. However, the test could use the constant instead to make the coupling explicit.

`max_tokens() == Some(96)` asserts behavior that test 2 Scenario A also verifies. Mild duplication, not harmful.

#### Is the name good?

Yes. `t10_11_tokenizer_json_is_preferred_over_vocab_txt` is precise, traceable to T-10.11, and describes the exact semantic contract being enforced.

#### Is it parameterizable?

Marginally. The model name `"preferred-tokenizer.onnx"` and `model_max_length: 96` are incidental fixture values with no behavioral significance beyond being distinct from defaults. Parameterization would not yield meaningful additional coverage.

**Verdict: KEEP.**  
Optional improvement: add a comment noting that the `expect()` itself is the primary assertion (if vocab.txt were preferred, load_model() would panic here), making the implicit proof explicit for future readers.

---

### `t10_11_max_tokens_reads_config_and_defaults_to_512`

**Covers**: RTM-099 — `tokenizer_config.json` supplies `model_max_length` when present; runtime falls back to 512 when the field is absent.

#### Is it load-bearing?

Yes. Scenario B (the default-to-512 path) is unique to this test. No other test constructs a `tokenizer_config.json` that omits `model_max_length` and then asserts the fallback. Scenario A (explicit `model_max_length: 128`) is the only mild overlap — test 1 also asserts `max_tokens() == Some(96)` with a non-default config — but Scenario A is still load-bearing because it uses a different config value and exercises the path independently.

#### Is the assertion correct?

**Mostly, but with three concrete problems:**

**Problem 1 — Hardcoded constant.**  
Scenario B asserts:
```rust
assert_eq!(vector::max_tokens(), Some(512));
```
The value `512` is `DEFAULT_MAX_TOKENS` from `crates/maenifold-core/src/vector/mod.rs`. If that constant is ever changed (e.g., to 768 to support a larger model), this assertion silently becomes wrong: the test will fail even though the behavior is correct (falling back to the new default). The assertion should read:

```rust
use maenifold_core::vector::DEFAULT_MAX_TOKENS;
assert_eq!(vector::max_tokens(), Some(DEFAULT_MAX_TOKENS));
```

**Problem 2 — The test name hardcodes the constant.**  
`defaults_to_512` will become a misleading name the moment `DEFAULT_MAX_TOKENS` changes. The name encodes an implementation detail that the test should be agnostic about.

**Problem 3 — Missing zero-value edge case.**  
The source in `tokenizer.rs` explicitly guards against `model_max_length: 0`:
```rust
.filter(|max_tokens| *max_tokens > 0)
.unwrap_or(DEFAULT_MAX_TOKENS)
```
This filter exists because `model_max_length: 0` is a valid (malformed) JSON value that must not propagate to ONNX inference. This edge case is not tested anywhere. A `model_max_length: 0` config should behave identically to a missing field. This is the **most underspecified path** in the tokenizer loading logic.

#### Is the test name good?

No. `defaults_to_512` couples the test name to a hardcoded constant. If `DEFAULT_MAX_TOKENS` changes, the name becomes actively misleading. Prefer `defaults_to_configured_default` or simply `falls_back_to_default_when_absent`.

#### Is it parameterizable?

Yes, and it should be. The two scenarios differ only in the config JSON and expected output:

| Scenario | Config JSON | Expected `max_tokens()` |
|----------|-------------|------------------------|
| A | `{"pad_token":"[PAD]","model_max_length":128}` | `Some(128)` |
| B | `{"pad_token":"[PAD]"}` | `Some(DEFAULT_MAX_TOKENS)` |
| C (missing) | `{"pad_token":"[PAD]","model_max_length":0}` | `Some(DEFAULT_MAX_TOKENS)` |

A helper function or macro that runs each row would make adding Scenario C trivial and make the pattern self-documenting.

**Verdict: REWRITE.**  
Split into two (or three) focused tests. Replace the hardcoded `512` literal with `DEFAULT_MAX_TOKENS`. Rename to remove the hardcoded value. Add the `model_max_length: 0` edge case as a third scenario — the source code has the guard, but it is untested.

---

## Recommendations

1. **KEEP test 1 as-is**, but add a one-line comment:
   ```rust
   // Primary assertion: load_model() succeeding here proves tokenizer.json was used,
   // because the deliberately-broken vocab.txt would cause WordPiece::from_file to panic.
   vector::load_model().expect("load model via tokenizer.json");
   ```

2. **REWRITE test 2** into three focused tests:

   ```rust
   // RTM-099a: explicit model_max_length is respected
   fn t10_11_max_tokens_reads_model_max_length_from_config() { ... assert_eq!(max_tokens(), Some(128)); }

   // RTM-099b: absent model_max_length falls back to DEFAULT_MAX_TOKENS
   fn t10_11_max_tokens_defaults_when_field_absent() { ... assert_eq!(max_tokens(), Some(DEFAULT_MAX_TOKENS)); }

   // RTM-099c: model_max_length: 0 is treated as absent and falls back to DEFAULT_MAX_TOKENS
   fn t10_11_max_tokens_treats_zero_as_absent() { ... assert_eq!(max_tokens(), Some(DEFAULT_MAX_TOKENS)); }
   ```

3. **Consider re-exporting `DEFAULT_MAX_TOKENS` as `pub`** (it is currently `pub(crate)`) so that integration test files can reference it without duplicating the literal. The `vector::max_tokens()` inspector was added specifically so boundary tests can verify this — the constant should be equally accessible.

4. **Do not merge the two top-level tests.** They cover distinct RTM requirements (RTM-098 vs RTM-099) and have different setup/teardown profiles. Merging would reduce diagnostic precision on CI failures.

---

## Load-Bearing Score: 2/2

Both tests cover distinct, non-overlapping requirement statements (RTM-098, RTM-099) with no other test providing equivalent coverage. The file earns a full load-bearing score. **Neither test is a candidate for removal.** The rewrite of test 2 is a quality improvement, not a removal.
