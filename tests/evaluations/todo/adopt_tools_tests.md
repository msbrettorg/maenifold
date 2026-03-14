# Test Evaluation: adopt_tools_tests
**File:** `crates/maenifold-core/tests/adopt_tools_tests.rs`
**Test count:** 5
**Evaluated:** 2025-07-10
**Traceability:** T-1.4 · FR-1 · RTM-001 through RTM-007 · NFR-2
**Color:** 🔘 Gray — Skeptical Inquiry

---

## Summary

Five tests cover input validation, error shape, output template content, and asset
differentiation for the `Adopt` tool. The suite is leaner than `adopt_boundary.rs`
but more targeted: each test isolates a single behavioral contract. The global-state
pattern (`TEST_MUTEX` + `Config::override_root`) is correctly applied and all five
tests are structurally sound.

**What is actually proven:** The `type` field is normalized case-insensitively before
dispatch; a non-existent identifier yields the correct `AssetNotFound` error variant
with the correct Display format; an empty identifier yields `InvalidArgument` with the
correct message; the adoption instruction template contains each of its four critical
instruction markers; two different role IDs load two distinct asset bodies.

**What is not proven:** Whitespace-only identifiers (the other branch in
`validate_adopt_identifier` — `" ".trim().is_empty()`); path-traversal identifiers
(`../secret`) that exercise the security-critical `validate_relative_path` path; the
`"type":"workflow"` rejection by `parse_adopt_kind` (which rejects workflow unlike
`AssetKind::parse`); the `perspective` asset type (untested in this file); `learn:
true` mode; missing `type` / missing `identifier` → `MissingProperty` errors; and
concurrent invocation behavior beyond what the shared mutex serializes.

**Context from prior evaluation:** `adopt_boundary.md` recommends adding
`adopt_nonexistent_identifier_returns_not_found_error` as a missing gap — that test
exists here under a slightly different name, so this file partially addresses the
gap identified by the prior review.

---

## Verdicts

| Test Function | Verdict | Rationale |
|---|---|---|
| `adopt_type_is_case_insensitive` | **KEEP** | Load-bearing for `to_ascii_lowercase()` normalization. Unique. Correct assertions. Minor: missing guard on `catalog.roles.len() >= 1`. |
| `adopt_with_non_existent_identifier_returns_asset_not_found_error` | **KEEP** | Unique coverage for `AssetNotFound` error variant shape and Display format. Fills gap flagged in `adopt_boundary.md`. All assertions verified against Display impl. |
| `adopt_with_empty_identifier_returns_invalid_argument_error` | **REWRITE** | Correct for `""` but leaves the whitespace-only branch (`" "`) untested despite being an explicit `trim()` code path. Security-critical path traversal (e.g., `"../"`) also absent. Should be expanded. |
| `adopt_includes_critical_instructions` | **KEEP** | Oracle parity for the adoption instruction template. All four substring assertions confirmed against the literal format string in `adopt.rs`. Choice of `colors[0]` for the asset is unexplained but functionally correct. |
| `adopt_distinguishes_between_different_assets_of_same_type` | **KEEP** | Load-bearing for lookup correctness. `assert_ne!` is logically redundant given the `contains` checks but harmless. The `len() >= 2` guard is exemplary defensive practice. |

---

## Detailed Findings

### Finding 1 — Whitespace-Only Identifier: Explicit Code Path, Zero Test Coverage

**Test affected:** `adopt_with_empty_identifier_returns_invalid_argument_error`

`validate_adopt_identifier` in `assets/support.rs` (line 141) follows this logic:

```rust
let trimmed = identifier.trim();
if trimmed.is_empty() {           // ← covers both "" AND "   "
    return Err(MaenifoldError::InvalidArgument(
        "Identifier must not be empty or whitespace-only.".to_string(),
    ));
}
```

The test only exercises the `""` branch. A whitespace-only string (`"   "`) hits
exactly the same `trim().is_empty()` condition and is even called out explicitly in
the error message — "or whitespace-only." — yet no test probes it. The error message
documents a contract the test does not enforce.

The test name says "empty" but the production error message says "empty or
whitespace-only". There is a factual gap between what the test claims to cover and
what the code actually handles.

**Remediation:** Expand to parameterize over `""`, `"   "`, and `"\t"`. A local
helper or `rstest`-style table avoids tripling the test body.

---

### Finding 2 — Security Path: Path-Traversal Identifiers Not Tested

**Test affected:** `adopt_with_empty_identifier_returns_invalid_argument_error` (or a
new sibling test)

`validate_adopt_identifier` explicitly rejects path traversal:

```rust
if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
    return Err(invalid_identifier());
}
```

And before that, calls `validate_relative_path(trimmed)` which handles OS-level path
validation. This is a security control. There is no test for any of:

- `"../secret"` — upward traversal
- `"../../etc/passwd"` — deep traversal
- `"role/../../secret"` — embedded slash

A security-critical validation path with zero test coverage is a finding regardless
of how simple the implementation looks. If a future refactor reorders the checks or
changes the `validate_relative_path` signature, there is no regression safety net.

