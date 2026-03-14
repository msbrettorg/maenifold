# Test Evaluation: config_decay_config
**File:** `crates/maenifold-core/tests/config_decay_config.rs`
**Test count:** 8
**Evaluated:** 2025-07-16
**Traceability:** T-12.5 · T-GRAPH-DECAY-001.2/3/4/6 · RTM NFR-1 · RTM NFR-7.5.x

---

## Summary

The file has two structural problems. First, four of the eight tests are near-identical: each acquires the lock, clears the env, creates an isolated env, and asserts a single hardcoded default — they could collapse into one parameterized test or a single multi-assertion test without any loss of signal. Second, `get_config_summary_includes_decay_config` asserts only that label strings exist in the output, never that the correct default *values* appear; it is format theater that would pass even if the defaults were silently changed. The two genuinely strong tests are `get_config_summary_shows_actual_values` (env-var override plumbing) and `invalid_integer_decay_env_vars_fall_back_to_defaults` (graceful degradation).

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `decay_grace_days_sequential_default_is_seven` | **MERGE** | Trivially tests `env_i64("…_SEQUENTIAL", 7)` returns `7` with no env var set. Correct, load-bearing as a group, but structurally identical to the next three tests. Merge all four default-value checks into `decay_config_all_defaults_are_correct`. |
| `decay_grace_days_workflows_default_is_fourteen` | **MERGE** | Same pattern as above. |
| `decay_grace_days_default_default_is_twenty_eight` | **MERGE** | Same pattern. Note: the name is tautological ("default_default") — symptom of missing parameterization. |
| `decay_half_life_days_default_is_thirty` | **MERGE** | Same pattern. |
| `decay_function_default_is_power_law` | **MERGE** | Same pattern for string variant. Merge with the four integer-default tests into a single consolidated defaults test. |
| `get_config_summary_includes_decay_config` | **REWRITE** | Asserts only that label strings (`"Grace Days (Sequential):"`, etc.) appear in the summary output — never the *values*. If the default for `decay_grace_days_sequential` changed from 7 to 99, this test would still pass. Add value assertions for the default case (e.g., `assert!(summary.contains("Grace Days (Sequential): 7"))`), making it a genuine complement to `get_config_summary_shows_actual_values`. |
| `get_config_summary_shows_actual_values` | **KEEP** | Exercises the full pipeline: set non-default env vars → `Config::current()` → `get_config()` tool invocation → output string. Both label and value are asserted. Load-bearing for the env-var-to-summary plumbing path. |
| `invalid_integer_decay_env_vars_fall_back_to_defaults` | **KEEP** | Covers graceful degradation: invalid strings (non-parseable) must not crash or silently produce wrong defaults. The strongest test in the file. Asserts both `cfg.method()` return values and the `config_summary` output. Load-bearing. |

---

## Recommendations

1. **Merge tests 1–5 into one test**, e.g.:
   ```rust
   #[test]
   fn decay_config_all_defaults_are_correct() {
       let _guard = acquire_test_lock();
       let _env = clear_decay_env();
       let env = isolated_env();
       assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
       assert_eq!(env.cfg.decay_grace_days_workflows(), 14);
       assert_eq!(env.cfg.decay_grace_days_default(), 28);
       assert_eq!(env.cfg.decay_half_life_days(), 30);
       assert_eq!(env.cfg.decay_function(), "power-law");
   }
   ```
   Five lock acquisitions and five `isolated_env()` setups for five trivially related facts is unnecessary friction. If a sixth default is added, there is already a home for it.

2. **Rewrite `get_config_summary_includes_decay_config`** to assert actual default values in the summary string, not just label presence. As written, it verifies formatting but not correctness. The test is redundant with the individual defaults tests on the values side, and with `get_config_summary_shows_actual_values` on the label side — it needs to own the "defaults are rendered correctly in the summary" contract:
   ```rust
   assert!(summary.contains("Grace Days (Sequential): 7"));
   assert!(summary.contains("Grace Days (Workflows): 14"));
   // etc.
   ```

3. **Consider adding a test for `decay_function` with an invalid non-empty string** (e.g., `"garbage"`). The current `invalid_integer_decay_env_vars_fall_back_to_defaults` only tests integer fields. The `env_string` implementation does *not* fall back on invalid values — it accepts any non-empty string. If the downstream `decay_function()` consumer has an allowed-values contract, a test that passes `"garbage"` and observes the behavior would be load-bearing.

4. **Question the architecture assumption**: `decay_grace_days_sequential()` reads the environment variable at *call time*, not at construction time. This means `Config` is a live view of env vars — demonstrated by the fact that tests must clear env vars *before* constructing `Config`. This is subtle and could cause non-determinism if tests leak env state. The `TEST_MUTEX` and `DecayEnvGuard` RAII pattern is the right mitigation, but each test must follow the acquire-lock → clear-env → build-env order exactly. Deviation would produce false passes in parallel test runs.

---

## Load-Bearing Score: 5/8

Tests 1–5 are collectively load-bearing (they pin all five defaults) but structurally redundant with each other — they contribute 1 unit of unique coverage across 5 functions. Test 6 is not load-bearing on values. Tests 7–8 are each independently load-bearing.
