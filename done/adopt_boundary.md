---
id: adopt_boundary
status: done
pipeline: tdd
---

# Test Evaluation: adopt_boundary
**File:** `crates/maenifold-core/tests/adopt_boundary.rs`
**Test count:** 7
**Evaluated:** 2026-06-23
**Traceability:** T-1.4a · FR-1 · RTM-001 through RTM-007 · NFR-2
**Color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

Seven tests cover the `Adopt` tool's boundary layer. The suite has correct structural bones — each RTM item from RTM-001 through RTM-007 has at least one test, and the `isolated_config()` + mutex pattern correctly prevents cross-test state bleed. However, the three happy-path type tests (role, color, perspective) are near-identical structural clones with assertions so permissive they would pass against almost *any* string output that echoes back the identifier. The most rigorous test in the file (`adopt_output_contains_json_configuration_followed_by_valid_json`) carries disproportionate weight for the entire output-format contract — and it only runs against the role type. Two tests have assertion logic that would survive output degradation without failing.

**What is actually proven:** The tool exists, accepts all three asset types, rejects an invalid type, can locate assets by `id` field fallback, and embeds parseable JSON in its output for the role type under the `JSON Configuration:` marker.

**What is not proven:** Output shape for color/perspective types (RTM-002/RTM-003 only verify identifier echo, not JSON embedding), `learn=true` for color/perspective types, behavior on missing asset file (no 404 test), behavior on malformed asset JSON, concurrent invocation safety beyond the mutex, and whether `extract_json_configuration` correctly handles nested JSON with escaped quotes (the helper is used exactly once).

---

## Verdicts

| Test Function | Verdict | Reason |
|---|---|---|
| `adopt_role_returns_instruction_containing_role_and_identifier` | MERGE | Structurally identical to the color/perspective variants. All three should become a parameterized table test. Assertions are also too thin — `lower.contains("role")` passes if "role" appears anywhere in any error message. |
| `adopt_color_returns_adoption_instruction` | MERGE | Clone of the role test. Duplication provides false coverage confidence. Merge into parameterized sibling. |
| `adopt_perspective_returns_adoption_instruction` | MERGE | Same. Third structural clone. Merge. |
| `adopt_invalid_type_returns_invalid_type_error` | KEEP | Sole negative test for the type enum. Load-bearing. Error message coupling to `"Invalid type"` is acceptable at boundary layer. |
| `adopt_falls_back_to_json_id_field_when_filename_differs` | KEEP | Tests a non-obvious, user-facing lookup behavior that nothing else exercises. High signal if it breaks. |
| `adopt_learn_true_returns_help_documentation_not_adoption_instruction` | REWRITE | The negative assertion (`!output.contains("JSON Configuration:")`) is the only trustworthy signal. The positive OR-of-three (`"Adopt" \|\| "Loads" \|\| "Parameters"`) would pass if output contained merely the word "Adopt" in a failure message. Tighten to a structural help-output invariant. |
| `adopt_output_contains_json_configuration_followed_by_valid_json` | KEEP | Most rigorous test in the file. Parses actual embedded JSON, validates `id` and `name` fields. The `extract_json_configuration` helper adds real value. Should be extended to cover color/perspective types, not just role. |

---

## Detailed Findings

### Finding 1 — Assertion Strength: Happy-Path Type Tests (Tests 1–3)

**Tests:** `adopt_role_*`, `adopt_color_*`, `adopt_perspective_*`

The positive assertions reduce to:
```rust
assert!(lower.contains("role"));       // passes if output is "Error in role handler"
assert!(output.contains("test-role")); // passes if identifier echoed in any error
```

These check that *some* output was produced containing a type keyword and the identifier. They do not verify:
- That `JSON Configuration:` marker is present (only test 7 does this, only for role)
- That the embedded JSON is parseable
- That the asset's `name` field appears in output
- That the output structure matches the adoption instruction template

**Verdict:** These three tests, merged and rewritten with the assertions from test 7, would cover RTM-001/002/003 properly. As written, they would survive significant output regression.

---

### Finding 2 — Parameterization Opportunity (Tests 1–3)

All three tests follow an identical pattern:
1. Write a minimal asset JSON to `{type}s/{id}.json`
2. Invoke `Adopt` with the same asset type
3. Assert the type keyword and identifier appear in output

Rust does not have native parameterized tests but `rstest` or a local macro would reduce 3×40 lines to a single table. More importantly, the same coverage gap (missing JSON validation) would be fixed once instead of three times.

---

### Finding 3 — Weak Positive in `learn=true` Test (Test 6)

```rust
assert!(
    output.contains("Adopt") || output.contains("Loads") || output.contains("Parameters"),
    "expected help documentation shape: {output}"
);
```

This OR-of-alternatives assertion would pass if output contained merely the word "Adopt" in a prefix like "Adopt failed". The intent is to verify a *help documentation* shape, but "Adopt" is present in any output about this tool. The negative assertion (`!output.contains("JSON Configuration:")`) is the only part of this test with real discriminating power.