**Severity:** Medium. The path is exercised through integration tests indirectly, but
a dedicated boundary test is the minimum acceptable bar for RTM-171/RTM-183 style
security gates.

**Remediation:** Add `adopt_with_path_traversal_identifier_is_rejected` asserting on
`InvalidArgument` or `PathTraversal` error variant for `"../bad"`, `"a/b"`, and
`"..secret"` (the last is legal as a slug in some systems — verify whether
`validate_relative_path` rejects it).

---

### Finding 3 — `workflow` Type Silently Absent from Adopt's Allow-List

**Systemic finding, no single test affected**

`AssetKind::parse()` accepts "workflow". `parse_adopt_kind()` in `assets/adopt.rs`
does NOT:

```rust
fn parse_adopt_kind(value: &str) -> Result<AssetKind> {
    match value.to_ascii_lowercase().as_str() {
        "role" => Ok(AssetKind::Role),
        "color" => Ok(AssetKind::Color),
        "perspective" => Ok(AssetKind::Perspective),
        _ => Err(MaenifoldError::InvalidArgument(format!(
            "Invalid type '{value}'. Must be one of: role, color, perspective"
        ))),
    }
}
```

A caller who reads `AssetKind::all()` (which includes `Workflow`) might assume
`Adopt` accepts all four types. There is no test asserting that `type:"workflow"`
is explicitly rejected with a useful error message containing the allowed-types list.

**Assumption at risk:** "All asset types are adoptable." — This assumption is false
but untested, leaving documentation as the only guard.

**Remediation:** Add `adopt_workflow_type_is_rejected_with_allowed_types_in_message`
asserting `InvalidArgument` containing "Must be one of: role, color, perspective"
but NOT "workflow". Pairs with the existing `adopt_invalid_type_returns_invalid_type_error`
test in `adopt_boundary.rs`.

---

### Finding 4 — `assert_ne!` in Test 5 Is Logically Redundant

**Test affected:** `adopt_distinguishes_between_different_assets_of_same_type`

The three assertions in order:

```rust
assert!(first_output.contains(&first_role_id));   // output₁ ∋ id₁
assert!(second_output.contains(&second_role_id)); // output₂ ∋ id₂
assert_ne!(first_output, second_output);          // output₁ ≠ output₂
```

Given that `first_role_id != second_role_id` (they are two different catalog entries),
and `first_output` contains `first_role_id`, and `second_output` contains
`second_role_id`, the `assert_ne!` follows trivially unless one output also contains
the other's id by coincidence (e.g., one role's JSON body embeds the second role's
id string). This edge case is possible but unlikely with real bundled assets.

The `assert_ne!` is **not wrong** — it's a belt-and-suspenders check. But it
provides no additional signal in normal circumstances and creates a misleading
impression that it is independently verifying something. A code reviewer encountering
this test might ask: "what would fail the `assert_ne!` that the `contains` checks
would NOT?" — and find no satisfying answer.

**Verdict:** Harmless. Do not remove (it is documentarily clear), but note it in a
comment: `// logically follows from the contains assertions above given distinct ids`.

---

### Finding 5 — `catalog.colors[0]` in Test 4 Is Unexplained and Unguarded

**Test affected:** `adopt_includes_critical_instructions`

```rust
let color_id = asset_id(&catalog(&config).colors[0]);
```

No `assert!` guards that at least one color exists before indexing. Compare with
test 5 which correctly guards: `assert!(catalog.roles.len() >= 2, ...)`. If the
asset initialization step ever produces zero colors (e.g., missing bundled file, CI
environment change), this test panics with an index-out-of-bounds rather than a
meaningful diagnostic failure.

Additionally: the choice of a color asset for testing instruction content is
arbitrary. The CRITICAL INSTRUCTIONS template is asset-type-agnostic (the format
string uses `kind.as_str()` uniformly). Using `colors[0]` tells the reader nothing
meaningful about *why* color was chosen for this test.

**Remediation:** Add `assert!(!catalog.colors.is_empty(), "expected at least one bundled color asset")` before the index access. Optionally: add a comment explaining the color type was chosen to demonstrate the template is asset-type-agnostic.

---

### Finding 6 — `perspective` Type Has No Coverage in This File

**Systemic finding**

`adopt_boundary.rs` covers all three adoptable types (role, color, perspective).
This file covers role (tests 1, 5) and color (test 4). The perspective type is
untested here. Given these tests target different behavioral properties than
`adopt_boundary.rs` (error variants vs. happy-path output shape), the absence of
perspective is not critical — but it means no test in this file would catch a
regression where `perspective` type parsing breaks while role and color remain intact.

**Impact:** Low, because `adopt_boundary.rs` covers perspective. No action required,
but worth noting.

---

### Finding 7 — Double Validation: `validate_adopt_identifier` Called Twice

**Systemic finding, production code smell surfaced by test analysis**

`adopt()` calls `validate_adopt_identifier(identifier)` on line 27, and
`read_adopt_asset()` calls `validate_adopt_identifier(identifier)` again internally
(line 210 of `manager.rs`). This means every successful adoption runs the identifier
validator twice.

