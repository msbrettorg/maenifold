---
id: asset_hot_loading_tests
status: done
pipeline: tdd
---

# Test Evaluation: asset_hot_loading_tests.rs

**Traceability**: T-0.1 · NFR-3 · NFR-2  
**Evaluator**: Red Team / Gray (Skeptical Inquiry)  
**File**: `crates/maenifold-core/tests/asset_hot_loading_tests.rs`  
**Tests evaluated**: 3

---

## Summary

These three tests are presented as ports of `AssetHotLoadingTests.cs` and claim to verify that
the asset subsystem responds to filesystem lifecycle events (create, modify, delete). They do not.

**The central problem**: The `AssetManager` has no background watcher, no cache, and no resource
registry. `read_asset()` is a synchronous, on-demand `fs::read_to_string()` call. There is nothing
to "hot load." The 200 ms `thread::sleep` calls that punctuate every test are cargo-cult timing
borrowed from the C# original, where a watcher-backed MCP cache *did* require settling time. In
Rust, the sleep buys nothing and silently communicates a mechanism that does not exist.

The real hot-loading tests live in `asset_watcher_events.rs`, which uses an actual
`AssetWatcherHarness`, proper `wait_for_batch_with_path()` synchronization, and notification-count
assertions. Every behavioral scenario covered here — create, modify, delete — is covered there with
higher fidelity, better synchronization, and no tautological intermediate assertions.

**Secondary problems** identified across all three tests:

- Intermediate `fs::read_to_string` assertions are trivially true immediately after a synchronous
  `fs::write`. Asserting that the file you just wrote contains what you wrote is noise, not signal.
- `assert!(test_file.exists())` immediately after `fs::write` is only falsifiable by an OS-level
  fault that would abort the test anyway.
- Test names use "adds resource", "updates resource", "removes resource" — language implying a
  registry or cache, neither of which exists. They test filesystem read round-trips.
