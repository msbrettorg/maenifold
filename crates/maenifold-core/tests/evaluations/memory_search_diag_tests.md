# Test Evaluation: memory_search_diag_tests
**File:** `crates/maenifold-core/tests/memory_search_diag_tests.rs`
**Test count:** 5
**Evaluated:** 2025-07-17
**Traceability:** T-QUAL-002.6a · RTM FR-25.2 (completed; see CHANGELOG) · NFR-1

---

## Summary

Five tests guard two behaviorally distinct subsystems: the decay-weight fallback pipeline (`get_decay_weight_for_file` and its degraded DB and disk paths) and the snippet-display fallback pipeline (`get_file_display_info` and its path-redaction security property). All five cover real, observable failure modes that no other test file targets at the unit level. The suite is therefore broadly load-bearing.

However, the file has four structural problems that reduce confidence:

1. **Infrastructure duplication.** `capture_stderr`, `isolated_env()`, and `TEST_MUTEX` are private reimplementations of the shared infrastructure in `crates/maenifold-core/tests/degraded_diag/support.rs`. They are byte-for-byte equivalent. A future change to the pipe/dup2 ceremony must be made in at least three places.

2. **Stale traceability annotation.** Every test comment reads `// T-QUAL-002.6a: RTM FR-25.2`. In the *current* PRD and RTM, FR-25 is "Azure Deployment Manifests" (T-19.x, RTM-137–140). The annotation is a ghost of a prior requirement numbering that was overloaded when the sprints were renumbered. A maintainer reading the comment and checking the live RTM will find no matching requirement — creating false confidence that the test is traceable when the trace is actually broken.

3. **AC2 violates single-responsibility.** `fr25_2_ac2_success_path_behavior_remains_unchanged` tests two unrelated functions in one closure, with assertions so loose that a freshly-created file returning `weight = 0.001` would still pass. The composition inflates apparent coverage while reducing failure locality.

4. **Two tests share the `ac1` infix.** Both fallback tests are named `fr25_2_ac1_*`, which implies they test the same acceptance criterion. They test different functions. The naming ambiguity is benign today but adds cognitive overhead.

---

## Verdicts

| Test Function | Load-Bearing? | Unique? | Correct Assertions? | Good Name? | Parameterizable? | Verdict |
|---|---|---|---|---|---|---|
| `fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal` | ✅ Yes | ✅ Yes | ✅ Mostly | ⚠️ `ac1` infix shared | — | **KEEP** |
| `fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal` | ✅ Yes | ✅ Yes | ⚠️ Weak on snippet | ⚠️ `ac1` infix shared | — | **REWRITE** |
| `fr25_2_ac2_success_path_behavior_remains_unchanged` | ⚠️ Partial | ⚠️ Partial | ❌ Weight bounds too loose | ✅ Yes | ❌ Should be split | **REWRITE** |
| `fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic` | ✅ Yes | ✅ Yes | ✅ Solid | ✅ Yes | — | **KEEP** |
| `fr25_2_ac4_path_diagnostics_redacts_absolute_paths` | ✅ Yes | ✅ Yes | ✅ Strong (positive + negative) | ✅ Yes | — | **KEEP** |

---

## Per-Test Analysis

### `fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal`

**What it tests:** `get_decay_weight_for_file` with a `memory://` URI that does not exist in the DB or on disk. The expected path is: DB lookup succeeds (no row) → `get_decay_weight_from_disk` → `read_created_date_for_identifier` → file absent → `emit_degraded_decay_diagnostic("GetDecayWeightFromDisk", "Falling back to neutral decay weight", ...)` → return `1.0`.

**Load-bearing:** Yes. No other test in the suite exercises this exact code path. `sync_perf_indexing.rs` tests `get_decay_weight_for_file` with a *valid* indexed file (the success path), not the fallback.

**Unique:** Yes.

**Assertions:** `assert_eq!(weight, 1.0)` is exact and correct. The two `contains` checks on `stderr` are appropriate for log strings — pinning exact format would be unnecessarily brittle. No missing negative assertion (there is nothing to verify *absent* in the success direction here).

