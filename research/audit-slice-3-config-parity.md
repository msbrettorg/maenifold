# Audit Slice 3: Config & Environment Variable Parity

**Task**: T-10.9 | **RTM**: FR-15 | **Sequential Thinking**: session-1773104046670-38000 / audit-slice-3-config

## Executive Summary

Compared every `MAENIFOLD_*` environment variable in C# `src/` (29 vars) against Rust `crates/` `src/` (29 vars). Found **6 new parity issues** (2 blocking, 3 important, 1 cosmetic). 25+ shared env vars have correct names and defaults. Sprint 10 renames (SESSION_ABANDONMENT_MINUTES → SESSION_ABANDON_MINUTES, ENABLE_INCREMENTAL_SYNC → AUTO_SYNC) are NOT re-reported.

---

## Findings

### BLOCKING

```
Env Var: MAENIFOLD_SESSION_ABANDON_MINUTES
C# default: 30
Rust default: 120
Status: DRIFT
Severity: blocking
Location: sessions.rs:427-431
Note: Sessions are abandoned 4x slower in Rust. Cleanup behavior diverges
      in shared-root production use. Rust code uses .filter(|v| *v > 0)
      but C# does not (C# negative → uses default; Rust negative → uses default too).
```

```
Env Var: (all bool env vars — MAENIFOLD_AUTO_SYNC, SESSION_CLEANUP, SYNC_LOGGING, EMBEDDING_LOGS, VECTOR_LOGS)
C# parser: "true"/"1"/"yes"/"on" → true; "false"/"0"/"no"/"off" → false; else → default
Rust parser: "true"/"1" → true; anything else (including "yes"/"on") → false (default fallback)
Status: DRIFT (parsing behavior)
Severity: blocking
Location: config.rs:188-193 vs Config.cs:208-219
Note: If a user sets MAENIFOLD_AUTO_SYNC=yes, C# enables incremental sync,
      Rust does NOT. Shared-root environments with shell-idiomatic env values
      will silently diverge.
```

### IMPORTANT

```
Env Var: MAENIFOLD_COMMUNITY_NMI_THRESHOLD (C#) vs MAENIFOLD_COMMUNITY_NMI_DRIFT_THRESHOLD (Rust)
C# default: 0.8
Rust default: 0.8
Status: DRIFT (name mismatch)
Severity: important
Location: Config.cs:122-123 vs community.rs:233, tools.rs:658
Note: C# reads "MAENIFOLD_COMMUNITY_NMI_THRESHOLD", Rust reads
      "MAENIFOLD_COMMUNITY_NMI_DRIFT_THRESHOLD". Setting one does NOT
      affect the other. Shared env → Rust ignores C#'s variable.
```

```
Env Var: MAENIFOLD_SYNC_LEASE_MS
C# default: 10000 (clamped 1000-60000 via Math.Clamp)
Rust default: N/A — hardcoded 60 seconds at callsites
Status: MISSING_IN_RUST
Severity: important
Location: Config.cs:126 vs leader.rs (not present), search.rs:302 (hardcoded 60), watchers.rs:265 (hardcoded 60)
Note: C# passes Config.SyncLeaseMs to every TryAcquire call. Rust passes
      literal `60` (seconds) = 60_000ms. Default lease is 6x longer in Rust
      AND not configurable. Add MAENIFOLD_SYNC_LEASE_MS env var to Rust
      with default 10 (seconds) and clamping to 1-60s.
```

```
Env Var: MAENIFOLD_DEBOUNCE_MS (StartWatcher fallback default)
C# default (when no payload.debounceMs): 150 (Config.DefaultDebounceMs)
Rust default (when no payload.debounceMs and no env): 500
Status: DRIFT (fallback default)
Severity: important
Location: IncrementalSyncTools.cs:16 vs watchers.rs:63
Note: Both read MAENIFOLD_DEBOUNCE_MS correctly (default 150 for GetConfig
      output), but the StartWatcher tool itself uses different fallback when
      the user doesn't provide debounceMs parameter: C# falls to
      Config.DefaultDebounceMs (150), Rust falls to literal 500.
      Fix: Change watchers.rs:63 unwrap_or(500) → unwrap_or(150).
```

### COSMETIC

```
Env Var: MAENIFOLD_TEST_MEMORY
C# default: relative to AppContext.BaseDirectory
Rust default: N/A
Status: MISSING_IN_RUST
Severity: cosmetic
Location: Config.cs:59-75
Note: C# test infrastructure variable. Rust uses isolated_config() with
      TempDir for test isolation. No production impact.
```

---

## Rust-Only Additions (not bugs)

```
Env Var: MAENIFOLD_CATALOG_ASSETS_OVERRIDE
C# equivalent: none
Rust default: falls back to config.assets_path
Status: MISSING_IN_CS
Severity: N/A (intentional Rust addition per T-10.17)
Note: Added when removing cross-repo asset masking. Not a parity gap.
```

