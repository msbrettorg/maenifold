# Codebase Audit — 2026-04-25

## Scope

This audit covers the Rust workspace, the Electron/Vue desktop app, Gate task state, ACP/MCP boundaries, supply-chain risks, design-system enforcement, and repository validation commands.

Grounding used:

- Current source tree under `crates/`, `apps/desktop/`, `docs/`, `todo/`, and workspace manifests.
- Official ACP documentation:
  - `https://agentclientprotocol.com/protocol/overview`
  - `https://agentclientprotocol.com/protocol/initialization`
  - `https://agentclientprotocol.com/protocol/prompt-turn`
  - `https://agentclientprotocol.com/protocol/tool-calls`
  - `https://agentclientprotocol.com/protocol/file-system`
  - `https://agentclientprotocol.com/protocol/terminals`
- Project docs: `docs/acp-client-architecture.md`, `docs/SECURITY_MODEL.md`, `docs/UAT-PLAN.md`, `apps/desktop/DESIGN.md`.
- Memory graph context queried: `memory://sprints/mcp-stateless-http-transport-fix`, `memory://architecture/t146-2-architecture-analysis-acp-transport-rewrite-from-tauri-stdio-to-websocket`, `memory://build/sprints/sprint-174-pipeline-hardening-retrospective`, `memory://research/rust-code-quality-tooling-inventory`, `memory://build/research/maenifold-rs/current-code-sanity-check-for-maenifoldrs-20260330`, `memory://sprints/codebase-audit-crate-integration-opportunities`.

## Executive summary

| Severity | Count | Summary |
|---|---:|---|
| P0 | 1 | Production ACP execution can return mock success. This violates the no-mocks directive. |
| P1 | 13 | ACP/MCP boundary bypasses, permission auto-approval, cwd trust escape, cancellation protocol violation, unauthenticated config mutation, telemetry opt-out default, native/model supply-chain risks, Mermaid sanitizer bypass, and known production dependency vulnerability. |
| P2 | 17 | Filesystem containment gaps, allowlist gaps, stale UAT/test coverage, silent UI fallbacks, design-audit bypasses, unused/deprecated/mismatched dependencies, and simplification opportunities. |

No compile-breaking P0 was found: `cargo build --release -p maenifold-cli` succeeded. The repository audit target does currently fail on enforced Rust hygiene violations.

## ACP reference requirements

The ACP findings below are measured against these protocol requirements:

- ACP uses JSON-RPC methods and notifications; successful responses carry `result`, errors carry `error`, and notifications do not receive responses. Official overview: "The protocol follows the JSON-RPC 2.0 specification."
- Initialization must precede sessions. Official initialization docs: "Before a Session can be created, Clients MUST initialize the connection."
- Capability omissions are unsupported. Official initialization docs: "Clients and Agents MUST treat all capabilities omitted in the initialize request as UNSUPPORTED."
- Cancellation must be semantic. Official prompt-turn docs: after cancellation, "the Agent MUST respond to the original `session/prompt` request with the `cancelled` stop reason."
- ACP file paths must be absolute. Official overview and file-system docs: "All file paths in the protocol MUST be absolute."
- Terminal methods must be capability-gated and terminals released. Official terminals docs: Agents "MUST NOT" call terminal methods if unsupported, and "MUST release the terminal" when done.

## P0 findings

### P0-1 — Production ACP execution contains a mock success path

**Evidence**

- `crates/maenifold-jockey/src/acp/session/execute.rs:26-30` returns `mock_execute(...)` when `runtime_kind` is empty or `"mock"`.
- `crates/maenifold-jockey/src/acp/session/execute_stream.rs:243-260` builds an `AcpPromptResult` with `ok: true`, mock metadata, and synthetic deltas.
- `crates/maenifold-core/src/tools/acp_exec.rs:166-179` accepts `"mock"` during runtime inference.

**Impact**

Production code can report successful agent work without launching a real ACP runtime. In a Gate or pipeline context, this can create fake completion and violate the repo's no-mocks/no-fakes directive.

