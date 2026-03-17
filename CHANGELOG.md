# Changelog — maenifold-rs

All notable completed work is documented here, organized by sprint.
For detailed retrospectives including lessons learned, see [RETROSPECTIVES.md](RETROSPECTIVES.md).

---

## Sprint 33 — Full ACP Protocol Compliance — 2026-03-17

### Added
- **Content block refactor** (T-43.1): `ChatMessage.blocks: ContentBlock[]` supporting text, image, audio, resource, and resource_link types with full rendering
- **Session list** (T-43.2): `session/list` RPC with capability detection, cursor pagination, agent↔local session merge, "from agent" indicator
- **Terminal subsystem** (T-43.6): All 5 `terminal/*` methods (create, output, wait_for_exit, kill, release), `TerminalManager`, `TerminalOutputBlock.vue`, Rust backend with ring-buffer output
- **Prompt capabilities** (T-43.7): Client declares `prompt.image/audio/embeddedContext` capabilities; UI gates attachment controls based on agent support
- **Multiple permissions** (T-43.10): Permission requests queued via `Map<string, PendingPermissionEntry>` instead of singleton; sequential UI with queue count
- **Permission fidelity** (T-43.15): Full tool-call context (content, rawInput, rawOutput) displayed in permission dialogs
- **Tool content rendering** (T-43.13): Expandable tool-call cards with text/diff/terminal content blocks
- **Resource link fields** (T-43.14): `title`, `description`, `size` (including bigint) on resource_link content blocks
- **Frame validation** (T-43.16): JSON-RPC validation on both relay (WebSocket) and app (Tauri IPC) inbound/outbound paths
- **ACP extensibility** (T-43.3): `_meta` on all request/response types, `_session/set_model` wire name, capability advertisement

### Changed
- **Per-method timeouts** (T-43.5, T-43.9): Replaced blanket 60s timeout with method-specific values; `session/prompt` gets 30min
- **SDK type alignment** (T-43.4): `SlashCommand` → SDK `AvailableCommand` type; no more flattening
- **Config during generation** (T-43.20): Mode/model/config changes allowed while prompt is generating; optimistic updates with rollback
- **configOptions presence** (T-43.21): Check field presence (`'configOptions' in response`) not array length; empty array ≠ absent
- **session_info targeting** (T-43.18): `session_info_update` scoped by `sessionId`; updates `savedSessions` and `currentSession` independently
- **updatedAt null** (T-43.19): `updatedAt: null` clears the timestamp (sets to 0) instead of `Date.now()`
- **cwd validation** (T-43.8): `session/load` validates absolute path including Windows UNC paths
- **Tool-only turns** (T-43.11): Stub assistant message synthesized when `tool_call` arrives before any text
- **rawInput/rawOutput** (T-43.12): Stored on `ToolCallInfo`, propagated through creation and update paths
- **Clean stdio** (T-43.17): Removed login shell wrapper (`-l` flag) to prevent stdout contamination from shell profiles

### Pipeline Stats
- 21 specs through SWE → Red-team → Blue-team pipeline
- 2 red-team rejects caught and fixed (4.9% reject rate)
- ~50 agent dispatches across GPT-5.4 (SWE/Blue) and Claude Sonnet 4.6 (Red)

---

## Sprint 32 — ACP Protocol Compliance Audit (Initial) — 2026-03-16

### Added
- **Protocol version check** (T-42.1): Shared `performInitialization()` helper, `agentInfo`/`promptCapabilities` stored
- **Agent plan support** (T-42.6): `plan` session update handler, `PlanView.vue` component
- **Session info updates** (T-42.7): `session_info_update` notification handler
- **ConfigOptionPicker** (T-42.4): Generic dropdown for any ACP configOption; legacy pickers gated

### Changed
- **Session setup** (T-42.2): Absolute cwd validation, `loadSession` capability re-check, resume abort guards
- **Prompt turn** (T-42.3): `cancelPermission` on cancel, tool call cancellation, `stopReason` UI
- **Tool call content** (T-42.5): `content` field stored and accumulated on updates; `locations` with line
- **Slash hint** (T-42.8): Hint rendered as placeholder, dead code removed, listener leak fixes
- **Filesystem** (T-42.9): Absolute path validation, `mkdir -p`, null return
- **Transport bridge** (T-42.10): JSON-RPC relay frames, bridge method, content warnings

