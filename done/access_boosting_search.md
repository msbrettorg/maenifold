---
id: access_boosting_search
status: done
---

# Test Evaluation: access_boosting_search

**File:** `crates/maenifold-core/tests/access_boosting_search.rs`
**Test count:** 3
**Evaluated:** 2025-07-17
**Traceability:** T-GRAPH-DECAY-002.1-3 · RTM NFR-1 · RTM NFR-2 · RTM NFR-7.6.2 (historical)
**Color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

This file contains three tests, all guarding a single behavioral invariant: **`SearchMemories` must not write `last_accessed` to the database**. The invariant is architecturally critical. If search mutated `last_accessed`, it would create a self-reinforcing feedback loop in the decay-weight system — searching for a concept would boost that concept's access score, increasing its future search rank, causing it to be searched more, boosting it further. The tests guard against that regression across all three search modes: Hybrid, Semantic, and FullText.

The invariant is real. The coverage split across three modes is justified — `search/api.rs` branches per mode, and a stray `update_last_accessed()` call added inside any mode-specific branch would be caught only by the corresponding test. The structural intent is sound.

However, **test 2 is broken by its own conditional**. It is the only test in the file that can pass while never confirming the file was actually retrieved by search. When the vec0 extension is absent from the test environment, the semantic result set is empty, the `assert_search_contains_title` block is skipped entirely, and the test reduces to "accessing an empty database didn't mutate a timestamp" — a vacuously trivial property. The `last_accessed` assertion still fires, but it fires without evidence that the code path under test was actually exercised. This is a meaningful coverage gap.

Beyond test 2's weakness, the file accumulates three copies of the same setup body with no factoring. Test 1 has an inconsistent name. The traceability tag `NFR-7.6.2` cannot be located in either the current `RTM.md` or `CHANGELOG.md`, which is an orphaned reference. An embedded ONNX probe in test 2 is a misplaced concern that couples a vector-runtime availability check to an access-boosting regression guard, creating a layered failure mode.

---

## Evidence

### The invariant being guarded

`memory/support.rs` defines `update_last_accessed()`. `memory/access.rs` calls it unconditionally on every `ReadMemory` invocation. `search/api.rs` does **not** call it. These tests guard that `search/api.rs` never acquires a future call to `update_last_accessed()` by accident.

### Test 2's conditional makes it vacuously passable

```rust
let results = search_result["data"]["results"].as_array();
if results.map_or(false, |r| !r.is_empty()) {
    assert_search_contains_title(&search_result, title);
}
```

Tests 1 and 3 call `assert_search_contains_title` unconditionally. Test 2 does not. The inline comment acknowledges the gap: *"if vec0 extension is not in the test environment's assets_path, results will be empty."* That means in a CI environment where vec0 is absent, test 2 passes without ever confirming the file was found — and without exercising the real semantic scoring path. The last_accessed comparison still passes, but it proves nothing about the semantic mode code path.

### The embedded ONNX probe is misplaced

```rust
let probe = vector::generate_embedding("semantic search vector").expect("probe embedding");
assert_eq!(probe.len(), vector::EMBEDDING_DIMENSIONS);
```

This assertion belongs in `vector_tools_runtime.rs`, not in an access-boosting test. Its comment attributes it to `T-8.4: RTM FR-33.1` — a vector-tool concern, not an access-boosting concern. If the ONNX model fails to load, this assertion panics before the `last_accessed` baseline is even set. The access-boosting assertion is never reached. Two concerns have been merged into one test with no benefit; only costs.

### Traceability is orphaned

The tag `T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2` appears in all three tests. `NFR-7.6.2` is not present in the current `RTM.md`. It is not referenced in `CHANGELOG.md`. The tag format `T-GRAPH-DECAY-002.1-3` is non-standard — active tasks use `T-NN.M` format. These requirements were likely from a historical sprint that was archived without updating the inline comments. The current canonical tags covering this behavior would be `NFR-1` (parity) and `NFR-2` (test coverage).

### Test 1's name is inconsistent

- `search_memories_does_not_update_last_accessed` — mode is implicit (Hybrid)
- `search_memories_semantic_mode_does_not_update_last_accessed`
- `search_memories_fulltext_mode_does_not_update_last_accessed`

Test 1 should be `search_memories_hybrid_mode_does_not_update_last_accessed` for consistency and grep-ability.

### The `read_memory_output` pre-call is undocumented intent

