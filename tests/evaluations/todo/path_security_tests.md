# Test Evaluation: path_security_tests

**File**: `crates/maenifold-core/tests/path_security_tests.rs`  
**Evaluator**: Red Team (Gray — Skeptical Inquiry)  
**Traceability**: T-QUAL-002.6h (per test comments) · RTM-182 (symlink canonicalization) · RTM-183 (path traversal denial) · NFR-2 · NFR-3  
**Date**: 2025-07-17

---

## Summary

This file contains 6 integration tests covering two path-security properties across two
tools (`WriteMemory`, `ListMemories`): (1) rejection of forbidden path forms (absolute
paths, dot-sequence traversal, whitespace-obfuscated variants, symlink escape), and (2)
acceptance of legitimate relative paths. The file also functions as a cross-tool
consistency check — asserting that both tools apply equivalent validation.

**What the tests prove** (when they pass): Both `WriteMemory` and `ListMemories` reject
known malicious path inputs and accept well-formed relative paths.

**What they do not prove**: That the two validators are genuinely shared — they are in
fact two parallel implementations (`validate_relative_folder` in `memory/support.rs` and
`validate_relative_path` in `util/paths.rs`), not a single shared function. A divergence
between them would require these tests to be updated in two places.

**Three structural defects require immediate attention**:

1. **Phantom assertion in Test 4** — `!list_result.contains("Directory traversal
   detected")` asserts the absence of a string that does not exist anywhere in the
   production codebase. The assertion is vacuously true and protects against nothing.

2. **Traceability mismatch** — All tests are tagged `T-QUAL-002.6h: RTM FR-25.5` or
   `RTM FR-25.6`. In the current PRD/RTM, FR-25 is *Azure Deployment Manifests*
   (Dockerfile, Bicep, CI). The correct RTM entries for path security in tool contexts
   are RTM-182 (symlink canonicalization) and RTM-183 (path traversal denial) under
   FR-29. The `T-QUAL-002.6h` sprint tag appears to predate the current FR numbering.

3. **Name claim "shared_path_validator" is false** — Tests 1, 2, and 5 call the two
   validators "shared" but they are separate implementations with identical logic copied
   into two files. The tests do not verify structural sharing; they only verify behavioral
   agreement. If the logic diverges, the name will mislead future maintainers.

**Production code paths exercised**:

| Test | Entry points | Validators invoked |
|------|-------------|-------------------|
| Tests 1, 2, 3, 5 (write) | `memory::write_memory` → `access.rs` | `validate_relative_folder` (`memory/support.rs`) |
| Tests 2, 4, 5, 6 (list) | `memory::list_memories` → `listing.rs` | `validate_relative_folder` (`memory/support.rs`) + post-canonicalize escape check |
| Test 6 (symlink) | `memory::list_memories` → `listing.rs` | `fs::canonicalize` escape guard only (bypasses string-level validator) |

Note: `validate_relative_path` in `util/paths.rs` is **not exercised** by any test in
this file. It is called by `join_validated_relative_path`, used in `memory_uri_to_path`
and write-path construction, but the `folder` parameter to `write_memory` goes through
`validate_relative_folder` in `memory/support.rs` instead.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `fr25_5_ac1_shared_path_validator_absolute_path_rejected_by_memory_and_system_tools` | ✅ High | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Could add formats | **REWRITE** (name + add variants) |
| 2 | `fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools` | ✅ High | ✅ Yes | ✅ Yes | ⚠️ Partial | ✅ Single vector only | **KEEP** (rename) |
| 3 | `fr25_5_ac2_path_traversal_protection_memory_tools_still_allows_valid_relative_folder` | ⚠️ Low | ❌ No | ✅ Yes | ⚠️ Inverted semantics | No | **MERGE** into Test 4 |
| 4 | `fr25_6_ac2_path_traversal_protection_system_tools_still_allows_valid_relative_folder` | ✅ High | ✅ Yes | ❌ Flawed | ⚠️ Partial | No | **REWRITE** (phantom assertion) |
| 5 | `fr25_5_ac3_shared_path_validator_rejects_dot_space_traversal_variants_consistently` | ✅ High | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Already looped | **KEEP** |
| 6 | `fr25_6_ac4_list_memories_rejects_symlink_escaping_memory_path` | ✅ High | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Could add variants | **REWRITE** (strengthen + cfg) |

---

## Recommendations

