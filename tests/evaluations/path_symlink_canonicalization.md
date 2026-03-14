# Test Evaluation: path_symlink_canonicalization

**File**: `crates/maenifold-core/tests/path_symlink_canonicalization.rs`
**Evaluator**: Red Team (Gray — Skeptical Inquiry)
**Color**: 🔘 Gray — *How do we know this is true? What are we assuming?*
**Traceability**: T-14.5 / FR-12 / RTM-182 / RTM-183
**Test count**: 4

---

## Summary

This file was introduced under T-14.5 to verify that `memory_uri_to_path` and
`is_watchable_asset_json_path` reject symlink-based escapes from their configured
root directories. The premise is sound — symlink canonicalization is a distinct
security guard from the pre-canonicalization dot-sequence check, and deserves its
own test file.

The premise, however, is not fully delivered upon.

**What the file actually contains**: 4 tests covering 2 unique scenarios
(symlink escape rejection) and 2 happy-path / dot-sequence cases. The happy-path
test and the dot-sequence test are **near-identical duplicates** of tests already
present in `memory_boundary_paths.rs` with *stronger* assertions. They add no
new coverage and weaken the signal: a reader of this file is left wondering
whether these tests exist because they cover something different, or because
someone was unsure whether the existing coverage was enough.

The two symlink tests are **genuinely load-bearing and unique**. No other test
file exercises the post-canonicalization `starts_with` boundary guard via a real
filesystem symlink. These are the only tests that would catch a regression in
`canonicalize_existing_or_parent` or its caller `join_validated_path_buf`. They
should be kept and strengthened.

**Silent-skip pattern**: Both symlink tests use an inline soft-skip (`if Err →
eprintln + return`) when the OS rejects `symlink()`. This is acceptable for CI
environments without symlink privileges, but the tests then pass vacuously — the
passing result tells us nothing about the production guard. This risk is documented
below.

**Production code exercised**:

| Test | Entry point | Guard exercised |
|------|-------------|-----------------|
| `path_validation_accepts_normal_relative_memory_uri` | `paths::memory_uri_to_path` | Happy path only — no guard |
| `path_validation_rejects_dot_dot_components` | `paths::memory_uri_to_path` → `validate_relative_path` | Pre-canonicalization dot-sequence check |
| `path_validation_rejects_symlink_escape_for_nonexistent_leaf` | `paths::memory_uri_to_path` → `join_validated_path_buf` | **Post-canonicalization `starts_with` boundary guard** |
| `path_validation_rejects_symlinked_asset_json_outside_assets_root` | `assets::is_watchable_asset_json_path` → `join_validated_path_buf` | **Post-canonicalization `starts_with` boundary guard** |

---

## Verdicts

| # | Test Name | Load-bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `path_validation_accepts_normal_relative_memory_uri` | ⚠️ Low | ❌ No | ✅ Yes | ⚠️ Partial | ✅ Yes | **REMOVE** |
| 2 | `path_validation_rejects_dot_dot_components` | ⚠️ Low | ❌ No | ⚠️ Weak | ✅ Yes | ✅ Yes | **REMOVE** |
| 3 | `path_validation_rejects_symlink_escape_for_nonexistent_leaf` | ✅ High | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes | **REWRITE** |
| 4 | `path_validation_rejects_symlinked_asset_json_outside_assets_root` | ✅ High | ✅ Yes | ⚠️ Weak | ✅ Yes | ✅ Yes | **REWRITE** |

---

## Recommendations

### Finding 1 — Test 1: Redundant happy-path duplicate

**Severity**: Low (noise)
**RTM**: T-14.5 / RTM-182

`memory_boundary_paths.rs` already contains two tests that fully cover the happy
path for `memory_uri_to_path`:

