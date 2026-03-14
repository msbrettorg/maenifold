# Test Evaluation: `update_assets_boundary.rs`

**Evaluator color:** 🔘 Gray — Skeptical Inquiry
**Traceability:** T-1.4c (tag on every test) | RTM: FR-3 (asset update pipeline)
**File:** `crates/maenifold-core/tests/update_assets_boundary.rs`
**Tests evaluated:** 6
**Competing files examined:**
- `asset_manager_update_detection.rs` — 5 tests, direct function layer via `invoke_update_assets()`
- `asset_manager_errors.rs` — 5 tests, direct function layer + error paths
- `asset_manager_support/mod.rs` — shared helper module (notably **not used** by this file)

---

## Summary

This file is the **ToolRegistry dispatch layer** for `UpdateAssets` tests. Every test calls
`ToolRegistry::global().try_invoke("UpdateAssets", &json!({...}), OutputFormat::Markdown)` —
the full MCP tool registration and dispatch pipeline — rather than invoking `update_assets()` directly
as the sibling files do. That layer distinction is **legitimate grounds for coexistence** with
`asset_manager_update_detection.rs`, but it must earn its keep: each test should carry assertions
that the direct-fn layer cannot provide, or cover a scenario the sibling files do not.

**Skeptical inquiry surfaces four concrete problems:**

1. **Private helper duplication.** The file declares eight local structs and functions (`test_mutex`,
   `isolated_config`, `workspace_assets_root`, `unique_asset_relative`, `write_file`,
   `remove_empty_ancestors`, `ConfigResetGuard`, `PackagedAssetGuard`) that are semantically
   identical to those in `asset_manager_support/mod.rs`. The support module exists precisely to
   avoid this. This is a maintenance liability: bug fixes or semantic changes to the canonical
   helpers will not automatically propagate here.

2. **Broken traceability tag.** Every test is annotated `// T-1.4c: RTM FR-3`, but `T-1.4c`
   appears nowhere in `TODO.md` or `RTM.md`. The tag is either a stale ID from a renamed task or
   was never registered. Either way it is untraceable.

3. **Two tests carry a permissive `||` disjunction** that masks a potential classification bug:
   `output.contains("Added files: 1") || output.contains("Updated files: 1")`. A new file that
   has never existed in the target should be classified as *Added*, not *Updated*. The OR accepts
   the wrong classification silently.

4. **Tests 2 and 4 have thin assertion sets** relative to the behavior they claim to verify.
   Test 2 asserts only a count, not the file name or section heading. Test 4 asserts only the
   "no changes" message without negative guards for "Added files:" or "Updated files:".

Tests 5 and 6 are the strongest members of this file: both cover scenarios absent from the entire
sibling test suite and carry assertions precise enough to catch real regressions.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|---------|
| 1 | `update_assets_dry_run_detects_files_missing_from_target` | ✅ Yes | ⚠️ Partial — adds count+not-copied over detection tests | ⚠️ Missing `### Added Files` section heading check | ✅ Yes | 🔘 No | **KEEP** (add section heading assertion) |
| 2 | `update_assets_dry_run_detects_sha_changes_for_existing_files` | ⚠️ Partial | ⚠️ Partial — same-length setup is subtle, not documented | ❌ Count only; missing name, section, no-copy guard | ⚠️ "sha_changes" not proven by assertions | 🔘 No | **REWRITE** |
| 3 | `update_assets_non_dry_run_copies_added_files_into_runtime_assets` | ✅ Yes | ✅ Content equality at ToolRegistry layer | ❌ `||` disjunction masks Added vs Updated misclassification | ✅ Yes | 🔘 No | **REWRITE** |
| 4 | `update_assets_reports_no_changes_when_source_and_target_match` | ⚠️ Partial | ⚠️ Overlap — detection file has identical scenario at fn layer | ❌ Single positive assertion; no negative guards | ✅ Yes | 🔘 No | **REWRITE** |
| 5 | `update_assets_learn_mode_returns_help_instead_of_update_summary` | ✅ Yes | ✅ Unique — not covered in any sibling file | ⚠️ OR in positive assertion is loose | ✅ Yes | 🔘 No | **KEEP** (tighten positive assertion) |
| 6 | `update_assets_uses_sha256_not_file_size_for_equal_length_content` | ✅ Yes | ✅ Unique — only explicit SHA-256 test in the suite | ⚠️ Missing `### Updated Files` section heading | ✅ Yes | 🔘 No | **KEEP** (add section heading assertion) |