**Remediation**

Remove production mock runtime support. If still needed for tests, put it behind `#[cfg(test)]` or a test-only feature. Production execution should fail closed on empty or `"mock"` runtimes.

## P1 findings

### P1-1 — ACP relay accepts client-injected MCP servers while hard-coding permission auto-approval

**Evidence**

- `crates/maenifold-core/src/api/acp_relay_session.rs:83-89` accepts client `session/new` `mcpServers`.
- `crates/maenifold-core/src/api/acp_relay_session.rs:160` does the same for `session/load`.
- `crates/maenifold-core/src/api/acp_relay_mcp.rs:165-187` merges configured servers with any client-provided server whose name does not collide.
- `crates/maenifold-core/src/api/acp_relay_mcp.rs:44-58` sends `auto_approve: true` into Jockey prewarm.

**Impact**

A relay client can add an unconfigured MCP server to an ACP session. Because permissions are auto-approved, this creates an unreviewed tool-execution surface inside the agent session.

**Remediation**

Reject client-provided MCP servers by default. If the feature is required, add explicit allowlists for names, transports, and origins; preserve provenance; and do not hard-code permission auto-approval.

### P1-2 — Pipeline `cwd` can escape the repo and load agent config from the escaped directory

**Evidence**

- `crates/maenifold-core/src/workflow/pipeline/executor_nodes_cwd.rs:6-24` accepts absolute `cwd` values and relative traversal after canonicalization without enforcing containment under `maenifold_root`.
- `crates/maenifold-core/src/workflow/agent_session.rs:259-267` calls `AgentsConfig::load(Path::new(cwd))`.
- `crates/maenifold-core/src/acp/config/loader.rs:27-42` silently returns `AgentsConfig::default()` when the config is missing or unparsable.

**Impact**

A pipeline node can pivot execution to an external directory and load `maenifold.json` from that directory. If an attacker can influence a pipeline definition or external directory, they can influence runtime command resolution outside the trusted repo boundary.

**Remediation**

Require node `cwd` to canonicalize inside `context.config.maenifold_root` unless a deliberately named unsafe override is enabled. Load execution-sensitive agent config from the trusted root and treat invalid config as fatal.

### P1-3 — ACP cancellation can return JSON-RPC error instead of `cancelled`

**Evidence**

- `crates/maenifold-core/src/api/acp_relay.rs:202-213` tracks `cancelled`, but sends `rpc_error(...)` when the worker result is `Err`.
- `crates/maenifold-jockey/src/acp/worker/mod.rs:153-168` sends cancellation but does not guarantee prompt-result normalization.

**Impact**

Normal user cancellation can surface as an agent/runtime failure, violating ACP cancellation semantics and confusing clients.

**Remediation**

When `pending_prompt.cancelled` is true, return `{"stopReason":"cancelled"}` even if the worker future returns an abort/transport error. Add tests for both successful and errored cancellation paths.

### P1-4 — Unauthenticated localhost REST admin surface mutates agent/MCP configuration outside MCP permission flow

**Evidence**

- Desktop callers mutate config via direct REST:
  - `apps/desktop/src/components/SettingsConnection.vue:170-183`, `211-214`
  - `apps/desktop/src/components/SettingsMcpServers.vue:120-131`, `162-168`
  - `apps/desktop/src/lib/daemon-url.ts:1` defaults to `http://localhost:8080`.
- Backend exposes mutation routes without visible auth/permission gate:
  - `crates/maenifold-core/src/api/routes.rs:118-149`
  - `crates/maenifold-core/src/api/routes_config.rs:174-251`
- CORS allows `null`, `file://`, localhost, and all methods/headers in `crates/maenifold-core/src/mcp/mod.rs:17-39`.

**Impact**

Any local process, and some browser contexts allowed by CORS, can mutate agent commands, MCP server definitions, or settings without going through MCP/session permission semantics.

**Remediation**