The second call is safe (idempotent), but it is redundant and could mislead a
maintainer into thinking the first call is unnecessary. No test exercises a state
where the two calls could diverge (they cannot under current code). This is a
production code quality observation, not a test quality defect — but the test suite
is where such structural redundancy would normally be discovered and documented.

**Recommendation:** Remove the internal `validate_adopt_identifier` call from
`read_adopt_asset` (since callers of that function are trusted internal paths that
always pre-validate), or add a `// pre-validated by caller` comment to document the
layered approach intentionally.

---

## Recommendations

1. **REWRITE `adopt_with_empty_identifier_returns_invalid_argument_error`**: Expand to
   cover `""`, `" "`, `"\t"`, and `"../secret"` (path traversal). The existing
   empty-string case is correct; the whitespace and traversal cases are explicit code
   paths with no coverage. This is the highest-priority improvement.

2. **ADD `adopt_with_path_traversal_identifier_is_rejected`**: Security-critical gate.
   Test `"../"`, `"a/b"`, and `"../../etc/passwd"` all return `InvalidArgument` or
   `PathTraversal` (not a panic or a silent misread).

3. **ADD `adopt_workflow_type_is_rejected`**: Confirm the `parse_adopt_kind` allow-list
   exclusion of "workflow" is intentional and surfaced in the error message.

4. **ADD `assert!(!catalog.colors.is_empty())` guard in test 4** before indexing
   `catalog.colors[0]`. Pattern already set correctly in test 5 for roles.

5. **ANNOTATE the `assert_ne!` in test 5** with a comment explaining it is a
   belt-and-suspenders check that follows from the `contains` assertions.

6. **ADD missing-property tests** for `type` and `identifier` fields omitted
   entirely from the payload → `MaenifoldError::MissingProperty`. Currently zero
   coverage in this file.

7. **INVESTIGATE production double-validation** in `adopt()` + `read_adopt_asset()`.
   Document intent or remove redundancy.

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing? | Rationale |
|---|---|---|
| `adopt_type_is_case_insensitive` | ✅ Yes | Directly exercises `to_ascii_lowercase()` in `parse_adopt_kind`. Would catch a regression that broke case normalization. |
| `adopt_with_non_existent_identifier_returns_asset_not_found_error` | ✅ Yes | Only test in the codebase that pins the `AssetNotFound(kind, identifier)` variant shape AND its Display format string. Fills an explicit gap from `adopt_boundary.md`. |
| `adopt_with_empty_identifier_returns_invalid_argument_error` | ⚠️ Partial | Covers `""` but leaves `" "` (the other explicit branch) uncovered. Would not catch a regression that broke whitespace handling while preserving empty-string handling. Name claims more than it tests. |
| `adopt_includes_critical_instructions` | ✅ Yes | Pins the adoption instruction template format. Would catch a refactor that silently truncated or reworded the CRITICAL INSTRUCTIONS block. Oracle parity is the right justification. |
| `adopt_distinguishes_between_different_assets_of_same_type` | ✅ Yes | Would catch a bug where lookup always returns the same (first) asset regardless of identifier. Lookup correctness is non-trivial due to the fast-path/fallback-scan dual strategy in `read_adopt_asset`. |

> Tests 1, 2, 4, 5 are fully load-bearing. Test 3 is partial. After the REWRITE
> recommended above, the suite should reach 5/5. The three unwritten tests
> (path traversal, workflow rejection, missing-property) would bring the file to
> 8 tests total, all load-bearing.

---

## SWE Remediation

- **REWRITE** `adopt_with_empty_identifier_returns_invalid_argument_error` → `adopt_with_empty_or_whitespace_identifier_returns_invalid_argument_error`: parameterized over `""`, `"   "`, `"\t"` to cover the `trim().is_empty()` whitespace-only branch (Finding 1 / Rec 1, P0)
- **ADD** `adopt_with_path_traversal_identifier_is_rejected`: tests `"../secret"`, `"../../etc/passwd"`, `"a/b"`, `"role\\..\\secret"` all return `InvalidArgument` with slug-validation message (Finding 2 / Rec 2, P0)
- **ADD** `adopt_workflow_type_is_rejected_with_allowed_types_in_message`: confirms `parse_adopt_kind` rejects "workflow" with the correct allowed-types list (Finding 3 / Rec 3, P1)
- **ADD** `assert!(!cat.colors.is_empty(), ...)` guard in `adopt_includes_critical_instructions` before `colors[0]` access, matching the defensive pattern from test 5 (Finding 5 / Rec 4, P1)
- **ANNOTATE** `assert_ne!` in `adopt_distinguishes_between_different_assets_of_same_type` with belt-and-suspenders comment (Finding 4 / Rec 5, P2)
- **ADD** `adopt_missing_required_properties_returns_missing_property_error`: tests missing `type`, missing `identifier`, and empty payload all yield `MissingProperty` with correct field name (Rec 6, P2)
- **SKIP** Rec 7 (P3): production double-validation investigation is out-of-scope for test remediation
- Test count: 5 → 8 (1 rewritten, 3 added). All 8 pass.
- Commit: 7ba746f