**Name:** The `ac1` infix is shared with the snippet test below. These are not the same AC — one is about the decay subsystem, the other about the snippet subsystem. The `fr25_2` prefix is stale (see structural issues). Minor naming confusion only.

**Verdict: KEEP.** No behavioral changes needed. Address the `ac1` ambiguity as part of the structural cleanup described in Recommendations.

---

### `fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal`

**What it tests:** `get_file_display_info` with a non-existent `memory://` URI. Expected path: `read_markdown` fails → `Err(_)` arm → `emit_stderr_line("[DEGRADED]...")` → return `(fallback_title, "[DEGRADED] Snippet unavailable")`.

**Load-bearing:** Yes. No other test file calls `get_file_display_info` on a non-existent URI at this level of diagnostic granularity.

**Unique:** Yes.

**Assertions:**
- `assert_eq!(title, "non-existent-file")` — correct and exact. ✅
- `assert!(snippet.starts_with("[DEGRADED] Snippet unavailable"))` — **weak**. The source returns exactly `"[DEGRADED] Snippet unavailable".to_string()` — no suffix is ever appended. `starts_with` implies there might be additional content, which there isn't. This should be `assert_eq!(snippet, "[DEGRADED] Snippet unavailable")`. A future developer who adds context to the return value will produce a silent false pass if they forget to update the test.
- Both `stderr.contains` checks are appropriate.

**Name:** Shares `ac1` with the decay test. See Recommendations.

**Verdict: REWRITE.** Change `starts_with` to `assert_eq!` for the snippet value. Rename to distinguish from the decay `ac1` test.

---

### `fr25_2_ac2_success_path_behavior_remains_unchanged`

**What it tests:** Both `get_file_display_info` AND `get_decay_weight_for_file` against a valid, freshly written memory note — in a single test, in a single `capture_stderr` closure.

**Load-bearing:** Partial. `get_file_display_info` success-path coverage is thin elsewhere; this test is the primary guard for that path. `get_decay_weight_for_file` success paths are covered more thoroughly by `sync_perf_indexing.rs`.

**Unique:** Partially unique for the snippet success path. Not unique for the decay success path.

**Assertions:**

- `assert_eq!(title, "Valid Memory Note")` — correct and exact. ✅
- `assert!(snippet.contains("targeted query token"))` — appropriate: snippet extraction is positional and the exact output can depend on context windows; `contains` is correct here. ✅
- `assert!(!snippet.contains("[DEGRADED]"))` — necessary and correct. ✅
- `assert!(weight > 0.0)` and `assert!(weight <= 1.0)` — **too loose**. The file was written in the same test process, moments ago, with `Utc::now()` as the `created` timestamp. A freshly-created file should produce a decay weight near `1.0` (within the grace period, the weight is `1.0` exactly for `power-law` and `exponential` functions). Asserting `> 0.0` admits weights like `0.001`, which would indicate a broken decay calculation. The correct assertion is `assert_eq!(weight, 1.0)` — or at minimum `assert!(weight > 0.9, "freshly created file should have near-max decay weight, got {weight}")`.
- `assert!(!stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"))` — correct. ✅

**Composition problem:** The test invokes two unrelated functions inside one `capture_stderr` closure, producing a nested `(((title, snippet), weight), stderr)` destructure that is visually difficult to read. If the snippet function panics, the decay weight is never computed. The failure message names the outer test, not the failing assertion. Two focused tests would produce cleaner diagnostic output on failure.

**Verdict: REWRITE.** Split into `fr25_2_ac2_snippet_success_path_returns_title_and_content` and `fr25_2_ac2_decay_success_path_returns_nonzero_weight`. Tighten the weight assertion to `assert_eq!(weight, 1.0)` (file is freshly created, must be in grace period).

---

### `fr25_2_ac3_malformed_created_emits_explicit_degraded_diagnostic`

**What it tests:** `get_decay_weight_for_file` when the DB row exists but `created` is an unparseable timestamp string (`"not-a-valid-iso-timestamp"`). Expected path: row found → `last_accessed` is `None` → `created` is `Some("not-a-valid-iso-timestamp")` → `parse_datetime` returns `None` → `emit_degraded_decay_diagnostic("GetDecayWeightForFile(reuse-conn)", "Malformed file_content.created timestamp...")` → fall through to `get_decay_weight_from_disk_with_status` → file absent → emit second DEGRADED signal → return `1.0`.

