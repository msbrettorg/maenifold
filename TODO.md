# Backlog — maenifold-rs

For completed work (Sprints 1–3, 9–15, 20–23), see [CHANGELOG.md](CHANGELOG.md).

---

## Sprint 17 — Streamable HTTP Transport

### T-17.1: Add rmcp HTTP transport feature + axum dependency | RTM: FR-23

Update workspace `Cargo.toml`: add `transport-streamable-http-server` to rmcp features (gated behind `http` feature flag). Add `axum = "0.7"` and `tower-http = { version = "0.6", features = ["cors"] }` dependencies.

- **Files**: `Cargo.toml`, `crates/maenifold-core/Cargo.toml`
- **Status**: complete

### T-17.2: Implement `serve_http()` in MCP server | RTM: FR-23

Add `MaenifoldMcpServer::serve_http(port: u16)` alongside existing `serve_stdio()`. Uses `StreamableHttpService` from rmcp mounted on axum router. CORS permissive. Session tracking via `Mcp-Session-Id` headers. Same `MaenifoldHandler` used for both transports.

- **Files**: `crates/maenifold-core/src/mcp/http.rs` (new), `crates/maenifold-core/src/mcp/mod.rs`
- **Status**: complete
- **Depends on**: T-17.1

### T-17.3: CLI `--http` flag and `--port` option | RTM: FR-23

Add `--http` and `--port <PORT>` CLI arguments. `--http` starts `serve_http()`. `--mcp` unchanged. Mutually exclusive flags. Default port 8080.

- **Files**: `crates/maenifold-core/src/cli.rs`
- **Status**: complete
- **Depends on**: T-17.2

### T-17.5: HTTP transport tests | RTM: FR-23

Integration tests: start HTTP server on random port, send MCP initialize + tool/call requests via reqwest, verify JSON-RPC responses. Test session creation and cleanup.

- **Files**: `crates/maenifold-core/tests/http_transport_tests.rs` (new)
- **Status**: complete
- **Depends on**: T-17.2

---

### T-24.1: Add OpenCode shared runtime config helper + loader-safe exports | RTM: FR-31 (RTM-195 through RTM-200)

Implement `assets/integrations/opencode/plugins/lib/config.ts` and wire `persistence.ts`, `confession.ts`, and `compaction.ts` to consume it under the runtime plugin tree used by the `~/.config/opencode/plugins` symlink.
AC-1: `getMaenifoldPluginEnv(name)` returns `process.env[name]` when explicitly set
AC-2: Unset env values fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment`
AC-3: `getMaenifoldPluginFlag(name)` parses string booleans and returns `true`, `false`, or `undefined` without throwing
AC-4: Missing or malformed OpenCode config fails open and preserves plugin execution
AC-5: `persistence.ts` and `confession.ts` expose only callable plugin factories at module top level while keeping test seams reachable via attached properties
AC-6: `compaction.ts`, `confession.ts`, and `persistence.ts` all resolve shared helper imports successfully from the repo-backed plugin symlink path
AC-7: Focused OpenCode plugin tests pass from `assets/integrations/opencode/tests/plugins`

- **Status**: pending
- **Depends on**: none

---

## Maintenance — Traceability & Test Gaps

### T-GRAPH-DECAY-002: Access-boosting boundary tests for ReadMemory | RTM: NFR-8

Boundary tests validating that ReadMemory correctly maintains the `last_accessed` timestamp in the `file_content` table for the [[decay-weighting]] system. Covers positive paths (URI-based read, title-based read, sequential reads) and negative paths (missing identifier, learn-mode short-circuit).

- **Files**: `crates/maenifold-core/tests/access_boosting_reads.rs`, `crates/maenifold-core/tests/access_boosting/support.rs`
- **Status**: complete

---

## Sprint 53 — File-Authoritative Pipeline Gate Cache

### T-53.1: Build `gate` crate and CLI skeleton | RTM: FR-41

Create workspace crate `crates/maenifold-gate` with binary `gate` and subcommands `advance`, `status`, and `sync`.

  AC-1: Workspace builds with new crate `maenifold-gate` and binary name `gate`
  AC-2: `gate --help` lists `advance`, `status`, and `sync`
  AC-3: CLI accepts `--db` and task path/root arguments without implying DB authority

- **Files**: `Cargo.toml`, `crates/maenifold-gate/Cargo.toml`, `crates/maenifold-gate/src/main.rs`
- **Status**: pending
- **Depends on**: none

### T-53.2: Implement file-authoritative parsing + status derivation | RTM: FR-41

Implement canonical task parsing from `todo/`/`done/` files and derive effective status/attempts/verdicts from markdown sections rather than DB rows.

  AC-1: Parser derives `pending`, `red`, `blue`, and `done` from file location + canonical headings
  AC-2: Duplicate or out-of-order stage headings return an error
  AC-3: Attempts are derived from rejection/failure history in the file
  AC-4: `gate status` reports file-derived state even when DB is missing or stale

- **Files**: `crates/maenifold-gate/src/parser.rs`, `crates/maenifold-gate/src/status.rs`, `crates/maenifold-gate/src/types.rs`
- **Status**: pending
- **Depends on**: T-53.1

### T-53.3: Implement `gate advance` with disk-first cache updates | RTM: FR-41

Implement legal transition enforcement using task files as the source of truth, append canonical stage sections, and update `pipeline.db` only after durable file mutation succeeds.

CLI contract:
- `gate advance [--db <DB>] [--root <ROOT>] --stage swe <TASK>`
- `gate advance [--db <DB>] [--root <ROOT>] --stage red --verdict PASS|REJECT <TASK>`
- `gate advance [--db <DB>] [--root <ROOT>] --stage blue --verdict PASS|FAIL <TASK>`

  AC-1: Valid SWE advance appends `## SWE Remediation` and transitions effective state from `pending` to `red`
  AC-2: Valid red PASS appends `## Red-team Audit` and transitions effective state from `red` to `blue`
  AC-3: Valid blue PASS appends `## Blue-team Verification` and moves the file from `todo/` to `done/`
  AC-4: Red `REJECT` or blue `FAIL` returns effective state to `pending` without losing audit history
  AC-5: DB cache is not updated when file append or move fails