---

## Per-Test Analysis

### Test 1 — `update_assets_dry_run_detects_files_missing_from_target`

**What it does:** Adds a new file to packaged assets, runs dry-run via ToolRegistry, asserts count
("Added files: 1"), file name present in output, and file not copied to runtime target.

**What it proves:** The ToolRegistry dispatch pipeline correctly routes the `dryRun: true` payload,
the count line is emitted, the file name is listed, and the dry-run gate is respected (no file copy).

**Comparison with sibling tests:**
- `update_assets_dry_run_reports_new_files_without_copying` (detection) — checks file name + not
  copied; **missing count assertion**.
- `update_assets_new_file_in_source_appears_in_added_files` (detection) — checks `### Added Files`
  section + file name; **missing count and not-copied assertions**.

This boundary test is the most complete coverage of the new-file dry-run scenario across all files.
It has the count + name + not-copied combination that no single detection test has. However, it is
itself missing the `### Added Files` section heading assertion.

**Assertion gap:**
```rust
// Missing:
assert!(output.contains("### Added Files"), "expected Added Files section: {output}");
```

**Name:** Clear. "Detects files missing from target" accurately describes the scenario.

**Verdict: KEEP.** Add the missing section-heading assertion to complete the assertion matrix.

---

### Test 2 — `update_assets_dry_run_detects_sha_changes_for_existing_files`

**What it does:** Creates a source file at version 1, initializes (copies v1 to target), then
overwrites the *source* with version 2 (`"version":2` replaces `"version":1` — both strings are
the same byte length). Runs dry-run, asserts only `"Updated files: 1"`.

**What it proves (claimed):** SHA-256 change detection for equal-length content.

**What it actually proves:** That a count of 1 appears in the output when source and target differ.
The `version":1` → `version":2` substitution does produce equal-length content (both 33 bytes),
so the SHA-256 path is exercised — but the test makes no assertion that proves the *name* of the
SHA test was found, nor that the file was correctly listed under `### Updated Files`, nor that no
copy occurred.

**Compared to Test 6:** Test 6 (`update_assets_uses_sha256_not_file_size_for_equal_length_content`)
directly creates `{"a":1}` / `{"b":2}` pairs and names itself after the SHA-256 property. Test 2
reaches the same code path via a less legible route and with weaker assertions. If Test 6 is the
canonical SHA-256 regression guard, Test 2 needs a clearer unique purpose.

**Unique differentiation:** The setup direction differs — Test 2 modifies the *source* after
initialization, while Test 6 modifies the *target*. Both ultimately result in `source != target`
with equal byte lengths. This is a subtle structural difference that is not commented.

**Assertion gaps:**
```rust
// Missing:
assert!(output.contains("### Updated Files"), "expected Updated Files section: {output}");
assert!(output.contains(&file_name), "expected updated file name in summary: {output}");
assert!(!cfg.assets_path.join(&relative).exists() || /* content still v1 */, "dry-run must not update");
```

**Name:** "detects_sha_changes" implies the test is specifically about SHA-256 detection, but the
assertions say nothing about SHA. The name is aspirational rather than descriptive.

**Verdict: REWRITE.** Either:
- (a) Add the missing assertions (section heading, file name, and a comment explaining why this
  test's source-modification setup is distinct from Test 6's target-modification setup), **or**
- (b) Remove it if Test 6 is accepted as the canonical SHA-256 regression guard and this test's
  only unique value was the count assertion (also present in Test 1).

---

### Test 3 — `update_assets_non_dry_run_copies_added_files_into_runtime_assets`

**What it does:** Adds a new packaged file, runs apply mode (`dryRun: false`), asserts a count
summary and that the runtime target's byte content equals the source content.

