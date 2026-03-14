# Test Evaluation: memory_path_security

**File**: `crates/maenifold-core/tests/memory_path_security.rs`
**Evaluator**: Red Team (Gray — Skeptical Inquiry)
**Traceability**: T-0.1 (RTM NFR-3), T-W1-4 (RTM NFR-2), FR-29 (RTM-183)
**Date**: 2025-01-16

---

## Summary

This file contains 4 tests covering path-traversal rejection across three memory tools
(`WriteMemory`, `ReadMemory`, `MoveMemory`) and one public utility function
(`markdown::uri_to_path`). The file was introduced by T-0.1 to split path-security
coverage into a focused integration test file.

**What the tests prove** (when they pass): each traversal input results in *some* error
response. What they do **not** prove: that the error is the *traversal-specific* rejection
rather than an unrelated failure, and in one case the assertion is trivially satisfied by
accident (the test vector itself contains the substring used to check the error).

**Cross-file duplication** is the primary structural risk. `path_security_tests.rs`
already covers `WriteMemory` traversal rejection at tool level with specific error-message
assertions. `memory_boundary_paths.rs` covers `paths::memory_uri_to_path` traversal
rejection at the utility level. The overlap forces future maintainers to update multiple
files for every change to traversal logic.

**Production code paths exercised**:

| Test | Entry point | Path through production code |
|------|-------------|------------------------------|
| `write_memory_rejects_*` | `memory::write_memory` | → `join_validated_relative_path` → `validate_relative_path` (util/paths.rs) |
| `read_memory_rejects_*` | `memory::read_memory` | → `markdown::uri_to_path` → `validate_uri` (markdown/paths.rs) |
| `move_memory_rejects_*` | `memory::move_memory` | → `destination_to_uri` → `validate_uri` → `markdown::uri_to_path` |
| `uri_to_path_directly_rejects_*` | `markdown::uri_to_path` | → `validate_uri` (markdown/paths.rs) — same as test 2 but unit-level |

---

## Verdicts

| # | Test Name | Load-bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `write_memory_rejects_path_traversal_attempts` | ⚠️ Medium | ⚠️ Partial | ⚠️ Weak | ✅ Yes | ✅ (loop) | **REWRITE** |
| 2 | `read_memory_rejects_path_traversal_attempts` | ✅ High | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ (loop) | **REWRITE** |
| 3 | `move_memory_rejects_path_traversal_in_destination` | ✅ High | ✅ Yes | ❌ Flawed | ✅ Yes | ✅ (loop) | **REWRITE** |
| 4 | `uri_to_path_directly_rejects_path_traversal` | ✅ High | ✅ Yes | ⚠️ Partial | ⚠️ Partial | ✅ (loop) | **REWRITE** |

---

## Recommendations

### Finding 1 — Test 1: Weak assertions + significant duplication with `path_security_tests.rs`

**Severity**: Medium  
**RTM**: RTM NFR-2, RTM NFR-3

`path_security_tests.rs` already contains `fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools`,
which tests `WriteMemory` with a traversal folder and asserts the **specific** error message:
`"Path traversal components (dot sequences) are not allowed."`.

Test 1 in this file tests the same tool with 5 traversal vectors but only asserts
`starts_with("ERROR:")`. This is **weaker** than the existing coverage — it cannot
distinguish a traversal rejection from any other error (e.g., a DB failure, a missing
required field, a slugification failure). It would pass green even if the traversal
logic regressed and threw an unrelated error.

```rust
// CURRENT (weak — any error passes)
assert!(result.starts_with("ERROR:"), "...");

// REQUIRED — pin to traversal-specific message
assert!(
    result.contains("dot sequences") || result.contains("Path traversal"),
    "path traversal should be rejected with traversal error for {malicious_folder}: {result}"
);
```

**Action**: Strengthen the assertion to match the specific error message. The extra
vector breadth (5 patterns vs 1) adds value — keep it here and remove the redundant
single-vector test from `path_security_tests.rs::fr25_6_ac1`, or consolidate all
traversal vectors there. Do not maintain two locations for the same code path.

---

### Finding 2 — Test 2: Dual `Ok`/`Err` return shape reveals API instability; `Err` arm accepts "escape" as a control

**Severity**: Medium  
**RTM**: RTM NFR-1, RTM NFR-2

The test branches on `match read_memory_output(...)`:
- `Ok(result)` → asserts `result.starts_with("ERROR:")`
- `Err(err)` → asserts `err.to_string().contains("Path traversal") || contains("path traversal") || contains("Invalid URI") || contains("escape")`