- **Files**: `crates/maenifold-gate/src/advance.rs`, `crates/maenifold-gate/src/fs.rs`, `crates/maenifold-gate/src/db.rs`
- **Status**: pending
- **Depends on**: T-53.2

### T-53.4: Implement `gate sync` cache rebuild from disk | RTM: FR-41

Implement cache regeneration that scans authoritative task files and repopulates `pipeline.db` as a performance index.

  AC-1: `gate sync` recreates task cache rows for files in both `todo/` and `done/`
  AC-2: Sync derives status/location/attempts from file contents and directory location
  AC-3: Sync removes stale DB entries for task files no longer present on disk
  AC-4: Sync succeeds when the DB file is missing by creating a fresh cache

- **Files**: `crates/maenifold-gate/src/sync.rs`, `crates/maenifold-gate/src/db.rs`
- **Status**: pending
- **Depends on**: T-53.2

### T-53.5: Add focused verification coverage for `gate` | RTM: FR-41

Add focused tests covering CLI surface, parser invariants, disk-first transitions, and cache sync recovery.

  AC-1: Tests cover all acceptance criteria for T-53.1 through T-53.4
  AC-2: Tests verify DB deletion + `gate sync` recovers cache from disk state
  AC-3: Tests verify invalid heading order/duplicates are rejected
  AC-4: Focused `maenifold-gate` test command passes cleanly

- **Files**: `crates/maenifold-gate/tests/gate_cli.rs`, `crates/maenifold-gate/tests/gate_advance.rs`, `crates/maenifold-gate/tests/gate_sync.rs`, `crates/maenifold-gate/tests/gate_status.rs`
- **Status**: pending
- **Depends on**: T-53.3, T-53.4

---

## Sprint 56 — Gate Correctness + Skill Alignment

### T-56.1: Make `gate` transitions produce parser-recognized canonical artifacts | RTM: FR-41, RTM-182

Align `gate append`, `gate pass`, and `gate reject` with the file-authoritative model so command-driven transitions leave markdown artifacts that `parser`, `status`, and `sync --rebuild` can all interpret consistently without DB-only state.

  AC-1: `gate append --agent swe` appends a canonical `## SWE Remediation` section rather than a noncanonical agent-note heading
  AC-2: `gate pass` and `gate reject` append canonical stage/verdict artifacts that `parse_task_content()` recognizes when deriving effective status
  AC-3: A task advanced from pending → swe → red → blue → done via commands can be reparsed from disk after each step without relying on location-only fallback except for legacy files
  AC-4: `gate sync --rebuild` reproduces the same status/location/attempts as the pre-rebuild cache for command-driven task histories