**What it proves:** End-to-end copy fidelity at the ToolRegistry layer — the most valuable
assertion in this file. If `fs::copy` were replaced with a stub that wrote wrong content, or if
the ToolRegistry routing truncated the payload before reaching apply mode, this test would catch it.

**The `||` problem:**
```rust
assert!(
    output.contains("Added files: 1") || output.contains("Updated files: 1"),
    "expected copy summary for changed file: {output}"
);
```
A file created after `initialize_assets` has never existed in the target. It is definitionally
*Added*. If the implementation misclassified it as *Updated*, this OR would pass silently while
masking a classification bug. The same OR appears in `asset_manager_update_detection.rs::
update_assets_apply_copies_new_files_to_target` — it was apparently copied verbatim, which means
neither test challenges the classification. The intended strict assertion is:
```rust
assert!(output.contains("Added files: 1"), "new file should be classified Added, not Updated: {output}");
```
If there is a genuine edge case that forces the OR (e.g., a race where `initialize_assets` ran
after the source was created), it must be documented with a comment — otherwise it is cargo-cult
permissiveness.

**Content equality assertion:** `assert_eq!(target_content, source_content)` is correct and
load-bearing. This is the only place in the boundary test file where byte content is verified,
and it uniquely validates the apply path at the ToolRegistry invocation level.

**Verdict: REWRITE.** Change `||` to strict `output.contains("Added files: 1")`. Add a comment
if the OR is genuinely needed. The content equality assertion is the test's crown jewel — preserve it.

---

### Test 4 — `update_assets_reports_no_changes_when_source_and_target_match`

**What it does:** Initializes runtime assets (source == target), runs dry-run via ToolRegistry,
asserts `"No changes detected"`.

**Comparison with sibling:**
`asset_manager_update_detection.rs::update_assets_identical_file_is_not_reported_as_added_or_updated`
has the same setup and the same single assertion. The detection test evaluation noted the
`PackagedAssetGuard` in that test is vestigial — this boundary test correctly omits it, which is
actually cleaner. But the assertion body is still just one line.

**The single-assertion problem:** `"No changes detected"` is a positive substring match. It would
pass if the output were `"No changes detected. Added files: 1"` (hypothetically malformed). The
stronger statement is:
```rust
assert!(output.contains("No changes detected"), "expected no-change summary: {output}");
assert!(!output.contains("Added files:"), "no-change output must not list Added counts: {output}");
assert!(!output.contains("Updated files:"), "no-change output must not list Updated counts: {output}");
```

**Unique value:** This test exercises `ToolRegistry::global()` on the no-change path. The
detection-layer equivalent exercises `invoke_update_assets()`. Both paths converge on the same
`render_update_summary` call, so the ToolRegistry layer adds the value of verifying routing
(payload is received, `dryRun` defaults correctly, etc.).

**Verdict: REWRITE.** Add the two negative assertions. The test is one minute of work from being
genuinely load-bearing; right now it is a one-line test that cannot distinguish between a correct
no-change response and a garbled response that happens to contain the magic string.

---

### Test 5 — `update_assets_learn_mode_returns_help_instead_of_update_summary`

**What it does:** Passes `learn: true` to UpdateAssets via ToolRegistry. Asserts the output
contains help content and does **not** contain `"Asset Update Summary"`.

**Uniqueness:** This scenario is not covered in any other file in the test suite. The `learn`
parameter branches immediately at the top of `update_assets()` before any filesystem access.
The only way to test this path at the ToolRegistry layer is exactly what this test does.

**The OR in the positive assertion:**
```rust
assert!(
    output.contains("# UpdateAssets") || output.contains("Refreshes"),
    "expected help documentation: {output}"
);
```
The `||` is less confident than it should be. If the help doc changes its heading from
`# UpdateAssets` to `## UpdateAssets`, the first branch fails but the second ("Refreshes")
catches it vacuously. The canonical anchor for this assertion should be the help document's
own structure — if the heading is `# UpdateAssets`, assert that. If it might vary by format,
pick the invariant part (`UpdateAssets` alone) and document why. The current OR implies
uncertainty about what the help doc actually contains.