Two problems:

**A. Schizophrenic return shape.** A security-rejection path should have a consistent
return type. Accepting *either* `Ok("ERROR: ...")` *or* `Err(traversal_err)` means the
test cannot prove *which* contract the implementation satisfies. If the implementation
oscillated between the two shapes across a refactor, the test would stay green while the
API contract changed underneath it.

**B. `contains("escape")` is a tautological assertion.** The test input
`"memory://folder/../../../system-file"` does NOT contain "escape", so the `Err` arm
would only fire `contains("escape")` for vectors that happen to include that substring.
If added in future, it would pass because the error message echoes back the input URI:

```
"URI contains path traversal (..) which is not allowed: 'memory://escape-path/../../..'"
                                                                    ^^^^^^
```

The assertion is satisfied by the input content, not by the error semantics.

**Action**: Remove the `Err` arm entirely (or assert `is_err()` as an acceptable
variant), and narrow the `Ok` arm assertion:

```rust
// REWRITE
for malicious_uri in [...] {
    let result = read_memory_output(&env, malicious_uri)
        .expect("read_memory should return Ok(error string) for traversal, not propagate Err");
    assert!(
        result.starts_with("ERROR:") && result.contains("path traversal"),
        "traversal URI should be rejected with traversal message for {malicious_uri}: {result}"
    );
}
```

---

### Finding 3 — Test 3: `contains("escape")` is a false-positive fallback in `move_memory` destination test

**Severity**: High  
**RTM**: RTM NFR-2, FR-29 (RTM-183)

This is the only test for `MoveMemory` destination traversal rejection — it covers a
uniquely important scenario. However, the assertion logic contains a structural flaw:

```rust
Ok(result) => assert!(
    result.starts_with("ERROR:")
        || result.contains("Invalid")   // ← any validation error
        || result.contains("escape"),   // ← satisfied by echoed input content
    "path traversal move should be rejected for {malicious_destination}: {result}"
),
Err(err) => assert!(
    err.to_string().contains("Path traversal")
        || err.to_string().contains("Invalid")
        || err.to_string().contains("escape"), // ← same flaw
    "unexpected error for {malicious_destination}: {err}"
),
```

For the input `"memory-path-tests/../escape-test-folder/file"`:
- `destination_to_uri` transforms it to approximately `"memory://memory-path-tests/../escape-test-folder/file"`
- `validate_uri` rejects with: `"URI contains path traversal (..) which is not allowed: 'memory://memory-path-tests/../escape-test-folder/file'"`
- The error message echoes back the URI, which contains `"escape-test-folder"`
- So `result.contains("escape")` is `true` — but **only because the test vector contains that substring**

This means: if the production code were changed to strip the path from the error
message (a legitimate change for information disclosure reasons), `contains("escape")`
would stop matching, and *both* other arms must then carry the full weight. The assertion
chain is fragile by design.

Additionally, `result.contains("Invalid")` would accept any "Invalid" text — including
"Invalid source" or "Invalid WikiLinks" — masking a false-positive pass if the source URI
validation fires instead of destination traversal validation.

**Action**: Pin to the specific rejection message and remove the ambiguous fallbacks:

```rust
for malicious_destination in [...] {
    let result = move_memory_output(
        &env,
        "memory://memory-path-tests/valid-source",
        malicious_destination,
    )
    .expect("move_memory should return Ok(error string) for traversal");

    assert!(
        result.starts_with("ERROR:") && result.contains("path traversal"),
        "path traversal in destination should be rejected for '{malicious_destination}': {result}"
    );
}
```

---

### Finding 4 — Test 4: Error assertion accepts "escape" and "Invalid URI" as valid traversal signals; name is imprecise

**Severity**: Low  
**RTM**: RTM NFR-1, RTM NFR-2

`uri_to_path_directly_rejects_path_traversal` tests `markdown::uri_to_path` at unit
level — this is the **primary** and **only** unit test for traversal rejection in
`markdown/paths.rs` (the inline `#[cfg(test)]` block in that file covers extension
handling only, not traversal). This is load-bearing and irreplaceable.

Issues:
- `contains("Invalid URI")` would match any validation error from `validate_uri`, not
  just traversal (e.g., a missing `memory://` prefix would also produce "Invalid URI").
  All test inputs do start with `memory://`, so this branch cannot fire for the given
  vectors — but it creates a maintenance trap: a future reader will not know if
  "Invalid URI" is an expected traversal signal or defensive dead code.
