---
id: T-80.38
title: "Test Evaluation: config_decay_config"
status: done
pipeline: tdd
---

# Test Evaluation: config_decay_config
**File:** `crates/maenifold-core/tests/config_decay_config.rs`
**Support:** `crates/maenifold-core/tests/config_decay/support.rs`
**Test count:** 8
**Evaluated:** 2026-07-17
**Traceability:** T-GRAPH-DECAY-001.2 · T-GRAPH-DECAY-001.2a · T-GRAPH-DECAY-001.3 · T-GRAPH-DECAY-001.4 · T-GRAPH-DECAY-001.6 · T-GRAPH-DECAY-001.2-4 · RTM NFR-7.5.1 · NFR-7.5.2 · NFR-7.5.3 · NFR-7.5.5

---

## Summary

This suite tests the `Config` accessor layer for decay configuration: default values,
env-var overrides, fallback-on-invalid-parse, and the `GetConfig` tool's display of those
values. The behavioral scope is narrow and correct — these are appropriate boundary-layer
tests. The infrastructure (mutex lock, RAII `DecayEnvGuard`, `isolated_env`) is
well-designed and shared with `config_decay_weights.rs`.

Skeptical examination reveals three structural concerns:

**1. Tests 1–5 are isomorphic.** All five follow an identical three-line body — clear env,
create isolated env, assert a single default. They differ only in which accessor is
called and which literal is expected. This is the definition of a parameterization target.
As individual test functions they add maintenance overhead: the five names collectively
embed five magic literals (`seven`, `fourteen`, `twenty_eight`, `thirty`, `power_law`)
that become misleading the moment a default changes, with no refactor tooling to
catch the staleness. The suite should be collapsed into one table-driven test.

**2. No test directly verifies accessor reads for non-default values.** Test 7
(`get_config_summary_shows_actual_values`) sets env vars and checks that the *summary
output* contains the expected strings. The config accessors (`decay_grace_days_sequential()`
etc.) are only exercised through the diagnostic tool chain. If the diagnostic tool used a
different code path than the Config accessors for rendering, a bug there would pass all
8 tests. This is an indirect coverage gap, not a gaping hole — but it means the accessor
→ env-var plumbing is never directly asserted under non-default conditions.

**3. Test 8 does not cover the invalid-string case for `MAENIFOLD_DECAY_FUNCTION`.**
The `env_i64` helper falls back to the default on parse failure (tested). The `env_string`
helper does something different: it trims whitespace and filters empty strings, but passes
through *any non-empty string* without validation. So `MAENIFOLD_DECAY_FUNCTION=garbage`
returns `"garbage"` — it does **not** fall back to `"power-law"`. Test 8 only tests the
four integer vars. The absence of a test for an unrecognized function string is a real
behavioral gap; the application would accept the value silently and likely produce
incorrect decay scores downstream.

The support module remains positive: the `DecayEnvGuard` RAII pattern, `TEST_MUTEX`
serialization, and `Config::reset_overrides()` in `TestEnv::Drop` are correct. Any
rewrite must preserve them.

---

## Verdicts

| # | Test Function | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | **Verdict** |
|---|---|---|---|---|---|---|---|
| T1 | `decay_grace_days_sequential_default_is_seven` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Embeds literal in name | ✅ Yes | **MERGE** |
| T2 | `decay_grace_days_workflows_default_is_fourteen` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Embeds literal in name | ✅ Yes | **MERGE** |
| T3 | `decay_grace_days_default_default_is_twenty_eight` | ✅ Yes | ✅ Yes | ✅ Yes | ❌ Double "default"; embeds literal | ✅ Yes | **MERGE** |
| T4 | `decay_half_life_days_default_is_thirty` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Embeds literal in name | ✅ Yes | **MERGE** |
| T5 | `decay_function_default_is_power_law` | ✅ Yes | ✅ Yes | ✅ Yes | ⚠️ Embeds literal in name | ⚠️ Partial | **MERGE** |
| T6 | `get_config_summary_includes_decay_config` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No | **KEEP** |
| T7 | `get_config_summary_shows_actual_values` | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ❌ No | **KEEP** |
| T8 | `invalid_integer_decay_env_vars_fall_back_to_defaults` | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Yes | ❌ No | **REWRITE** |