### Finding 1 — Test 4: Phantom `"Directory traversal detected"` assertion [CRITICAL]

**Severity**: Critical  
**RTM**: RTM-183, NFR-2  
**Location**: Line 194

```rust
// CURRENT — phantom assertion, always passes, protects nothing
assert!(
    !list_result.contains("Directory traversal detected"),
    "unexpected: {list_result}"
);
```

The string `"Directory traversal detected"` does not appear anywhere in the production
codebase (`grep` confirms zero matches). The assertion is vacuously satisfied regardless
of what `list_memories` returns. A complete regression in the path-validation logic
— including the removal of all path checks — would leave this test green.

The *intent* was to assert that the result is a valid directory listing, not an error.
The correct assertion is: the result starts with `"# Directory:"` and does not start with
`"ERROR:"`.

```rust
// REWRITE
assert!(
    list_result.starts_with("# Directory:"),
    "valid path should return directory listing, got: {list_result}"
);
assert!(
    list_result.contains("system-list-target.md"),
    "listing should contain the written file, got: {list_result}"
);
// Phantom line REMOVED
```

---

### Finding 2 — Tests 1–5: Traceability tag mismatch [HIGH]

**Severity**: High  
**RTM**: RTM-182, RTM-183

All tests are tagged `// T-QUAL-002.6h: RTM FR-25.5` or `// T-QUAL-002.6h: RTM FR-25.6`.
In the current RTM, FR-25 covers Azure Deployment Manifests (Dockerfile, Bicep, ACR CI —
RTM-137 to RTM-140). It has no sub-requirements `.5` or `.6`.

The correct RTM entries for path security in tool execution contexts are:
- **RTM-182** (`FR-29`): Symlink paths canonicalized before allowlist evaluation → Test 6
- **RTM-183** (`FR-29`): Path traversal attempts denied in agent file requests → Tests 1, 2, 5

Tests 3 and 4 (positive controls) are most accurately traced to **NFR-2** (test coverage
requirement) since they verify the security layer doesn't break legitimate operations.

**Action**: Update comments on all 6 tests to reference the correct RTM entries.

```rust
// Before
// T-QUAL-002.6h: RTM FR-25.5
#[test]
fn fr25_5_ac1_...

// After
// RTM-183 (FR-29): path traversal rejection — absolute path form
#[test]
fn absolute_path_rejected_by_write_memory_and_list_memories() {
```

---

### Finding 3 — Tests 1, 2: Naming — "memory and system tools" is wrong [MEDIUM]

**Severity**: Medium  
**RTM**: NFR-2

Both `write_memory` and `list_memories` are memory tools. No "system tool" is tested in
either test. The name misleads readers into believing there is a distinct "system" code
path under test.

Additionally, the prefix `fr25_5_` / `fr25_6_` encodes the stale FR tag directly into
the function name, propagating the traceability error into the symbol table.

**Suggested renames**:

| Current | Proposed |
|---------|----------|
| `fr25_5_ac1_shared_path_validator_absolute_path_rejected_by_memory_and_system_tools` | `absolute_path_rejected_by_write_memory_and_list_memories` |
| `fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools` | `dotdot_traversal_rejected_by_write_memory_and_list_memories` |
| `fr25_5_ac2_path_traversal_protection_memory_tools_still_allows_valid_relative_folder` | *(merge into Test 4)* |
| `fr25_6_ac2_path_traversal_protection_system_tools_still_allows_valid_relative_folder` | `valid_nested_relative_folder_accepted_by_write_memory_and_list_memories` |

---

### Finding 4 — Test 3 is a strict subset of Test 4 [MEDIUM]

**Severity**: Medium  
**RTM**: NFR-2

Test 3 writes to `fr25-5-path-validator-valid-memory/nested` and asserts the write
succeeds. Test 4 writes to `fr25-5-path-validator-valid-system/system` and asserts both
the write and the subsequent list succeed.

Test 3's only unique assertion is the URI format in the write response — it checks
`write_result.contains("memory://fr25-5-path-validator-valid-memory/nested/valid-relative-path")`.
This is a useful assertion but it belongs in Test 4, which already validates the full
round-trip. Keeping two separate tests creates two separate temp folders and two separate
write operations for what can be one coherent "valid path round-trip" test.

**Action**: Delete Test 3. Strengthen Test 4 to include the URI-format assertion on the
write response, plus the `!starts_with("ERROR:")` fix from Finding 1.