- `contains("escape")` — same tautological issue as above; present in the test input
  `"memory://folder/../../../escape"` which would echo back in the error message.
- The name `uri_to_path_directly_rejects_path_traversal` is adequate but not precise.
  Prefer: `markdown_uri_to_path_rejects_dotdot_segment_traversal` to distinguish from
  the util-level `memory_uri_to_path` tests in `memory_boundary_paths.rs`.

**Action**:

```rust
#[test]
fn markdown_uri_to_path_rejects_dotdot_segment_traversal() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    for malicious_uri in [
        "memory://../outside",
        "memory://../../system",
        "memory://folder/../../../escape",
    ] {
        let err = markdown::uri_to_path(malicious_uri, &env.cfg.memory_path)
            .expect_err("uri_to_path should reject dotdot traversal");
        assert!(
            err.to_string().to_lowercase().contains("path traversal"),
            "expected traversal rejection for {malicious_uri}, got: {err}"
        );
    }
}
```

---

### Finding 5 — No positive (valid-path) control in the security file

**Severity**: Informational  
**RTM**: RTM NFR-2

All 4 tests are purely negative (asserting rejection). None verify that a *legitimate*
path through the same code succeeds. Without a positive control, a catastrophic
over-restriction (e.g., `validate_relative_path` always returns `Err`) would leave all 4
tests green while breaking every real user operation.

`path_security_tests.rs` does include `fr25_5_ac2` and `fr25_5_ac3` as positive
controls. But within *this* file — which declares itself focused on security — the
absence of even one "valid path does not get rejected" assertion is a blind spot.

**Action**: Add one positive-path test per tool exercised:

```rust
#[test]
fn memory_security_tools_accept_valid_paths() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let result = write_memory_output(&env, "Valid", "Content [[concept]].", Some(TEST_FOLDER));
    assert!(result.starts_with("Created memory FILE:"), "{result}");

    let write_uri = format!("memory://{TEST_FOLDER}/valid");
    let read_result = read_memory_output(&env, &write_uri).expect("valid read should succeed");
    assert!(!read_result.starts_with("ERROR:"), "{read_result}");
}
```

---

### Finding 6 — `memory_path_security.rs` and `path_security_tests.rs` use incompatible harnesses for the same tool

**Severity**: Informational  
**RTM**: RTM NFR-3

`path_security_tests.rs` defines its own private `isolated_env()`, `write_memory_output()`, and `TEST_MUTEX` — duplicated verbatim from the shared `memory_path_support/mod.rs`. This is tolerable for now but will drift. Consider migrating `path_security_tests.rs` to consume the shared harness, reducing duplication across both files.

---

## Load-Bearing Score: 3/4

| Test | Score | Rationale |
|------|-------|-----------|
| `write_memory_rejects_path_traversal_attempts` | ⚠️ 0.5 | Overlaps `path_security_tests.rs::fr25_6_ac1`; same code path; weaker assertions |
| `read_memory_rejects_path_traversal_attempts` | ✅ 1.0 | Unique tool-level coverage of `read_memory`; no other file tests this |
| `move_memory_rejects_path_traversal_in_destination` | ✅ 1.0 | Unique; only test for `move_memory` destination traversal |
| `uri_to_path_directly_rejects_path_traversal` | ✅ 0.5 | Primary unit test for `markdown::uri_to_path` traversal — irreplaceable, but flawed assertions reduce confidence |

**Total: 3.0 / 4** — the file earns its place in the suite; none of the 4 tests should
be deleted outright. All 4 need assertion hardening. Test 1 needs consolidation with
`path_security_tests.rs` to eliminate the weaker duplicate vector.

**Priority order for rewrites**: Finding 3 (Test 3) > Finding 2 (Test 2) > Finding 1
(Test 1) > Finding 4 (Test 4) > Finding 5 (positive control).

---
## Blue-team Verification
- `cargo test --test integration_tests -p maenifold-core -- memory_path_security`: 3 passed, 1 failed
- Failures:
  - `memory_path_security::move_memory_rejects_path_traversal_in_destination` — panicked at line 96: the `Err` arm assertion checks `contains("Path traversal")` (capital P) but production error message uses lowercase `"path traversal"`. For input `"../outside-memory"`, neither `contains("Invalid")` nor `contains("escape")` match either, so all three disjuncts fail. This confirms red-team Finding 3 (High severity): the assertion is structurally flawed by case-sensitivity mismatch and the `contains("escape")` fallback only accidentally passes for vectors whose input text contains that substring.
- Verdict: **FAIL**
