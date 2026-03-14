# Test Evaluation: memory_tools_listing

**File:** `crates/maenifold-core/tests/memory_tools_listing.rs`
**Test count:** 9
**Evaluated:** 2026-07-14
**Traceability:** T-12.6 (RTM NFR-1)

## Summary

The test suite covers the right behaviors — created timestamps, last_accessed, decay weight, tier-specific grace periods, and fallback handling. However, four tests (1, 2, 3, 8) carry redundant `output.contains(field)` assertions that can never fail independently of the `metadata_value` call that follows them. Tests 4, 5, and 6 are structurally identical and should be collapsed into a parameterized test. Test 7 uses the weakest possible decay assertion (`weight < 1.0`) — a weight of `0.9999` satisfies it even if the decay formula is completely wrong.

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `list_memories_output_includes_created_timestamp` | **REWRITE** | `assert!(output.contains("created"))` is dead code. `metadata_value(..., "created")` already panics if the key is absent, rendering the prior `contains` check redundant. Remove it; keep only the date-string check. |
| `list_memories_output_includes_last_accessed_timestamp` | **REWRITE** | Same redundant `output.contains("last_accessed")` assertion. Also: `set_last_accessed` inserts directly into the DB — the test doesn't exercise the write path, only the read path against a known DB state. That's fine, but the first assertion adds no value. Remove it. |
| `list_memories_output_includes_decay_weight` | **REWRITE** | `assert!(output.contains("decay_weight"))` is dead code (same pattern). The meaningful assertion is the numeric range check `(0.0..=1.0).contains(&weight)`. Keep only that. |
| `list_memories_sequential_file_uses_7_day_grace_period` | **MERGE** | Structurally identical to tests 5 and 6: create a file N days old in tier T, assert weight == 1.0. Parameterize with tests 5 and 6 into one test with 3 cases. |
| `list_memories_workflow_file_uses_14_day_grace_period` | **MERGE** | See above. Same pattern as test 4. |
| `list_memories_standard_file_uses_14_day_grace_period` | **MERGE** | See above. Same pattern as tests 4 and 5. |
| `list_memories_sequential_file_past_grace_period_has_decayed_weight` | **REWRITE** | `assert!(weight < 1.0)` is the weakest possible decay assertion. A weight of `0.9999` passes. With `power-law` decay, `half_life=30`, sequential grace=7, and a file that is 14 days old: `age_past_grace = 14 - 7 = 7` days. The expected weight is calculable. Assert a tight range (e.g., `0.7..=0.9`) or the exact expected value from the decay formula. The current assertion cannot detect a regression where decay is applied at 1/10th the correct rate. |
| `list_memories_file_without_last_accessed_uses_fallback` | **REWRITE** | Three assertions: `output.contains("last_accessed")`, `output.contains("decay_weight")`, and `!metadata_value(..., "last_accessed").is_empty()`. The first two are dead code — if `metadata_value` succeeds the others trivially pass; if it panics, they never run. Keep only the third. Also: assert that the fallback is a *valid* ISO 8601 timestamp, not just non-empty. |
| `list_memories_new_file_has_full_decay_weight` | **KEEP** | Load-bearing. Tests a file with no `created` frontmatter timestamp — the "new file" path. Distinct from tests 4-6 which use explicit timestamps. `assert_eq!(weight, 1.0)` is precise. |

## Recommendations

1. **Remove all `output.contains(field_name)` assertions** before `metadata_value` calls (tests 1, 2, 3, 8). The `metadata_value` helper already panics with a useful error if the key is missing — the prior `contains` check adds noise and misleads readers into thinking it's a meaningful guard.

2. **Collapse tests 4, 5, 6** into a single parameterized test:
   ```rust
   #[test]
   fn within_grace_period_weight_is_1_0() {
       // cases: (tier_path, type_value, grace_days, file_stem)
       let cases = [
           ("thinking/sequential/...", "sequential", 7, "seq-grace"),
           ("thinking/workflows/...", "workflow", 14, "wf-grace"),
           ("decay-metadata-tests/standard", "memory", 14, "std-grace"),
       ];
       for (path, ty, grace_days, stem) in cases {
           let days_within = grace_days - 1;  // just inside
           // ... write file, sync, assert weight == 1.0
       }
   }
   ```

3. **`list_memories_sequential_file_past_grace_period_has_decayed_weight`**: Calculate the expected weight from the formula and assert a tight range. The decay parameters are set deterministically by `DecayEnvGuard::with_csharp_defaults()` — there is no excuse for an imprecise assertion here.

4. **`list_memories_file_without_last_accessed_uses_fallback`**: Assert the fallback value is a valid ISO 8601 timestamp (parse it) rather than just non-empty. An empty string would fail the current check, but so would the string `"null"` or `"N/A"` — only `DateTime::parse_from_rfc3339` can actually verify it.

## Load-Bearing Score: 6/9

Tests 4, 5, 6 are load-bearing but parameterizable redundancy (3→1). Test 7 is structurally present but its assertion is too weak to catch decay regressions. Tests 1, 2, 3, 8 carry redundant dead assertions alongside genuinely load-bearing ones.