Move privileged mutations behind an authenticated local channel: per-install token, Electron-only IPC bridge, OS keychain-backed secret, or explicit MCP/admin tools with user confirmation. Separate read-only browser routes from privileged mutation routes.

### P1-5 — Telemetry is opt-out by default and sends a stable machine identifier

**Evidence**

- `apps/desktop/src/composables/useAppSession.ts:118-120` defaults missing `telemetryEnabled` to `true` and calls `initTelemetry`.
- `apps/desktop/src/lib/telemetry.ts:9` embeds an Application Insights connection string.
- `apps/desktop/src/lib/telemetry.ts:24-57` fetches `/api/machine-id`, sets authenticated user context, and tracks `AppLaunch`.
- `crates/maenifold-core/src/api/routes_config.rs:123-127` returns `machine_uid::get()`.

**Impact**

First launch can transmit telemetry tied to a stable machine identifier before explicit user consent.

**Remediation**

Default telemetry to disabled until explicit opt-in. Do not fetch machine ID unless telemetry is explicitly enabled. Clear authenticated user context on disable and prefer a rotating or pseudonymous ID.

### P1-6 — Native `vec0` extension download/load trusts latest GitHub release without pinned integrity

**Evidence**

- `apps/desktop/electron/vec0.ts:44-50` uses GitHub's `releases/latest`.
- `apps/desktop/electron/vec0.ts:119-145` selects an asset by wildcard and validates only protocol/host.
- `apps/desktop/electron/vec0.ts:230-239` extracts the native extension and calls `verifyLoadable(tempPath)` before rename.
- `apps/desktop/electron/preload.ts:18-23` exposes `vec0Download()` to the renderer.

**Impact**

A compromised upstream release, release asset, or renderer path can lead to loading native code in the desktop process context. Host allowlisting is not artifact integrity.

**Remediation**

Pin expected version and SHA-256/Sigstore signature; verify before load; require explicit user confirmation for install/update; prefer shipping vetted native assets.

### P1-7 — Unverified HuggingFace model downloads and unsigned external model catalog

**Evidence**

- `crates/maenifold-core/src/tools/embedding_download.rs:76-168` downloads model/tokenizer files and moves them into place without hash/signature verification.
- `crates/maenifold-core/src/tools/embedding_models.rs:54-66` prefers an external `{assets_path}/models/catalog.json`.
- `crates/maenifold-core/assets/models/catalog.json:67-91` includes large remote models and a noncommercial `CC-BY-NC-4.0` entry.

**Impact**

Remote ONNX/tokenizer assets become trusted runtime inputs. Mutable upstream repos and unsigned catalog overrides create provenance ambiguity.

**Remediation**

Add immutable revisions and per-file hashes to the catalog. Verify every download before finalization. Require explicit trust for external catalog overrides and opt-in-gate non-MIT/Apache models.

### P1-8 — Silent fallback loads the largest flat ONNX file when no model is pinned

**Evidence**

- `crates/maenifold-core/src/vector/discovery.rs:88-115`, `128-149` falls back to scanning candidate directories and choosing the largest top-level `.onnx`.
- `crates/maenifold-core/src/tools/embedding_models_scan.rs:46-56`, `74-83` mirrors the behavior for UI status.

**Impact**

An arbitrary stale or unexpected large ONNX file under assets can become the effective model, violating no-silent-defaults and compounding the unverified download risk.

**Remediation**

Require explicit `settings.vector.active_model`. Remove "largest file wins" fallback or restrict it to an explicit migration command with catalog/hash verification.

### P1-9 — Build-time/native binary supply chain via `ort` download-binaries

**Evidence**

- `Cargo.toml:55` enables `ort` with `download-binaries` and `copy-dylibs`.
- `Cargo.lock:4322-4344` resolves `ort` / `ort-sys` `2.0.0-rc.12`.

**Impact**

Native binary acquisition during build increases provenance risk beyond crates.io checksums, and a release-candidate runtime expands audit surface.

**Remediation**