---

## Sprint 29 — System Tray Icon & Remove Feature Flags — 2026-03-15

### Added
- **System tray icon** (FR-35): macOS menu bar icon with "Open Web UI", "Help", "Quit" menu items when running `--web` or `--http`. Uses `tray-icon` + `winit` for native event loop on main thread; tokio server on background thread. Graceful shutdown via `CancellationToken`.
- **Cognitive Geodesics** tray icon art: generative algorithmic art at `assets/tray-icon.png` — concept nodes warping a knowledge manifold with geodesic particle flows.
- Dependencies: `tray-icon`, `muda`, `winit`, `open`, `image`, `objc2-core-foundation` (macOS)

### Changed
- **Feature flags removed** (FR-36): Removed `http` and `web` cargo feature flags from all crates. All deps compile unconditionally. `cargo build --release -p maenifold-cli` works without `--features`. All 10 `#[cfg(feature = "...")]` gates removed.
- `Makefile`: `--features web` removed from build command
- `main.rs` restructured: dispatches `--web`/`--http` to tray event loop, `--mcp` stays headless

---

## Sprint 28 — Replace chrome-acp with ACP UI — 2026-03-15

### Changed
- **Web UI replaced**: chrome-acp (minimal React) → ACP UI (feature-complete Vue 3 SPA)
- New features from ACP UI: multi-agent sessions, agent thinking view, slash commands, permission dialogs, model picker, mode selector, traffic monitor, session management
- **Relay simplified**: chrome-acp proxy protocol → transparent JSON-RPC passthrough (browser sends raw ACP JSON-RPC over WebSocket, relay pipes to agent stdin/stdout)
- `agent-client-protocol` crate dependency removed — relay is now pure subprocess + pipe management
- `manager.rs` (ACP SDK wrapper) deleted — relay handles subprocess lifecycle directly

### Added
- `vendor/acp-ui` submodule (fork of formulahendry/acp-ui, Tauri shell stripped)
- `src/lib/relay.ts` — WebSocket adapter replacing Tauri invoke/listen
- stderr forwarding as `{"type":"stderr","data":"..."}` for ACP UI startup progress display

### Removed
- `vendor/chrome-acp` submodule
- `crates/maenifold-core/src/acp/manager.rs`
- `agent-client-protocol` dependency from Cargo.toml

---

## Sprint 27 — MCP Server Hardening — 2026-03-15

### Fixed
- **CRITICAL**: Tool/resource handlers offloaded to `spawn_blocking` — no longer block async runtime (filesystem, SQLite, ONNX inference)
- **CRITICAL**: HTTP sessions expire after 5 min idle (`keep_alive: 300s`) — no more unbounded session accumulation
- CORS applied to `/mcp` endpoint in web mode (was only on API sub-router)
- `Mcp-Session-Id` header exposed in CORS responses for browser MCP clients
- HTTP/web transports no longer falsely advertise `resources/list_changed` capability
- `MaenifoldError::Message` removed from validation error classifier — internal errors no longer misclassified
- Panic in `replace_section` replaced with proper error return (`expect` → `ok_or_else`)
- Redundant `initialize_runtime` call removed from resource request hot path

### Changed
- `rust-mcp-expert.agent.md` fully rewritten for rmcp 1.1.0 (was written for v0.8+, contained 18+ non-compilable examples)

### Requirements Completed
- FR-37 (RTM-227–236): MCP Server Hardening
- FR-38 (RTM-237–239): Rust MCP Expert Agent Rewrite

---

## Sprint 26 — Web UI Full Feature Parity — 2025-07-23

### Added
- 22 UI components promoted from chrome-extension to shared package (chain-of-thought, plan, checkpoint, suggestion, artifact, image, sources, task, loader, open-in-chat, queue, web-preview, canvas, controls, panel, toolbar, node, edge, connection, inline-citation, confirmation, context)
- 3 shared UI primitives promoted (alert, carousel, progress)
- ChatInterface.tsx wired with ChainOfThought, Plan, Checkpoint, Suggestions, Artifact, Task, Sources
- New shared deps: @xyflow/react, embla-carousel-react, @radix-ui/react-progress, tokenlens