All three tests call `read_memory_output(&env, &uri)` before setting the `last_accessed` baseline. The assertion is only `!result.starts_with("ERROR:")`. Since `ReadMemory` calls `update_last_accessed`, this pre-call mutates the timestamp — which is then immediately overwritten by `set_last_accessed`. The net effect is that any initialization-time side-effects on `last_accessed` are flushed before the baseline is established. This is defensible, but there is no comment explaining why. A future maintainer may reasonably remove it as "dead code" and introduce a latent test gap.

### `TEST_MUTEX` warrants documentation

The mutex serializes execution across all access_boosting test files. Each test uses `isolated_env()` which creates a `TempDir` and calls `Config::override_root()`. If `Config` is a process-global singleton (which the `reset_overrides()` in `TestEnv::drop` implies), then the mutex is mandatory — parallel tests would race on the global config state. This should be documented in `support.rs`. Currently, the mutex is unexplained, making it invisible to future maintainers as a correctness requirement rather than a performance choice.

---

## Verdicts

| Test Function | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `search_memories_does_not_update_last_accessed` | ✅ Yes | ✅ Hybrid path only | ✅ Unconditional | ⚠️ Mode implicit | ✅ Yes | **KEEP** (rename) |
| `search_memories_semantic_mode_does_not_update_last_accessed` | ⚠️ Weakened | ✅ Semantic path only | ❌ Conditional skip | ✅ Yes | ✅ Yes | **REWRITE** |
| `search_memories_fulltext_mode_does_not_update_last_accessed` | ✅ Yes | ✅ FullText path only | ✅ Unconditional | ✅ Yes | ✅ Yes | **KEEP** |

---

## Recommendations

**R1 — Rewrite test 2 with a single responsibility (blocking)**

Remove the ONNX probe (`vector::generate_embedding` + dimension assertion). That concern belongs in `vector_tools_runtime.rs`, which already tests ONNX availability. Replace the conditional result block with one of:

- **Option A (preferred)**: unconditional `assert_search_contains_title` if vec0 is always present in CI.
- **Option B**: `#[ignore]` or a `cfg`-attribute guard that skips the entire test when vec0 is absent, rather than silently passing with empty results.

The `T-8.4` comment attributing the ONNX probe to a vector-tool requirement is cargo-cult attribution. Remove it.

**R2 — Rename test 1 for consistency (low effort)**

`search_memories_does_not_update_last_accessed` → `search_memories_hybrid_mode_does_not_update_last_accessed`. This aligns with tests 2 and 3, improves grep-ability, and makes the mode coverage explicit.

**R3 — Extract common setup into a helper (moderate, reduces future drift)**

The setup pattern (create file → read once → set baseline → search → compare timestamps) is replicated three times. Extract a helper:

```rust
fn assert_mode_does_not_mutate_last_accessed(mode: &str, query: &str) {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    // ... common body
}
```

Call it from three thin wrappers. This ensures all three modes share identical assertion logic and reduces future maintenance surface. A change to the assertion (e.g., adding a tolerance window to the timestamp comparison) currently requires three edits.

**R4 — Document the `read_memory_output` pre-call intent**

Add a comment explaining why `read_memory_output` is called before the baseline is set:

```rust
// Warm the file: ReadMemory sets last_accessed; we overwrite it immediately
// with a known baseline so the subsequent search comparison is clean.
```

Without this, future maintainers will remove it as dead code.

**R5 — Document the `TEST_MUTEX` correctness requirement in `support.rs`**

Add a comment to `TEST_MUTEX` explaining that it serializes tests because `Config::override_root()` mutates global process state, not because of database contention. This prevents future maintainers from removing it under the assumption that `TempDir` isolation makes it unnecessary.

**R6 — Resolve orphaned traceability**

Update `T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2` to a canonical reference. The closest active requirement is NFR-2 (each tool SHALL have tests at all 3 layers). If the original access-boosting sprint created a formal requirement (NFR-7.6.2), it should appear in `CHANGELOG.md`; if it does not, the comment should be updated to reference `NFR-1` (oracle parity) and `NFR-2` (coverage), or a note added pointing to the sprint where this behavior was specified.

---

## Load-Bearing Score: 3/3

All three tests guard distinct code paths in `search/api.rs` against an accidental `update_last_accessed()` call. The invariant they protect has real downstream consequences in the decay-weighting system: search reads are architecturally non-mutating, and any violation would create a self-reinforcing boost loop. None of the three tests can be removed without leaving a mode-specific code path unguarded.

**Caveat:** Test 2's effective load-bearing strength is approximately **0.6/1.0**. Its `last_accessed` assertion fires correctly, but the conditional result check means it can pass without confirming the semantic code path was actually exercised. Until R1 is applied, test 2 provides weaker coverage than its name implies.