- **Files**: `crates/maenifold-gate/src/append.rs`, `crates/maenifold-gate/src/pass.rs`, `crates/maenifold-gate/src/reject.rs`, `crates/maenifold-gate/src/parser.rs`, `crates/maenifold-gate/src/status.rs`, `crates/maenifold-gate/src/sync.rs`
- **Status**: pending
- **Depends on**: T-55.7

### T-56.2: Reconcile task path semantics and stale cache cleanup in `gate sync` | RTM: FR-42, RTM-183

Fix the mismatch between recursive task discovery and flat path resolution, and make non-rebuild sync remove stale cache rows so `pipeline.db` behaves like a true derived index.

  AC-1: If nested task files are supported, `append`, `pass`, `reject`, `status <ID>`, and `move_to_done` all resolve the same discovered file path deterministically
  AC-2: If nested task files are not supported, discovery and sync reject nested tasks with a clear error instead of accepting them and breaking later commands
  AC-3: `gate sync` without `--rebuild` removes DB rows for task files no longer present on disk
  AC-4: ID-to-file mapping remains unique and correct after sync reconciliation, moves to `done/`, and renamed/deleted task scenarios

- **Files**: `crates/maenifold-gate/src/fs.rs`, `crates/maenifold-gate/src/db.rs`, `crates/maenifold-gate/src/sync.rs`, `crates/maenifold-gate/src/append.rs`
- **Status**: pending
- **Depends on**: T-55.2, T-55.7

### T-56.3: Remove process-global stderr suppression from quiet parsing | RTM: FR-41, RTM-184

Replace the Unix-only `dup2`-based stderr redirection in quiet parsing with an explicit warning capture or suppression mechanism that is safe for tests, libraries, and concurrent execution.

  AC-1: `TaskState::parse_quietly()` no longer performs process-global file descriptor mutation or uses `unsafe` stderr redirection
  AC-2: `status` can still suppress noisy parse warnings during full scans while preserving diagnostics for targeted inspection paths
  AC-3: Warning behavior is covered by tests and remains stable across Unix and non-Unix targets

- **Files**: `crates/maenifold-gate/src/types.rs`, `crates/maenifold-gate/src/parser.rs`, `crates/maenifold-gate/src/status.rs`
- **Status**: pending
- **Depends on**: T-54.2

### T-56.4: Align test-driven-development skill assets with shipped `gate` behavior | RTM: FR-41, RTM-185

Bring the skill package, scaffold, and prompt templates back into agreement with the current `gate` CLI and the intended file-authoritative workflow.

  AC-1: No skill document or scaffold file references obsolete `gate advance` commands
  AC-2: Skill docs do not claim the scaffold contains a seeded `pipeline.db` unless such a file is actually shipped
  AC-3: `setup-template.md`, scaffold README, and prompt templates refer only to shipped skills/assets and current command names
  AC-4: Prompt templates and scaffold examples use canonical section names and verdict formatting consistent with `maenifold-gate` parser expectations
  AC-5: Repository noise files such as `.DS_Store` are removed from the skill asset tree

- **Files**: `assets/integrations/skills/test-driven-development/SKILL.md`, `assets/integrations/skills/test-driven-development/references/setup-template.md`, `assets/integrations/skills/test-driven-development/references/scaffold/README.md`, `assets/integrations/skills/test-driven-development/references/{swe,red,blue}-prompt-template.md`, `assets/integrations/skills/test-driven-development/.DS_Store`
- **Status**: pending
- **Depends on**: T-56.1

### T-56.5: Add regression coverage for roundtrip state, nested paths, and skill consistency | RTM: FR-41, RTM-186

Add focused tests that lock in the repaired source-of-truth model and the new sync/path semantics so future changes cannot silently reintroduce DB/file drift.

  AC-1: Integration tests cover command-driven state roundtrip through `status` and `sync --rebuild`
  AC-2: Tests cover the chosen nested-task behavior end-to-end
  AC-3: Tests cover stale-row deletion during non-rebuild sync
  AC-4: Tests fail if noncanonical command output causes parser/status drift
  AC-5: Focused `maenifold-gate` test command passes cleanly after the new coverage is added

- **Files**: `crates/maenifold-gate/tests/integration.rs`, `crates/maenifold-gate/tests/pipeline_tests.rs`, `crates/maenifold-gate/tests/parser_rework_tests.rs`, `crates/maenifold-gate/src/{append,pass,reject,sync,fs}.rs`
- **Status**: pending
- **Depends on**: T-56.1, T-56.2, T-56.3, T-56.4