- The file name `asset_hot_loading_tests.rs` is a historical artifact. It will mislead every future
  reader about the presence or absence of a watcher/cache invalidation system.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `file_created_adds_resource_to_asset_manager` | ✗ | ✗ | ✗ | ✗ | Yes (with #2) | **REMOVE** |
| 2 | `file_changed_updates_resource_in_asset_manager` | ✗ | ✗ | ✗ | ✗ | Yes (with #1) | **REMOVE** |
| 3 | `file_deleted_removes_resource_from_asset_manager` | Marginal | Marginal | Partial | ✗ | No | **REMOVE** |

---

## Per-Test Analysis

### Test 1 — `file_created_adds_resource_to_asset_manager`

```rust
// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileCreatedAddsResourceToMcpServer.
#[test]
fn file_created_adds_resource_to_asset_manager() { ... }
```

**What it claims to test**: That creating a file in the workflows directory "adds a resource to the
asset manager," implying some kind of registration or notification mechanism.

**What it actually tests**: That `AssetManager::read_asset(AssetKind::Workflow, "test-workflow")`
returns the correct JSON after the caller writes that file to disk.

**Assertion audit**:

| Assertion | Value |
|-----------|-------|
| `assert!(test_file.exists())` | ❌ Tautological — you just wrote it |
| `assert_eq!(fs::read_to_string(&test_file), test_content)` | ❌ Tautological — nothing has modified it |
| `assert_eq!(asset["name"], "test-workflow")` | ✅ Meaningful — exercises `read_asset` parse |
| `assert_eq!(asset["version"], "1.0.0")` | ✅ Meaningful — but redundant with name check |

**The sleep problem**: `thread::sleep(Duration::from_millis(200))` after `fs::write`. `read_asset`
calls `fs::read_to_string` synchronously. There is no async path, no watcher, no debounce buffer.
The sleep cannot influence the outcome. It actively misleads: it implies a system that isn't there.

**Superseded by**: `asset_watcher_events.rs::file_created_triggers_debounce_and_notification` —
identical `read_asset(AssetKind::Workflow, "test-create")` call with real watcher synchronization
and notification-count assertions. Strictly stronger.

**Verdict**: **REMOVE**. Zero unique coverage. The two meaningful assertions are a proper subset of
what `asset_watcher_events.rs` already asserts with correct synchronization.

---

### Test 2 — `file_changed_updates_resource_in_asset_manager`

```rust
// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileChangedUpdatesResourceInMcpServer.
#[test]
fn file_changed_updates_resource_in_asset_manager() { ... }
```

**What it claims to test**: That modifying a file propagates an update through the asset manager.

**What it actually tests**: That `read_asset(AssetKind::Role, "modify-test")` returns updated JSON
after the caller overwrites the file.

**Assertion audit**:

| Assertion | Value |
|-----------|-------|
| `assert_eq!(read_content, updated_content)` | ❌ Tautological — synchronous write, nothing else touched it |
| `assert!(read_content.contains("modified-role"))` | ❌ Fully subsumed by the `assert_eq!` above |
| `assert!(!read_content.contains("original-role"))` | ❌ Fully subsumed by the `assert_eq!` above |
| `assert_eq!(asset["name"], "modified-role")` | ✅ Meaningful |
| `assert_eq!(asset["version"], "2.0.0")` | ✅ Meaningful, but name check suffices |

**Three assertions in a row** on `read_content` that are all logically equivalent to `assert_eq!(read_content, updated_content)`. This is three times the assertion text for one unit of information.

**Unique coverage claim** — `AssetKind::Role` vs `AssetKind::Workflow`:
The `read_asset` code path for Role differs from Workflow only in the subdirectory resolved by
`kind.dir_name()` → `"roles"` vs `"workflows"`. There is no conditional logic, no trait dispatch,
no type parameter on the reader. The kind difference contributes zero behavioral coverage.

**Superseded by**: `asset_watcher_events.rs::file_changed_triggers_debounce_and_notification` —
covers the same write → read pattern for Workflow kind with real watcher notification assertions.

**Verdict**: **REMOVE**. Three tautological assertions and a cargo-cult sleep dominate the test.
The only valuable assertions are duplicated with better tooling in `asset_watcher_events.rs`.

---

### Test 3 — `file_deleted_removes_resource_from_asset_manager`

```rust
// T-0.1: RTM NFR-3 - Ports AssetHotLoadingTests.cs FileDeletedRemovesResourceFromMcpServer.
#[test]
fn file_deleted_removes_resource_from_asset_manager() { ... }
```

**What it claims to test**: That deleting a file removes it from the asset manager.

**What it actually tests**: That `read_asset(AssetKind::Color, "delete-test")` returns
`MaenifoldError::AssetNotFound("color", "delete-test")` after the backing file is deleted.

**Assertion audit**:

| Assertion | Value |
|-----------|-------|
| `assert!(test_file.exists())` before deletion | ⚠️ Marginally useful as guard, mostly noise |
| `assert!(read_asset(...).is_ok())` before deletion | ⚠️ Marginally useful as pre-condition baseline |
| `assert!(!test_file.exists())` after deletion | ❌ Tautological — `fs::remove_file` just succeeded |
| `matches!(&error, MaenifoldError::AssetNotFound(kind, id) if kind == "color" && id == "delete-test")` | ✅ Specific and correct |

**The one genuinely good assertion in this file** is the error variant match in test 3. It checks
the exact discriminant, the exact kind string, and the exact id string. This is what all three
tests should aspire to.

**Unique coverage** — `AssetKind::Color` error path:
`asset_watcher_events.rs::file_deleted_triggers_debounce_and_notification` covers the same error
pattern for `AssetKind::Workflow`. The error is emitted from the same code path:

```rust
Err(MaenifoldError::AssetNotFound(
    kind.as_str().to_string(),  // "color" vs "workflow" — same code, different string
    id.to_string(),
))
```

`degraded_diag_assets.rs` also tests `read_asset(AssetKind::Workflow, "missing-id")` for
`AssetNotFound`. The Color kind variant is marginally unique but tests no distinct code path —
only a different string produced by `kind.as_str()`.

**Verdict**: **REMOVE**. The watcher events file covers the same error pattern with a real
watcher harness. The only thing unique here — the "color" kind string in the error — is not
a meaningful behavioral distinction. The pre-deletion `is_ok()` check is the only useful setup
guard and is already implied by the test structure.

---

## Recommendations

### 1. Delete `asset_hot_loading_tests.rs` in its entirety

The file has been superseded. Keeping it:
- Teaches future maintainers that a watcher/cache exists when it does not
- Adds 3× 200 ms of artificial wall-clock time per test run for zero signal
- Perpetuates tautological assertion patterns that dilute assertion discipline

Before deletion, confirm that **no test in this file is the only test** exercising a code path not
covered by `asset_watcher_events.rs`, `degraded_diag_assets.rs`, or `contracts.rs`. Based on this
analysis, no such orphaned path exists — but run `cargo test` with and without the file to confirm.

### 2. Add `AssetKind::Role` and `AssetKind::Color` coverage to `asset_watcher_events.rs` if gap matters

If the team determines that per-kind happy-path coverage is required (it is not unreasonable to
want one smoke test per `AssetKind` variant), add a **parameterized** test to `asset_watcher_events.rs`:

```rust
// Sketch — assert all four AssetKind variants are readable via the same code path
#[test]
fn read_asset_resolves_each_kind_from_its_subdirectory() {
    for (kind, dir, id) in [
        (AssetKind::Workflow, "workflows", "wf-kind-check"),
        (AssetKind::Role,     "roles",     "role-kind-check"),
        (AssetKind::Color,    "colors",    "color-kind-check"),
        (AssetKind::Perspective, "perspectives", "persp-kind-check"),
    ] {
        let harness = AssetHotLoadingHarness::new();
        let file = harness.config.assets_path.join(dir).join(format!("{id}.json"));
        fs::write(&file, format!(r#"{{"id":"{id}"}}"#)).expect("write");
        let asset = harness.asset_manager().read_asset(kind, id).expect("read");
        assert_eq!(asset["id"], id);
    }
}
```

No sleep. No intermediate filesystem assertions. No misleading name.

### 3. Purge cargo-cult sleeps as a general pattern

The `thread::sleep(Duration::from_millis(200))` idiom without a corresponding async mechanism is
a code smell that should be linted or reviewed whenever it appears. Sleeps are only justified when
they are waiting for a background system (watcher, debounce, timer). Mark the ones in
`asset_watcher_events.rs` and `asset_watcher_timing.rs` as intentional with inline comments
explaining *what* they are waiting for.

### 4. Rename the concept of "hot loading" to "watcher events" in comments

The T-0.1 comment on these tests references `AssetHotLoadingTests.cs`. If that C# original is the
authoritative oracle, the C# test description should be updated in RTM.md to reflect that the Rust
implementation is a synchronous filesystem reader, not a hot-reload cache.

---

## Load-Bearing Score: 1/3

Only test 3 has a single load-bearing assertion (the `MaenifoldError::AssetNotFound` variant
match), and even that assertion is covered by `asset_watcher_events.rs` for a different
`AssetKind`. The other two tests assert nothing that is not either tautological or fully covered
elsewhere.

The file earns a **1/3** — not zero, because the error variant match in test 3 is genuinely
correct and specific — but that one assertion does not justify the cost of the file, its
misleading name, its cargo-cult sleeps, or its tautological preamble.

**Recommended action**: Delete the file. Optionally add a single parameterized `read_asset_resolves_each_kind_from_its_subdirectory` test to `asset_watcher_events.rs` for exhaustive kind coverage.

---

*Evaluation produced under [[Gray]] (Skeptical Inquiry). References: [[test-quality]], [[maenifold-rs]], [[AssetManager]], [[asset-watcher]]*

---
## SWE Remediation
- REMOVE file_created_adds_resource_to_asset_manager: superseded by asset_watcher_events::file_created_triggers_debounce_and_notification
- REMOVE file_changed_updates_resource_in_asset_manager: superseded by asset_watcher_events::file_changed_triggers_debounce_and_notification
- REMOVE file_deleted_removes_resource_from_asset_manager: superseded by asset_watcher_events::file_deleted_triggers_debounce_and_notification
- Deleted `asset_hot_loading_tests.rs` entirely (all 3 verdicts REMOVE, 0 unique coverage)
- Removed `mod asset_hot_loading_tests;` from `main.rs`
- Verified: `cargo test -p maenifold-core --test integration_tests -- asset_watcher_events` — 4 passed, 0 failed
- Verified: `cargo test -p maenifold-core --test integration_tests -- asset_hot_loading` — 0 tests found (confirmed removal)
- Commit: 3aa2b1ebc6468eef0f2c8ea6123080a1bbc1e8b1

---
## Red-team Audit

**Traceability**: T-0.1 · NFR-3 · NFR-2

### Per-verdict verification

- ✅ REMOVE `file_created_adds_resource_to_asset_manager`: File deleted. Superseding test `asset_watcher_events::file_created_triggers_debounce_and_notification` (line 12) covers all meaningful assertions (`read_asset` parse, name/version equality) plus real watcher synchronization via `wait_for_batch_with_path()`. No unique coverage lost.
- ✅ REMOVE `file_changed_updates_resource_in_asset_manager`: File deleted. Superseding test `asset_watcher_events::file_changed_triggers_debounce_and_notification` (line 40) covers write→read round-trip with watcher notification. The eval explicitly ruled `AssetKind::Role` vs `AssetKind::Workflow` as zero behavioral distinction (same code path, different `dir_name()` string). No unique coverage lost.
- ✅ REMOVE `file_deleted_removes_resource_from_asset_manager`: File deleted. Superseding test `asset_watcher_events::file_deleted_triggers_debounce_and_notification` (line 74) asserts the same `MaenifoldError::AssetNotFound` variant match pattern. The eval explicitly ruled `AssetKind::Color` vs `AssetKind::Workflow` as no distinct code path. No unique coverage lost.

### Structural checks

- ✅ `mod asset_hot_loading_tests;` removed from `main.rs` — confirmed via git diff (single `-` line) and `grep` (zero matches).
- ✅ Commit `3aa2b1ebc6468eef0f2c8ea6123080a1bbc1e8b1` touches exactly 3 files: the deleted test file, `main.rs` (1 line), and this eval spec. No unrelated changes.
- ✅ Superseding tests independently verified: `cargo test -- asset_watcher_events` → 4 passed, 0 failed.
- ✅ Removed tests confirmed absent: `cargo test -- asset_hot_loading` → 0 tests found.

### Informational

- ⚠️ Stale comment in `asset_manager_support/mod.rs:16` still references `asset_hot_loading_tests` by name. This is a comment in a `NOTE` block about cross-binary mutex gaps, not a code dependency. Does not affect compilation or behavior. Not a rejection issue per audit scope (eval is the spec), but should be cleaned up in a future pass.

### Verdict: **PASS**

All 3 REMOVE verdicts executed correctly. Superseding tests in `asset_watcher_events.rs` cover every meaningful assertion from the deleted file with strictly stronger synchronization (real watcher vs cargo-cult sleep). No unique coverage lost. No unrelated changes introduced. Commit is clean and well-scoped.

---
## Blue-team Verification
- `cargo test --test asset_hot_loading_tests`: no test target found (binary correctly removed)
- File `crates/maenifold-core/tests/asset_hot_loading_tests.rs` confirmed absent
- All 3 tests were REMOVE verdicts; entire file deleted by SWE — no binary should exist
- Verdict: PASS