**Remediation:** Replace the positive check with something structural — e.g., that output contains at least two of: a tool description, a parameter list header, a usage example. Or assert `output.contains("learn")` to verify the parameter itself is documented.

---

### Finding 4 — `extract_json_configuration` Helper Used Once

The `extract_json_configuration` function (40+ lines, handles escape sequences, bracket depth) is a non-trivial parser that exists in test code. It is invoked in exactly one test. If the embedded-JSON contract is worth this much test infrastructure, it warrants being exercised against all three asset types, not just role.

---

### Finding 5 — No Missing-Asset Test

None of the seven tests verify what happens when `identifier` does not match any file in the asset directory — either by filename or by `id` field. This is a predictable failure mode (typos in tool calls). Whether it returns a graceful error or panics is unspecified by the test suite.

**Recommendation:** Add one negative test: `adopt_nonexistent_identifier_returns_not_found_error`.

---

### Finding 6 — Traceability: No T-* Item for This Evaluation Task

The tests correctly carry `// T-1.4a: RTM FR-1` attribution (T-1.4a is archived in CHANGELOG.md). This evaluation itself is driven by NFR-2 ("Each tool SHALL have tests at all 3 layers"). No active T-* item in TODO.md explicitly schedules test quality auditing; this is a gap in the sprint backlog traceability model.

---

## Recommendations

1. **Merge tests 1–3** into a single parameterized test (or a local helper called three times) with assertions matching test 7's rigor: verify `JSON Configuration:` marker present and embedded JSON contains `id` and `name` fields. Covers RTM-001, RTM-002, RTM-003 with real signal.

2. **Extend test 7** to exercise color and perspective asset types, not only role. The `extract_json_configuration` helper is already written.

3. **Rewrite test 6** (`learn=true`): replace the OR-of-three positive assertion with a structural invariant (e.g., parameter list present, tool name in output, no adoption instruction present). Keep the negative `JSON Configuration:` check.

4. **Add a missing-asset test**: `adopt_nonexistent_identifier_returns_not_found_error` — verifies graceful error, not panic, for unknown identifiers.

5. **Add a malformed-asset-JSON test**: What happens if the asset file contains invalid JSON? The tool should return a meaningful error, not an internal Rust panic. This is an unexercised failure path.

6. **Track test quality audits** under an explicit TODO backlog item (e.g., `T-QA.*`) to satisfy NFR-2 traceability requirements.

---

## Load-Bearing Score: 4/7

| Test | Load-Bearing? | Rationale |
|---|---|---|
| `adopt_role_returns_instruction_containing_role_and_identifier` | ⚠️ Partial | Covers RTM-001 happy path but assertions too thin to catch real regressions |
| `adopt_color_returns_adoption_instruction` | ⚠️ Partial | Covers RTM-002 but is a structural clone of test 1 |
| `adopt_perspective_returns_adoption_instruction` | ⚠️ Partial | Covers RTM-003 but is a structural clone of test 1 |
| `adopt_invalid_type_returns_invalid_type_error` | ✅ Yes | Sole test for the invalid-type error path |
| `adopt_falls_back_to_json_id_field_when_filename_differs` | ✅ Yes | Unique behavior; only coverage for id-field fallback lookup |
| `adopt_learn_true_returns_help_documentation_not_adoption_instruction` | ✅ Yes | Sole test for `learn=true`; negative assertion has real signal |
| `adopt_output_contains_json_configuration_followed_by_valid_json` | ✅ Yes | Most rigorous structural test; would catch JSON embedding regressions |

> Tests 1–3 are counted as partial load-bearing: they redundantly cover the same structural pattern and their assertions would survive significant output degradation. After the MERGE+REWRITE recommended above, the load-bearing score for the resulting 5-test suite should be 5/5.

---

## SWE Remediation

- MERGE tests 1–3 (`adopt_role_*`, `adopt_color_*`, `adopt_perspective_*`): collapsed into single parameterized test `adopt_all_asset_types_return_instruction_with_valid_json_configuration` via helper `assert_adopt_type_produces_valid_instruction`. Assertions strengthened to test-7 rigor: verifies `JSON Configuration:` marker present, embedded JSON parseable, `id` and `name` fields match. Covers RTM-001, RTM-002, RTM-003.
- REWRITE `adopt_learn_true_returns_help_documentation_not_adoption_instruction`: replaced weak OR-of-three positive assertion (`"Adopt" || "Loads" || "Parameters"`) with structural help-path invariant (`lower.contains("help")`) plus two negative adoption markers (`!contains("JSON Configuration:")`, `!contains("CRITICAL INSTRUCTIONS:")`). Original would pass on any output mentioning "Adopt"; replacement requires help code path evidence and absence of adoption instruction structure.
- KEEP `adopt_invalid_type_returns_invalid_type_error`: unchanged.
- KEEP `adopt_falls_back_to_json_id_field_when_filename_differs`: unchanged.
- KEEP `adopt_output_contains_json_configuration_followed_by_valid_json`: unchanged.
- ADD `adopt_nonexistent_identifier_returns_not_found_error` (R4): new negative test verifying graceful error on missing asset. Confirms no panic.
- ADD `adopt_malformed_asset_json_does_not_panic` (R5): new resilience test for invalid-JSON asset files. Documents current pass-through behavior (tool returns Ok with raw content rather than Err); asserts no panic in either path.
- Final test count: 7 (was 7; 3 removed by MERGE, 1 added by MERGE, 2 added by R4/R5)
- Load-bearing score: 7/7 (was 4/7)
- Commit: a66e608