**The negative assertion is correctly formed:**
```rust
assert!(!output.contains("Asset Update Summary"), "learn mode should not render update summary: {output}");
```
This pins the contract that `learn` mode bypasses the update logic.

**Verdict: KEEP.** Tighten the positive assertion to a single authoritative string (e.g.,
`output.contains("UpdateAssets")`) and remove the OR fallback, or document why both branches
are needed.

---

### Test 6 — `update_assets_uses_sha256_not_file_size_for_equal_length_content`

**What it does:** Creates a source file with content `{"a":1}` (7 bytes), initializes (copies
to target), then writes `{"b":2}` (7 bytes) directly to the *runtime target*. Runs dry-run,
asserts "Updated files: 1" count and the file name in output.

**Why this is the canonical SHA-256 regression guard:** The setup is unambiguous — content length
is identical, content is different, the only detection mechanism that could surface this as
"Updated" is SHA-256. If the implementation dropped SHA-256 and compared only byte lengths, the
update would be missed and this test would fail. No other test in any sibling file makes this
precise claim.

**Setup correctness:** The sequence (source=`{"a":1}` → initialize → target overwritten with
`{"b":2}`) is correct. The ToolRegistry invocation then sees source=`{"a":1}`, target=`{"b":2}`,
same length → SHA-256 comparison → differ → "Updated files: 1". ✓

**Assertion gap:** The test asserts count and file name, but not the `### Updated Files` section
heading. Adding this brings the test to parity with the detection-layer equivalent (Test 4 in
`asset_manager_update_detection.rs`), which asserts the section heading.

```rust
// Missing:
assert!(output.contains("### Updated Files"), "expected Updated Files section: {output}");
```

**Verdict: KEEP.** Add the section-heading assertion. This is the highest-signal test in the file;
it must not be weakened or removed.

---

## Cross-Cutting Findings

### Finding 1 — Private Helper Duplication (Infrastructure Debt)

This file declares eight private helpers that are semantic duplicates of those in
`asset_manager_support/mod.rs`. The support module was introduced under T-12.7 specifically to
eliminate this pattern. The boundary test file predates the support module split and was never
migrated.

The practical risk:
- `PackagedAssetGuard` in the boundary file stores the root as `assets_root: PathBuf`; the
  support module uses `root: PathBuf`. Field name only — behavior is identical.
- `unique_asset_relative()` here takes `(dir, prefix) → String`; support module's
  `unique_relative_path()` takes `(dir, prefix, extension) → String`. The boundary version
  hard-codes `.json` into the filename suffix. Any future extension requirement is blocked.
- `write_file()` and `remove_empty_ancestors()` are verbatim duplicates.

**Recommendation:** Replace all local helpers with `#[path = "asset_manager_support/mod.rs"]` and
migrate the call sites. This is a refactor-only change with no behavioral impact.

### Finding 2 — Traceability Gap: `T-1.4c` Is Unregistered

Every test carries `// T-1.4c: RTM FR-3`. A grep of `TODO.md`, `RTM.md`, and `PRD.md` finds no
entry for `T-1.4c`. The tag is either:
- A stale ID from a task that was renamed or deleted, or
- A sub-item of `T-1.4` that was never formally registered.

In either case, the tag cannot be used for traceability tracking. If the task was completed and
archived, the tag should be replaced with a completed-sprint reference. If it was a sub-task,
it should be registered or collapsed into its parent.

### Finding 3 — The `||` Classification Mask (Cross-File Pattern)

The `output.contains("Added files: 1") || output.contains("Updated files: 1")` disjunction
appears in both this file (Test 3) and in `asset_manager_update_detection.rs`
(`update_assets_apply_copies_new_files_to_target`). It was apparently copied without scrutiny.
Neither test challenges the misclassification scenario. If `compare_asset_directories` were
changed to always report new files as "Updated" (a behavioral regression), both tests would pass.

### Finding 4 — Cross-Binary Mutex Gap (Inherited from Sibling Files)

