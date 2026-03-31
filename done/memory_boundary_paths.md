---
id: memory_boundary_paths
status: done
---

# Test Evaluation: memory_boundary_paths

**File:** `crates/maenifold-core/tests/memory_boundary_paths.rs`
**Test count:** 15
**Evaluated:** 2025-07-15
**Traceability:** T-14.5 / T-15.19 / T-7.8 / T-12.5 / FR-12 / FR-21 / NFR-1

---

## Summary

The strongest of the three files. Security tests are specific, multi-case, and test actual rejection behavior. Decay tests use relative comparisons rather than hardcoded floats — a sound design. Two concerns: test 2 is partially redundant with test 1 (both verify `.md` extension), and the decay tests' correctness is **implicitly coupled to default config values** (`MAENIFOLD_DECAY_GRACE_DAYS_*`) that could be overridden via environment variables in CI, making the tests environmentally fragile without being obviously so.

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `memory_uri_to_path_resolves_nested_uri_under_memory_root` | **KEEP** | Tests full resolved path equality including nested directories and `.md` extension; the single most comprehensive path-resolution test |
| `memory_uri_to_path_adds_md_extension_for_extensionless_uris` | **REWRITE** | The assertion `extension() == Some("md")` is partially covered by test 1, which already embeds `.md` in its expected path. Rewrite to test the edge case that's NOT covered by test 1: that a URI already ending in `.md` does NOT get a double extension (e.g., `memory://note.md` → `note.md`, not `note.md.md`) |
| `validate_relative_path_rejects_absolute_paths` | **KEEP** | Critical security test; the 3-clause OR on the error message is acceptable (guards against minor message rewording without losing intent) |
| `validate_relative_path_rejects_dot_sequence_traversal` | **KEEP** | Best test in the file — 5 traversal variants in a loop, including whitespace-padded dots and embedded traversal. All distinct attack patterns |
| `validate_relative_path_accepts_valid_nested_relative_folder` | **KEEP** | Happy path; confirms valid inputs aren't rejected by the traversal checks |
| `slugify_title_matches_memory_tool_contract_for_simple_title` | **REWRITE** | **Naming problem**: "memory tool contract" is vague jargon. The test name should describe what it's testing. Rename to `slugify_title_lowercases_and_hyphenates_spaces`. The assertion is correct but only covers the happy path; the name makes it sound like a contract verification when it's a single example |
| `validate_relative_path_rejects_url_encoded_traversal_sequences` | **KEEP** | Critical security test; exercises URL-decoding pipeline (`%2e%2e/`, `%2E%2E/`, `%2e%2e%2f`). Load-bearing against any regression in `urlencoding::decode` usage |
| `memory_uri_to_path_rejects_traversal_uri` | **KEEP** | Confirms that `memory_uri_to_path` doesn't bypass `validate_relative_path`; tests the composition, not just the components. Load-bearing against a refactor that decouples the two functions |
| `slugify_title_falls_back_to_untitled_when_all_characters_are_filtered` | **KEEP** | Tests the `collapsed.is_empty()` → `"untitled"` fallback; would catch removal of that guard |
| `sanitize_user_input_strips_url_encoded_path_traversal_sequences` | **KEEP** | Tests URL-decoded traversal stripping in user input; separate security surface from `validate_relative_path` |
| `sanitize_user_input_strips_mixed_encoded_and_plain_path_traversal_sequences` | **KEEP** | Distinct from test 10: tests mixed (`%2e./`) encoding, where partial URL-encoding is combined with literal dots |
| `compute_decay_weight_defaults_to_one_without_last_accessed` | **KEEP** | Tests the `None` timestamp → `1.0` short-circuit return; would catch removal of the early-return in `compute_decay_weight_with_created` |
| `compute_decay_weight_uses_shorter_sequential_grace_period_than_default` | **KEEP — with caveat** | Correct relative assertion. **Caveat:** at 10 days, sequential (grace=7d) has started decaying but default (grace=28d) hasn't — so `sequential < 1.0 == default`. This works with default config, but if `MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL` is set to >10 in the test environment, both return `1.0` and the assertion becomes `1.0 < 1.0` → **test silently passes the wrong condition**. The test should pin config values explicitly |
| `compute_decay_weight_keeps_validated_assumptions_at_one` | **KEEP** | Tests the unconditional `return 1.0` for `"validated"` status; 90-day age ensures decay would occur without that guard. Most important decay test |
| `compute_decay_weight_decays_invalidated_assumptions_faster_than_active` | **KEEP — with caveat** | Correct: at 20 days, invalidated (grace=7, hl=14) weight ≈ 0.53 vs active (grace=14, hl=30) weight ≈ 0.87. Same env-var fragility as test 13: the 20-day value was chosen to satisfy default constants. Should document the chosen day value relative to grace periods, or set config explicitly |