Move native downloads behind an explicit opt-in feature. Prefer vendored, checksummed artifacts or system-provided ONNX Runtime with documented provenance.

### P1-10 — Mermaid SVG sanitizer bypass

**Evidence**

- `apps/desktop/src/composables/useMermaid.ts:83-90` inserts Mermaid output using `preBlock.outerHTML = ... ${svg} ...` and error blocks with `insertAdjacentHTML`.
- The app already uses DOMPurify elsewhere for Markdown rendering, but the successful Mermaid SVG path bypasses that sanitizer.
- Companion detail is recorded in `docs/dependency-simplification-audit.md:39-67`.

**Impact**

Mermaid renders SVG/HTML-capable content. Injecting the returned SVG without DOMPurify creates a sanitizer bypass around user-authored diagram content.

**Remediation**

Sanitize Mermaid SVG with existing DOMPurify using an SVG profile or tight allowlist. Prefer DOM construction and `textContent` for error rendering.

### P1-11 — Frontend production dependency vulnerability via Mermaid's `uuid`

**Evidence**

- `apps/desktop/package.json:53-58` includes `mermaid`.
- `apps/desktop/package-lock.json:12146-12172` resolves `mermaid 11.14.0`.
- `apps/desktop/package-lock.json:16466-16469` resolves `uuid 11.1.0`.
- Dependency audit reported `GHSA-w5hq-g745-h8pq` for `uuid < 14.0.0`.

**Impact**

Moderate vulnerability in a production dependency path that handles document/user diagram content.

**Remediation**

Upgrade Mermaid when a compatible fixed release is available or use an override after compatibility testing. Do not blindly apply a breaking `npm audit fix --force`.

### P1-12 — Home quick prompt silently loses user input on backend/session send failure

**Evidence**

- `apps/desktop/src/components/HomeDashboard.vue:61-68` emits prompt, switches tabs, and clears input immediately.
- `apps/desktop/src/App.vue:80-98` tries to preserve input when `sendPrompt` returns false, but the Vue event path does not propagate that result back to `HomeDashboard`.

**Impact**

Backend/session failure is masked as navigation success, and the user's prompt can be lost.

**Remediation**

Model prompt submit as an explicit async callback/result. Clear input and navigate only after confirmed send or session creation; otherwise keep text and show an inline failure.

### P1-13 — Knowledge graph UI bypasses MCP with direct daemon REST calls

**Evidence**

- `apps/desktop/src/stores/knowledge-graph-actions.ts:55-88` fetches graph communities and concepts from direct daemon REST endpoints.
- Nearby memory operations use MCP-style APIs, but graph topology bypasses the common boundary.

**Impact**

Knowledge graph behavior can diverge from MCP capability/session/error semantics, violating the "AI is the new UI" common-interface principle.

**Remediation**

Expose graph communities/concepts through MCP resources/tools and consume them through the shared MCP/memory client path.

## P2 findings

### P2-1 — ACP filesystem client methods accept unvalidated paths and silently create parent directories

**Evidence**

- `crates/maenifold-jockey/src/acp/client.rs:225-263` reads `args.path` directly and creates parent directories before writes.
- `crates/maenifold-jockey/src/acp/session/cold_start_helpers.rs:10-16` advertises filesystem and terminal capabilities by default.

**Impact**

ACP requires absolute file paths, but these handlers do not enforce absolute paths, workspace containment, or symlink policy. A hostile or buggy agent can request broad filesystem reads/writes.

**Remediation**

Reject non-absolute paths; enforce containment under session cwd/workspace root where appropriate; do not silently create parent directories without explicit request semantics.

### P2-2 — Pipeline tool-node allowlist applies only to external MCP tools, not local tools

**Evidence**

- `crates/maenifold-core/src/workflow/pipeline/executor_run_exec.rs:107-127` dispatches local tools before allowlist enforcement.
- `crates/maenifold-core/src/workflow/pipeline/external_mcp.rs:120-130` enforces `require_tool_allowed` only for external MCP.
- `crates/maenifold-core/src/mcp/schema/pipeline_node_schema_data_variants.rs:152-156` describes `allowedTools`.