```rust
// MERGED Test 3 + 4
#[test]
fn valid_nested_relative_folder_accepted_by_write_memory_and_list_memories() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let valid_folder = "security-valid-test/nested";

    let write_result = write_memory_output(
        &env,
        "Valid Relative Path",
        "Valid path operation with [[memorytools]].",
        Some(valid_folder),
    );
    assert!(
        write_result.starts_with("Created memory FILE:"),
        "write should succeed: {write_result}"
    );
    assert!(
        write_result.contains(&format!("memory://{valid_folder}/valid-relative-path")),
        "URI should reflect folder: {write_result}"
    );

    let list_result = list_memories_output(&env, valid_folder);
    assert!(
        list_result.starts_with("# Directory:"),
        "list should return directory heading: {list_result}"
    );
    assert!(
        list_result.contains("valid-relative-path.md"),
        "listing should include the written file: {list_result}"
    );
}
```

---

### Finding 5 — Test 6: Silent skip on Windows; missing `starts_with("ERROR:")` guard [MEDIUM]

**Severity**: Medium  
**RTM**: RTM-182

Two issues:

**A. Silent skip is opaque in CI.** The `if let Err(e) = create_directory_symlink(...) { return; }`
pattern means the test reports `ok` on Windows even though zero assertions ran. On CI
dashboards this looks like a passing test, masking the fact that the symlink code path was
never exercised. The correct approach is `#[cfg(unix)]` on the test itself, combined with
a separate `#[cfg(windows)]` variant if Windows symlink coverage is ever needed.

**B. Missing `starts_with("ERROR:")` assertion.** The test only checks
`list_result.contains("Resolved path escapes memory root")`. If the error format changes
to embed the message inside a success response (e.g., a future refactor that returns a
warning in a valid listing), the containment check would still pass. Adding
`starts_with("ERROR:")` makes the assertion two-sided.

```rust
// REWRITE
#[cfg(unix)]
#[test]
fn list_memories_rejects_symlink_escaping_memory_path() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();
    let test_folder = "symlink-escape-test";
    let test_folder_path = env.cfg.memory_path.join(test_folder);
    let symlink_path = test_folder_path.join("link-outside");
    let outside_tmp = TempDir::new().expect("outside tmp");

    fs::create_dir_all(&test_folder_path).expect("create test folder");
    std::os::unix::fs::symlink(outside_tmp.path(), &symlink_path)
        .expect("symlink creation should succeed in test environment");

    let list_result = list_memories_output(&env, &format!("{test_folder}/link-outside"));

    assert!(
        list_result.starts_with("ERROR:"),
        "symlink escape should produce ERROR response: {list_result}"
    );
    assert!(
        list_result.contains("Resolved path escapes memory root"),
        "error should name the escape condition: {list_result}"
    );
}
```

---

### Finding 6 — Two parallel validators, not one "shared" validator [LOW]

**Severity**: Low / Design Observation  
**RTM**: NFR-3

The test names assert "shared_path_validator" but two functionally-identical functions
exist independently:

- `validate_relative_folder` in `memory/support.rs` (used by both `write_memory` and
  `list_memories`)
- `validate_relative_path` in `util/paths.rs` (used by `join_validated_relative_path`,
  which is used by `memory_uri_to_path` and path join operations)

Both define their own `ABSOLUTE_PATH_ERROR` and `DOT_SEQUENCE_ERROR` constants with
identical strings. Both implement identical URL-decoding + whitespace-compact + all-dots
checks. They are currently in sync, but nothing enforces that sync.

