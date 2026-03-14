# Test Evaluation: access_boosting_search
**File:** `crates/maenifold-core/tests/access_boosting_search.rs`
**Test count:** 3
**Evaluated:** 2025-07-17
**Traceability:** T-GRAPH-DECAY-002.1-3 · RTM NFR-1 · RTM NFR-2 · RTM NFR-7.6.2 (historical)

---

## Summary

All three tests guard the same invariant: that `SearchMemories` (across all three modes — Hybrid, Semantic, FullText) does **not** write the `last_accessed` timestamp back to the database. This is the deliberate complement to the access-boosting behavior tested in `access_boosting_reads.rs`, which asserts that `ReadMemory` *does* update it. The split is architecturally correct.

Confirming that the source code contains no `update_last_accessed` calls in `search/api.rs` — the tests guard against a future regression, not a current bug. That is load-bearing work.

However, the file has two problems worth acting on:

1. **Test 2 embeds an ONNX runtime probe (`T-8.4`) that has nothing to do with `last_accessed` behavior.** It makes the semantic test dual-purpose, adds a latent failure mode (ONNX load failure kills the `last_accessed` assertion for unrelated reasons), and duplicates coverage already provided by `vector_tools_runtime.rs`.

2. **Test 2's result assertion is conditionally gated** (`if results.map_or(false, |r| !r.is_empty())`). The comment acknowledges the gap ("if vec0 extension is not in the test environment's assets_path, results will be empty"). This means the semantic test passes vacuously when the vector search produces no hits — the `last_accessed` invariant still fires, but the precondition that the file was actually *found* is never verified. The other two modes assert `assert_search_contains_title` unconditionally. The asymmetry is unexplained and suspicious.

The three modes (Hybrid / Semantic / FullText) are worth testing separately *if* the code paths genuinely diverge after the mode branch. Looking at `search/api.rs`, they do: Hybrid combines FTS5 + optional vector ranking, Semantic uses vector-only scoring, FullText uses FTS5-only. A `last_accessed` write accidentally added in mode-specific post-processing would only be caught by the relevant test. The three-way split is justified. Parameterization would improve readability without reducing coverage.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `search_memories_does_not_update_last_accessed` | **KEEP** | Load-bearing regression guard for the Hybrid code path. Well-structured, unconditional `assert_search_contains_title` ensures the file was actually retrieved before the timestamp comparison fires. Name is accurate and specific. No issues. |
| `search_memories_semantic_mode_does_not_update_last_accessed` | **REWRITE** | Two problems: (1) embedded ONNX probe (`vector::generate_embedding` + dimension assertion) is a misplaced T-8.4 concern — it belongs in `vector_tools_runtime.rs` or its own dedicated test, not here; (2) the `assert_search_contains_title` call is conditionally skipped when the vec0 extension is absent, making the result-found precondition unverified in some environments. Rewrite: remove the ONNX probe, replace the conditional block with an unconditional assertion (or skip the whole test cleanly when vec0 is absent via a cfg flag), and keep only the `last_accessed` invariant as the single purpose. |
| `search_memories_fulltext_mode_does_not_update_last_accessed` | **KEEP** | Load-bearing regression guard for the FullText code path. Structurally identical to test 1 (the good version). Unconditional result assertion. No issues. |

---

## Recommendations

1. **Rewrite `search_memories_semantic_mode_does_not_update_last_accessed`** with a single responsibility. Remove the ONNX probe entirely — `vector_tools_runtime.rs` already covers ONNX availability. Replace the conditional result assertion with a clean `cfg`-guarded skip or an unconditional assertion. The test comment ("T-8.4 — ensure the semantic-path regression test exercises the real ONNX runtime") is cargo-cult attribution: T-8.4 is a vector-tool concern, not an access-boosting concern.

2. **Address the asymmetry in the semantic result check.** The conditional `if results.map_or(false, |r| !r.is_empty())` block means the test in CI environments without vec0 reduces to: "does searching produce no timestamp change?" — which is trivially true for any read-only operation on a database row that was never touched. If the test is expected to run without vec0, skip the whole test with `#[cfg_attr(not(feature = "vec0"), ignore)]`. If vec0 is always present, remove the conditional entirely and assert unconditionally.

3. **Consider parameterizing all three tests** behind a shared `assert_search_mode_does_not_update_last_accessed(mode: &str)` helper. The structural duplication (create file → set baseline → search → compare timestamps) is identical across all three. This would reduce the file to ~25 lines plus three one-liner wrapper tests, making future mode additions trivial and ensuring all modes share the same assertion logic.

4. **Strengthen the baseline-setting sequence.** All three tests call `read_memory_output` before setting the baseline timestamp — this ensures the file is "warm" and any initialization-time side-effects on `last_accessed` have already fired. This is correct design. However, the `read_memory_output` assertion (`!read_result.starts_with("ERROR:")`) is a weak signal. Consider asserting the content contains the expected WikiLinks to confirm the right file was read.

5. **Clarify T-GRAPH-DECAY-002.1-3 traceability.** The tag format `T-GRAPH-DECAY-002.1-3` is non-standard (active TODO items use `T-NN.M` format). These appear to be historical sprint-task identifiers from a completed sprint. The functional requirement they guard (NFR-7.6.2, search does not mutate access timestamps) is not currently listed in `RTM.md` (it was likely delivered in sprint 12 and moved to `CHANGELOG.md`). The comment markers should be updated to reference the canonical NFR-2 test coverage requirement, or annotated with a note pointing to `CHANGELOG.md`.

6. **Verify the `TEST_MUTEX` is still necessary.** The mutex serializes all tests in this file (and all sibling access_boosting tests that share the same `support.rs`). Each test uses `isolated_env()` with a `TempDir` and `Config::override_root()`. If the isolation is genuine, the mutex guards only against `Config::override_root()` being a process-global side-effect — not a database contention issue. Document this explicitly in `support.rs` so future maintainers don't assume the mutex is protecting something structural about the DB. If `Config` isolation is complete, the mutex may be removable, which would re-enable parallel test execution for this file.

---

## Load-Bearing Score: 3/3

All three tests guard distinct code paths (Hybrid, Semantic, FullText) against an accidental `update_last_accessed()` regression in `search/api.rs`. The invariant they protect — search reads are non-mutating with respect to access timestamps — is a deliberate architectural choice with real decay-weight consequences downstream. Removing any one of them would leave the corresponding mode path unguarded. However, test 2 is currently **weakened** by its conditional result assertion; its effective load-bearing value is closer to 0.7/1 pending the rewrite recommended above.