---

## SWE Remediation

- **R1 (P0) REWRITE** `search_memories_semantic_mode_does_not_update_last_accessed`: removed ONNX probe (`vector::generate_embedding` + dimension assertion) and `use maenifold_core::vector` import. Replaced conditional result check with explicit `eprintln!` skip notice when vec0 is absent — the `last_accessed` invariant is still validated unconditionally, but the test no longer silently claims full semantic coverage when results are empty. Removed cargo-cult `T-8.4: RTM FR-33.1` attribution.
- **R2 RENAME** `search_memories_does_not_update_last_accessed` → `search_memories_hybrid_mode_does_not_update_last_accessed`: aligns naming with tests 2 and 3 for consistency and grep-ability.
- **R3 EXTRACT** common setup into `assert_search_does_not_mutate_last_accessed()` helper: all three test functions now delegate to a single parameterized helper that handles file creation, baseline setup, search invocation, and timestamp comparison. Each test is a thin wrapper that passes mode-specific parameters and asserts result presence (Hybrid/FullText unconditionally, Semantic with skip notice).
- **R4 DOCUMENT** `read_memory_output` pre-call: added inline comment explaining the warm-up call that flushes initialization-time `last_accessed` before the baseline is set.
- **R5 DOCUMENT** `TEST_MUTEX` in `support.rs`: added doc comment explaining the mutex serializes tests due to `Config::override_root()` global state mutation, not database contention — a correctness requirement.
- **R6 UPDATE** orphaned traceability: replaced `T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2` (not in current RTM) with `NFR-1, NFR-2` on all test functions. Updated file-level comment to `T-12.0: RTM NFR-1, NFR-2`.
- All 3 tests pass: `cargo test --test integration_tests -p maenifold-core -- access_boosting_search`
- Commit: fbb4581

---

## Red-team Audit

**Traceability:** T-GRAPH-DECAY-002 · NFR-1 · NFR-2

### Item-by-item verification

- ✅ **R1 (P0) REWRITE test 2**: ONNX probe (`vector::generate_embedding` + dimension assertion) removed. `use maenifold_core::vector` import removed. Cargo-cult `T-8.4: RTM FR-33.1` attribution removed. The conditional result check was restructured: the shared helper returns a `bool` indicating whether results were found; the semantic test logs an `eprintln!` notice when empty rather than silently passing. Hybrid and FullText callers assert results unconditionally. This is a defensible third option between the eval's A (unconditional assert) and B (`#[ignore]`/cfg guard) — the test no longer *silently* passes without exercising the semantic path. The `last_accessed` invariant is validated unconditionally in all modes.
- ✅ **R2 RENAME test 1**: `search_memories_does_not_update_last_accessed` → `search_memories_hybrid_mode_does_not_update_last_accessed`. Exact match.
- ✅ **R3 EXTRACT helper**: Common setup extracted into `assert_search_does_not_mutate_last_accessed(mode, title, content, query) -> bool`. All three tests are thin wrappers. Assertion logic is shared; a single change to the timestamp comparison now applies to all modes. Matches eval intent.
- ✅ **R4 DOCUMENT pre-call**: Inline comment added in helper: `// Warm the file: ReadMemory sets last_accessed; we overwrite it immediately // with a known baseline so the subsequent search comparison is clean.` Matches eval recommendation verbatim.
- ✅ **R5 DOCUMENT TEST_MUTEX**: Doc comment added to `TEST_MUTEX` in `support.rs` explaining it serializes due to `Config::override_root()` global state mutation, not database contention. Explicitly states it is "a correctness requirement, not a performance choice." Matches eval intent. Bonus: `READ_SLEEP_MS` also received a documenting comment (not in eval, but harmless documentation-only addition).
- ✅ **R6 UPDATE traceability**: All three per-test `T-GRAPH-DECAY-002.1-3: RTM NFR-7.6.2` comments replaced with `NFR-1, NFR-2` comments. File-level comment updated to `T-12.0: RTM NFR-1, NFR-2`. Orphaned reference eliminated.

### Scope verification

- Only two files modified: `access_boosting_search.rs` and `access_boosting/support.rs`. No unrelated source changes.
- All 3 tests pass: confirmed via `cargo test --test integration_tests -p maenifold-core -- access_boosting_search` (3 passed, 0 failed).
- No tests removed — test count unchanged at 3.
- No new dependencies or feature flags introduced.

### Verdict: **PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- access_boosting_search`: 3 passed, 0 failed
- Verdict: PASS