---

## Security Coverage Assessment

The path traversal tests are thorough and cover the expected OWASP path-traversal attack patterns:

| Attack Pattern | Covered By |
|---|---|
| Absolute path | test 3 |
| `../` classic traversal | test 4 (variant 1) |
| Embedded `/../` traversal | test 4 (variant 2) |
| Whitespace-padded dots `.. ` | test 4 (variant 3) |
| Dot-space-dot `. .` | test 4 (variant 4) |
| Embedded whitespace traversal `safe/. ./` | test 4 (variant 5) |
| `%2e%2e/` URL-encoded | test 7 (variant 1) |
| `%2E%2E/` uppercase-encoded | test 7 (variant 2) |
| `%2e%2e%2f` fully-encoded slash | test 7 (variant 3) |
| Traversal via `memory://` URI | test 8 |

**Not covered:** null bytes (`%00`), Unicode look-alikes (U+2024 ONE DOT LEADER as substitute for `.`), overlong UTF-8 sequences. These are low-probability in Rust's string model but worth documenting as known non-coverage.

---

## Missing Tests (Gaps)

1. **`memory_uri_to_path` with a URI already ending in `.md`**: `memory://note.md` should resolve to `note.md`, not `note.md.md`. The implementation handles this, but there is no test for it. This is the bug that test 2 should be testing instead of duplicating test 1's coverage.

2. **`slugify_title` with trailing/leading hyphens**: `"---hello---"` should become `"hello"` (the `trim_matches('-')` step). The `"!!! ### ***"` test covers the all-filtered case, but not trim-on-valid-content.

3. **`compute_decay_weight` with future `last_accessed` date**: what happens if the timestamp is in the future? The implementation has `if age_days < 0 { return 1.0; }`. Untested.

4. **`sanitize_user_input` with a clean input** (no traversal sequences): no regression test ensures the function is a no-op on safe input. Should add `assert_eq!(sanitize_user_input("hello world"), "hello world")`.

---

## Decay Test Environmental Fragility

Tests 13 and 15 implicitly assume default config values:
- Sequential grace: 7 days
- Default grace: 28 days  
- Invalidated grace/half-life: 7/14 days
- Active grace/half-life: 14/30 days

If any `MAENIFOLD_DECAY_*` env var is set in the test runner's environment, the relative comparisons can silently pass while testing wrong conditions. **Recommendation**: Use `Config::override_root` (already done for isolation) and explicitly set the decay config by calling the env-override helpers, or document which day values are safe with which config ranges.

---

## Recommendations

1. **Rewrite test 2** to cover the double-extension prevention case (`memory://note.md` → `note.md` not `note.md.md`). The current behavior it tests is already covered by test 1.

2. **Rename `slugify_title_matches_memory_tool_contract_for_simple_title`** → `slugify_title_lowercases_and_hyphenates_spaces`.

3. **Pin config values in decay tests 13 and 15** or add a comment documenting the day values were chosen for the default config. At minimum, add a `assert_eq!(cfg.decay_grace_days_sequential(), 7, "test relies on default")` guard at the top of those tests.

4. **Add missing test**: `memory_uri_to_path_does_not_double_extension_md_uri`.

5. **Add missing test**: `sanitize_user_input_passes_clean_input_unchanged`.

---

## Load-Bearing Score: 14/15