**Load-bearing:** Yes. This is the *only* test that exercises the malformed-DB-timestamp branch. The `degraded_diag_assets.rs` file covers frontmatter-level malformed timestamps via a higher-level tool call, which exercises a different code path (`GetDecayMetadataForFile` in `SystemTools`, not `GetDecayWeightForFile(reuse-conn)` in `MemorySearchTools.Fusion`).

**Unique:** Yes.

**Assertions:**
- `assert_eq!(weight, 1.0)` — exact. ✅
- Both `stderr.contains` checks are correct and specific. ✅

**Gap:** The test does not assert the *absence* of the `GetDecayWeightFromDisk` DEGRADED signal that will also be emitted (because the file doesn't exist on disk either). This is acceptable — both signals being present is expected behavior, and the test is verifying the *first* branch, not the disk fallback. However, a future refactor that skips the disk fallback on the reuse-conn path would silently change the stderr output; the test would still pass. This is a low-priority concern.

**Verdict: KEEP.**

---

### `fr25_2_ac4_path_diagnostics_redacts_absolute_paths`

**What it tests:** `get_file_display_info` invoked with an absolute path that lives outside the memory root. Verifies: (a) the title is derived from the basename, not the full path; (b) the DEGRADED diagnostic contains the basename; (c) the DEGRADED diagnostic does NOT contain the full absolute path.

**Load-bearing:** Yes. This is the only test in the suite that asserts path redaction as a security property. The absence of this test would leave the absolute-path-leakage risk undetected in `redact_display_info_path`.

**Unique:** Yes. `path_security_tests.rs` covers path-boundary enforcement for `read_markdown` and similar APIs, but not the diagnostic output of `get_file_display_info`.

**Assertions:**
- `assert_eq!(title, ...)` using `file_stem()` — precise and correct. ✅
- `assert!(snippet.starts_with("[DEGRADED] Snippet unavailable"))` — same minor issue as AC1 snippet test: the value is exact; prefer `assert_eq!`. Low priority here since the test's primary purpose is the redaction check, not the snippet value.
- `assert!(stderr.contains(&format!("for '{file_name}'")))` — positive redaction check. ✅
- `assert!(!stderr.contains(&absolute_path.to_string_lossy().to_string()))` — the key security assertion. ✅

**Note on UUID usage:** The UUID in `file_name` prevents path collisions across parallel test runs. This is appropriate. The dynamic expected value `format!("for '{file_name}'")` is derived from the same UUID, keeping the assertion self-consistent.

**Verdict: KEEP.** Optional: change `starts_with` to `assert_eq!` for the snippet value (cosmetic).

---

## Recommendations

### Priority 1 — Structural (affects maintainability)

**Extract shared infrastructure.** `capture_stderr`, `isolated_env()` / `TestEnv::setup()`, and `TEST_MUTEX` are duplicated verbatim from `crates/maenifold-core/tests/degraded_diag/support.rs`. This file should replace its private copies with:

```rust
#[path = "degraded_diag/support.rs"]
mod support;
use support::{TEST_MUTEX, TestEnv, capture_stderr};
```

And replace `isolated_env()` calls with `TestEnv::setup()`. One truth, one maintenance point. The `write_memory_note` helper is specific to this file and can stay local.

### Priority 2 — Behavioral (affects regression fidelity)

**Rewrite AC2 as two focused tests.** Split `fr25_2_ac2_success_path_behavior_remains_unchanged` into:

```rust
#[test]
fn snippet_success_path_returns_title_and_content() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let uri = write_memory_note(&env, "...", "Valid Memory Note", "...targeted query token...");
    let ((title, snippet), stderr) = capture_stderr(|| {
        get_file_display_info(&uri, "targeted", &env.cfg)
    });
    assert_eq!(title, "Valid Memory Note");
    assert!(snippet.contains("targeted query token"));
    assert!(!snippet.contains("[DEGRADED]"));
    assert!(!stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"));
}

#[test]
fn decay_success_path_freshly_created_file_returns_full_weight() {
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let uri = write_memory_note(&env, "...", "Valid Memory Note", "...");
    let (weight, stderr) = capture_stderr(|| {
        get_decay_weight_for_file(&uri, &env.cfg)
    });
    assert_eq!(weight, 1.0, "freshly created file must be in grace period");
    assert!(!stderr.contains("[DEGRADED][MemorySearchTools.Fusion]"));
}
```

**Tighten the weight assertion.** A file written with `Utc::now()` as its `created` timestamp is within the grace period. The decay function returns exactly `1.0` for files in-grace. `assert_eq!(weight, 1.0)` is the correct assertion; `assert!(weight > 0.0)` does not catch decay miscalculation.

### Priority 3 — Naming (affects readability)

**Disambiguate the `ac1` infix.** Both decay and snippet fallback tests are named `fr25_2_ac1_*`. The `ac1` label implies they cover the same acceptance criterion, which they don't. Rename:
- `fr25_2_ac1_decay_fallback_emits_explicit_degraded_signal` → `decay_fallback_to_disk_emits_degraded_signal_for_missing_uri`
- `fr25_2_ac1_snippet_fallback_emits_explicit_degraded_signal` → `snippet_fallback_emits_degraded_signal_for_missing_uri`

### Priority 4 — Traceability (affects auditability)

**Update stale RTM annotation.** Every test comment reads `// T-QUAL-002.6a: RTM FR-25.2`. In the live PRD and RTM, FR-25 is "Azure Deployment Manifests" (T-19.x, RTM-137–140). `T-QUAL-002` does not appear anywhere in the current `TODO.md` or `RTM.md`. The requirement was completed and moved to CHANGELOG, but the annotation was not updated to reflect the old FR number. Maintainers checking against the live RTM will find no matching row, creating false confidence in traceability.

Recommended replacement: add a note linking to the CHANGELOG entry where FR-25.2 (old numbering) was completed, or update to the most semantically applicable live requirement (e.g., NFR-1 / NFR-2 for diagnostic quality coverage).

### Priority 5 — Minor assertion quality

**Change `starts_with` to `assert_eq!`** for the snippet value in AC1-snippet and AC4:
```rust
// Current (both tests):
assert!(snippet.starts_with("[DEGRADED] Snippet unavailable"));

// Correct:
assert_eq!(snippet, "[DEGRADED] Snippet unavailable");
```
The source returns this exact string with no suffix. `starts_with` implies suffix variation that doesn't exist.

---

## Coverage Gap Identified

**The `memory://` URI redaction path in `get_file_display_info` is untested in this file.** AC4 tests absolute-path redaction (non-`memory://` identifiers). The `redact_display_info_path` function has a separate branch for `memory://` URIs (stripping the prefix and returning the last path segment). No test verifies that a `memory://` URI is also redacted correctly in the diagnostic output — it's only tested implicitly via AC1, which doesn't assert what appears *after* `"for '"`. Consider adding:

```rust
#[test]
fn snippet_fallback_diagnostic_redacts_memory_uri_to_basename() {
    // Ensures the DEGRADED diagnostic shows "non-existent-file", not the full URI
    let _guard = TEST_MUTEX.lock();
    let env = TestEnv::setup();
    let (_, stderr) = capture_stderr(|| {
        get_file_display_info(
            "memory://some/path/non-existent-file",
            "query",
            &env.cfg,
        )
    });
    assert!(stderr.contains("for 'non-existent-file'"),
        "must redact URI to basename in diagnostic: {stderr}");
    assert!(!stderr.contains("memory://"),
        "must not leak full URI in diagnostic: {stderr}");
}
```

---

## Load-Bearing Score: 4/5

- AC1-decay: **load-bearing** — sole guard for `GetDecayWeightFromDisk` DEGRADED path
- AC1-snippet: **load-bearing** — sole guard for `GetFileDisplayInfo` DEGRADED path
- AC2: **partial** — covers snippet success path (not well covered elsewhere); decay success path is covered elsewhere; weight assertion is too weak to catch miscalculation
- AC3: **load-bearing** — sole guard for the malformed-DB-timestamp branch
- AC4: **load-bearing** — sole guard for absolute-path redaction security property in diagnostics
