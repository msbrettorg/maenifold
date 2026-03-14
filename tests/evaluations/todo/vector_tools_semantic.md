# Test Evaluation: vector_tools_semantic.rs

**Traceability**: T-12.6, T-12.7, T-14.9, T-0.1 | FR-6 (RTM-042, RTM-043) | NFR-1, NFR-3  
**Reviewed by**: Red Team / Gray (Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/vector_tools_semantic.rs`  
**Support module**: `crates/maenifold-core/tests/vector_tools/support.rs`  
**Test count**: 3

---

## Summary

`vector_tools_semantic.rs` is a focused, three-test file that validates the semantic-validation
branch of `repair_concepts`. Its central concern is the RTM-042/RTM-043 behavioral contract: when
embeddings are degenerate (NULL, empty-string), the tool must return a `BLOCKED` warning payload
rather than an `Err`; when they are above threshold, it must continue into the repair scan. The
concurrency test bridges to the C# oracle for thread-safety parity.

**Gray concern — coverage uniqueness is real.** Every `repair_concepts_replacement.rs` and
`repair_concepts_batch.rs` test passes `"minSemanticSimilarity": 0.0`, which hard-bypasses semantic
validation entirely. This file is the *only* location in the test suite that exercises the
BLOCKED-path and the above-threshold success-path. Removing or weakening any test here creates a
coverage gap directly against RTM-042 and RTM-043 with no fallback.

**Gray concern — test 2 name is misleading.** `thread_safe_initialization_works` inherits its name
from a C# fixture that tested model initialization. The Rust test does not touch initialization
(model load/unload) — it spawns 10 threads that each run `repair_concepts` concurrently and
verifies output consistency. The name creates a false map from the C# oracle and masks what the
test is actually asserting.

**Gray concern — tests 2 and 3 share 100% of their core assertions.** Both use identical embeddings
at threshold 0.99 and assert `"All variants pass semantic similarity check"`. Test 2 adds a
concurrency wrapper but its single-threaded correctness premise is identical to test 3. There is no
isolated single-threaded regression that would fail test 3 while test 2 passes, since test 2
*requires* test 3's happy-path to be true for every thread.

**Gray concern — cosine similarity is 1.0, not near-threshold.** `deterministic_embedding(0.75)`
applied to both "tool" and "tools" produces vectors where cosine similarity is exactly 1.0 (verified
analytically). Threshold 0.99 is never stressed: a bug that returns 0.985 would still pass. There is
no test for a vector pair where similarity is measurably above the threshold but below 1.0.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Assertions Correct? | Name Accurate? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `empty_and_null_inputs_produce_safe_embeddings` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Partial | ⚠️ Yes — threshold | **REWRITE** |
| 2 | `thread_safe_initialization_works` | ✅ Yes | ⚠️ Partial | ✅ Yes | ❌ No | 🔲 Minor | **REWRITE** |
| 3 | `above_threshold_similarity_returns_success_output` | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | ⚠️ Yes — threshold | **KEEP** |

---

## Per-Test Analysis

---

### Test 1: `empty_and_null_inputs_produce_safe_embeddings`

**Traceability**: T-14.9 | RTM-042 | FR-6

**What it tests**: Inserts a NULL embedding for the canonical concept ("tool") and an empty-string
embedding for the variant ("tools"), then calls `repair_concepts` with
`minSemanticSimilarity: 0.10`. Asserts the output is `BLOCKED`, includes
`"tools: similarity 0.00 (threshold: 0.10)"`, and does *not* contain `"Scanning"`.

**Load-bearing?** Yes. RTM-042 requires degenerate embeddings return a `BLOCKED` warning payload,
not `Err`. No other test in the suite inserts NULL or empty-string embeddings and exercises this
code path. Removing this test deletes the only behavioral contract for the degenerate-input guard.

**Unique?** Yes. Cross-checked: `repair_concepts_replacement.rs` and `repair_concepts_batch.rs`
use `minSemanticSimilarity: 0.0` in all cases, which bypasses semantic validation before any
similarity is computed. This file is the sole owner of the BLOCKED path.

**Assertions correct?** Mostly. Three assertions are good: BLOCKED marker, specific
similarity/threshold string, and the short-circuit guard (`!contains("Scanning")`). One
unexplored angle: the canonical concept ("tool") has a NULL embedding while the variant ("tools")
has an empty-string embedding. The asymmetry means the test checks that either degenerate
representation triggers BLOCKED, but a reader cannot tell which is the observed trigger without
reading the implementation. Consider splitting into `null_canonical_blocked` and
`empty_variant_blocked` to isolate failure modes.

**Name accurate?** Partially. "safe embeddings" reads as "produces embeddings safely" (a generation
concern) rather than "degenerate stored embeddings are handled safely in the validation path" (the
actual concern). Misleading to newcomers.

**Parameterizable?** The threshold (0.10) and similarity string are tightly coupled. The core
behavior (NULL/empty → similarity 0.00, any threshold > 0.0 → BLOCKED) does not require 0.10
specifically. The string literal `"tools: similarity 0.00 (threshold: 0.10)"` makes the test
fragile if the threshold changes in future parameterized coverage.

**Verdict: REWRITE** — Preserve the test's behavioral intent. Rename to
`null_and_empty_embeddings_produce_blocked_warning`. Optionally split the NULL-canonical case from
the empty-string-variant case. Decouple the threshold literal from the assertion if the format
string is subject to change.

---

### Test 2: `thread_safe_initialization_works`

**Traceability**: T-0.1 | RTM-043 (concurrency aspect) | NFR-1

**What it tests**: Spawns 10 threads; each creates its own `ToolContext` and calls
`repair_concepts` with identical deterministic embeddings at threshold 0.99, dry-run mode. Asserts
every thread independently observes `"All variants pass semantic similarity check"` and
`"Total replacements: 1"`.

**Load-bearing?** Yes, but narrowly. It guards against: (a) data races on shared mutable global
state in the vector search path, and (b) non-determinism in SQLite concurrent reads under the
`ensure_regular_vec_concepts_table` setup. Removing it removes the only concurrency regression
guard on the semantic validation pipeline.

**Unique?** Partially. The *concurrency framing* is unique — no other test in the file spawns
threads. However, the *correctness premise* (identical embeddings pass threshold 0.99 and produce
the success banner) is asserted by test 3 as well. Test 2 cannot pass if test 3's happy-path is
broken, making test 2 a superset of test 3's single-threaded assertion in terms of dependency, but
not in terms of fault isolation.

**Assertions correct?** Yes. Checking `"All variants pass semantic similarity check"` and
`"Total replacements: 1"` on every thread handle is the right approach. The use of
`handle.join().expect("thread join")` catches panics from child threads. One concern: panics in
child threads could be obscured. If a thread silently returns wrong output instead of panicking,
the assertion catches it.

**Name accurate?** No. `thread_safe_initialization_works` names a C# test that validated ONNX model
initialization lifecycle. This Rust test does not call `vector::load_model()`, `vector::cleanup()`,
or `vector::is_initialized()`. Those are in `vector_tools_runtime.rs`. This test is about
concurrent execution correctness of `repair_concepts`, not initialization.

**Parameterizable?** The thread count (10) is reasonable. Not worth parameterizing given the
isolation-focused intent.

**Verdict: REWRITE** — Rename to `concurrent_repair_concepts_reads_produce_consistent_output`.
Update the comment to accurately describe what is tested in Rust (concurrent reads on shared SQLite,
not initialization). The behavioral assertions are correct as-is.

---

### Test 3: `above_threshold_similarity_returns_success_output`

**Traceability**: T-14.9 | RTM-043 | FR-6

**What it tests**: Inserts identical deterministic embeddings (cosine similarity = 1.0) for both
"tool" and "tools", calls `repair_concepts` with `minSemanticSimilarity: 0.99`, and asserts the
success banner and continuation into the file scan (`"Scanning 1 markdown files"`).

**Load-bearing?** Yes. This is the canonical happy-path regression for semantic validation. It
verifies the BLOCKED gate does not fire when similarity is genuinely above threshold, and that
execution continues into the repair scan. Critical complement to test 1's BLOCKED path.

**Unique?** Partially. The `"Scanning 1 markdown files"` assertion is the uniquely load-bearing
element not present in test 2. Test 2's assertions are a subset. If this test were removed, test 2
provides partial coverage, but the clean single-threaded success-path assertion would be gone.
Worth keeping as the clean baseline.

**Assertions correct?** Yes. The two assertions form a complete success-path contract: the semantic
gate passes, and execution proceeds into the file scan. This matches RTM-043's acceptance criteria.

**One gap**: Cosine similarity is 1.0 (identical vectors). The threshold is 0.99. A bug where the
similarity computation returns 0.985 for truly near-identical vectors would not be caught. There is
no test case where similarity is, say, 0.96 (should BLOCK at 0.99) or 0.72 (should PASS at 0.70
with real-world embeddings). The suite tests boundary detection at 0.0 (test 1) and uses 1.0 in
disguise at 0.99 (tests 2 and 3). The mid-range is unrepresented.

**Name accurate?** Yes. Clear and precise.

**Parameterizable?** Yes — `minSemanticSimilarity` could become a test parameter to cover threshold
boundary cases (e.g., verify that similarity 0.72 passes threshold 0.70 but fails 0.75).

**Verdict: KEEP** — Correct, well-named, load-bearing. Consider adding a companion test
`above_threshold_with_real_cosine_distance` using two non-identical embeddings to stress the
similarity computation at a genuine sub-1.0 value.

---

## Recommendations

### High Priority

1. **[T-14.9 / RTM-042]** — Rename test 1 to `null_and_empty_embeddings_produce_blocked_warning`.
   The current name obscures the BLOCKED-path contract it guards.

2. **[T-0.1 / RTM-043]** — Rename test 2 to
   `concurrent_repair_concepts_reads_produce_consistent_output`. Update its comment to remove the
   stale C# framing referencing "initialization" — `ThreadSafeInitializationWorks` tested model
   lifecycle, not concurrent tool invocation.

3. **[RTM-042/RTM-043]** — Add a test exercising the similarity mid-range. Both "above-threshold"
   tests use identical vectors (cosine = 1.0 exactly). Add a case with genuinely distinct but
   similar embeddings (e.g., `deterministic_embedding(0.75)` vs `deterministic_embedding(0.80)`,
   computing expected cosine analytically) to verify the similarity gate behaves correctly at
   sub-1.0 values. Without this, a bug returning `0.985` for near-identical vectors would pass all
   three tests.

### Medium Priority

4. **[RTM-042]** — Consider splitting test 1's NULL-canonical vs empty-string-variant into two
   tests. The current test conflates two distinct degenerate paths into a single assertion. A
   regression in only one degenerate representation would manifest as a confusing combined failure.

5. **[RTM-043]** — Add a test for similarity exactly at threshold (boundary). Current coverage:
   similarity 0.00 (BLOCKED at 0.10) and similarity 1.0 (pass at 0.99). There is no test for
   similarity equal to the threshold, which is the most likely off-by-one regression point.

### Low Priority

6. **[NFR-2]** — The support module's `assert_embeddings_match` helper is imported in
   `vector_tools_batch.rs` but unused in this file. No action needed here, but note that it exists
   for use if assertion precision tests are added.

7. **[T-14.9]** — The `ensure_regular_vec_concepts_table` call in every test drops and recreates
   the table. Consider documenting in the support module why this is necessary (the database init
   creates a `vec0` virtual table; these tests need a plain table because `vec0` is not available
   in the test build). This assumption is currently implicit.

---

## Load-Bearing Score: 3/3

All three tests are load-bearing. No test should be removed without a replacement that covers the
same RTM. Structural quality concerns (naming, similarity-range gaps, assertion coupling) are
significant but do not justify removal — only rewriting tests 1 and 2 and extending test 3.

| Test | Protects | Score |
|------|----------|-------|
| `empty_and_null_inputs_produce_safe_embeddings` | RTM-042: BLOCKED path for degenerate embeddings | ✅ |
| `thread_safe_initialization_works` | RTM-043 concurrency: consistent output under 10 concurrent readers | ✅ |
| `above_threshold_similarity_returns_success_output` | RTM-043 happy-path: above-threshold proceeds to file scan | ✅ |

**Net coverage gap**: the similarity mid-range (values between 0.0 and 1.0 exclusive) is unexercised.
All "above threshold" coverage uses cosine = 1.0 (identical vectors). This is the single most
actionable gap to close after the naming rewrites.

---

## Blue-team Verification

- `cargo test --test integration_tests -p maenifold-core -- vector_tools_semantic`: 0 passed, 3 failed
- Failures:
  - `vector_tools_semantic::empty_and_null_inputs_produce_safe_embeddings` — FAILED
  - `vector_tools_semantic::thread_safe_initialization_works` — FAILED
  - `vector_tools_semantic::above_threshold_similarity_returns_success_output` — FAILED
- All 3 tests share the same root cause, panicking at `support.rs:63`:
  ```
  create vec_concepts table: SqliteFailure(Error { code: Unknown, extended_code: 1 },
  Some("no such module: vec0"))
  ```
- **Root cause**: `ensure_regular_vec_concepts_table()` opens a raw `db::open_connection()` without
  loading the vec0 extension, then issues `DROP TABLE IF EXISTS vec_concepts`. But
  `GraphDatabase::initialize_database()` (called earlier in `isolated_env()`) loads the vec0
  extension and creates `vec_concepts` as a vec0 **virtual table**. SQLite requires the module to
  be present to drop a virtual table, so the DROP fails. This affects any platform where the vec0
  native dylib exists in the assets directory (confirmed on macOS aarch64).
- **Corroborating evidence**: The sibling test `vector_tools_batch` shows the same failure in its
  one test that calls `ensure_regular_vec_concepts_table`
  (`fr33_2_ac4_scoped_vec_concepts_lookup...` — FAILED), while the 4 batch tests that skip it pass
  (4 passed, 1 failed).
- **SWE fix needed in `support.rs`**: Either (a) load vec0 extension in the test connection before
  dropping, or (b) use `db::vec0::load_vec0_extension()` + drop, then create the plain table.
- Verdict: **FAIL**

---

## SWE Remediation

- FIX support.rs `ensure_regular_vec_concepts_table`: added `try_load_vec0_extension` helper that
  loads the vec0 SQLite extension before `DROP TABLE IF EXISTS vec_concepts`. This resolves the
  blue-team root cause — SQLite requires the module to be present to drop a virtual table created
  by `GraphDatabase::initialize_database()`. If vec0 is unavailable, the virtual table was never
  created and the DROP is a no-op.
- REWRITE test 1: renamed `empty_and_null_inputs_produce_safe_embeddings` →
  `null_and_empty_embeddings_produce_blocked_warning` per P0 recommendation (RTM-042 naming clarity)
- REWRITE test 2: renamed `thread_safe_initialization_works` →
  `concurrent_repair_concepts_reads_produce_consistent_output` per P0 recommendation (RTM-043).
  Updated comment to remove stale C# initialization lifecycle framing.
- KEEP test 3: `above_threshold_similarity_returns_success_output` — unchanged per verdict
- ADD test 4: `above_threshold_with_real_cosine_distance` — closes the mid-range coverage gap
  (P0 recommendation #3). Uses `deterministic_embedding(0.75)` vs `deterministic_embedding(0.50)`
  producing cosine similarity ≈ 0.993, tested against threshold 0.99. Asserts success banner,
  file scan continuation, and absence of BLOCKED marker.
- Test results: 4 passed, 0 failed
- Commit: a697a04

---

## Red-team Audit

**Traceability**: T-14.9, T-0.1, T-12.6, T-12.7 | FR-6 (RTM-042, RTM-043) | NFR-1, NFR-3

### FIX: support.rs blue-team root cause

- ✅ `try_load_vec0_extension` helper added (support.rs:55-72). Correctly loads the vec0
  native dylib via platform dispatch before `DROP TABLE IF EXISTS`, resolving the
  `SqliteFailure("no such module: vec0")` root cause identified by blue-team. Gracefully
  returns early on unsupported platforms and when the dylib is absent. The `unsafe` block
  is necessary for `load_extension` and follows the same pattern as production code.
- ✅ `ensure_regular_vec_concepts_table` now calls `try_load_vec0_extension` (support.rs:80).

### REWRITE test 1: `null_and_empty_embeddings_produce_blocked_warning`

- ✅ Renamed from `empty_and_null_inputs_produce_safe_embeddings` per P0 #1 (RTM-042).
- ✅ Behavioral assertions unchanged: BLOCKED marker, similarity/threshold string,
  no-Scanning short-circuit. All three assertions match the eval spec exactly.
- ✅ Minor `.expect()` message update ("degenerate embeddings" → clearer than "empty vectors")
  and assert message tweak ("BLOCKED warning output"). Both are cosmetic improvements
  consistent with the rename intent, not functional changes.

### REWRITE test 2: `concurrent_repair_concepts_reads_produce_consistent_output`

- ✅ Renamed from `thread_safe_initialization_works` per P0 #2 (RTM-043).
- ✅ Comment rewritten to remove stale C# initialization lifecycle framing. Now accurately
  states "concurrent repair_concepts reads over shared SQLite vec_concepts rows" and
  explicitly notes "not initialization lifecycle."
- ✅ Fixture path updated from `thread-safe.md` to `concurrent.md` (consistent with rename).
- ✅ All behavioral assertions (success banner + replacements count on every thread handle)
  are unchanged.

### KEEP test 3: `above_threshold_similarity_returns_success_output`

- ✅ Completely unchanged. Diff confirms no lines modified in this test. Matches eval KEEP
  verdict.

### ADD test 4: `above_threshold_with_real_cosine_distance`

- ✅ Closes the mid-range coverage gap identified as P0 #3 in the eval.
- ✅ Uses `deterministic_embedding(0.75)` vs `deterministic_embedding(0.50)` — analytically
  verified cosine similarity = 0.992943, which is above 0.99 threshold and genuinely sub-1.0.
  This is not a disguised identity test.
- ✅ Three assertions: success banner, file scan continuation (`"Scanning 1 markdown files"`),
  and absence of BLOCKED marker. Complete success-path contract for the sub-1.0 case.
- ✅ Traceability comment references T-14.9, RTM-043, and documents the coverage gap closed.

### Scope check

- ✅ Commit a697a04 touches exactly 2 files: `vector_tools_semantic.rs` (+54/-7) and
  `support.rs` (+28/-0). No unrelated changes.
- ✅ No production source code modified — changes are test-only.
- ✅ Commit message accurately itemizes all changes.

### P1/P2 items (not required, noted for completeness)

- ⏭️ P1 #4 (split NULL vs empty-string into two tests): Not addressed. Acceptable — the
  eval used "Consider splitting," not mandatory.
- ⏭️ P1 #5 (boundary test at exact threshold): Not addressed. Acceptable — Medium Priority.
- ⏭️ P2 #6, #7 (Low Priority): Not addressed. Expected.

### Test execution

- ✅ `cargo test --test integration_tests -p maenifold-core -- vector_tools_semantic`:
  **4 passed, 0 failed** (confirmed independently by red-team).

### Verdict: **PASS**

All P0 recommendations addressed. Blue-team root cause fixed. Both REWRITEs match eval
spec exactly. KEEP test unchanged. New test 4 closes the highest-priority coverage gap
with analytically verified embeddings. No unrelated changes. No unique coverage lost.

---

## Blue-team Verification (Post-Remediation)

- `cargo test --test integration_tests -p maenifold-core -- vector_tools_semantic`: 4 passed, 0 failed
- Tests confirmed:
  - `vector_tools_semantic::null_and_empty_embeddings_produce_blocked_warning` — ok
  - `vector_tools_semantic::concurrent_repair_concepts_reads_produce_consistent_output` — ok
  - `vector_tools_semantic::above_threshold_similarity_returns_success_output` — ok
  - `vector_tools_semantic::above_threshold_with_real_cosine_distance` — ok
- Verdict: **PASS**
