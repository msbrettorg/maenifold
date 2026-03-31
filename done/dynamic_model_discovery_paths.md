---
id: dynamic_model_discovery_paths
status: done
pipeline: tdd
---

# Test Evaluation: dynamic_model_discovery_paths

**File**: `crates/maenifold-core/tests/dynamic_model_discovery_paths.rs`
**Evaluated by**: Red Team — 🔘 Gray (Skeptical Inquiry)
**Traceability**: T-20.5 · FR-7 · RTM-147 · RTM-148
**Production code under test**: `vector::load_model()`, `vector::model_filename()`,
`vector::embedding_dimensions()`, `vector::cleanup()` via `find_model_path()` in
`crates/maenifold-core/src/vector/discovery.rs`

---

## Summary

This file contains **two integration tests** targeting the two non-standard discovery paths
introduced in T-20.5. Both share the `TEST_MUTEX` from `dynamic_model/support.rs` to serialize
against the singleton `model_state()` global and delegate setup/teardown to the `TestContext`
RAII guard.

For context, `find_model_path()` in `discovery.rs` searches three locations in order:

1. `{canonicalized_exe_dir}/assets/models` — exe-adjacent path (requires `current_exe().canonicalize()`)
2. Cargo.toml ancestor walk-up → `ancestor/assets/models` — dev-build fallback
3. `Config::current().assets_path/models` — shared runtime root (C# compatibility path)

`dynamic_model_loading.rs` covers path 1 (non-symlink), path 2 (implicitly, as fallback in
tests that don't install anything), and the error case. This file covers the two specialized
sub-scenarios: the **symlinked binary variant of path 1** (RTM-147) and the
**explicit path 3 fallback** (RTM-148).

The file split is architecturally sound. Both tests require setup that is incompatible with the
simpler tests in `dynamic_model_loading.rs`: one spawns a subprocess, the other installs into
a separate root from the shadow dir.

**Critical observation**: Path 2 (Cargo.toml walk-up) has no dedicated test anywhere in the
suite. In `dynamic_model_loading.rs`, path 1 succeeds before the walk-up is ever reached. In
this file, path 2 is explicitly disabled via `hide_packaged_model()`. This is a systemic gap
outside the scope of this file but worth flagging at the suite level.

**Critical observation (test 2)**: The test correctly relies on the unique model filename
(`"shared-runtime.onnx"`) as implicit evidence that path 3 was used — a different model found
via a higher-priority path would produce a different filename, failing the assertion. However,
this implicit protection is fragile: it depends on no ONNX file accidentally existing in
walk-up ancestors at test time. A regression in test isolation (e.g., leftover artifacts from
a prior test run) could make test 2 pass while exercising path 2 rather than path 3.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path` | ✅ Yes | ✅ Yes | ⚠️ Fragile | ✅ Yes | ⬛ No | **KEEP** |
| 2 | `t20_5_model_discovery_falls_back_to_shared_runtime_root` | ✅ Yes | ✅ Yes | ⚠️ Fragile | ✅ Yes | ⬛ No | **KEEP** |

---

## Per-Test Analysis

### Test 1 — `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path`

**RTM coverage claimed**: RTM-147 — a symlink-invoked binary must still resolve runtime models
from the *real* executable's adjacent assets directory.

**What it tests**: The critical line in `discovery.rs` is:
```rust
let executable = env::current_exe()?.canonicalize()?;
```
Without `canonicalize()`, a symlinked invocation would resolve `base_dir` to the symlink's
parent directory — a temporary bin dir where no assets exist. This test verifies that the
canonicalization step causes path 1 to succeed even when the binary is invoked through a symlink.

The test uses a **self-referential subprocess strategy**:
- **Driver mode** (normal test execution): installs the shadow model, hides the packaged model,
  creates a filesystem symlink pointing at the test binary, spawns the subprocess via the symlink
  with a sentinel env var set, then asserts subprocess exit status success.
- **Helper mode** (subprocess, detected by env var): calls `vector::cleanup()`,
  `vector::load_model()`, asserts `model_filename` and `embedding_dimensions`, then returns.

**Load-bearing?** Yes. This is the **only test** exercising the `canonicalize()` call in
`find_model_path()`. If that line were removed, `current_exe()` in a symlinked context would
return the symlink path, `base_dir` would resolve to a throwaway directory, and path 1 would
miss the shadow model. No other test would catch this regression. RTM-147 has zero redundant
coverage elsewhere.

**Unique?** Yes. The subprocess-via-symlink mechanism is unique in the entire test suite. The
SYMLINK_HELPER_ENV sentinel prevents recursive re-entry if the subprocess somehow spawns another
test runner. The `--exact` flag ensures only this one test function runs in the subprocess,
preventing other tests from executing (and potentially deadlocking on the test mutex or
triggering their own subprocess chains).

**Correct assertions?**
In helper mode:
- `vector::load_model().expect("load model through symlink helper")` — ✅ panics with diagnostic
  message if discovery fails; turned into subprocess failure captured by driver
- `assert_eq!(vector::model_filename().as_deref(), Some("symlinked-shadow.onnx"))` — ✅ confirms
  the model was found at the shadow path (uniquely named to this test), not some other ONNX file
- `assert_eq!(vector::embedding_dimensions(), Some(384))` — ⚠️ hardcoded 384. Same fragility
  documented in `dynamic_model_loading.md` for test 1. Correct today; breaks silently if the
  packaged model is replaced with a different-dimension model. Should be derived from
  `model_output_dimensions()` following test 2 in that file.

In driver mode:
- `assert!(output.status.success(), ...)` — ✅ with clear diagnostic message including
  stdout/stderr. This is the appropriate assertion for subprocess validation.

**Good name?** Yes. The T-20.5 prefix provides RTM traceability. "canonicalized_executable_path"
precisely names the mechanism under test. Verbose but justified for a non-obvious behavior.

**Parameterizable?** No. This test covers a single binary execution scenario. Parameterizing the
model name or symlink location depth would test the filesystem infrastructure, not the discovery
logic. Not worth it.

**Skeptical probes:**

1. **Silent skip on symlink creation failure** — If `create_file_symlink()` fails (e.g.,
   Windows non-admin, VFAT mount, Docker with certain security options), the test silently passes
   with only an `eprintln!`. This means CI green does NOT guarantee RTM-147 is satisfied on
   those platforms. The skip is pragmatic but should be surfaced more visibly (a `#[ignore]`
   or explicit skip record), or the CI matrix should assert that symlink tests must not skip on
   Unix.

2. **Subprocess inherits driver's filesystem mutations** — The subprocess runs after
   `hide_packaged_model()` renames the real ONNX file. This is intentional — the subprocess
   must NOT find the packaged model via the walk-up, forcing it to use the shadow model at path 1.
   This coupling is correct but invisible: the subprocess's behavior depends on a filesystem
   side-effect from the driver. If the subprocess ran before `hide_packaged_model()`, it would
   find the packaged model via the Cargo walk-up at path 2 and the `model_filename` assertion
   would fail with `"all-MiniLM-L6-v2.onnx"` vs `"symlinked-shadow.onnx"`. Correct ordering is
   assumed and enforced only by the sequential nature of the driver code.

3. **TEST_MUTEX guards the driver, not the subprocess** — The subprocess is a separate process;
   it does not hold or contend for `TEST_MUTEX`. Other tests in the same binary are blocked by
   the mutex while the driver runs (and while the subprocess runs, since the driver hasn't
   released the mutex). This is the correct behavior — no race condition exists. However, the
   subprocess does NOT acquire the mutex for itself, which means if any other binary runs
   in parallel (e.g., `cargo test --test another_test_binary`), the filesystem mutation
   (`hide_packaged_model()`) is unprotected across binary boundaries. This is an inherent
   limitation of process-local mutexes for filesystem mutations.

4. **`--nocapture` in subprocess args** — This causes the subprocess to emit stdout/stderr
   directly. This output is captured by `Command::output()` and is included in the assertion
   message on failure. The behavior is correct: in silent CI, the assertion message carries the
   diagnostic context. No issue.

5. **Hardcoded assertion value `Some(384)` in helper mode** — The subprocess is compiled into
   the same binary as the driver, so the assertion value is a compile-time constant. If the
   ONNX model's actual output changes, this assertion fails at the subprocess level, the
   subprocess exits non-zero, and the driver fails with the subprocess output. The failure is
   surfaced, just with one extra level of indirection. The hardcode is fragile but not dangerously
   silent.

**Verdict: KEEP** — Load-bearing, unique, and correctly structured for a genuinely complex
multi-process scenario. The `384` hardcode should be noted as a maintenance concern. The silent
skip behavior should be documented.

---

### Test 2 — `t20_5_model_discovery_falls_back_to_shared_runtime_root`

**RTM coverage claimed**: RTM-148 — when neither exe-adjacent nor Cargo walk-up probes find a
model, discovery must fall back to `Config::current().assets_path/models`.

**What it tests**: Installs a valid model **only** at `cfg.assets_path.join("models")` (the
isolated tmp path), hides the packaged model to block the walk-up probe, then calls
`load_model()` and asserts that `"shared-runtime.onnx"` was found with correct dimensions.

**Load-bearing?** Yes. This is the **only test** exercising the third discovery path in
`find_model_path()`. The block added in T-20.5:
```rust
let shared_models_dir = Config::current().assets_path.join("models");
if let Some(model_path) = discover_model(&shared_models_dir) {
    return Ok(model_path);
}
```
...has zero coverage without this test. A regression that removed this block would go undetected.

**Unique?** Yes. No other test installs to `cfg.assets_path.join("models")` with paths 1 and 2
disabled. The C# compatibility fallback path is unique to this test.

**Correct assertions?**
- `vector::load_model().expect("load model from shared runtime root")` — ✅ panics with a
  message if the fallback path failed, surfaces as test failure
- `assert_eq!(vector::model_filename().as_deref(), Some("shared-runtime.onnx"))` — ✅ the key
  discriminating assertion. The filename `"shared-runtime.onnx"` is installed only at path 3,
  so finding it by name provides implicit evidence that path 3 was used. If a stale ONNX file
  from path 1 or path 2 were found instead, this assertion would fail.
- `assert_eq!(vector::embedding_dimensions(), Some(384))` — ⚠️ hardcoded 384. Same fragility
  as noted in test 1 and in `dynamic_model_loading.md`. Functional today, fragile to model
  replacement.

**Missing assertion**: There is no explicit negative assertion verifying that paths 1 and 2
returned `None` before path 3 was reached. This is intentional — there is no production API to
inspect the discovery path taken — and the filename assertion provides adequate implicit
protection. However, this implicit protection depends on:
- The shadow `assets/models` dir being empty (guaranteed by `TestContext::new()` which creates
  the dir but installs nothing for this test)
- No stale ONNX artifact existing at any Cargo ancestor dir
- `hide_packaged_model()` correctly renaming the packaged model

All three preconditions are met today. The risk of a stale artifact invalidating the implicit
protection is low but non-zero in developer environments with local test artifacts.

**Good name?** Yes. "falls_back_to_shared_runtime_root" clearly states the discovery path under
test. The T-20.5 prefix preserves RTM traceability.

**Parameterizable?** No. The test covers a specific fallback scenario. Parameterizing over model
names or config paths adds filesystem infrastructure tests, not discovery logic tests.

**Skeptical probes:**

1. **Config root isolation** — `TestContext::new()` calls `Config::override_root(tmp.path())`
   which causes `Config::current()` in production code to return a config rooted at `tmp.path()`.
   `install_valid_shared_model_with_fallback_assets` installs to `self.cfg.assets_path.join("models")`,
   where `self.cfg` was obtained from `Config::current()` at `TestContext::new()` time.
   `find_model_path()` also calls `Config::current().assets_path` at runtime. Both must agree on
   the same tmp root. Since `Config::override_root` sets a global override, and the mutex ensures
   no concurrent tests can change it, the two `Config::current()` calls agree. ✅ Sound.

2. **Shadow dir empty check** — `TestContext::new()` calls
   `fs::create_dir_all(shadow_assets_root.join("models"))` but installs nothing. This test then
   calls `install_valid_shared_model_with_fallback_assets`, which installs to
   `cfg.assets_path.join("models")` (the tmp root), **not** the shadow dir. So path 1 sees an
   empty directory and returns `None`. ✅ Correct.

3. **Cargo walk-up probe** — `hide_packaged_model()` renames `assets/models/all-MiniLM-L6-v2.onnx`
   at the workspace root. The walk-up from `base_dir` (test binary parent) traverses ancestors
   looking for `Cargo.toml` + `assets/models/*.onnx`. With the packaged model hidden, the
   workspace-root check finds an empty/nearly-empty `assets/models` dir and returns `None`.
   Any ancestor above the workspace root that happens to have both a `Cargo.toml` and an
   `assets/models/*.onnx` would be a false-positive discovery site. In a standard developer
   environment this doesn't happen. In a monorepo or nested workspace, it could. ⚠️ Low-risk
   environmental assumption, not fixable without per-ancestor allowlisting.

4. **Hardcoded `384`** — Same concern as test 1. If the ONNX model installed at path 3 had
   different output dimensions (which it can't today since it's a hardlink/copy of the same
   packaged model), this assertion would fail correctly. The real fragility is if the packaged
   model is replaced with a 768-dim model and this assertion is not updated.

5. **`install_valid_shared_model_with_fallback_assets` calls `install_valid_model_with_fallback_assets`**
   which uses `link_or_copy` to install the real packaged model PLUS its `tokenizer_config.json`
   and `vocab.txt`. But `hide_packaged_model()` was called before this installation, so the
   packaged model at the source path has been renamed. Does `install_valid_model_with_fallback_assets`
   still work after `hide_packaged_model()`? Let's trace: `packaged_model_path()` returns
   `assets/models/all-MiniLM-L6-v2.onnx`. `hide_packaged_model()` renames it to
   `all-MiniLM-L6-v2.onnx.dynamic-model-tests`. So `link_or_copy` from `packaged_model_path()`
   would try to link a file that no longer exists at its original path. ⚠️ **This is a latent
   ordering bug**. If `install_valid_shared_model_with_fallback_assets` were called *after*
   `hide_packaged_model()`, the install would fail.

   However, looking at the test:
   ```rust
   ctx.install_valid_shared_model_with_fallback_assets("shared-runtime.onnx");
   ctx.hide_packaged_model();
   ```
   The install comes **first**, the hide comes **second**. The current ordering is correct and
   safe. This constraint is not documented and could be violated by a future refactor that
   reorders the calls. A comment asserting the required ordering would be prudent.

**Verdict: KEEP** — Load-bearing, unique, and correctly structured. The setup ordering constraint
(install before hide) is a silent landmine that warrants a comment. The 384 hardcode and the
implicit path-discrimination via filename are known minor concerns.

---

## Recommendations

### P1 — Document required call ordering in test 2
The call to `install_valid_shared_model_with_fallback_assets` **must** precede `hide_packaged_model()`
because the install copies from the packaged model path which the hide renames. A comment in the
test body or a panic-guard in `hide_packaged_model()` (checking that no install-from-packaged
operation is pending) would prevent a future refactor from silently breaking the setup.

```rust
// IMPORTANT: install_valid_shared_model_with_fallback_assets copies from the packaged
// model path. hide_packaged_model() renames that path. Install MUST come first.
ctx.install_valid_shared_model_with_fallback_assets("shared-runtime.onnx");
ctx.hide_packaged_model();
```

### P2 — Replace hardcoded `384` with derived values in both tests
Both tests assert `assert_eq!(vector::embedding_dimensions(), Some(384))`. This hardcodes an
assumption about the packaged model's output dimension. Following the pattern established in
`t9_6_embedding_dimensions_match_model_output_tensor`, the expected value should be derived from
`model_output_dimensions()` at test setup time:

```rust
// Test 2: derive before loading
let expected_dims = model_output_dimensions(
    &ctx.cfg.assets_path.join("models").join("shared-runtime.onnx")
);
// ...
assert_eq!(vector::embedding_dimensions(), Some(expected_dims));
```

For test 1, the derivation must happen in helper mode (subprocess) against the shadow model path.
This is slightly more complex but prevents the assertion from becoming stale when the packaged
model is updated.

### P3 — Surface symlink-skip more visibly in test 1
The current skip path in test 1:
```rust
if let Err(error) = create_file_symlink(&test_binary, &symlink_path) {
    eprintln!("Skipping symlink discovery test: {error}");
    return;
}
```
...produces a green test result on platforms where symlinks cannot be created. RTM-147 is then
unverified with no signal. Consider adding a `#[cfg_attr(not(unix), ignore)]` annotation, or
logging a `cargo test` warning-level message that makes the skip visible in CI output. At minimum,
the CI matrix should assert that this test runs (and does not skip) on the Linux and macOS runners.

### P4 — Add a Cargo walk-up path test (suite-level gap, out of scope for this file)
Neither this file nor `dynamic_model_loading.rs` explicitly tests that the Cargo.toml ancestor
walk-up (path 2 in `find_model_path()`) correctly discovers a model. In all existing tests,
path 2 is either pre-empted by path 1 succeeding or blocked by `hide_packaged_model()`. A
dedicated test for path 2 would complete the coverage triangle:

- `dynamic_model_loading.rs`: exe-adjacent (path 1, non-symlink)
- `dynamic_model_discovery_paths.rs`: symlinked exe-adjacent (path 1, symlink) + shared runtime (path 3)
- **Gap**: Cargo.toml walk-up (path 2) — no test

The path 2 test would need to: leave the shadow dir absent (or empty), NOT hide the packaged
model, and assert that `model_filename()` returns `"all-MiniLM-L6-v2.onnx"`. This is the
implicit behavior relied upon by the current `t9_6_no_model_generate_embedding_returns_err_and_dimensions_none`
setup, but it's never explicitly asserted as the *expected* success path.

### P5 — Consider asserting `vector::is_initialized()` post-load
Both tests call `load_model().expect(...)` but neither asserts the resulting initialized state
via `is_initialized()`. This is a zero-cost robustness signal. After a successful `load_model()`,
`is_initialized()` must return `true`. Asserting it makes the loaded-state post-condition
explicit and guards against future refactors that decouple loading from state registration.

---

## Load-Bearing Score: 2/2

Both tests in this file cover distinct, non-overlapping RTM items with no redundancy between
them or against `dynamic_model_loading.rs`. Each test is the **sole test** for its respective
discovery path. Neither can be removed without leaving an RTM requirement entirely uncovered.

| Test | RTM Coverage | Discovery Path Covered | Load-Bearing |
|------|-------------|----------------------|:------------:|
| `t20_5_symlinked_model_discovery_uses_canonicalized_executable_path` | RTM-147: symlink → canonicalization → path 1 success | Path 1 (symlinked variant) | ✅ Yes |
| `t20_5_model_discovery_falls_back_to_shared_runtime_root` | RTM-148: paths 1+2 miss → path 3 fallback | Path 3 (shared runtime root) | ✅ Yes |

**Suite-level gap**: Path 2 (Cargo.toml ancestor walk-up) is uncovered by any test in any file.
This does not affect the score for this file but represents a gap at the `dynamic_model_*`
suite level.