If one constant is updated for a UX improvement (e.g., "Absolute paths are not
permitted") and the other is not, the tests here will detect the drift — but only because
both tools happen to use `validate_relative_folder`. The `validate_relative_path` path
(exercised by URI operations, not folder params) would drift silently.

**Action**: Consolidate to a single `validate_relative_path` in `util/paths.rs`.
`memory/support.rs` should call through. The test names can then accurately claim
"shared validator."

---

### Finding 7 — Test 5: URL-encoded traversal variant not covered [LOW / Informational]

**Severity**: Low  
**RTM**: RTM-183

Test 5 verifies 6 whitespace-obfuscated dot variants. However, the production validator
also decodes URL-encoding (`urlencoding::decode`) before the dot-sequence check. A
`%2e%2e` input would be decoded to `..` and caught — but this is untested.

The risk is low because the URL-decoding is present in both validators and the path
security evaluation for `memory_path_security.rs` also doesn't cover this. However, if
the URL-decoding step is ever removed as "unnecessary overhead," there is no test to catch
the regression.

**Action**: Add one URL-encoded entry to the `traversal_variants` array in Test 5:

```rust
let traversal_variants = [
    ".. ",
    ". .",
    ".. .",
    "safe/.. /escape",
    "safe/. ./escape",
    "safe/.. ./escape",
    "%2e%2e/escape",       // ← ADD: URL-encoded traversal
    "safe/%2e%2e/escape",  // ← ADD: mid-path URL-encoded
];
```

---

## Load-Bearing Score: 5.5/6

| # | Test | Score | Rationale |
|---|------|:-----:|-----------|
| 1 | `fr25_5_ac1` — absolute path rejection | ✅ 1.0 | Unique coverage: absolute path rejection verified for both tools. No other file covers ListMemories here. |
| 2 | `fr25_6_ac1` — canonical dot-sequence rejection | ✅ 1.0 | Canonical `../` vector with strong assertions. Complementary to Test 5 (not redundant). |
| 3 | `fr25_5_ac2` — valid path for WriteMemory | ⚠️ 0.5 | Covered by Test 4 setup. The URI-format assertion is unique but belongs in Test 4. Merge candidate, not standalone value. |
| 4 | `fr25_6_ac2` — valid path for ListMemories | ✅ 1.0 | Unique: only test verifying ListMemories accepts valid relative paths. Phantom assertion is a defect, not a reason to downgrade load-bearing status. |
| 5 | `fr25_5_ac3` — whitespace-obfuscated variants | ✅ 1.0 | Unique, non-trivial, tests both tools. No other file covers these variants for ListMemories. |
| 6 | `fr25_6_ac4` — symlink escape | ✅ 1.0 | Unique: the only test for post-canonicalize escape check in ListMemories. RTM-182 anchor. |

**Total: 5.5 / 6** — The file earns its place in the suite. No test should be deleted
outright. Priority order for action:

1. **CRITICAL**: Fix the phantom assertion in Test 4 (Finding 1)
2. **HIGH**: Correct traceability tags across all 6 tests (Finding 2)
3. **MEDIUM**: Merge Test 3 into Test 4 (Finding 4)
4. **MEDIUM**: Rewrite Test 6 with `#[cfg(unix)]` + `starts_with("ERROR:")` (Finding 5)
5. **MEDIUM**: Rename Tests 1–4 to eliminate the stale `fr25_N_acN` prefixes (Finding 3)
6. **LOW**: Consolidate the two parallel validator implementations (Finding 6)
7. **LOW**: Extend Test 5 with URL-encoded traversal variants (Finding 7)

---

## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- path_security_tests`: 2 passed; 4 failed
- Failures:
  - `fr25_5_ac1_shared_path_validator_absolute_path_rejected_by_memory_and_system_tools` — panicked at line 54: `list_memories` returns `Err(Message("Invalid path - Absolute paths are not allowed. Use relative paths only."))` instead of `Ok("ERROR: ...")`
  - `fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools` — panicked at line 54: `list_memories` returns `Err(Message("Invalid path - Path traversal components (dot sequences) are not allowed."))` instead of `Ok("ERROR: ...")`
  - `fr25_5_ac3_shared_path_validator_rejects_dot_space_traversal_variants_consistently` — panicked at line 54: same `Err` propagation for all 6 dot-space variants
  - `fr25_6_ac4_list_memories_rejects_symlink_escaping_memory_path` — panicked at line 54: `list_memories` returns `Err(Message("Resolved path escapes memory root"))` instead of `Ok("ERROR: ...")`
- Root cause: **API error convention mismatch.** `write_memory` (in `access.rs:87-90`) wraps validation failures as `Ok("ERROR: Invalid folder path - {error}")`. `list_memories` (in `listing.rs:41`) propagates them as `Err(MaenifoldError::Message(...))`. The `list_memories_output` test helper on line 54 calls `.expect("list memories")` which panics on the `Err`. The 4 failing tests all exercise `list_memories` with invalid paths; the 2 passing tests exercise it with valid paths. **The security validation works correctly — the tests just unwrap the Result incorrectly.** The SWE fix should change the `list_memories_output` helper to convert `Err` into an `"ERROR: {msg}"` string (matching the test assertions), or change `list_memories` in production to return `Ok("ERROR: ...")` matching `write_memory`'s convention.
- Verdict: **FAIL**
