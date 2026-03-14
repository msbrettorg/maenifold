# Test Evaluation: dynamic_model_discovery_paths
**File:** `crates/maenifold-core/tests/dynamic_model_discovery_paths.rs`
**Test count:** 2
**Evaluated:** 2025-07-21
**Traceability:** T-20.5 · RTM-147 · RTM-148 · FR-7 · NFR-2

---

## Summary

This file covers a single functional slice: **runtime model discovery path resolution** — specifically the two edges of the `find_model_path()` priority chain that are hardest to exercise in normal test runs. Both tests map cleanly to discrete RTM entries (RTM-147 and RTM-148) and both tests are the *only* coverage for their respective invariants in the entire suite. Neither is duplicated elsewhere.

The implementation under test (`vector/discovery.rs :: find_model_path()`) has a three-step probe chain:

1. `{canonicalized_exe}/assets/models` — executable-adjacent (the symlink-canonicalization path)
2. Walk up ancestors for `Cargo.toml` + adjacent `assets/models` — dev-mode / monorepo path
3. `Config::current().assets_path/models` — shared runtime root (C# compatibility fallback)

**Test 1** covers step 1 with a symlink exercise. **Test 2** covers step 3 by surgically hiding steps 1 and 2. The support infrastructure in `dynamic_model/support.rs` is sophisticated and correct — `TestContext::Drop` properly restores hidden models and cleans up shadow assets, and the `TEST_MUTEX` correctly serializes against global `vector::MODEL_STATE`.

**One serious defect found:** Test 1 silently returns a passing result on Windows environments that lack symlink privilege. The test emits `eprintln!` and returns, meaning CI passes with zero assertions tested. This is a false green: the invariant protected by RTM-147 is completely unverified in those environments.

**One naming concern:** Both test names carry the `t20_5_` sprint-task prefix. This is consistent with sibling files (`dynamic_model_loading.rs`, `dynamic_model_tokenizer.rs`), so the pattern is deliberate, but it degrades readability and embeds a now-completed-sprint reference into the test identity. The RTM comment block directly above each `#[test]` already provides full traceability, making the prefix redundant.

---

## Verdicts

| # | Test Function | Load-Bearing? | Unique? | Assertions Correct? | Name Good? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path` | ✅ Yes (RTM-147) | ✅ Yes | ⚠️ Partially — vacuously passes on symlink-creation failure | ⚠️ `t20_5_` prefix is noise | ❌ No | **REWRITE** |
| 2 | `t20_5_model_discovery_falls_back_to_shared_runtime_root` | ✅ Yes (RTM-148) | ✅ Yes | ✅ Yes — filename + dimensions both asserted | ⚠️ `t20_5_` prefix is noise | ❌ No | **KEEP** |

---

## Detailed Findings

### Test 1: `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path`

**What it proves:** When the test binary is invoked through a filesystem symlink, `find_model_path()` canonicalizes `env::current_exe()` before deriving the adjacent assets directory, and therefore still resolves `symlinked-shadow.onnx` from the *real* binary's location rather than from the symlink directory.

**Pattern used:** Self-spawning subprocess with env-var guard. The function checks `MAENIFOLD_T20_5_SYMLINK_HELPER` at entry: if set, it runs as the "helper" (loads and asserts). If absent, it acts as "orchestrator" (sets up fixtures, creates symlink, spawns itself via the symlink path, asserts subprocess exit code). This is a recognized technique for testing executable-path-dependent behavior and it is correctly implemented here.

**Defect — silent skip on symlink failure (High):**

```rust
if let Err(error) = create_file_symlink(&test_binary, &symlink_path) {
    eprintln!("Skipping symlink discovery test: {error}");
    return;  // ← test PASSES with zero assertions
}
```

On any Windows environment without `SeCreateSymbolicLinkPrivilege` (default for non-admin accounts), this branch executes and the test returns `Ok(())`. The test runner records a pass. RTM-147 is unverified. A future regression in `find_model_path()` — for example, removing `canonicalize()` — would not be caught in that environment.

The correct fix depends on the target support matrix:
- **If symlink tests are Unix-only:** Add `#[cfg(unix)]` to the test function. Windows will skip at compile time rather than silently at runtime. The function-scoped `create_file_symlink` dispatch becomes redundant and can be removed.
- **If Windows symlink support is desired but optional:** Replace the `eprintln!` + `return` with `eprintln!(...); return;` wrapped in a macro that marks the test result as explicitly skipped rather than passed. Rust's `#[ignore]` attribute is compile-time only, but at minimum the log output should be unambiguous ("SKIPPED — symlink creation failed").

**Correctness of assertions (helper branch):**

```rust
assert_eq!(vector::model_filename().as_deref(), Some("symlinked-shadow.onnx"));
assert_eq!(vector::embedding_dimensions(), Some(384));
```

Both assertions are meaningful. `model_filename()` directly confirms the discovery resolved to the expected model by name (proving the exe-adjacent probe fired against the real binary's location, not the symlink's). `embedding_dimensions()` confirms the model actually loaded from that path. Minor: `Some(384)` hardcodes the embedding dimension of the packaged all-MiniLM-L6-v2 model. This will fail if a different model is used as the shadow fixture in the future. A more robust alternative would be `vector::embedding_dimensions().is_some()` plus a bounds check (`> 0`), since the test is about *discovery path* correctness, not model architecture verification.

**OS-specific behavior note (informational):**

On macOS, `env::current_exe()` typically resolves symlinks and returns the real path directly — which means the `canonicalize()` call in `find_model_path()` may be a no-op on macOS even when invoked via symlink. On Linux, `current_exe()` reads `/proc/self/exe` which follows symlinks, producing the real path. In both cases the `canonicalize()` call is a defensive guard that costs nothing. Whether the test actually exercises a *different code path* than a non-symlinked invocation is OS-dependent. The test is most meaningful as a regression guard on Linux-based CI.

---

### Test 2: `t20_5_model_discovery_falls_back_to_shared_runtime_root`

**What it proves:** When the exe-adjacent probe (step 1) and the Cargo walk-up probe (step 2) both fail to find a usable `.onnx` file, `find_model_path()` falls back to `Config::current().assets_path/models` and successfully loads the model found there.

**Fixture construction:** Well-engineered.
- `TestContext::new()` creates `shadow_assets_root/models/` but installs *no* model there — so `discover_model()` at step 1 reads an empty directory and returns `None`. ✓
- `hide_packaged_model()` renames `all-MiniLM-L6-v2.onnx` → `all-MiniLM-L6-v2.onnx.dynamic-model-tests`. The `discover_model()` filter requires `.extension().eq_ignore_ascii_case("onnx")` — `.dynamic-model-tests` fails this check, so the Cargo walk-up at step 2 returns `None`. ✓
- `install_valid_shared_model_with_fallback_assets("shared-runtime.onnx")` places the model under `cfg.assets_path/models/` — precisely where step 3 probes. ✓

**Assertions:**

```rust
assert_eq!(vector::model_filename().as_deref(), Some("shared-runtime.onnx"));
assert_eq!(vector::embedding_dimensions(), Some(384));
```

Both are appropriate and sufficient. The filename assertion confirms the fallback path was used (not some earlier step). The dimensions assertion confirms the model is functional. Same minor note as test 1: `Some(384)` is path-discovery-irrelevant; a `is_some()` check would be less brittle, but this is low severity.

**Missing negative assertion:** The test does not explicitly assert that steps 1 and 2 returned `None` — it infers this from the final outcome. This is acceptable given the careful fixture construction, but it does mean a future change to fixture setup that accidentally populates the shadow model dir would cause the test to pass while no longer testing the fallback. A comment explicitly noting the fixture invariants (shadow dir is empty, packaged model is hidden) would strengthen maintainability.

**Structural cleanliness:** This test is a model (no pun intended) of how to write a single-purpose, correctly structured integration test in this codebase. `TEST_MUTEX` → `TestContext::new()` → install fixture → exercise → assert → automatic cleanup via `Drop`. No leakage, no dual-role, no conditional assertions.

---

## Recommendations

1. **[High — Test 1] Fix the silent skip.**
   - Preferred: Add `#[cfg(unix)]` to `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path`. Remove the `cfg(unix)` / `cfg(windows)` dispatch in `create_file_symlink` — it becomes dead code. The test then simply does not compile on Windows, which is honest about what's being tested.
   - Alternative if Windows symlink coverage is needed: Add a Windows Developer Mode prerequisite check and `return` with a distinctive sentinel that the test harness marks as skipped, not passed.

2. **[Medium — Test 1] Decouple the dimension assertion from the path-discovery concern.**
   Replace `assert_eq!(vector::embedding_dimensions(), Some(384))` with `assert!(vector::embedding_dimensions().map_or(false, |d| d > 0), "model loaded with valid dimensions")`. The test is about *where* the model was found, not *what* its architecture is. Hardcoding `384` creates a brittle dependency on the shadow fixture being the MiniLM model.

3. **[Low — both] Drop the `t20_5_` prefix from test names.**
   Traceability is already provided by the `// T-20.5: RTM FR-7 / RTM-147` comment immediately above each `#[test]`. The prefix in the function name adds nothing and will become confusing once Sprint 20 is long past. Rename to:
   - `symlinked_model_discovery_uses_canonicalized_executable_path`
   - `model_discovery_falls_back_to_shared_runtime_root`

4. **[Low — Test 2] Add an inline invariant comment to the test body.**
   Before the `hide_packaged_model()` call, add a short comment block:
   ```rust
   // Step 1 probe: shadow_models_dir exists but contains no .onnx — returns None
   // Step 2 probe: packaged model hidden (renamed to .dynamic-model-tests) — returns None
   // Step 3 probe: shared-runtime.onnx installed at assets_path/models — returns Some
   ```
   This makes the three-probe chain explicit to a future reader who hasn't memorized `find_model_path()`.

5. **[Informational] Consider adding a negative-path test for the exhausted chain.**
   There is currently no test that exercises the `Err(MaenifoldError::Message("embedding model not found..."))` branch — the case where all three probes return `None`. This would require hiding the packaged model *and* not installing anything in the shared runtime root. Such a test would guard the error message text and confirm `load_model()` propagates the error correctly. Low priority but would complete RTM coverage for `find_model_path()`.

---

## Load-Bearing Score: 2/2

Both tests protect distinct, non-overlapping invariants in the model discovery chain. Neither is duplicated elsewhere in the 500+ test suite. Removing either would leave the corresponding RTM requirement (RTM-147 or RTM-148) without any test coverage. However, Test 1's effective assurance value is currently **degraded** by the silent-skip defect: on Windows CI it contributes nothing, and on macOS the `canonicalize()` call it targets may be a no-op. Applying Recommendation 1 restores full assurance on Unix and makes the Windows gap explicit rather than hidden.
