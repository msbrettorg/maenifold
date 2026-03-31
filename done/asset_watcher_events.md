---
id: asset_watcher_events
status: done
---

# Test Evaluation: asset_watcher_events.rs

> **Traceability**: T-0.1 · NFR-1 (C# Oracle Parity) · NFR-2 (Test Coverage) · NFR-3 (No Regressions)
> **Color**: 🔘 Gray — Skeptical Inquiry
> **File evaluated**: `crates/maenifold-core/tests/asset_watcher_events.rs`
> **Support modules**: `tests/asset_watcher/support.rs`, `tests/asset_watcher/runtime.rs`

---

## Summary

This file contains **four integration tests** covering the four fundamental filesystem event types
the asset watcher must detect: Create, Modify, Remove, and Rename. All four were ported from the
C# oracle under T-0.1 (NFR-3) and extracted into a focused file under T-12.7 (NFR-1). The test
harness (`AssetWatcherHarness`) is solid — it uses `TempDir`, a scoped `Config` override with
RAII reset, a `PollWatcher` with 50ms poll interval, and a `TEST_MUTEX` to serialize execution.

**Gray concern: the tests are load-bearing and mostly correct, but all four carry the same structural
flaw** — each wastes at least one assertion verifying a filesystem operation that the test itself
just performed. This is testing `std::fs` or the OS, not the SUT. Additionally, the rename test
is the weakest of the four: it bakes in an OS-event-count assumption (`wait_for_notifications(2,
...)`), uses weaker OR-logic in its batch assertion, and fails to verify the old asset is no longer
readable through `AssetManager` (an inconsistency with the delete test).

The tests do **not** verify the debounce coalescing behavior that gives the watcher its name — they
verify that *an* event arrives, not that a *burst* of events is coalesced into a single notification.
This is a coverage gap for NFR-1 (parity with the C# oracle's debounce semantics), but that concern
belongs in `asset_watcher_timing.rs`.

---

## Verdicts

| # | Test Name | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| 1 | `file_created_triggers_debounce_and_notification` | ✅ | ✅ | ⚠️ | ✅ | 🔘 | **REWRITE** |
| 2 | `file_changed_triggers_debounce_and_notification` | ✅ | ✅ | ⚠️ | ✅ | 🔘 | **REWRITE** |
| 3 | `file_deleted_triggers_debounce_and_notification` | ✅ | ✅ | ⚠️ | ✅ | 🔘 | **REWRITE** |
| 4 | `file_renamed_triggers_debounce_and_notification` | ✅ | ✅ | ❌ | ✅ | 🔘 | **REWRITE** |

**Legend**: ✅ Yes · ❌ No · ⚠️ Mostly · 🔘 Neutral

---

## Per-Test Analysis

---

### Test 1 — `file_created_triggers_debounce_and_notification`

**What it does**: Creates a `.json` workflow file, waits for the debounced batch to include that
path, then reads the asset through `AssetManager` to confirm the watcher-triggered state is
consistent.

**What it proves**:
- A `Create` event on a watched `.json` file appears in a debounced batch.
- `AssetManager` can read the created asset after the event fires.

**Assertion audit**:

```rust
assert!(test_file.exists(), "file should exist after creation");
```
❌ **Redundant.** The test just called `fs::write(…).expect(…)`. If the write succeeded
(panicking otherwise), the file exists. This assertion cannot fail unless the OS deletes the file
between the `write` and the `assert`, which is not a watcher concern and would mask a real
failure with a misleading message. *Remove it.*

```rust
assert!(batch.iter().any(|path| path == &test_file), …);
```
✅ **Load-bearing.** This is the primary watcher assertion.

```rust
assert_eq!(asset["name"], "test-workflow");
assert_eq!(asset["version"], "1.0.0");
```
⚠️ **Partially load-bearing.** These verify `AssetManager::read_asset` round-trips correctly,
but that concern belongs in the `asset_manager_*` test files. Here they add assurance that the
watcher's notification does not corrupt or pre-empt a subsequent read, which is legitimate. Keep,
but document intent.

**Parameterizable?** The file content fields could be parameterized, but the event type (Create)
is the axis of uniqueness. Content variation would not increase event-coverage confidence.

**Verdict: REWRITE** — drop the `exists()` assertion; add a comment clarifying why the
`AssetManager` assertions are present (not just content validation, but read-after-notify
consistency).

---

### Test 2 — `file_changed_triggers_debounce_and_notification`

**What it does**: Writes a file, drains the initial creation event, then overwrites the file and
waits for a second batch containing the modified path. Reads through `AssetManager` to confirm
updated content is visible.

**What it proves**:
- A `Modify` event on an already-watched `.json` file appears in a debounced batch.
- The initial create event is properly consumed before the modify event is tested (avoids conflation).
- `AssetManager` reflects the updated content after the event fires.

**Assertion audit**:

```rust
let content = fs::read_to_string(&test_file).expect("read updated asset");
assert!(content.contains("2.0.0"), "file should contain updated content");
```
❌ **Redundant.** The test just wrote `{"version":"2.0.0"}` and expected the write to succeed.
This asserts that `std::fs::write` + `std::fs::read_to_string` are symmetric, which is not under
test. *Remove it.*

```rust
assert!(batch.iter().any(|path| path == &test_file), …);
```
✅ **Load-bearing.** Primary watcher assertion.

```rust
assert_eq!(asset["version"], "2.0.0");
```
⚠️ **Mostly load-bearing.** As with Test 1, verifies read-after-modify consistency; arguably
belongs in `asset_manager_update_detection.rs` but is defensible here.

**Design note**: The `wait_for_notifications(1, …)` call before the second write is thoughtful —
it prevents the creation event from merging with the modification event in a single debounce window.
This is correct test design and should be preserved.

**Verdict: REWRITE** — remove the `content.contains("2.0.0")` assertion.

---

### Test 3 — `file_deleted_triggers_debounce_and_notification`

**What it does**: Creates a file, drains its creation event, deletes the file, then waits for a
batch containing the deleted path. Asserts `AssetManager` returns `AssetNotFound` for the deleted
path.

**What it proves**:
- A `Remove` event on a watched `.json` file appears in a debounced batch, including the path
  of the now-missing file.
- `AssetManager` returns the correct `MaenifoldError::AssetNotFound` variant for a deleted asset.

**Assertion audit**:

```rust
assert!(!test_file.exists(), "file should not exist after deletion");
```
❌ **Redundant.** The test just called `fs::remove_file(…).expect(…)`. If deletion failed, the
test would have panicked on the expect. *Remove it.*

```rust
assert!(batch.iter().any(|path| path == &test_file), …);
```
✅ **Load-bearing.** The deleted path appearing in the batch is a non-trivial assertion — the
watcher must surface the path of an event even when the file no longer exists at delivery time.

```rust
assert!(
    matches!(&error, MaenifoldError::AssetNotFound(kind, id)
        if kind == "workflow" && id == "test-delete"),
    …
);
```
✅ **Load-bearing.** Verifies both the error variant and the discriminant values. This is more
precise than `is_err()` and catches regressions in error construction.

**Verdict: REWRITE** — remove `!test_file.exists()` assertion; everything else is sound.

---

### Test 4 — `file_renamed_triggers_debounce_and_notification`

**What it does**: Creates a file, drains its creation event, renames it to a new path, waits for
two notifications, drains all batches, then checks that either the old or new path appears in any
batch.

**What it proves** (weakly): Something in the rename operation surfaced in the event stream.

**Assertion audit**:

```rust
assert!(new_file.exists(), "renamed file should exist at new path");
assert!(!old_file.exists(), "old path should not exist after rename");
```
❌ **Redundant.** The test just called `fs::rename(…).expect(…)`. If the rename failed, the test
would have panicked. *Remove both.*

```rust
harness.watcher.wait_for_notifications(2, Duration::from_secs(5));
```
❌ **Fragile OS assumption.** On Linux `inotify`/`fanotify`, a rename produces two events
(`MOVED_FROM` + `MOVED_TO`). On macOS `kqueue`/`FSEvents`, the behavior varies — a rename may
emit a single event or two, depending on whether `PollWatcher` sees them as distinct poll deltas.
Since this harness uses `PollWatcher` with 50ms intervals, successive polls may see the rename
as one diff (old path gone, new path appeared) or as two separate transitions depending on timing.
Waiting for exactly 2 notifications is an unstated OS contract that will bite CI on edge cases.

```rust
assert!(
    batches.iter().flatten().any(|path| path == &old_file || path == &new_file),
    …
);
```
❌ **Weak OR logic.** The `||` means this assertion passes if *either* path appears in *any*
batch. An implementation that only emits the `Remove` event (and drops the `Create`) would still
pass. The test does not assert that `test-new-name` is actually observable — only that something
path-like appeared.

```rust
assert!(
    harness.asset_manager().read_asset(AssetKind::Workflow, "test-new-name").is_ok(),
    …
);
```
✅ **Load-bearing** — this is actually the most valuable assertion in the test. It verifies the
new name is resolvable through `AssetManager`. But note:

**Missing assertion**: The test does not verify that `test-old-name` is *no longer* accessible
— an inconsistency with the delete test, which explicitly checks for `AssetNotFound`. After a
rename, reading the old name should either return `AssetNotFound` or the new content. This gap
leaves a silent regression window.

**Verdict: REWRITE** — this test needs the most work:
1. Remove `exists()` / `!exists()` redundant OS assertions.
2. Replace `wait_for_notifications(2, …)` + `drain_batches()` with
   `wait_for_batch_with_path(&new_file, …)` (consistent with Tests 1–3 and OS-agnostic).
3. Split the batch assertion into two explicit checks: new path IS in a batch AND old path is
   also surfaced (or accept a single batch containing both).
4. Add `assert!(harness.asset_manager().read_asset(AssetKind::Workflow, "test-old-name").is_err())`
   to match the delete test's pattern.

---

## Cross-Cutting Concerns

### Structural

- **Serialized via `TEST_MUTEX`**: All four tests cannot run in parallel. Given that each test
  uses a fresh `TempDir` with unique paths, parallelism should be safe — but the mutex is
  inherited from older harness design. *Whether to remove it requires confirming `Config::override_root`
  / `Config::set_database_path` are thread-local or use a per-test scope, not a global.*

- **Timing budget**: Each test waits up to 5s. With 200ms settle + 150ms debounce + 50ms poll
  jitter, each test typically takes ~500–800ms. Four tests × serial = ~3–4s minimum. This is
  acceptable for integration tests but should not regress further.

### Coverage Gaps

- **Debounce coalescing**: None of these tests verify that a burst of writes produces exactly
  one notification (the defining property of a debouncer). This is presumably covered in
  `asset_watcher_timing.rs`, but if not, NFR-1 parity is incomplete.

- **Non-JSON files**: No test confirms that `.txt`, `.yaml`, or no-extension files do *not*
  trigger notifications. The `is_watchable_asset_json_path` filter in `runtime.rs` handles this
  but is untested here.

- **Hidden path filtering**: `is_hidden_relative_path` in `runtime.rs` is also untested by these
  four tests. A `.hidden/file.json` should be silently ignored.

- **`EventKind::Any`**: `is_relevant_event_kind` explicitly includes `EventKind::Any` as a
  relevant kind. No test exercises this path.

- **Subdirectory events**: All tests operate in `workflows_path` directly. The harness watches
  recursively (`RecursiveMode::Recursive`) — nested directory events are untested.

---

## Recommendations

| Priority | Action | Rationale |
|----------|--------|-----------|
| 🔴 High | **REWRITE Tests 1–4**: remove all redundant `exists()` / `!exists()` / `content.contains()` assertions | These assertions test the OS, not the SUT; they add noise and can mask real failures |
| 🔴 High | **REWRITE Test 4**: replace `wait_for_notifications(2, …)` + `drain_batches()` with `wait_for_batch_with_path(&new_file, …)` | Current design is fragile across OS/filesystem implementations |
| 🔴 High | **REWRITE Test 4**: add `AssetNotFound` assertion for old name after rename | Inconsistency with delete test leaves silent regression window |
| 🟡 Medium | **Add Test**: burst-write → single notification (debounce coalescing) | NFR-1 core behavior; currently unverified unless covered in `asset_watcher_timing.rs` |
| 🟡 Medium | **Add Test**: non-JSON file write does not trigger notification | `is_watchable_asset_json_path` filter is untested in event context |
| 🟡 Medium | **Add Test**: `.hidden/file.json` is not surfaced | `is_hidden_relative_path` is untested from event context |
| 🟢 Low | Evaluate `TEST_MUTEX` necessity | If `Config` overrides are scoped per `TempDir`, tests can run in parallel, improving CI time |
| 🟢 Low | Add `EventKind::Any` coverage | The `is_relevant_event_kind` match arm for `Any` is never exercised |

---

## Load-Bearing Score: 4/4

All four tests are load-bearing — each covers a distinct, irreplaceable filesystem event type
(Create, Modify, Remove, Rename). None can be removed without losing meaningful coverage of the
`[[asset-watcher]]` event pipeline. The **REWRITE** verdicts reflect assertion hygiene issues
and fragility in Test 4, not test purpose.

> **Key [[test-quality]] signals**: redundant OS assertions erode signal-to-noise ratio; the
> rename test's OS-event-count assumption is the highest-risk latent failure in this file.
> Fixing assertion hygiene across all four tests is low-effort, high-value work tied to NFR-2
> and NFR-4.