```
Env Var: MAENIFOLD_COMMUNITY_RECOMPUTE_FILE_THRESHOLD
C# equivalent: none (C# uses different community recompute triggering logic)
Rust default: 10
Status: MISSING_IN_CS
Severity: N/A (Rust implementation detail)
Location: search.rs:1170-1176
Note: Controls minimum file-change count before community recomputation.
      C# triggers via different mechanism (CommunityDetectionRunner).
```

---

## Matched Env Vars (confirmed parity — 25 vars)

| Env Var | C# Default | Rust Default | Status |
|---------|-----------|-------------|--------|
| MAENIFOLD_ROOT | ~/maenifold | ~/maenifold | MATCH |
| MAENIFOLD_DATABASE_PATH | {root}/memory.db | {root}/memory.db | MATCH |
| MAENIFOLD_DEBOUNCE_MS | 150 | 150 | MATCH |
| MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY | 40 (min 1) | 40 (min 1) | MATCH |
| MAENIFOLD_INCREMENTAL_VACUUM_MINUTES | 720 | 720 | MATCH |
| MAENIFOLD_AUTO_SYNC | true | true | MATCH |
| MAENIFOLD_WATCHER_BUFFER | 65536 | 65536 | MATCH |
| MAENIFOLD_SNIPPET_LENGTH | 1000 | 1000 | MATCH |
| MAENIFOLD_SQLITE_BUSY_TIMEOUT | 5000 | 5000 | MATCH |
| MAENIFOLD_SESSION_CLEANUP | true | true | MATCH |
| MAENIFOLD_SYNC_LOGGING | true | true | MATCH |
| MAENIFOLD_EMBEDDING_LOGS | false | false | MATCH |
| MAENIFOLD_VECTOR_LOGS | false | false | MATCH |
| MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL | 7 | 7 | MATCH |
| MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS | 14 | 14 | MATCH |
| MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT | 28 | 28 | MATCH |
| MAENIFOLD_DECAY_HALF_LIFE_DAYS | 30 | 30 | MATCH |
| MAENIFOLD_DECAY_FUNCTION | "power-law" | "power-law" | MATCH |
| MAENIFOLD_LOUVAIN_GAMMA | 1.0 | 1.0 | MATCH |
| MAENIFOLD_COMMUNITY_MIN_SHARED | 3 | 3 | MATCH |
| MAENIFOLD_COMMUNITY_MIN_OVERLAP | 0.4 | 0.4 | MATCH |
| MAENIFOLD_COMMUNITY_MAX_SIBLINGS | 10 | 10 | MATCH |
| MAENIFOLD_COMMUNITY_DEBOUNCE_MS | 2000 | 2000 | MATCH |
| MAENIFOLD_DISABLE_VECTOR | truthy check | truthy check | MATCH |
| MAENIFOLD_SYNC_ROLE | "auto" | "auto" | MATCH |

Non-prefixed: `AGENT_ID` → both default "agent" (MATCH).

---

## Recommended Fix Priority (for T-10.9)

1. **sessions.rs:431** — Change `.unwrap_or(120)` → `.unwrap_or(30)` 
2. **config.rs:188-193** — Extend `env_bool` to accept "yes"/"on"/"no"/"off"
3. **community.rs:233, tools.rs:658** — Rename env var to `MAENIFOLD_COMMUNITY_NMI_THRESHOLD`
4. **leader.rs + config.rs** — Add `sync_lease_ms()` method reading `MAENIFOLD_SYNC_LEASE_MS` (default 10_000, clamp 1000-60_000)
5. **watchers.rs:63** — Change `.unwrap_or(500)` → `.unwrap_or(150)`

---

## Sources Consulted

- C# reference: `/Users/brett/src/ma-collective/maenifold/src/Utils/Config.cs` (lines 1-220)
- C# entry point: `/Users/brett/src/ma-collective/maenifold/src/Program.cs`
- C# leader: `/Users/brett/src/ma-collective/maenifold/src/Tools/SyncLeaderElection.cs`
- Rust config: `crates/maenifold-core/src/config.rs` (lines 1-193)
- Rust paths: `crates/maenifold-core/src/paths.rs`
- Rust sessions: `crates/maenifold-core/src/sessions.rs:427-431`
- Rust leader: `crates/maenifold-core/src/leader.rs`
- Rust watchers: `crates/maenifold-core/src/watchers.rs`
- Rust tools (GetConfig): `crates/maenifold-core/src/tools.rs:614-665`
- Rust community: `crates/maenifold-core/src/community.rs:233`
- Rust search: `crates/maenifold-core/src/search.rs` (multiple locations)
- Full `grep -rn "MAENIFOLD_"` across both repos (src-only, tests excluded)