**Impact**

Pipeline definitions can invoke local `maenifold.*` tools despite an `allowedTools` restriction, bypassing a documented execution policy.

**Remediation**

Apply `allowedTools` before dispatching either local or external tools. Require explicit allowlist entries for privileged local tools.

### P2-3 — Repository audit target fails on Rust hygiene

**Evidence**

`make audit` fails with:

- `crates/maenifold-core/src/workflow/pipeline/executor_nodes_tests.rs:445` exceeds the 300-line file limit.
- `crates/maenifold-core/src/schedule/store.rs:218-219` uses `.unwrap()`.
- `crates/maenifold-core/src/teams/handler.rs:223`, `288` use `.unwrap()`.

**Impact**

The build-enforced code standards are not currently green, and `make audit` stops before frontend checks.

**Remediation**

Split the oversized test module and replace the non-test `.unwrap()` instances with explicit `expect` context or fallible paths consistent with repo rules.

### P2-4 — Teams unread indicator silently collapses failures to zero

**Evidence**

- `apps/desktop/src/components/AppTabBar.vue:48-69` returns zero on missing/invalid JSON or read errors.
- `apps/desktop/src/components/AppTabBar.vue:82-92` swallows non-capability subscription errors.

**Impact**

A broken Teams MCP resource or disconnected session appears as "no unread work."

**Remediation**

Track unavailable/error state separately from unread count and surface it in the UI.

### P2-5 — Design-system enforcement is weaker than DESIGN.md

**Evidence**

- `apps/desktop/DESIGN.md:158-200` says no `<style>` blocks and no raw HTML controls.
- `apps/desktop/scripts/audit-components.mjs:102-118` only enforces `max-style-rules` for style blocks over 15 rules.
- `apps/desktop/scripts/audit-components.mjs:41-49` detects only literal raw controls.

**Impact**

The build can pass while violating the normative design contract.

**Remediation**

Fail on any `<style>` block outside sanctioned primitives and use AST/template parsing for raw-control detection.

### P2-6 — Hidden file inputs bypass raw-control audit

**Evidence**

- `apps/desktop/src/components/ChatAttachmentBar.vue:129-148` uses `<component :is="'input'" ... type="file">`.
- The raw-control audit only catches literal `<input...>`.

**Impact**

This creates an enforcement bypass instead of a documented exception for necessary native file input behavior.

**Remediation**

Create a sanctioned file-input wrapper or encode a narrow explicit audit exception.

### P2-7 — UAT and tab tests are stale for the Teams tab

**Evidence**

- `docs/UAT-PLAN.md:45-52` lists seven tabs and omits Teams.
- `apps/desktop/src/composables/useAppShell.ts:9-18` includes Teams in runtime tab order.
- Targeted desktop test run reported `AppTabBar.test.ts` expected 7 tabs while runtime renders 8.

**Impact**

Documented navigation acceptance does not match shipped UI, and the stale test can hide real regressions.

**Remediation**

Update UAT plan and AppTabBar tests for Teams, including Teams MCP unread/error behavior.

### P2-8 — Environment-variable secrets are visible in settings UI

**Evidence**

- `apps/desktop/src/components/SettingsConnectionKeys.vue:46-64` renders env values as `type="text"`.

**Impact**

Agent/API tokens are shoulder-surfable and screenshot-prone.

**Remediation**

Use masked inputs by default with explicit reveal. Detect key names containing `TOKEN`, `KEY`, `SECRET`, or `PASSWORD` and mask aggressively.

### P2-9 — Destructive schedule delete uses raw `window.confirm`

**Evidence**

- `apps/desktop/src/composables/useSchedulesSettings.ts:184-192` uses `window.confirm`.
- `apps/desktop/src/components/SettingsSchedules.vue:125-127` routes delete through that composable.

**Impact**

This bypasses the shadcn/design-system dialog pattern and is harder to style, localize, and verify.