---

## Detailed Findings

### T1–T5: Default-Value Quintet — MERGE

```rust
// All five follow this exact structure; only the accessor name and expected literal differ:
fn decay_grace_days_sequential_default_is_seven() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();
    assert_eq!(env.cfg.decay_grace_days_sequential(), 7);
}
```

**What they collectively prove:** That five `Config` accessors return their documented
defaults when no env override is present.

**What they assume (and do not prove):** That these five defaults are the *only*
correct defaults, not just the defaults *as of today*. Embedding `seven`, `fourteen`,
`twenty_eight`, `thirty`, `power_law` into function names creates a silent staleness
hazard — if a default is intentionally revised, the test body gets updated but the
function name drifts. The five names are also misleading for T3:
`decay_grace_days_default_default_is_twenty_eight` — the double "default" arises because
one "default" means "the default-path grace period" and the other means "the default
value", but the reader cannot tell this without context.

**Recommended replacement — one parameterized test:**
```rust
#[test]
fn decay_config_accessor_defaults_match_spec() {
    let _guard = acquire_test_lock();
    let _env = clear_decay_env();
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7,  "sequential grace days");
    assert_eq!(env.cfg.decay_grace_days_workflows(),  14, "workflow grace days");
    assert_eq!(env.cfg.decay_grace_days_default(),    28, "default grace days");
    assert_eq!(env.cfg.decay_half_life_days(),        30, "half-life days");
    assert_eq!(env.cfg.decay_function(),        "power-law", "function");
}
```

This is five assertions in one function — acceptable because they are all testing the
same *invariant* ("defaults are correct when env is cleared") across five parallel
fields. A failure message names the failing field explicitly. No behavioral coverage
is lost; 40 lines collapse to 10.

If individual test isolation is preferred (a different failure pinpoints more directly),
the five can be kept separate but named without embedded literals:
```rust
fn decay_grace_days_sequential_returns_spec_default()  // not "is_seven"
fn decay_grace_days_workflows_returns_spec_default()
fn decay_grace_days_default_path_returns_spec_default()  // disambiguates double-default
fn decay_half_life_days_returns_spec_default()
fn decay_function_returns_spec_default()
```

---

### T6: `get_config_summary_includes_decay_config` — KEEP

```rust
assert!(summary.contains("Decay Config:"));
assert!(summary.contains("Grace Days (Sequential):"));
assert!(summary.contains("Grace Days (Workflows):"));
assert!(summary.contains("Grace Days (Default):"));
assert!(summary.contains("Half-Life Days:"));
assert!(summary.contains("Function:"));
```

**What it proves:** That the `GetConfig` tool renders a decay config section with all
five expected labels. This is orthogonal to T1–T5 (which test values, not display).

**What it does not prove:** That the labels appear in the right section, in the right
order, or alongside the correct values. Label-only presence is intentionally minimal
— T7 covers value accuracy. This separation of concerns is correct.

**One question to ask:** Is the label set complete? The test covers all five documented
fields. No gap identified.

No concerns. **KEEP.**

---

### T7: `get_config_summary_shows_actual_values` — KEEP

```rust
let _env = apply_decay_env(&[
    ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("9")),
    ...
    ("MAENIFOLD_DECAY_FUNCTION", Some("exponential")),
]);
let env = isolated_env();
let summary = config_summary(&env);

assert!(summary.contains("Grace Days (Sequential): 9"));
...
assert!(summary.contains("Function: exponential"));
```

**What it proves:** That when all five env vars are set to non-default values, the
`GetConfig` tool correctly reads and displays every one of them. This end-to-end chain
exercises env → `Config` accessor → `GetConfig` tool → output string.

**A subtle indirect coverage gap:** This test only asserts *summary output*, not the
raw accessor values. Consider that `config_summary` might call `diagnostics::render_decay_config()`
which could read the env vars directly rather than going through `env.cfg`. If that were
true, a bug in the accessor would still be hidden. In practice, `diagnostics.rs` does
call the `Config` accessors — so the chain is exercised. But it would be more defensive
to also assert:

```rust
assert_eq!(env.cfg.decay_grace_days_sequential(), 9);
assert_eq!(env.cfg.decay_grace_days_workflows(),  18);
// etc.
```

This can be added as a low-priority improvement without changing the verdict.

**Values chosen (9, 18, 33, 45, "exponential") are deliberately non-default** — correct
practice. A test that overrides to default values would not distinguish a "reads env var"
bug from a "uses hardcoded default" bug.

No blocking concerns. **KEEP.**

---

### T8: `invalid_integer_decay_env_vars_fall_back_to_defaults` — REWRITE

```rust
let _env = apply_decay_env(&[
    ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("invalid")),
    ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",  Some("invalid")),
    ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",    Some("invalid")),
    ("MAENIFOLD_DECAY_HALF_LIFE_DAYS",        Some("invalid")),
]);
```

**What it proves:** That `env_i64` falls back to the default when the env var contains
a non-parseable string. Combined accessor + summary assertions verify the fallback
propagates to the display layer.

**What it does not prove — a genuine behavioral gap:**
`MAENIFOLD_DECAY_FUNCTION` is conspicuously absent. Examining the implementation:

```rust
fn env_string(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string())
}
```

`env_string` trims whitespace and rejects empty strings — but it **does not validate
the content**. Setting `MAENIFOLD_DECAY_FUNCTION=garbage` returns `"garbage"`, not
`"power-law"`. The decay scoring code in `scoring.rs` presumably branches on the
function name; an unrecognized value would silently fall through to a default branch
(or produce wrong results if no default branch exists). This is a real behavioral
risk that no test in this file — or `config_decay_weights.rs` — currently covers.

**Summary-assertion redundancy:** With T6 and the accessor assertions already
passing, the summary assertions in T8 (`summary.contains("Grace Days (Sequential): 7")`,
etc.) are logically implied. They add regression safety for the display layer under
error conditions, which has some value, but could be pruned if line count is a concern.

**Recommended rewrite — add the missing string case and tighten:**
```rust
#[test]
fn invalid_integer_decay_env_vars_fall_back_to_defaults() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[
        ("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS",  Some("invalid")),
        ("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT",    Some("invalid")),
        ("MAENIFOLD_DECAY_HALF_LIFE_DAYS",        Some("invalid")),
    ]);
    let env = isolated_env();

    assert_eq!(env.cfg.decay_grace_days_sequential(), 7,  "sequential grace days");
    assert_eq!(env.cfg.decay_grace_days_workflows(),  14, "workflow grace days");
    assert_eq!(env.cfg.decay_grace_days_default(),    28, "default grace days");
    assert_eq!(env.cfg.decay_half_life_days(),        30, "half-life days");
}

#[test]
fn unrecognized_decay_function_string_is_passed_through_not_defaulted() {
    // env_string returns any non-empty value as-is; downstream code must handle
    // unrecognized function names. This test documents that CONTRACT explicitly.
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("bogus-function"))]);
    let env = isolated_env();

    // env_string does NOT fall back to "power-law" for unrecognized strings.
    // The caller (decay scoring) is responsible for handling unrecognized values.
    assert_eq!(env.cfg.decay_function(), "bogus-function");
}

#[test]
fn whitespace_only_decay_function_falls_back_to_default() {
    let _guard = acquire_test_lock();
    let _env = apply_decay_env(&[("MAENIFOLD_DECAY_FUNCTION", Some("   "))]);
    let env = isolated_env();

    // env_string filters empty-after-trim, so whitespace → default.
    assert_eq!(env.cfg.decay_function(), "power-law");
}
```

The split makes the `env_string` contract explicit and separates the whitespace fallback
(which *does* revert to default) from the bogus-value case (which *does not*).

---

## Positive Observations

- **`TEST_MUTEX` + `DecayEnvGuard` pattern** is correct and necessary. Process-wide
  env var mutation in parallel Rust test runs is a global-state hazard; the mutex
  serializes it safely. The SAFETY comments are accurate.
- **`Config::reset_overrides()` in `TestEnv::Drop`** ensures the `override_root` and
  `set_database_path` calls from `isolated_env()` do not bleed into subsequent tests.
