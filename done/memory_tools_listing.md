---
id: memory_tools_listing
status: done
pipeline: tdd
---

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

---
## SWE Remediation
- REWRITE test 1 (`list_memories_output_includes_created_timestamp`): removed redundant `output.contains("created")` assertion
- REWRITE test 2 (`list_memories_output_includes_last_accessed_timestamp`): removed redundant `output.contains("last_accessed")` assertion
- REWRITE test 3 (`list_memories_output_includes_decay_weight`): removed redundant `output.contains("decay_weight")` assertion
- MERGE tests 4/5/6 → `within_grace_period_weight_is_1_0`: parameterized with 3 cases (sequential/7d, workflow/14d, standard/14d)
- REWRITE test 7 (`list_memories_sequential_file_past_grace_period_has_decayed_weight`): tightened from `weight < 1.0` to `|weight - 7^(-0.5)| < 0.02` (power-law exact range)
- REWRITE test 8 (`list_memories_file_without_last_accessed_uses_fallback`): removed dead `output.contains` guards, added ISO 8601 parse validation via `NaiveDate::parse_from_str` / `DateTime::parse_from_rfc3339`
- KEEP test 9 (`list_memories_new_file_has_full_decay_weight`): unchanged
- Test count: 9 → 7 (3 merged into 1)
- All 7 tests pass
- Commit: cba2bca3896c1c84ea75aaa521779f7c524d1ea3

---
## Red-team Audit
- ✅ REWRITE test 1 (`list_memories_output_includes_created_timestamp`): removed `assert!(output.contains("created"))` — matches eval spec. Only the `metadata_value` date-string check remains.
- ✅ REWRITE test 2 (`list_memories_output_includes_last_accessed_timestamp`): removed `assert!(output.contains("last_accessed"))` — matches eval spec.
- ✅ REWRITE test 3 (`list_memories_output_includes_decay_weight`): removed `assert!(output.contains("decay_weight"))` — matches eval spec. Only `(0.0..=1.0).contains(&weight)` remains.
- ✅ MERGE tests 4/5/6 → `within_grace_period_weight_is_1_0`: all 3 original cases present (sequential/7d, workflow/14d, standard/14d). Uses `grace_days - 1` for "just inside" coverage. Asserts `weight == 1.0`. No original coverage lost.
- ✅ REWRITE test 7 (`list_memories_sequential_file_past_grace_period_has_decayed_weight`): replaced `weight < 1.0` with `(weight - 7.0_f64.powf(-0.5)).abs() < 0.02`. Derives expected value from the deterministic power-law formula. Tighter than eval's illustrative range example — exceeds spec.
- ✅ REWRITE test 8 (`list_memories_file_without_last_accessed_uses_fallback`): both dead `output.contains(...)` guards removed. ISO 8601 parse validation added via `parse_from_rfc3339`, custom datetime format, and `NaiveDate` fallback — exceeds eval's `parse_from_rfc3339`-only suggestion by covering date-only fallbacks.
- ✅ KEEP test 9 (`list_memories_new_file_has_full_decay_weight`): unchanged, load-bearing.
- ✅ Test count 9 → 7: confirmed 7 `#[test]` annotations. All 7 pass.
- ✅ Commit scope: single file changed (`memory_tools_listing.rs`). No unrelated changes. Only new import (`DateTime, FixedOffset`) required by test 8 rewrite.
- ✅ All 4 eval recommendations addressed (R1: dead contains assertions, R2: parameterized merge, R3: tight decay range, R4: ISO 8601 validation).
- Verdict: **PASS**

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_tools_listing`: 7 passed, 0 failed
- Test names confirmed:
  - `list_memories_output_includes_created_timestamp` ✅
  - `list_memories_output_includes_last_accessed_timestamp` ✅
  - `list_memories_output_includes_decay_weight` ✅
  - `within_grace_period_weight_is_1_0` ✅ (merged from 3 grace-period tests)
  - `list_memories_sequential_file_past_grace_period_has_decayed_weight` ✅
  - `list_memories_file_without_last_accessed_uses_fallback` ✅
  - `list_memories_new_file_has_full_decay_weight` ✅
- Test count 9→7 confirmed (3 merged into 1 parameterized test)
- Verdict: **PASS**