**Remediation**

Replace with the existing confirmation dialog/store or shadcn `AlertDialog`.

### P2-10 — Voice companion launch defaults on and swallows failures

**Evidence**

- `apps/desktop/src/stores/voice-handy.ts:15`, `23-41`, `44-60` defaults enabled and swallows check/launch failures.
- `apps/desktop/electron/voice.ts:19-24`, `88-111` resolves `handy` via `PATH` and launches it.

**Impact**

Users see an apparently available control that can silently fail; PATH resolution can launch an unexpected local executable.

**Remediation**

Default the integration off, surface errors, and prefer an explicit validated executable path or signed bundle identity.

### P2-11 — Manual tokenizer fallback is removable custom code

**Evidence**

- `crates/maenifold-core/src/vector/tokenizer.rs:24-47`, `98-218` builds a manual WordPiece tokenizer when `tokenizer.json` is absent.

**Impact**

This duplicates `tokenizers::Tokenizer::from_file` behavior and adds model-parity risk.

**Remediation**

Require curated/downloaded models to provide `tokenizer.json`, then delete the manual fallback.

### P2-12 — Unused dependencies detected

**Evidence**

`cargo machete --with-metadata` reported:

- `maenifold-gate`: `walkdir`
- `maenifold-core`: `assert_cmd`, `ctor`, `filetime`, `ndarray`, `predicates`, `sacp`, `similar-asserts`
- `maenifold-jockey`: `dirs`, `futures`, `which`

**Impact**

Extra dependencies increase audit surface and update burden.

**Remediation**

Remove each candidate one at a time with targeted builds/tests. If false positive, add explicit cargo-machete ignore metadata with rationale.

### P2-13 — Deprecated `serde_yaml` remains in workspace

**Evidence**

- `Cargo.toml:39`
- `crates/maenifold-core/Cargo.toml:33`
- `crates/maenifold-gate/Cargo.toml:26`
- `crates/maenifold-pipeline-driver/Cargo.toml:13`
- `Cargo.lock:5738-5749` resolves `serde_yaml 0.9.34+deprecated`.

**Impact**

Deprecated YAML parser with `unsafe-libyaml` transitive dependency remains in the graph.

**Remediation**

Delete YAML support where feasible or constrain parsing to trusted local files until a maintained replacement is selected.

### P2-14 — Duplicate `reqwest` stacks increase HTTP/TLS audit surface

**Evidence**

- `Cargo.toml:30`, `68`
- `Cargo.lock:5127-5185` contains both `reqwest 0.12.28` and `0.13.2`.

**Impact**

Duplicate HTTP/TLS stacks increase binary size and security review surface.

**Remediation**

Track `hf-hub` alignment with `reqwest 0.13`; feature-gate model download code if it is not needed in all builds.

### P2-15 — Stale workspace `tray-icon` version

**Evidence**

- `Cargo.toml:57` declares `tray-icon = "0.19"`.
- `crates/maenifold-core/Cargo.toml:61-63` and `crates/maenifold-gate/Cargo.toml:32-34` directly declare `tray-icon = "0.21"`.

**Impact**

Stale workspace dependency declaration can mislead audits and upgrades.

**Remediation**

Remove the workspace declaration or make crates consume `tray-icon.workspace = true` at the intended version.

### P2-16 — RustSec warnings in transitive graph

**Evidence**

Dependency audit reported:

- `glib 0.18.5` unsoundness via tray/native GUI stack.
- `boxfnonce 0.1.1` unmaintained via `sacp`.
- `instant 0.1.13` unmaintained via `notify`.
- `paste 1.0.15` unmaintained via `tokenizers` / GUI stack.
- `proc-macro-error 1.0.4` unmaintained via GTK stack.

**Impact**

Native GUI/tray dependencies pull older GTK/transitive dependencies.

**Remediation**

Upgrade tray/native GUI dependencies, feature-gate tray/native UI away from CLI/server builds, and re-evaluate direct `sacp` usage.