- `memory_uri_to_path_resolves_nested_uri_under_memory_root` — resolves
  `memory://nested/deep/read-note` to a full path equality assertion (deeper than
  test 1's `notes/canonicalization-check`)
- `memory_uri_to_path_adds_md_extension_for_extensionless_uris` — checks `.md`
  extension appending

Test 1 in this file (`memory://notes/canonicalization-check`) sits between these
two: less deep than the first, less focused than the second. It tests nothing new.

**Action**: Remove. If the `notes/canonicalization-check` path has symbolic
significance as a regression baseline, document that in `memory_boundary_paths.rs`.

---

### Finding 2 — Test 2: Redundant dot-sequence test with *weaker* assertion than existing coverage

**Severity**: Medium (assertion quality regression risk)
**RTM**: T-14.5 / RTM-183

The dot-sequence traversal pattern `"memory://notes/../escape"` is functionally
equivalent to `"memory://safe/../escape"` tested in
`memory_boundary_paths.rs::memory_uri_to_path_rejects_traversal_uri`. The only
difference is the prefix (`notes` vs `safe`), which exercises no different code
path.

More critically: `path_security_tests.rs::fr25_6_ac1_shared_path_validator_dot_sequence_rejected_by_memory_and_system_tools`
pins the **full** error message:
```
"Path traversal components (dot sequences) are not allowed."
```

Test 2 here only checks:
```rust
err.to_string().contains("Path traversal")
```

This weaker form would pass green even if the error message was changed to any
string that happens to contain "Path traversal" — including an unrelated I/O error
or panic message. It provides less regression protection than the test it duplicates.

**Action**: Remove. The existing 5-vector loop in
`memory_boundary_paths.rs::validate_relative_path_rejects_dot_sequence_traversal`
already covers this pattern with a proper assertion, and
`path_security_tests.rs::fr25_6_ac1` pins the exact error string.

---

### Finding 3 — Test 3: Silent-skip vacuously passes; assertion is under-specified

**Severity**: Medium (test validity)
**RTM**: T-14.5 / RTM-182 (symlink canonicalization)

**Silent-skip pattern**: If `create_directory_symlink` fails (e.g., CI environment
without symlink privileges on a NTFS filesystem or in a restricted sandbox), the
test prints to stderr and returns `Ok`. The test runner reports `PASSED`. An
operator reading the CI results cannot distinguish between "symlink escape is
correctly rejected" and "symlink creation failed so we checked nothing". This is
a confidence illusion.

```rust
// CURRENT: vacuous pass on skip
if let Err(error) = create_directory_symlink(outside_tmp.path(), &symlink_path) {
    eprintln!("Skipping symlink test: {error}");
    Config::reset_overrides();
    return;
}
```

**Recommendation**: Either mark the test `#[cfg(unix)]` and remove the soft-skip
entirely (the `create_directory_symlink` helper is already `#[cfg(unix)]`-gated),
or use `#[ignore]` with a documented reason and require explicit opt-in. At minimum,
the skip should be surfaced as a test warning, not a silent pass.

**Assertion under-specification**: The assertion:
```rust
assert!(err.to_string().contains("escapes base directory"), ...)
```
correctly maps to `MaenifoldError::PathTraversal("Resolved path escapes base
directory: ...")`. However, it does not verify the error *variant* — a different
`MaenifoldError` variant that happened to contain the substring in its `Display`
impl would also satisfy this check. Consider asserting on the variant:
```rust
assert!(matches!(err, MaenifoldError::PathTraversal(_)), "wrong error type: {err}");
assert!(err.to_string().contains("escapes base directory"), "wrong message: {err}");
```

**Parameterization gap**: Only tests a non-existent leaf (`new-note`). The
*existent* symlink target case (where `outside-link` itself is the file) is not
covered — this would exercise a different code path in `canonicalize_existing_or_parent`
(the fast path `fs::canonicalize(path)` succeeds immediately).

**Rewritten test structure**:
```rust
#[cfg(unix)]
#[test]
fn symlink_escape_nonexistent_leaf_is_rejected() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let outside_tmp = TempDir::new().expect("outside tmp");
    let symlink_path = cfg.memory_path.join("outside-link");

    std::os::unix::fs::symlink(outside_tmp.path(), &symlink_path)
        .expect("symlink creation must succeed on unix");

    let err = paths::memory_uri_to_path("memory://outside-link/new-note", &cfg)
        .expect_err("symlink escape should be rejected");

    assert!(
        matches!(err, maenifold_core::error::MaenifoldError::PathTraversal(_)),
        "expected PathTraversal error, got: {err}"
    );
    assert!(
        err.to_string().contains("escapes base directory"),
        "unexpected error: {err}"
    );
    Config::reset_overrides();
}
```

---

### Finding 4 — Test 4: Compound boolean assertion hides which guard fires

**Severity**: Medium (assertion opacity)
**RTM**: T-14.5 / RTM-182

`is_watchable_asset_json_path` has four chained `&&` conditions:

```rust
join_validated_path_buf(...).is_ok()           // (A) symlink escape check
    && path.extension()...eq_ignore_ascii_case("json")   // (B) extension check
    && watched_asset_kind_for_path(...).is_some()         // (C) kind check
    && !is_hidden_asset_relative_path(...)                 // (D) hidden check
```

Test 4 creates a symlink whose resolved target escapes `assets_root`. This should
cause condition (A) to fail. **But "roles" is a valid `AssetKind::Role`**, so (C)
would pass if (A) were removed. The test's `assert!(!...)` would therefore pass
even if guard (A) was silently deleted — as long as (B), (C), or (D) returns false
for independent reasons.

The test does NOT isolate condition (A). A separate positive test asserting that
an *in-bounds* symlink (or a real file) IS watchable would close this gap by
confirming (A) can succeed when the symlink is within the root.

**Also**: same silent-skip pattern as Test 3.

**Rewritten test structure**:
```rust
#[cfg(unix)]
#[test]
fn symlinked_asset_json_outside_assets_root_is_not_watchable() {
    let _guard = TEST_MUTEX.lock();
    let (_tmp, cfg) = isolated_config();
    let role_dir = cfg.assets_path.join("roles");
    let outside_tmp = TempDir::new().expect("outside tmp");
    let outside_file = outside_tmp.path().join("outside.json");
    let symlink_path = role_dir.join("linked.json");

    fs::create_dir_all(&role_dir).expect("create role dir");
    fs::write(&outside_file, r#"{"id":"outside"}"#).expect("write outside json");

    std::os::unix::fs::symlink(&outside_file, &symlink_path)
        .expect("symlink creation must succeed on unix");

    // Primary assertion: symlink escaping assets root → not watchable
    assert!(
        !assets::is_watchable_asset_json_path(&cfg.assets_path, &symlink_path),
        "symlinked asset outside root should not be watchable"
    );

    // Isolation check: the same filename as a real in-bounds file IS watchable
    fs::remove_file(&symlink_path).expect("remove symlink");
    fs::write(&symlink_path, r#"{"id":"inside"}"#).expect("write real in-bounds file");
    assert!(
        assets::is_watchable_asset_json_path(&cfg.assets_path, &symlink_path),
        "real in-bounds asset json should be watchable"
    );

    Config::reset_overrides();
}
```

---

### Finding 5 — Cross-cutting: TEST_MUTEX provides isolation but Config::reset_overrides is fragile

**Severity**: Low
**RTM**: NFR-3

All 4 tests call `Config::reset_overrides()` as cleanup, but only on the normal
code path — not in the `?` error propagation case (Rust tests that `panic!` mid-test
would skip the cleanup). Since the mutex is acquired at test start, a panicking test
will poison the mutex and block subsequent tests. The `isolated_config()` +
`TEST_MUTEX` pattern is sound; the cleanup placement is not. A `scopeguard::defer`
or `Drop` wrapper would be more robust:

```rust
let _cleanup = scopeguard::defer(|| Config::reset_overrides());
```

---

## Load-Bearing Score: 2/4

Tests 3 and 4 are the only load-bearing tests — they are the **sole coverage** for
the post-canonicalization symlink-escape guard in `join_validated_path_buf`. Without
them, a regression that removed the `starts_with` check or broke
`canonicalize_existing_or_parent` would go undetected.

Tests 1 and 2 are not load-bearing: stronger duplicates exist in
`memory_boundary_paths.rs` and `path_security_tests.rs`. Removing them shrinks
the file without reducing coverage.

The practical **security coverage score** for RTM-182 is therefore:

| Guard | Covered? | By which test in this file? |
|-------|:--------:|-----------------------------|
| Pre-canonicalization dot-sequence rejection | ✅ (elsewhere) | Test 2 (weaker duplicate) |
| Post-canonicalization symlink escape — memory root | ✅ | Test 3 |
| Post-canonicalization symlink escape — assets root | ✅ | Test 4 |
| Symlink escape — existent leaf (fast-path canonicalize) | ❌ | Not covered anywhere |
| Symlink chain (symlink → symlink → outside) | ❌ | Not covered anywhere |
| Silent-skip vacuous pass risk | ⚠️ | Tests 3 + 4 on non-Unix |