Test 2 (`memory_uri_to_path_adds_md_extension_for_extensionless_uris`) as currently written is partially subsumed by test 1. The `.md` extension fact is already proven by test 1's full path equality assertion. All other 14 tests cover distinct behavior not exercised elsewhere.

---

## SWE Remediation

- REWRITE test 2 (`memory_uri_to_path_adds_md_extension_for_extensionless_uris`): replaced with `memory_uri_to_path_does_not_double_extend_md_uri` — tests that `memory://note.md` resolves to `note.md` not `note.md.md` (R1/R4, P0)
- REWRITE test 6 (`slugify_title_matches_memory_tool_contract_for_simple_title`): renamed to `slugify_title_lowercases_and_hyphenates_spaces` (R2, P1)
- KEEP+HARDEN test 13 (`compute_decay_weight_uses_shorter_sequential_grace_period_than_default`): added guard assertions pinning `decay_grace_days_sequential() == 7` and `decay_grace_days_default() == 28` (R3, P1)
- KEEP+DOCUMENT test 15 (`compute_decay_weight_decays_invalidated_assumptions_faster_than_active`): added comment documenting that assumption status grace/half-life values are hardcoded in `scoring.rs` and not affected by config env vars (R3, P1)
- ADD `sanitize_user_input_passes_clean_input_unchanged`: regression test ensuring function is a no-op on safe input (R5, P2)
- Test count: 15 → 16 (1 added, 1 rewritten, 1 renamed)
- All 16 tests pass: `cargo test --test integration_tests -p maenifold-core -- memory_boundary_paths`
- Commit: 2dd6932

---

## Red-team Audit

**Scope:** Commit `2dd6932` — 1 file changed (35 insertions, 5 deletions). Only `memory_boundary_paths.rs` touched. No source code changes. No unrelated modifications.

**Traceability:** T-14.5 / T-15.19 / T-7.8 / T-12.5 / FR-12 / FR-21 / NFR-1

- ✅ **REWRITE test 2** (`memory_uri_to_path_does_not_double_extend_md_uri`): Matches eval R1/R4 (P0). Input changed to `"memory://note.md"`. Asserts `file_name() == Some("note.md")` and `extension() == Some("md")` — covers the exact double-extension gap the eval identified. Traceability comment `T-15.19: RTM FR-12` present.
- ✅ **REWRITE test 6** (`slugify_title_lowercases_and_hyphenates_spaces`): Matches eval R2 (P1). Rename exact. Assertion body unchanged (eval confirmed it was correct). Traceability comment `T-14.5: RTM FR-12` present.
- ✅ **KEEP+HARDEN test 13**: Matches eval R3 (P1). Added `assert_eq!(cfg.decay_grace_days_sequential(), 7, ...)` and `assert_eq!(cfg.decay_grace_days_default(), 28, ...)` guard assertions at top of test — fails fast if defaults change, preventing the silent-pass scenario the eval warned about. Traceability comment `T-12.5: RTM FR-21` present.
- ✅ **KEEP+DOCUMENT test 15**: Matches eval R3 alternative (P1). Added inline documentation explaining that assumption status grace/half-life values are hardcoded in `scoring.rs` (not env-configurable), with exact values (invalidated: grace=7d, hl=14d; active: grace=14d, hl=30d) and worked examples. This is stronger than a guard assertion — it proves the eval's env-var fragility concern does not apply to this specific test's assertion pair.
- ✅ **ADD `sanitize_user_input_passes_clean_input_unchanged`**: Matches eval R5 (P2). Asserts `sanitize_user_input("hello world") == "hello world"` — exact regression test the eval specified.
- ✅ **KEEP tests untouched**: All 11 KEEP-verdict tests (1, 3–5, 7–12, 14) are unmodified in the diff.
- ✅ **Test count**: 15 → 16 confirmed (16 `#[test]` markers, 16 tests run).
- ✅ **All 16 pass**: Independently verified via `cargo test --test integration_tests -p maenifold-core -- memory_boundary_paths` — `ok. 16 passed; 0 failed`.

**Verdict: PASS**

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_boundary_paths`: 16 passed, 0 failed
- Verdict: PASS