`test_mutex()` in this file is a private static. `test_mutex()` in `asset_manager_support/mod.rs`
is a different static in a different binary. `PackagedAssetGuard::create()` writes into the
**shared workspace `assets/` directory** — not a temp dir. If `cargo test` runs integration test
binaries in parallel (the default), a race exists between:
- This file's `PackagedAssetGuard::Drop` (deleting a workspace asset file)
- A concurrent test in `asset_manager_errors.rs` or `asset_manager_update_detection.rs` that
  reads or enumerates workspace assets

This is a known pattern risk flagged in the `asset_manager_errors.md` evaluation and reproduced
here for completeness. The probability is low but non-zero. The fix is a cross-binary lock
(e.g., a filesystem advisory lock on the workspace assets dir) or an explicit `--test-threads=1`
contract in documentation.

---

## Recommendations

Ordered by priority:

1. **[High] Migrate to shared support module.**
   Replace all private helpers with `#[path = "asset_manager_support/mod.rs"]` declarations.
   This eliminates ~80 lines of duplicated infrastructure and ensures the boundary tests receive
   the same helper improvements as the detection and error test files.

2. **[High] REWRITE Test 3** — change `||` to strict `output.contains("Added files: 1")`. A new
   file that was never in the target is *Added* by definition. The content-equality assertion is
   the load-bearing value; do not let the OR undermine the classification assertion.

3. **[High] REWRITE Test 4** — add negative guards:
   `!output.contains("Added files:")` and `!output.contains("Updated files:")`. The current
   single-assertion test could pass against a malformed response.

4. **[Medium] REWRITE Test 2** — either add section heading + file name + no-copy assertions and
   add a comment explaining how it differs from Test 6 (source-modified vs target-modified), **or**
   remove it if Test 6 is accepted as the canonical SHA-256 regression guard and Test 2's
   unique contribution cannot be articulated.

5. **[Medium] Fix the traceability tag** on all 6 tests. Replace `T-1.4c` with the current
   registered task ID, or note in a comment that the task was completed under the now-archived
   sprint ID.

6. **[Low] Add missing section-heading assertions to Tests 1 and 6.**
   Test 1 is missing `### Added Files`; Test 6 is missing `### Updated Files`. One line each.

7. **[Low] Tighten Test 5's OR** from
   `output.contains("# UpdateAssets") || output.contains("Refreshes")`
   to a single authoritative string. Document the choice with a comment referencing the help
   document's structure.

8. **[Low] Add a comment to Test 2** (if kept after item 4) explaining that it exercises the
   source-modification direction while Test 6 exercises target-modification, and that both hit
   the SHA-256 code path. Without this comment, the relationship between Tests 2 and 6 is opaque.

---

## Load-Bearing Score: 4/6

| # | Test | Load-Bearing? | Reasoning |
|---|------|:---:|-------|
| T1 | `dry_run_detects_files_missing_from_target` | ✅ Yes | Most complete new-file dry-run test in the suite (count + name + not-copied). No single sibling test has all three. |
| T2 | `dry_run_detects_sha_changes_for_existing_files` | ⚠️ Partial | Count-only assertion at ToolRegistry layer; partially subsumed by T6. Load-bearing only if source-modification direction is documented as intentionally distinct. |
| T3 | `non_dry_run_copies_added_files_into_runtime_assets` | ✅ Yes | Content-equality assertion at ToolRegistry layer is unique. The `||` mask must be fixed for the test to actually exercise what it claims. |
| T4 | `reports_no_changes_when_source_and_target_match` | ⚠️ Partial | Single positive assertion. Fully subsumed by detection-layer equivalent once negative guards are absent. Not load-bearing until rewritten. |
| T5 | `learn_mode_returns_help_instead_of_update_summary` | ✅ Yes | Unique across entire suite. Tests a distinct execution path. Both assertions (positive + negative) contribute. |
| T6 | `uses_sha256_not_file_size_for_equal_length_content` | ✅ Yes | Unique explicit SHA-256 regression guard. The setup and intent are unambiguous. Missing one section-heading assertion but otherwise correct. |

**Effective load-bearing count (post-rewrite):** T1, T3, T5, T6 = **4 fully load-bearing**; T2 conditional; T4 promoted to load-bearing only after the rewrite in Recommendation 3.
