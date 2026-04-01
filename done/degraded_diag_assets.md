---
id: T-80.59
title: "Test Evaluation: `degraded_diag_assets.rs`"
status: done
pipeline: tdd
---

# Test Evaluation: `degraded_diag_assets.rs`

**Evaluator:** Gray 🔘 (Skeptical Inquiry)
**Traceability:** T-0.1 | RTM-114 (FR-25.2) | NFR-1 (C# Oracle Parity) | NFR-2 (Test Coverage)
**File:** `crates/maenifold-core/tests/degraded_diag_assets.rs`
**Support:** `crates/maenifold-core/tests/degraded_diag/support.rs`
**Peer file:** `crates/maenifold-core/tests/degraded_diag_graph.rs` (3 tests; different subsystem)
**Date:** 2025-07-25

---

## Summary

This file holds 4 integration tests ported from C# oracle parity cases (`T-0.1: RTM FR-25.2`).
They exercise the *asset* and *memory-listing* halves of the degraded-diagnostic contract —
the guarantee that subsystem failures produce a `[DEGRADED][…][…]` stderr line while the
tool call succeeds and returns usable output.

Before passing verdicts, the following questions were answered against production code:

1. **What does each test uniquely exercise?** Checked against `degraded_diag_graph.rs`,
   `asset_manager_errors.rs`, and all other test files touching `list_memories`, `read_asset`,
   and `list_assets`.
2. **Are the assertions specific enough to catch regressions?** Mapped every assertion to the
   code path it could plausibly fail to catch.
3. **Is the shared infrastructure sound?** The `TEST_MUTEX` / `capture_stderr` / `TestEnv`
   machinery was inspected for correctness and known failure modes.
4. **Are there coverage gaps within each tested function?** Compared the test inputs against
   all `emit_…_diagnostic` call sites in the production source.

**Bottom line:** All four tests are load-bearing and unique. There are no merge or remove
candidates. Two rewrite candidates are minor (one parameterization opportunity, one
coverage-gap note), not structural regressions. The most substantive finding is a coverage
gap in Test 1: `load_last_accessed_timestamps` has four independent emit sites but only the
first (connection-open failure) is exercised. The second substantive finding is that
`capture_stderr` has no RAII guard for fd restoration on panic.

---

## Verdicts

| # | Test Name | Verdict | Load-Bearing | Unique | Correct Assertions | Good Name | Parameterizable |
|---|-----------|:-------:|:---:|:---:|:---:|:---:|:---:|
| 1 | `load_last_accessed_timestamps_emits_degraded_on_db_failure` | **KEEP** | ✅ | ✅ | ⚠️ partial | ✅ | ⚠️ 3 branches uncovered |
| 2 | `get_decay_metadata_for_file_emits_degraded_on_frontmatter_failure` | **KEEP** | ✅ | ✅ | ✅ | ✅ | ❌ not needed |
| 3 | `find_asset_file_by_id_emits_degraded_on_malformed_json` | **KEEP** | ✅ | ✅ | ✅ | ✅ | ❌ not needed |
| 4 | `get_asset_metadata_emits_degraded_on_malformed_json` | **KEEP** | ✅ | ✅ | ✅ | ✅ | ❌ not needed |

---

## Per-Test Analysis

### T1 — `load_last_accessed_timestamps_emits_degraded_on_db_failure`

**Production path:** `memory/listing.rs` → `load_last_accessed_timestamps()` → first
`emit_system_tools_diagnostic("LoadLastAccessedTimestamps", ...)` call site (the
`db::open_connection` failure branch, lines 168–178).

**Setup:**
```rust
// A valid SQLite schema is initialized by TestEnv::setup().
// The db file is then overwritten with raw garbage text.
fs::write(&env.cfg.database_path, "THIS IS NOT A VALID SQLITE DATABASE FILE")
```

**Assertions (3):**
1. `result.is_ok()` — `list_memories` must not propagate the DB error
2. `output.contains("# Directory: /")` — header is present despite degraded path
3. `stderr.contains("[DEGRADED][SystemTools][LoadLastAccessedTimestamps]")` — diagnostic emitted

**Are the assertions correct?**

- Assertion 1: Correct. The production code returns `Ok(empty_cache)` on connection failure,
  so `list_memories` continues and should succeed.
- Assertion 2: Correct. `list_memories` builds the `"# Directory: /"` header before calling
  `load_last_accessed_timestamps`, so it is always present in the output regardless of DB state.
- Assertion 3: The token `[DEGRADED][SystemTools][LoadLastAccessedTimestamps]` is shared by
  all four emit sites in the function. The test assertion cannot distinguish *which* branch
  fired, only that one of them did. This is acceptable for the purpose of verifying the
  contract (diagnostic is emitted), but is a coverage underspecification.

**Coverage gap:** `load_last_accessed_timestamps` has four independent `emit_system_tools_diagnostic`
call sites:
1. `open_connection` fails — **this is what T1 exercises**
2. `connection.prepare(...)` fails — not covered
3. `statement.query_map(...)` fails — not covered
4. Row iteration error — not covered

Covering branches 2–4 would require a valid SQLite file with a schema that causes `prepare`
or `query_map` to fail (e.g., a table with the wrong column names). These are plausible
real-world failure modes (schema migration left behind a renamed column). Adding one more
test for branch 2 would provide meaningful incremental coverage.

**Name quality:** Accurate and specific — method name and failure condition are both stated.

**Is it load-bearing?** Yes. If the `Err` from `db::open_connection` were propagated instead
of caught (reverting the graceful-degradation contract), this test would fail. If the
diagnostic call were removed, assertion 3 would fail.

**Is it unique?** Yes. No other test file induces a corrupt-database-file scenario against
`list_memories`. `degraded_diag_graph.rs` operates against a valid DB for all three of its
tests.

**Verdict: KEEP.** Consider adding a companion test for the `prepare` failure branch (a valid
DB file whose `file_content` table has been renamed), but do not block on it.

---

### T2 — `get_decay_metadata_for_file_emits_degraded_on_frontmatter_failure`

**Production path:** `memory/listing.rs` → `list_memories()` → `get_decay_metadata_for_file()`
→ `decay/metadata.rs::read_created_date()` → `parse_frontmatter_map()` returns `Err`
→ `emit_system_tools_diagnostic("GetDecayMetadataForFile", ...)` → fallback to
`system_time_to_datetime(file_path)`.

**Setup:**
```rust
write_memory_file(&env, "broken-frontmatter.md",
    "---\ncreated: [not valid yaml\n---\n\n# Broken\n\ncontent");
```

The frontmatter `created: [not valid yaml` is a YAML flow sequence without a closing `]`.
`serde_yaml::from_str` will return `Err`. The degraded path falls back to the file's mtime.

**Assertions (3):**
1. `result.is_ok()` — `list_memories` must succeed
2. `output.contains("📄 broken-frontmatter.md")` — file remains listed (not silently dropped)
3. `output.contains("decay_weight:")` — fallback computed a numeric weight and rendered it
4. `stderr.contains("[DEGRADED][SystemTools][GetDecayMetadataForFile]")` — diagnostic emitted

**Are the assertions correct?**

- Assertion 1: Correct. `read_created_date` catches the YAML parse error and returns
  `Ok(system_time_to_datetime(...))`. The `?` in `get_decay_metadata_for_file` therefore
  does not short-circuit `list_memories`.
- Assertion 2: Correct and important — verifies the file is not silently dropped from the
  listing when its frontmatter is broken.
- Assertion 3: Correct and non-trivial. `decay_weight:` appears in the output only if
  `get_decay_metadata_for_file` returned `Ok` *with a valid `DecayMetadata` struct*. If the
  fallback failed silently and returned `0.0` or `NaN`, this assertion would still pass, but
  it confirms the happy path of the fallback. An assertion on the numeric value (e.g., `> 0`)
  would be tighter but would couple the test to decay weight business logic.
- Assertion 4: Correct.

**Additional subtlety:** The test environment has a valid SQLite DB (initialized by
`TestEnv::setup()`), so `load_last_accessed_timestamps` succeeds and T2 genuinely exercises
the *frontmatter* failure path rather than a DB failure that would mask it.

**Coverage gap:** None beyond the multiple-failure-type question (malformed UTF-8, wrong YAML
types, etc.). The current representative case is sufficient.

**Name quality:** Accurate.

**Is it load-bearing?** Yes. If the YAML parse error in `read_created_date` were propagated
instead of caught (reverting the graceful-degradation contract), `list_memories` would return
`Err` and assertion 1 would fail. If the diagnostic were removed, assertion 4 would fail. If
the fallback `system_time_to_datetime` were broken, assertion 3 would fail.

**Is it unique?** Yes. No other test file induces a malformed-YAML-frontmatter scenario
against `list_memories` to verify the `GetDecayMetadataForFile` degraded path.

**Verdict: KEEP.**

---

### T3 — `find_asset_file_by_id_emits_degraded_on_malformed_json`

**Production path:** `assets/manager.rs::read_asset()` → fast path (filename match) misses
`"missing-id"` → fallback JSON-id scan loop reads `broken.json` → `serde_json::from_str`
returns `Err` → `emit_asset_degraded_diagnostic("FindAssetFileById", &path, &error)` →
`continue` → loop exhausted with no match → `Err(MaenifoldError::AssetNotFound(...))`.

**Setup:**
```rust
let dir = env.cfg.assets_path.join("workflows");
fs::create_dir_all(&dir)?;
fs::write(dir.join("broken.json"), r#"{"id": BROKEN_VALUE_NOT_QUOTED}"#)?;
let manager = AssetManager::new(&env.cfg);
```

The ID `"missing-id"` does not match the filename stem `"broken"` (fast path fails).
The JSON value `BROKEN_VALUE_NOT_QUOTED` is not valid JSON, so the slow path parse fails too.

**Assertions (3):**
1. `Err(MaenifoldError::AssetNotFound(kind, id))` with `kind == "workflow"` and `id == "missing-id"`
2. `stderr.contains("[DEGRADED][AssetResources][FindAssetFileById]")` — diagnostic emitted

**Are the assertions correct?**

- Assertion 1: Correct and precise. Uses a pattern match that verifies *both* the error
  variant *and* its payload values. This pins the user-facing error contract tightly.
- Assertion 2: Correct. The `emit_asset_degraded_diagnostic` call site is at
  `manager.rs:126` and emits exactly this token.

**Subtlety:** T3 only places one malformed file in the directory. The scan loop in `read_asset`
would continue past multiple malformed files, emitting a diagnostic per file. If two malformed
files were present, assertion 2 would still pass. This is a known limitation — multiple-file
degradation is not tested — but is acceptable given the single-file scenario covers the
contractual minimum.

**Subtle correctness question:** The fast path in `read_asset` does `fs::read_to_string(path)?`
(line 121) *without* catching the I/O error. If `broken.json` were unreadable (permission
error), the error would propagate and the test would get `Err(IoError(...))` not
`Err(AssetNotFound(...))`. The test's broken file is readable, so this is correct, but it
means the test implicitly assumes the fast path only emits diagnostics for *parse* failures,
not I/O failures.

**Name quality:** Accurate.

**Is it load-bearing?** Yes. If the `emit_asset_degraded_diagnostic` call at `manager.rs:126`
were removed, assertion 2 fails. If the `Err` from `serde_json::from_str` were propagated
instead of caught, assertion 1 would receive a different error variant.

**Is it unique?** Yes. The `asset_manager_errors.rs` tests cover `update_assets`, not
`read_asset`. The `adopt_boundary.rs` / `adopt_tools_tests.rs` files presumably test the
happy path of `read_adopt_asset`, not the degraded path of `read_asset`.

**Overlap with T4:** Both T3 and T4 exercise JSON parse failure in the asset manager via
different method surfaces (`read_asset` vs `list_assets`/`load_kind_metadata_from_root`).
They emit different diagnostic method names (`FindAssetFileById` vs `GetAssetMetadata`).
They are not redundant.

**Verdict: KEEP.**

---

### T4 — `get_asset_metadata_emits_degraded_on_malformed_json`

**Production path:** `assets/manager.rs::list_assets(Some(AssetKind::Workflow))` →
`load_kind_metadata_from_root()` → iterates `workflows/` directory → `valid.json` parsed OK
→ metadata appended to `items` → `broken.json` parse fails → `emit_asset_degraded_diagnostic`
→ `continue` → returns `Ok(items)` with 1 entry.

**Setup:**
```rust
let dir = env.cfg.assets_path.join("workflows");
fs::create_dir_all(&dir)?;
fs::write(dir.join("valid.json"),
    r#"{"id":"valid-1","name":"Valid Asset","description":"A valid asset"}"#)?;
fs::write(dir.join("broken.json"), r#"{ THIS IS NOT VALID JSON AT ALL }}}"#)?;
let manager = AssetManager::new(&env.cfg);
```

**Assertions (4):**
1. `result.is_ok()` — `list_assets` must succeed despite a broken file
2. `items.len() == 1` — exactly one item: the valid file; broken file silently skipped
3. `items[0]["id"] == "valid-1"` — the returned item is the valid one, payload correct
4. `stderr.contains("[DEGRADED][AssetResources][GetAssetMetadata]")` — diagnostic emitted

**Are the assertions correct?**

- Assertion 1: Correct.
- Assertion 2: Strong and correct. `len() == 1` (not `>= 1`) verifies that exactly the
  broken file was excluded, not that the valid file was included twice.
- Assertion 3: Correct and non-trivial. Probes the JSON response payload directly rather than
  using a string-contains check. Pins the field value to the production extraction path
  (`metadata.insert("id", value.get("id").cloned())` in `load_kind_metadata_from_root`).
  However: the test does *not* assert `items[0]["multiAgent"] == false`. The `multiAgent`
  defaulting behaviour (line 167–176 in `manager.rs`) is a C# parity contract. It is covered
  elsewhere, but this test implicitly under-asserts on a field that is actively populated in
  the same code path.
- Assertion 4: Correct.

**Name quality:** Accurate. `get_asset_metadata` maps clearly to the private
`load_kind_metadata_from_root` function (which is what `list_assets` delegates to).

**Is it load-bearing?** Yes. If the `emit_asset_degraded_diagnostic` call at `manager.rs:157`
were removed, assertion 4 fails. If the `continue` were replaced with error propagation,
assertion 1 fails. If the broken file were silently omitted *and* the valid file count
changed for another reason, assertion 2 catches it.

**Is it unique?** Yes. No other test in the evaluations reviewed combines a mixed-valid/broken
asset directory against `list_assets`. T3 uses only a broken file; T4 uses valid + broken.

**Verdict: KEEP.**

---

## Cross-Cutting Findings

### Finding 1 — Coverage Gap: Three `LoadLastAccessedTimestamps` Branches Untested

`load_last_accessed_timestamps` in `memory/listing.rs` has four independent
`emit_system_tools_diagnostic` call sites. T1 covers only the first (connection-open failure).
The remaining three:

| Branch | Trigger condition |
|--------|-------------------|
| `connection.prepare(...)` fails | Valid SQLite file, `file_content` table renamed/dropped |
| `statement.query_map(...)` fails | Unusual; would require a schema type mismatch |
| Row iteration error | A row becomes unreadable mid-scan |

The connection-open branch is the most common real-world failure (corrupted file, permissions,
disk error). The `prepare` failure branch is realistic under schema drift (e.g., after a failed
migration). **Recommendation:** Add one companion test for the `prepare` failure branch using
a valid SQLite file with a renamed or dropped `file_content` table.

### Finding 2 — `capture_stderr` Has No RAII Guard for fd Restoration on Panic

`capture_stderr` in `support.rs` redirects stderr via `dup2(pipe[1], stderr_fd)`, runs the
operation, then restores via `dup2(saved, stderr_fd)`. If the operation panics, the restore
`dup2` is never called. The raw write-end pipe descriptor (`pipe_fds[1]`) and the saved
stderr descriptor (`saved_stderr`) are leaked. Subsequent tests that use `capture_stderr`
would have their stderr pointing at a dead pipe.

In practice, all four tests in this file call `result.expect(...)` outside the closure (the
closure returns `Result`, not `panic`). But defensive code is better than lucky code.

**Recommendation:** Wrap the fd save/restore in a zero-cost RAII guard struct in `support.rs`
that restores on `drop`. This is a support-module fix, not a test logic fix.

### Finding 3 — Unix-Only Gate Does Not Reflect the Scope of What Is Tested

`#![cfg(unix)]` is required because `capture_stderr` uses POSIX `pipe()`/`dup()`/`dup2()`.
However, the *behaviors under test* — graceful degradation and diagnostic emission for
`LoadLastAccessedTimestamps`, `GetDecayMetadataForFile`, `FindAssetFileById`, and
`GetAssetMetadata` — are all cross-platform. The production `emit_stderr_line` function is
platform-agnostic.

This means Windows CI provides zero coverage of these four degraded-diagnostic contracts.
The gap is structural (inherits from `capture_stderr` design) rather than a test-logic flaw.
**No action required now;** document as a known gap in `support.rs` with a comment.

### Finding 4 — T3 Implicitly Assumes the JSON Id-Scan Fallback Is Reached

T3 seeds only one file (`broken.json`) with stem `"broken"` and looks up `"missing-id"`.
The fast path in `read_asset` compares stem `"broken"` against target `"missing-id"` (both
lowercased) — correctly misses. This means the test depends on the stem comparison in the
fast path doing the right thing. If the fast path were changed to a prefix-match instead of
an exact match (e.g., `"broken".starts_with("miss")` → false, still correct), T3 would still
pass. The fast path logic itself is tested elsewhere.

**Verdict:** Acceptable as written; no action required.

### Finding 5 — T4's `multiAgent` Field Under-Assertion

T4 verifies `items[0]["id"] == "valid-1"` but does not assert `items[0]["multiAgent"] == false`.
The `multiAgent` field is unconditionally inserted by `load_kind_metadata_from_root` (lines
167–176) and defaults to `false` when absent in the JSON. This is a C# parity contract.
If the field defaulting were accidentally broken (e.g., the field were omitted), T4 would
not catch it.

**Recommendation:** Add `assert_eq!(items[0]["multiAgent"], Value::Bool(false));` to T4.
Low effort, pins the parity contract.

---

## Recommendations

Ordered by priority:

1. **[Medium] Add companion test for `LoadLastAccessedTimestamps` prepare-failure branch.**
   Seed a valid SQLite file that is missing the `file_content` table. Verify graceful
   degradation and `[DEGRADED][SystemTools][LoadLastAccessedTimestamps]` emission. This covers
   a realistic real-world failure mode (schema migration artifact). Suggested name:
   `load_last_accessed_timestamps_emits_degraded_on_missing_table`.

2. **[Low] Add `multiAgent` assertion to T4.** One additional line in
   `get_asset_metadata_emits_degraded_on_malformed_json`:
   ```rust
   assert_eq!(items[0]["multiAgent"], Value::Bool(false));
   ```
   Pins the C# parity contract that `multiAgent` defaults to `false` when absent.

3. **[Low] Add RAII fd-restoration guard to `capture_stderr` in `support.rs`.** Prevents
   stderr corruption in the test harness if a future test closure panics. A zero-line change
   to test logic; the fix lives entirely in the support module.

4. **[Info] Document `#![cfg(unix)]` scope limitation in `support.rs`.** Add a comment near
   `capture_stderr` noting that the behaviors tested here are cross-platform, but the fd
   redirection mechanism is Unix-only, and Windows CI has no coverage of these degraded paths.

---

## Load-Bearing Score: 4/4

| Test | Load-Bearing? | Reasoning |
|------|:---:|-------|
| T1 `load_last_accessed_timestamps_emits_degraded_on_db_failure` | ✅ | Only test that exercises the `open_connection` failure branch in `load_last_accessed_timestamps`; would catch re-introduction of error propagation |
| T2 `get_decay_metadata_for_file_emits_degraded_on_frontmatter_failure` | ✅ | Only test that verifies malformed-YAML frontmatter does not suppress a file from `list_memories` output; fallback decay weight computation confirmed |
| T3 `find_asset_file_by_id_emits_degraded_on_malformed_json` | ✅ | Only test for the `FindAssetFileById` degraded path in `read_asset`; pins the `AssetNotFound` error contract under parse failure |
| T4 `get_asset_metadata_emits_degraded_on_malformed_json` | ✅ | Only test for the `GetAssetMetadata` degraded path in `list_assets`; uniquely combines valid + broken files to verify the skip-and-continue contract |