### Fixed
- **SECURITY**: CORS restricted to localhost origins (replaced `CorsLayer::permissive()`)
- **SECURITY**: Relay MPSC channel bounded to 256 (was unbounded — OOM risk)
- **SECURITY**: Exec validation uses `libc::access(X_OK)` instead of mode bitmask
- **SECURITY**: `agent-client-protocol` pinned to specific git rev (was floating HEAD)
- Consolidated duplicate shimmer.tsx and reasoning.tsx between extension and shared
- 10 pre-existing TypeScript errors in shared package fixed

### Requirements Completed
- FR-32 (RTM-204–218): Web UI — full component parity with chrome extension
- FR-32-SEC: 4 security fixes from red-team audit

---

## Sprint 23 — Streamable HTTP Transport — 2026-03-14

### Added
- MCP Streamable HTTP transport via `maenifold --http [--port <PORT>]`
- `serve_http(port)` using rmcp `StreamableHttpService` + axum 0.8 router
- `LocalSessionManager` for `Mcp-Session-Id` session lifecycle
- Permissive CORS for browser-based MCP clients
- Graceful shutdown via `CancellationToken` + SIGINT handler
- `--http` / `--port` CLI flags (mutually exclusive with `--mcp`, default port 8080)
- 3 HTTP transport integration tests (initialize, session validation, tool invocation)
- HTTP deps gated behind `http` Cargo feature flag

### Requirements Completed
- FR-23 (RTM-126–130): Streamable HTTP Transport

---

## Sprint 22 — WikiLink Filter in Sync — 2026-03-13

### Added
- `is_concept_blocked()` API on WikiLinkFilter — single-concept filter check with exact + prefix matching
- WikiLink filter wired into `upsert_processed_file` for full sync (`run_sync`)
- WikiLink filter wired into `sync_single_file` for incremental sync
- 9 boundary tests covering RTM-151 through RTM-156
- Incremental sync reports filtered concept count in output

### Fixed
- **CRITICAL**: Prefix normalization bug in WikiLink filter — blocked prefixes were matching unrelated concepts

### Requirements Completed
- FR-19 (RTM-151–156): WikiLink filter applied during sync

---

## Sprint 21 — Vec0 Table Creation + Verification Parity — 2026-03-12

### Added
- `ensure_vec0_tables` uses 4-level dimension fallback chain (runtime → metadata → table introspection → DEFAULT 384)
- `vec0_extension_available` verifies table accessibility with real query, not just extension load
- Sync verification report queries actual `vec_concepts`/`vec_memory_files` row counts
- Phase timing telemetry in `run_sync()` — 5 Instant checkpoints for file processing, graph rebuild, embeddings, community detection, total

### Requirements Completed
- FR-8 (RTM-058, RTM-060, RTM-149, RTM-150): Dynamic vec0 schema parity

---

## Sprint 20 — Vector Sync Diagnostics — 2026-03-11

### Added
- ONNX inference error details surfaced for concept embedding failures (actionable diagnostics)
- Model discovery resolves ONNX models via canonicalized executable path (symlink support)
- Model discovery checks shared runtime root `~/maenifold/assets/models/` for C# cross-compat

### Requirements Completed
- FR-7 (RTM-147, RTM-148): Dynamic model loading — symlink + shared root discovery

---

## Sprint 15 — CLI Parity Remediation — 2026-03-11