### P2-17 — Protocol/client implementation duplication increases maintenance and review surface

**Evidence**

- Backend hand-rolled MCP stdio/discovery code: `crates/maenifold-core/src/mcp/stdio_session.rs`, `crates/maenifold-core/src/api/mcp_discovery/stdio.rs`.
- Frontend hand-rolled protocol clients: `apps/desktop/src/lib/acp-client-core.ts`, `apps/desktop/src/lib/acp-rpc.ts`, `apps/desktop/src/composables/useMcpClient.ts`, `apps/desktop/src/composables/mcpTransport.ts`.
- Companion detail: `docs/dependency-simplification-audit.md`.

**Impact**

Custom JSON-RPC, timeout, reconnect, stderr, and session logic increases security review burden and protocol drift risk.

**Remediation**

Prefer already-installed upstream SDKs/libraries where parity exists: `rmcp` for backend MCP stdio/client transport, `@agentclientprotocol/sdk` for ACP types/transport helpers, `@modelcontextprotocol/sdk` for MCP Streamable HTTP if Electron renderer parity is confirmed.

## Positive controls observed

- `cargo build --release -p maenifold-cli` succeeded.
- `cd apps/desktop && npm run lint:design && npm run audit:components` passed.
- `cd apps/desktop && npx vue-tsc --noEmit` passed.
- Electron baseline controls are present: `contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`, restricted navigation/window-open handling.
- Gate state-machine basics derive transitions from process definitions, reject terminal transitions, and validate task path containment/symlink metadata.
- Pipeline driver has explicit locking/recovery handling.
- Jockey model/thought-level config failures are generally treated as fatal rather than silently falling back.

## Tooling issue fixed during audit

Rust LSP was initially unusable because the active pinned toolchain did not include `rust-analyzer` or `rust-src`, and rust-analyzer initialization hit the default timeout.

Changes made:

- `rust-toolchain.toml` now declares `["rustfmt", "clippy", "rust-analyzer", "rust-src"]`.
- `.github/lsp.json` now sets `initializationTimeoutMs` to `180000` for `rust-analyzer`.
- Installed `rust-analyzer` and `rust-src` for `1.91.0-aarch64-apple-darwin`.

Validation:

- `rust-analyzer --version` reports `rust-analyzer 1.91.0`.
- `cargo metadata --no-deps --format-version 1` succeeds.
- LSP operations succeeded afterward: `workspaceSymbol`, `documentSymbol`, `goToDefinition`, `findReferences`, `incomingCalls`, `outgoingCalls`, and `hover` on Rust files.

## Validation commands

| Command | Result |
|---|---|
| `maenifold.gate` `gate_Sync` via `GatewayExecute` | Passed; rebuilt 516 tasks |
| `maenifold.gate` `gate_Status` via `GatewayExecute` | Passed; open task list returned |
| `make audit` | Failed on Rust audit: 1 file-length violation, 4 `.unwrap()` violations |
| `cd apps/desktop && npm run lint:design && npm run audit:components` | Passed |
| `cd apps/desktop && npx vue-tsc --noEmit` | Passed |
| `cargo build --release -p maenifold-cli` | Passed |
| `rust-analyzer --version` | Passed |
| `cargo metadata --no-deps --format-version 1` | Passed |

Sub-audits also ran scoped dependency checks including `cargo machete --with-metadata`, `cargo audit`, `npm audit --omit=dev`, `cargo tree -d`, `npm ls --depth=0`, and targeted frontend tests. The targeted AppTabBar test failed because it expected 7 tabs while the runtime now renders 8.

## Recommendation feasibility and latest-version verification

Context7 status: attempted after the audit challenge, but Context7 returned "Monthly quota exceeded" for every library lookup. The feasibility notes below are therefore **not Context7-backed**. They are grounded in current npm/crates metadata plus first-party docs fetched directly from official project docs, docs.rs, and package READMEs.