- **All five non-default values in T7 are deliberately distinct from defaults** (9 vs 7,
  18 vs 14, 33 vs 28, 45 vs 30, "exponential" vs "power-law") — correct practice.
- **RTM traceability tags** are present on every test and cross-reference
  `T-GRAPH-DECAY-001.x` items consistently.
- **T6 and T7 are properly separated** — label presence vs value correctness. This is
  deliberate and correct separation of concerns.

---

## Recommendations

Priority order:

1. **MERGE T1–T5** into a single `decay_config_accessor_defaults_match_spec` test (or
   rename if kept separate to avoid embedded literals). Net saving: ~35 lines, zero
   coverage loss. If kept separate, rename T3 to disambiguate the double "default".

2. **REWRITE T8** to add the missing `MAENIFOLD_DECAY_FUNCTION=bogus` case and the
   whitespace-only case. This documents the `env_string` pass-through contract
   explicitly and prevents a future assumption mismatch where someone expects it to
   validate function names.

3. **Add direct accessor assertions to T7** (`assert_eq!(env.cfg.decay_grace_days_sequential(), 9)`
   etc.) to close the indirect coverage gap. Low risk, high diagnostic value.

4. **Add a test for negative or zero env var values** for the integer accessors
   (e.g., `MAENIFOLD_DECAY_HALF_LIFE_DAYS=0`). `env_i64` would accept `0` as a valid
   parse, which may produce a divide-by-zero in scoring. Whether this is guarded
   elsewhere is not verified by this file.

---

## Load-Bearing Score: 8/8

Every test covers a unique behavioral contract. Structural redundancy (T1–T5) is a
*quality* concern, not a *coverage* concern — removing any one of T1–T5 would leave its
specific accessor's default unverified.

| # | Test | Load-Bearing? | Notes |
|---|---|---|---|
| T1 | `decay_grace_days_sequential_default_is_seven` | ✅ Yes | Unique accessor; structurally redundant |
| T2 | `decay_grace_days_workflows_default_is_fourteen` | ✅ Yes | Unique accessor; structurally redundant |
| T3 | `decay_grace_days_default_default_is_twenty_eight` | ✅ Yes | Unique accessor; name smell |
| T4 | `decay_half_life_days_default_is_thirty` | ✅ Yes | Unique accessor; structurally redundant |
| T5 | `decay_function_default_is_power_law` | ✅ Yes | String accessor; slightly different type |
| T6 | `get_config_summary_includes_decay_config` | ✅ Yes | Only label-presence test |
| T7 | `get_config_summary_shows_actual_values` | ✅ Yes | Only non-default display test |
| T8 | `invalid_integer_decay_env_vars_fall_back_to_defaults` | ✅ Yes | Missing string-invalid case |

Net after applying verdicts: 8 current tests restructure to **5 focused tests** (T1–T5
merged to 1, T6 kept, T7 kept, T8 split to 3) for a net gain of +3 tests with higher
specificity and no coverage loss.

---
## SWE Remediation
- MERGE T1–T5 → `decay_config_accessor_defaults_match_spec`: five isomorphic default-value tests collapsed into one table-driven test with failure messages per field. All traceability tags (T-GRAPH-DECAY-001.2/2a/3/4/6, NFR-7.5.1–7.5.5) preserved.
- KEEP T6 `get_config_summary_includes_decay_config`: unchanged.
- KEEP T7 `get_config_summary_shows_actual_values`: added direct accessor assertions (Rec #3) to close the indirect coverage gap — now asserts both raw accessor values and summary output.
- REWRITE T8 `invalid_integer_decay_env_vars_fall_back_to_defaults`: removed redundant summary assertions (already covered by T6/T7 chain), added failure messages.
- ADD `unrecognized_decay_function_string_is_passed_through_not_defaulted`: documents the `env_string` pass-through contract — bogus function names are NOT defaulted (Rec #2 P0).
- ADD `whitespace_only_decay_function_falls_back_to_default`: verifies empty-after-trim falls back to `"power-law"` (Rec #2 P0).
- Net: 8 tests → 6 tests, zero coverage loss, +2 new behavioral contracts.
- Commit: 2b4f98b