### Fixed
- 27 CLI parity fixes from oracle comparison across all tool categories:
  - SearchMemories NULL distance crash (Hybrid/Semantic/Tags modes)
  - BuildContext format + depth traversal matches C# oracle
  - FindSimilarConcepts displays weighted_similarity
  - Removed 'Invalid argument:' prefix from error messages
  - CLI help tool groupings match C# (MCP group, no Adopt:/Assets: prefixes)
  - Missing `--payload` now rejected (was silently using `{}`)
  - Empty title + integer folder type validation
  - Nested WikiLink regex rejects malformed brackets
  - RepairConcepts blocks when vec0 unavailable
  - Workflow step JSON pretty-printed
  - RepairConcepts output format matches C#
  - ExtractConcepts zero-result message matches C#
  - ReadMemory shows N/A for missing dates, timestamps in local time
  - `--help` exits 1 (matches C#)
  - IO error mapped to correct JSON error code
- Vec0 extension loading deduplicated and reliable

### Requirements Completed
- NFR-1: CLI output matches C# oracle across all 28 tools

---

## Sprint 14 — Red-Team Drift Remediation — 2026-03-10

### Fixed
- Decay fallback: uses creation date when `last_accessed` is None (was returning 1.0)
- Graceful frontmatter parse: malformed YAML returns empty frontmatter, not Err
- CRLF line endings supported in frontmatter detection
- `co_occurrence_count` counts unique files, not sum of weights (fixes community detection)
- Symlink canonicalization added to path validation (security fix)
- NMI threshold env var renamed to `MAENIFOLD_COMMUNITY_NMI_THRESHOLD` (C# parity)
- Removed co_occurrence_count fallback in community edge weights
- Long titles truncated to 255 chars instead of rejected
- Semantic validation returns warning text, not Err (matches C# API contract)
- Legacy session IDs accepted (suffix-free format for cross-runtime compatibility)
- Session ID suffix uses random entropy (security hardening)

### Requirements Completed
- 11 red-team findings remediated with full traceability

---

## Sprint 13 — Parity Audit Remediation — 2026-03-10

### Added
- Separate search mode paths: Semantic (vector-only), FullText (text-only, normalized), Hybrid (RRF fusion)
- MemoryStatus returns full stats: folder count, file count, total size, graph stats, DB size
- `ToolResponse.WithNextSteps` and `WithHint` helpers wired into 26 tool return paths

### Fixed
- PreFusion multiplier: 3x (was 5x) matching C# `PreFusionMultiplier`
- FullText score normalization produces scores in [0.0, 1.0] range

### Removed
- 9 dead code functions behind `#[allow(dead_code)]` — evaluated and resolved

### Requirements Completed
- FR-6 (search parity), FR-7 (MemoryStatus), FR-8 (ToolResponse helpers)

---

## Sprint 12 — Structural Refactor / Facade Preservation — 2026-03-09

### Changed
- Extracted `workflow_ops.rs`, `mcp_resources.rs`, `graph_preview.rs`, `graph_analyzer.rs`, `similar.rs` from monolithic modules
- Extracted `sync.rs` from `search.rs` (sync/incremental-sync logic)
- Extracted `repair.rs` from `search.rs` (RepairConcepts, AnalyzeConceptCorruption)
- Extracted `similar.rs` from `search.rs` (FindSimilarConcepts)
- Extracted `decay.rs` from `memory.rs` (shared decay/date/path helpers)
- All public API facades preserved — zero behavioral change

### Requirements Completed
- NFR-1: search.rs decomposed from 3069 lines into focused modules

---

## Sprint 11 — Search Core Fix + MCP Resources — 2026-03-10

### Added
- MCP `resources/list` + `resources/read` capability
- MCP resource-change notifications via notify crate
- Auto-start sync watcher in `--mcp` mode
- Oracle parity tests for Adopt, UpdateAssets, RecentActivity, RunFullBenchmark, RepairConcepts, AnalyzeConceptCorruption

### Fixed
- SearchMemories text scoring: occurrence counting + title bonus (was binary matching)
- Real vector search: two-pipeline RRF architecture replacing `content.contains()` heuristic
- Vector search folder prefix overmatching
- Vector search tag filtering moved before RRF fusion
- `vec_memory_files` graceful degradation when table missing

### Requirements Completed
- FR-11 (RTM-068–071): MCP resource parity
- FR-12 (RTM-072–073): MCP auto-start watchers
- FR-13 (RTM-074–079): Oracle parity tests for all tools
- FR-21 (RTM-107–114): SearchMemories scoring parity

---

## Sprint 10 — Port Parity Completion — 2026-03-09

### Added
- 28 missing workflow JSONs copied (69/69 parity with C#)
- CI runs workspace-root tests including oracle parity
- docs/DEVELOPMENT.md created
- Bundled integration assets (claude-code hooks, README, QUICK_START, SKILL)

### Fixed
- Config env vars renamed to match C# (`MAENIFOLD_SESSION_ABANDON_MINUTES`, `MAENIFOLD_AUTO_SYNC`, + 8 missing vars added)
- RepairConcepts Mode B tests use real assertions (28/28 pass, no stub-acceptance)
- 4 Sprint 9 RTM boundary tests added (tokenizer.json, max tokens, vec0 dims, vec0 recreation)
- WikiLink filter reads `.wikilink-filter.json` (JSON format, matches C#)
- Asset catalog cross-repo masking removed
- 6 MCP stdio test failures fixed (framing mismatch)
- 12 oracle parity test failures fixed (6 fixed, 6 documented as ignored drift)

### Requirements Completed
- FR-10 (RTM-066–067): Workflow asset parity (69/69)
- FR-14 (RTM-080): CI parity gate
- FR-15 (RTM-081–090): Config env var parity (10 vars)
- FR-16 (RTM-091–093): Documentation remediation
- FR-17 (RTM-094–097): RepairConcepts Mode B verification
- FR-18 (RTM-098–101): Sprint 9 RTM coverage
- FR-19 (RTM-102–104): WikiLink filter format parity
- FR-20 (RTM-105–106): Asset catalog isolation

---

## Sprint 9 — Dynamic Model Loading + CI/CD — 2026-03-08

### Added
- Dynamic ONNX model discovery: scan `assets/models/*.onnx`, select largest, read dimensions from tensor
- `tokenizer.json` preferred (HuggingFace standard), fallback to `vocab.txt` + config
- `model_metadata` table for tracking model filename + dimensions
- Vec0 tables created with runtime dimensions; recreated on model change
- `build.yml`: fmt + clippy + test on push/PR to dev/main
- `release.yml`: matrix build for osx-arm64, linux-x64, win-x64

### Requirements Completed
- FR-7 (RTM-050–056): Dynamic model loading
- FR-8 (RTM-057–060): Dynamic vec0 schema
- FR-9 (RTM-061–065): CI/CD pipeline

---

## Sprints 1–3 — Core Tool Parity — 2026-03-07

### Added
- **Adopt** tool: role/color/perspective asset reading with two-step lookup (filename → JSON id scan)
- **AnalyzeConceptCorruption** tool: WikiLink variant scanning with 5-category classification + repair command generation
- **UpdateAssets** tool: recursive directory comparison (size + SHA256), dry-run mode, file copy
- **RecentActivity** tool: file_content DB query with type filtering, timespan, H2 section extraction
- **RunFullBenchmark** tool: 5 benchmark categories (CTE vs N+1, search modes, sync, complex traversal, system health)
- **RepairConcepts** tool: Mode A (WikiLink replacement), Mode B (WikiLink creation via pulldown-cmark AST), semantic validation via vec0 embeddings
- Boundary, CLI, and parity tests for all 6 tools
- Pre-existing clippy errors fixed (23 errors), `normalize_concept` + `extract_h2_sections` helpers added

### Requirements Completed
- FR-1 (RTM-001–007): Adopt tool
- FR-2 (RTM-008–013): AnalyzeConceptCorruption tool
- FR-3 (RTM-014–019): UpdateAssets tool
- FR-4 (RTM-020–030): RecentActivity tool
- FR-5 (RTM-031–037): RunFullBenchmark tool
- FR-6 (RTM-038–046): RepairConcepts tool
- NFR-4 (RTM-047–049): Clippy baseline

---

## Cumulative Status

| Metric | Value |
|--------|-------|
| Tools implemented | 28/28 (zero placeholders) |
| Tests passing | 500+ |
| Transports | stdio + Streamable HTTP |
| CLI parity | Matches C# oracle (27 format fixes in Sprint 15) |
| Sprints completed | 16 (Sprints 1–3, 9–15, 20–23) |
