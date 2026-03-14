# Test Evaluation: `asset_manager_errors.rs`

**Evaluator:** Gray (Skeptical Inquiry)
**Traceability:** T-12.7 | FR-3 (RTM-014–019) | NFR-1 (C# Oracle Parity)
**File:** `crates/maenifold-core/tests/asset_manager_errors.rs`
**Date:** 2025-07-25

---

## Summary

This file holds 5 integration tests split from the legacy `asset_manager_tests.rs` under T-12.7. They exercise error paths and output formatting for `update_assets`. The production under test is `crates/maenifold-core/src/assets/update.rs`.

Before passing judgment, establish what is actually provable from reading the code:

1. **What does each test exercise that nothing else does?** Answered by cross-examining `update_assets_boundary.rs`, `asset_manager_update_detection.rs`, `asset_manager_initialize.rs`, and the production `update_assets` function.
2. **Are the assertions specific enough to catch regressions?** Each assertion was mapped against the production code paths it could plausibly fail on.
3. **Is the shared infrastructure sound?** The `test_mutex()` / `ConfigResetGuard` / `PackagedAssetGuard` pattern was inspected for correctness.

**Bottom line:** The file has one clear redundancy (Test 1 is a strict subset of Test 4), one god test that should be split (Test 3), and three genuinely load-bearing tests. The mutex isolation story has a latent cross-binary race condition that deserves a note even though it has not caused a real failure.

---

## Verdicts

| # | Test Name | Keep / Remove / Merge / Rewrite | Load-Bearing | Notes |
|---|-----------|----------------------------------|:---:|-------|
| 1 | `update_assets_when_source_dir_missing_returns_error` | **MERGE → T4** | ❌ | Identical setup to T4; positive assertion is a strict subset of T4's assertions |
| 2 | `update_assets_when_target_dir_missing_returns_error` | **KEEP** | ✅ | Only test that exercises the target-missing guard; unique setup |
| 3 | `update_assets_summary_formats_counts_sections_and_errors` | **REWRITE** | ✅ | 12 assertions covering 4 distinct concerns; Unix-only gate; split recommended |
| 4 | `update_assets_source_missing_error_surfaces_without_summary_counts` | **KEEP** | ✅ | Superset of T1; negative assertion (`!contains("Added files:")`) is load-bearing |
| 5 | `update_assets_source_target_mapping_discovers_all_four_rust_asset_subdirectories` | **KEEP** | ✅ | Only test that verifies all four `AssetKind` categories pass through the pipeline together |

---

## Per-Test Analysis

### T1 — `update_assets_when_source_dir_missing_returns_error`

**What it tests:** `resolve_packaged_assets_root()` returns `None` (or the resolved path is not a directory) → `update_assets` returns `Err("Source assets directory not found")`.

**Setup fingerprint:**
```rust
isolated_config();
hide_packaged_asset_roots();
invoke_update_assets(&cfg, true).expect_err(...)
```

**Assertions (1):**
- `err.to_string().contains("Source assets directory not found")`

**Is it unique?** No. Test 4 has the **exact same four-line setup** and makes the **same positive assertion** plus an additional negative one. T1's only assertion is a strict subset of T4's. If T1 were deleted today, no regression could slip past T4.

**Is the name good?** Descriptive but generic. `_returns_error` is weak — what kind of error, what format?

**Parameterizable with T4?** The setup is identical; these two tests can be merged into one function. The only risk is that merging makes the test slightly longer, but that is a net gain in clarity.

**Verdict: MERGE into T4.** Remove T1; fold its intention into T4's name and comments.

---

### T2 — `update_assets_when_target_dir_missing_returns_error`

**What it tests:** After `initialize_assets`, the runtime assets directory is forcibly removed. `update_assets` must catch this and return `Err("Target assets directory does not exist. Run initialization first.")`.

**Setup fingerprint:**
```rust
isolated_config();
initialize_assets(&cfg);
fs::remove_dir_all(&cfg.assets_path);
invoke_update_assets(&cfg, true).expect_err(...)
```

**Assertions (1):**
- `err.to_string().contains("Target assets directory does not exist. Run initialization first.")`

**Is it unique?** Yes — no other test removes the runtime assets directory after initialization. This verifies the guard at line 42–45 in `update.rs`. If that block were deleted, the function would proceed to `compare_asset_directories` which would likely panic or return an I/O error with a different message, and this test would catch it.

**Is the name good?** Yes — action/condition/expectation clearly stated.

**Assertion quality:** Adequate. The exact error message string is pinned, which is correct here because this message is part of the user-facing contract (NFR-1 parity implies the message matters).

**Parameterizable?** Could technically be parameterized with T1 (both test "wrong directory state → error"), but the setups differ non-trivially. Not worth the indirection.

**Verdict: KEEP.** Load-bearing, unique, well-named.

---

### T3 — `update_assets_summary_formats_counts_sections_and_errors`

**What it tests:** With one added file, one updated file, and one symlink (which becomes an error per PARITY-U01), the formatted markdown summary contains the correct counts, section headers, file names, and critically does **not** contain the pre-parity "Skipped symlinks" line or section.

**Setup fingerprint:** 70+ lines. Three `PackagedAssetGuard` instances across three different categories. A manual symlink created in the runtime assets directory to trigger the error path.

**Assertions (12):**
1. `output.contains("Asset Update Summary")`
2. `output.contains("Added files: 1")`
3. `output.contains("Updated files: 1")`
4. `!output.contains("Skipped symlinks:")` — C# parity: count line removed
5. `output.contains("Errors: 1")`
6. `output.contains("### Added Files")`
7. `output.contains("### Updated Files")`
8. `!output.contains("### Skipped Symlink Targets")` — C# parity: section removed
9. `output.contains("### Errors")`
10. `output.contains(<added_file_name>)`
11. `output.contains(<updated_file_name>)`
12. `output.contains("skipped symlink target")`

**Is it unique?** Partially. The PARITY-U01 negative assertions (4, 8) are unique across the entire test suite — nothing else verifies that the pre-parity "Skipped symlinks" count line and section were removed. These are genuinely load-bearing. However, assertions 1–3 and 6–7 are also checked (individually) by tests in `update_assets_boundary.rs` and `asset_manager_update_detection.rs`.

**Problems:**

1. **God test.** 12 assertions across 4 distinct concerns (count formatting, section structure, C# parity deletions, file-level content). A failure anywhere produces one failing test; diagnosis requires reading all 12.

2. **Unix-only gate.** The `#[cfg(unix)]` gate is correct — `std::os::unix::fs::symlink` is unavailable on Windows — but the PARITY-U01 behaviour (symlink → error, not skip) is a cross-platform concern. The negative parity assertions (no "Skipped symlinks:") are implicitly un-tested on non-Unix platforms.

3. **Assertion 4 (`!output.contains("Skipped symlinks:")`) is under-specified.** If the count line were renamed to "Skipped symlink count:" it would not be caught. A more robust assertion would check the full `## Results` section structure.

4. **The setup creates files across three distinct categories.** This is intentional to exercise the combined path, but it makes the test fragile to ordering of `AssetKind::all()` sort output and timing of `PackagedAssetGuard` teardown races.

**Name:** Acceptable — "formats counts sections and errors" covers the intent. But it is now inaccurate: it also tests negative C# parity assertions, which are arguably the most important part.

**Parameterizable?** The combined-state scenario (added + updated + error) is intentional and should stay together. But the 12 assertions should be distributed across 2 tests:
- **Test 3a:** Three-state summary formatting (counts, sections, file names) — positive assertions only.
- **Test 3b:** PARITY-U01 negative assertions (no "Skipped symlinks:" count, no "### Skipped Symlink Targets" section) — focused on what was deliberately *removed* for C# parity.

**Verdict: REWRITE.** Split into two focused tests. T3b carrying the PARITY-U01 negative assertions should have its own name that clearly marks it as a parity guard, e.g., `update_assets_parity_u01_omits_skipped_symlinks_count_and_section`.

---

### T4 — `update_assets_source_missing_error_surfaces_without_summary_counts`

**What it tests:** Same failure path as T1, but adds a negative assertion: the error message returned when source assets are missing must not contain "Added files:" — i.e., the error path must not accidentally render a partial summary.

**Setup fingerprint:** Identical to T1.

**Assertions (2):**
1. `text.contains("Source assets directory not found")`
2. `!text.contains("Added files:")`

**Is it unique?** The negative assertion is unique. However, examining the production code, the error is returned via the `?` operator before the summary rendering call, so assertion 2 is testing a structural property of the code flow, not just a string. The risk it guards against is real but narrow: a developer who accidentally wraps errors in summary output. Given this is `Err(...)` propagation, assertion 2 is a sanity check, not a high-value regression guard.

**Name:** Long but precise. The "without_summary_counts" suffix distinguishes it from T1 correctly.

**What to do with T1 after merge:** Absorb T1's name into T4's test comment, e.g., `// Also verifies the happy-path failure: error type is Err, not Ok.`

**Verdict: KEEP.** Absorb T1 into this test by adding a comment acknowledging T1's original intent.

---

### T5 — `update_assets_source_target_mapping_discovers_all_four_rust_asset_subdirectories`

**What it tests:** One file is seeded into each of the four `AssetKind` directories (`workflows`, `roles`, `colors`, `perspectives`). A single dry-run invocation must report all four file names in the summary — verifying that `compare_asset_directories` iterates all four via `AssetKind::all()`.

**Setup fingerprint:**
```rust
isolated_config();
initialize_assets(&cfg);
// loop creates one PackagedAssetGuard per category
invoke_update_assets(&cfg, true)
```

**Assertions (4):** One `output.contains(file_name)` per category.

**Is it unique?** Yes. No other test seeds all four categories simultaneously. Individual category tests exist in `asset_manager_update_detection.rs` but they test categories in isolation. T5 is the only test that would catch `AssetKind::all()` silently dropping a category.

**Name:** Verbose but accurate. The word "Rust" is slightly confusing (it refers to the Rust implementation's asset subdirectories, as opposed to categories the C# oracle might not have). A reviewer could mistake it for "Rust source files." Consider: `update_assets_discovers_all_four_asset_catalog_subdirectories`.

**Loop vs parameterized:** Using a `for category in categories` loop is intentional here — the loop builds `expected_names` and collects guards before the single invocation. This is correct design for a combined-invocation test. Making it a true parameterized test (one test per category) would require four separate invocations and lose the "all four in one call" property.

**Correctness concern:** The hardcoded `["workflows", "roles", "colors", "perspectives"]` in the test mirrors the hardcoded values in `AssetKind::dir_name()`. If both were wrong together (e.g., both used "workflow" singular), the test would pass while being incorrect. However, this is a test-infrastructure concern, not a test design flaw — the test exercises the full pipeline, not just the enum.

**Verdict: KEEP.** Rename for clarity.

---

## Cross-Cutting Findings

### Finding 1 — T1 / T4 Redundancy

T1's single positive assertion (`err.contains("Source assets directory not found")`) is tested identically in T4's assertion 1. T1 adds no new coverage. The `hide_packaged_asset_roots()` mechanism is called in both, placing the same infrastructure cost on the test suite for zero additional coverage.

**Recommendation:** Delete T1. Add a one-line comment to T4: `// Subsumes the basic error-type check from the split predecessor.`

### Finding 2 — T3 God Test: PARITY-U01 Assertions Are Buried

The most important assertions in T3 are the two negative ones (assertions 4 and 8) that enforce C# parity by verifying that the "Skipped symlinks" output was deliberately removed. These are buried in a 12-assertion test that also checks general summary formatting. If the parity contract regresses, the failure message will be one line among 12, with no test name that says "parity violation."

**Recommendation:** Extract T3b as a named parity guard:
```rust
#[cfg(unix)]
#[test]
fn update_assets_parity_u01_omits_skipped_symlinks_count_and_section() { ... }
```

### Finding 3 — Cross-Binary Mutex Gap (Latent)

`test_mutex()` in `asset_manager_support/mod.rs` is a process-global `static OnceLock<Mutex<()>>`. Within the `asset_manager_errors` integration test binary it serializes all 5 tests. However:

- `update_assets_boundary.rs` declares its **own** local `test_mutex()` pointing to a **different** static.
- `asset_hot_loading_tests.rs` does the same.

`PackagedAssetGuard::create()` writes files into the **workspace** `assets/` directory (not a temp dir). `HiddenDirGuard` **renames** the workspace assets root. These operations are shared filesystem state. If `cargo test` runs integration test binaries in parallel (the default when test binaries are independent processes), races between `hide_packaged_asset_roots()` in `asset_manager_errors` and `PackagedAssetGuard::create()` in `update_assets_boundary` are possible.

This has likely not caused a real failure because the wall-clock overlap probability is low, but it is a correctness assumption rather than a guarantee.

**Recommendation:** Consider a cross-binary coordination mechanism, or document the assumption explicitly in `asset_manager_support/mod.rs` that callers accept this risk.

### Finding 4 — `#[cfg(unix)]` Does Not Gate PARITY-U01's Cross-Platform Invariant

The PARITY-U01 behaviour (symlinks are recorded as errors, not as a separate "skipped" category) is a formatting contract that applies on all platforms, but T3 only tests it on Unix because the symlink creation is Unix-only. There is no Windows-equivalent test for the negative PARITY-U01 assertions.

This is a low-risk gap since the rendering logic in `render_update_summary` is platform-independent, but it means Windows CI passes without verifying PARITY-U01 formatting.

---

## Recommendations

Ordered by priority:

1. **[High] MERGE T1 into T4.** Delete `update_assets_when_source_dir_missing_returns_error`. Update T4's name to `update_assets_source_dir_missing_errors_without_summary_counts` (removes the awkward "surfaces"). Add a comment.

2. **[High] REWRITE T3.** Split into:
   - T3a: `update_assets_summary_formats_combined_counts_and_sections` — keeps the combined-state scenario, retains positive formatting assertions (assertions 1–3, 5–7, 9–12).
   - T3b: `update_assets_parity_u01_omits_skipped_symlinks_count_and_section` — carries only the two negative PARITY-U01 assertions (4, 8) with the minimum setup needed to trigger a symlink scenario.

3. **[Medium] RENAME T5.** Change `_discovers_all_four_rust_asset_subdirectories` to `_scans_all_four_asset_catalog_subdirectories`. Remove the potentially confusing "Rust" qualifier.

4. **[Low] Document or mitigate the cross-binary mutex gap** (Finding 3). The simplest mitigation is a comment in `asset_manager_support/mod.rs` near `PackagedAssetGuard::create` noting that callers accept the cross-binary write-race risk on the shared workspace assets directory.

5. **[Low] Consider a platform-independent PARITY-U01 test.** If symlink creation can be mocked or if the rendering path can be unit-tested directly, the PARITY-U01 formatting contract can be verified on Windows CI without requiring `std::os::unix::fs::symlink`.

---

## Load-Bearing Score: 4/5

| Test | Load-Bearing? | Reason |
|------|:---:|-------|
| T1 `when_source_dir_missing_returns_error` | ❌ | Strict subset of T4; contributes no unique coverage |
| T2 `when_target_dir_missing_returns_error` | ✅ | Unique setup; only coverage of the target-missing guard |
| T3 `summary_formats_counts_sections_and_errors` | ✅ | PARITY-U01 negative assertions are unique in the codebase |
| T4 `source_missing_error_surfaces_without_summary_counts` | ✅ | Negative assertion is unique; superset of T1 |
| T5 `mapping_discovers_all_four_rust_asset_subdirectories` | ✅ | Only test exercising all four `AssetKind` categories in a single invocation |

---

## SWE Remediation

- REMOVE `update_assets_when_source_dir_missing_returns_error` (T1): deleted; strict subset of T4, zero unique coverage
- REWRITE `update_assets_summary_formats_counts_sections_and_errors` (T3): split into two focused tests:
  - T3a `update_assets_summary_formats_combined_counts_and_sections`: 10 positive assertions for counts, sections, file names
  - T3b `update_assets_parity_u01_omits_skipped_symlinks_count_and_section`: 2 negative PARITY-U01 assertions with minimum symlink setup
- RENAME T4 → `update_assets_source_dir_missing_errors_without_summary_counts`: clearer name, added comment documenting T1 absorption
- RENAME T5 → `update_assets_source_target_mapping_scans_all_four_asset_catalog_subdirectories`: removed confusing "Rust" qualifier per P1
- DOCUMENT cross-binary mutex gap in `asset_manager_support/mod.rs` per P2 recommendation (Finding 3)
- NOTE: pre-existing environment issue found — stale empty `target/debug/deps/assets/` directory causes `resolve_packaged_assets_root()` to resolve exe-adjacent (empty) dir instead of workspace `assets/`, making all PackagedAssetGuard-based tests fail with "No changes detected"
- All 5 tests pass: `cargo test -p maenifold-core --test integration_tests 'asset_manager_errors'`
- Commit: cba1d9af013f87217ca5507a4e6e9fa59d7622b1
