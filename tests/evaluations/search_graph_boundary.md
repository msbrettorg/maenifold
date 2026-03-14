# Test Evaluation: search_graph_boundary
**File:** `crates/maenifold-core/tests/search_graph_boundary.rs`
**Test count:** 7
**Evaluated:** 2026-03-14
**Traceability:** T-7.8, T-15.6, RTM FR-7, FR-15, NFR-1

---

## Summary

Three of the seven tests are dead weight: two scaffold remnants that assert only what output does *not* contain (negative assertions with zero behavioral coverage), and one registry smoke test fully superseded by `tool_registry_tests.rs`. The remaining four tests are genuinely load-bearing, including one C# parity regression (T-15.6) and two decay-ranking behavioral tests that directly verify a non-trivial scoring invariant.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `search_graph_tools_are_registered_case_insensitively` | REMOVE | Fully duplicated by `tool_registry_tests.rs`: `new_registry_registers_all_expected_tool_names` asserts all four tool names exist in the registry; `try_invoke_case_insensitive_lookup_always_resolves` asserts case-insensitive invocation resolves correctly. This test adds no new signal. |
| `search_memories_is_no_longer_a_placeholder` | REMOVE | Asserts `!output.contains("Tool not yet implemented: SearchMemories")`. A function returning an empty string, a panic message, or garbage passes this test. It is a historical artifact from scaffold time (the name says so). The decay ranking tests below provide real behavioral coverage of SearchMemories. |
| `sync_is_no_longer_a_placeholder` | REMOVE | Same critique. `!output.contains("Tool not yet implemented: Sync")` proves nothing about correctness. `build_context_surfaces_related_concepts_after_sync` exercises Sync with observable side-effects. This placeholder remnant should be deleted. |
| `build_context_surfaces_related_concepts_after_sync` | KEEP | End-to-end integration of WriteMemory → Sync → BuildContext. Asserts that linked concepts (`oauth2`, `retry-policy`) surface in context output. Behaviorally correct. Minor nit: `sync.contains("Sync complete:")` is format-sensitive and could be dropped without loss. |
| `build_context_expanded_relations_is_flat_string_array` | KEEP | C# oracle parity regression for T-15.6. Parses JSON output, walks the array, and asserts element types. This is the right way to test schema contracts. Also verifies that DirectRelations remained rich objects — confirming the fix didn't over-reach. |
| `search_memories_applies_decay_weight_to_ranking_only` | KEEP | Inserts two records with different `last_accessed` timestamps directly into `file_content`, queries SearchMemories, and asserts: (1) fresh result ranks first, (2) both results are retrievable. Correctly separates ranking effect from retrieval effect. Load-bearing for the decay weighting invariant. |
| `search_memories_preserves_validated_assumptions_from_decay` | KEEP | Inserts validated vs. invalidated assumptions with identical stale timestamps, asserts validated ranks first and carries a higher `decayWeight`. Directly tests RTM FR-7.8 (T-7.8). Load-bearing. |

---

## Recommendations

1. **DELETE `search_memories_is_no_longer_a_placeholder` and `sync_is_no_longer_a_placeholder`**: These are negative-only assertions with no behavioral value. Every test added after scaffolding implicitly proves "the placeholder is gone." Keeping them creates false confidence — a reviewer seeing green tests assumes meaningful coverage exists.

2. **DELETE `search_graph_tools_are_registered_case_insensitively`**: `tool_registry_tests.rs::new_registry_registers_all_expected_tool_names` and `try_invoke_case_insensitive_lookup_always_resolves` cover this completely. Duplicate registry checks across files are maintenance debt.

3. **MINOR: Strip format assertion from `build_context_surfaces_related_concepts_after_sync`**: The line `assert!(sync.contains("Sync complete:"))` couples the test to the Sync output phrasing. The actual invariant being tested is that concepts appear in BuildContext after Sync — the sync output phrasing is irrelevant to that proof. Remove or replace with `assert!(!sync.contains("ERROR"))`.

4. **Consider adding a negative-ranking test to `search_memories_applies_decay_weight_to_ranking_only`**: The test verifies that fresh > stale when base scores tie, but does not verify that the stale result is still returned (it does actually — via the `collect` assertion on both paths). This is already handled. No change needed.

5. **Question the direct SQL seeding pattern in decay tests**: Both decay tests bypass WriteMemory/Sync and insert directly into `file_content`. This is intentional for a boundary test, but it couples the tests to the internal schema. If `file_content` gains mandatory columns or changes a NOT NULL constraint, these tests break silently with an `expect("seed")` panic. Consider documenting why direct SQL seeding is preferred here (isolating the scorer from the indexer) so future maintainers don't "fix" it by switching to WriteMemory + Sync.

---

## Load-Bearing Score: 4/7

Three tests (`search_graph_tools_are_registered_case_insensitively`, `search_memories_is_no_longer_a_placeholder`, `sync_is_no_longer_a_placeholder`) could be removed today without allowing any real bug to ship undetected.