| Recommendation target | Installed/direct version | Latest observed version | Feasibility evidence | Audit position |
|---|---:|---:|---|---|
| DOMPurify for Mermaid SVG sanitization | `dompurify` 3.4.0 | 3.4.1 | DOMPurify README says it is an XSS sanitizer for "HTML, MathML and SVG", supports `DOMPurify.sanitize(...)`, and warns that modifying markup after sanitization can void the effect. | Feasible. Also update to 3.4.1 if compatibility checks pass. |
| JSON Forms for schema forms | `@jsonforms/core` / `@jsonforms/vue` 3.7.0 | 3.7.0 | JSON Forms Vue docs say Vue 3 support exists via `@jsonforms/vue` and HTML5 renderers via `@jsonforms/vue-vanilla`, but the Vue integration is in "preview state". | Feasible only as a parity spike, not a drop-in guarantee. Preserve existing safety guards. |
| VueUse storage helpers | `@vueuse/core` 14.2.1 | 14.2.1 | VueUse `useStorage` docs show reactive localStorage/sessionStorage refs, custom serializers, merge defaults, and error hooks. | Feasible for low-risk localStorage helper consolidation. |
| MCP TypeScript SDK for frontend MCP client | `@modelcontextprotocol/sdk` 1.29.0 | 1.29.0 | v1 docs say the high-level client supports Streamable HTTP, `listTools`, `callTool`, `listResources`, and `readResource`. The main README warns v2 is pre-alpha and v1.x remains recommended for production. | Feasible as a spike against installed v1 API only. Do not migrate against v2/main docs. |
| ACP TypeScript SDK for frontend ACP bridge | `@agentclientprotocol/sdk` 0.19.0 | 0.20.0 | Official README describes it as the official TS implementation and points client integrations to `ClientSideConnection`; npm metadata shows Apache-2.0 and `zod` peer dependency. | Feasible for types/client abstraction spike; upgrade to 0.20.0 only after compatibility check. |
| `rmcp` for backend MCP stdio/client transport | workspace `rmcp` 1.5.0 | 1.5.0 | docs.rs describes `rmcp` as the official Rust SDK, with `client`, `transport-child-process`, `TokioChildProcess`, and `ServiceExt`/client transport support. | Feasible in `maenifold-core`, which already enables client/child-process features. Preserve stderr, timeout, and poisoning semantics. |
| `tempfile` for atomic task writes | workspace `tempfile` 3.23.0 | 3.27.0 | docs.rs documents `NamedTempFile`, named temporary files, and persist errors. | Feasible for temp-file creation; preserve explicit directory sync durability behavior. |
| `fd-lock` for Gate file locks | workspace `fd-lock` 4.0.4 | 4.0.4 | docs.rs documents advisory `RwLock` and RAII read/write guards, and explicitly notes advisory locks coordinate cooperating programs but do not prevent all access. | Feasible as consolidation of an already-present dependency; not a security boundary by itself. |
| Mermaid vulnerability handling | `mermaid` 11.14.0, transitive `uuid` 11.1.0 | `mermaid` 11.14.0, `uuid` 14.0.0 | npm metadata shows Mermaid is already latest while `uuid` latest is 14.0.0; dependency audit reported `uuid < 14` advisory. | Do not claim "upgrade Mermaid fixes it" yet. Use override/sandbox/removal only after compatibility and audit review. |

## Companion report

Detailed dependency-reuse and simplification recommendations are in `docs/dependency-simplification-audit.md`. Highest-value deletion/reuse opportunities from that report:

1. Sanitize Mermaid output with existing DOMPurify and remove custom HTML insertion paths.
2. Replace backend hand-rolled MCP stdio/discovery transport with `rmcp` where parity is proven.
3. Replace or wrap frontend custom ACP/MCP JSON-RPC clients with installed protocol SDKs where parity exists.
4. Replace Gate custom atomic-write/flock helpers with existing `tempfile` and `fd-lock` where semantics match.
5. Collapse custom schema-form rendering into existing JSON Forms packages where safety checks can be preserved.