---

## Red-team Audit

**Audited:** 2026-06-23 · **Ref:** T-1.4a · FR-1 · NFR-2

### MERGE tests 1–3 → `adopt_all_asset_types_return_instruction_with_valid_json_configuration`

- ✅ Three original tests (`adopt_role_*`, `adopt_color_*`, `adopt_perspective_*`) removed from file — confirmed via diff
- ✅ Single parameterized test calls shared helper `assert_adopt_type_produces_valid_instruction` for all three asset types (role, color, perspective)
- ✅ Assertions upgraded to test-7 rigor: `output.contains("JSON Configuration:")` marker check present, `extract_json_configuration` parses embedded JSON, `parsed["id"]` and `parsed["name"]` validated against expected values
- ✅ RTM-001, RTM-002, RTM-003 explicitly called out in test comments
- ✅ `extract_json_configuration` helper now exercised against all three types (was role-only) — satisfies eval Finding 4 / Recommendation 2
- ✅ No unique coverage lost: every assertion from the original three tests (type keyword + identifier echo) is preserved, with strictly stronger assertions added on top

### REWRITE `adopt_learn_true_returns_help_documentation_not_adoption_instruction`

- ✅ Weak OR-of-three (`"Adopt" || "Loads" || "Parameters"`) removed — confirmed via diff
- ✅ Replaced with `lower.contains("help")` — "help" is diagnostic of the help code path (unlike "Adopt" which appeared in all tool output). Eval spec said "something structural" and offered examples; this satisfies the intent
- ✅ Original negative assertion `!output.contains("JSON Configuration:")` preserved
- ✅ Second negative assertion added: `!output.contains("CRITICAL INSTRUCTIONS:")` — strictly stronger than eval spec required (eval only asked to keep one negative)
- ✅ The combination of one positive help-path signal + two negative adoption-path guards gives real discriminating power against false passes

### KEEP `adopt_invalid_type_returns_invalid_type_error`

- ✅ Unchanged — confirmed via diff (no modifications to this test in commit)

### KEEP `adopt_falls_back_to_json_id_field_when_filename_differs`

- ✅ Unchanged — confirmed via diff (no modifications to this test in commit)

### KEEP `adopt_output_contains_json_configuration_followed_by_valid_json`

- ✅ Unchanged — confirmed via diff (no modifications to this test in commit)

### ADD `adopt_nonexistent_identifier_returns_not_found_error` (R4)

- ✅ New test present, matches eval Recommendation 4 exactly
- ✅ Asserts `result.is_err()` — verifies graceful error, not panic
- ✅ Uses `does-not-exist` identifier against clean isolated config (no assets written)
- ✅ Follows existing test patterns: mutex guard, isolated_config, Config::reset_overrides

### ADD `adopt_malformed_asset_json_does_not_panic` (R5)

- ✅ New test present, matches eval Recommendation 5
- ✅ Writes `"{ not valid json !!!"` to asset file — exercises invalid JSON path
- ✅ Handles both Ok and Err outcomes via `match` — no-panic property proven regardless of which path the tool takes
- ✅ Documents current behavior (pass-through) vs ideal (error) in comments — honest characterization

### Cross-cutting

- ✅ Test count: 7 (3 removed by MERGE + 1 added by MERGE + 2 added by R4/R5 = net 0 change). Matches remediation claim
- ✅ All 7 tests pass: `cargo test -p maenifold-core adopt_boundary` — 7 passed, 0 failed
- ✅ No unrelated changes: commit touches exactly 1 file (`adopt_boundary.rs`), only the tests specified in verdicts
- ✅ Traceability: all test comments reference T-1.4a, RTM FR-1, and specific RTM/TC items
- ✅ Commit message (a66e608) explicitly lists each verdict applied and references the eval spec file

### Not addressed (eval Recommendations 3 and 6 — informational only)

- Recommendation 3 (extend test 7 to color/perspective): The intent is satisfied by the MERGE — `extract_json_configuration` now runs against all three types in the merged test. Test 7 remains as a focused role-only regression test. No coverage gap.
- Recommendation 6 (T-QA.* backlog item): Meta-recommendation about sprint planning, not a code change. Outside SWE scope; PM responsibility.

### Verdict: **PASS**

---
## Blue-team Verification
- `cargo test -p maenifold-core --test integration_tests adopt_boundary`: 7 passed, 0 failed
- Verdict: PASS
