# Dependency Simplification Audit

Context: [[supply-chain-security]], [[code-simplification]], [[maenifold]], [[dependency-audit]]

## Simplification Audit Results

### Executive Summary

- Total custom code identified: approximately 131K active source LOC scanned across `crates/` and `apps/desktop/src/`; approximately 3.1K LOC mapped to concrete simplification candidates.
- Replacement opportunities: 7
  - Tier 1: 3
  - Tier 2: 2
  - Tier 3: 2
- Estimated code reduction: approximately 1.7K-2.3K LOC if all recommended migrations land, plus reduced bespoke protocol/security review surface.
- Supply chain risk reduction: strongest gains come from deleting hand-rolled JSON-RPC/MCP/ACP transport code, replacing custom atomic file/lock primitives with already-present crates, and routing Mermaid SVG insertion through the existing DOMPurify sanitizer instead of custom escaping. Recommendations prefer existing dependencies already present in manifests/lockfiles, minimizing new supply-chain exposure.

Scope audited:

- Rust backend: `crates/maenifold-core`, `crates/maenifold-gate`, `crates/maenifold-jockey`, `crates/maenifold-pipeline-driver`, `crates/maenifold-cli`
- Electron/Vue frontend: `apps/desktop/src`, relevant `apps/desktop/electron`, `apps/desktop/package.json`, `apps/desktop/package-lock.json`
- Excluded: `target/`, `node_modules/`, `dist/`, `out/`, `.git/`, `worktrees/`

Manifest/license observations:

- Existing Rust candidates:
  - `rmcp` 1.5.0: Apache-2.0; latest 1.5.0; crates.io updated 2026-04-16; 8.4M total / 5.6M recent downloads; OSV query for 1.5.0 returned 0 vulnerabilities.
  - `tempfile` 3.23.0 in workspace: MIT OR Apache-2.0; latest 3.27.0; crates.io updated 2026-03-11; 538M total / 107M recent downloads; OSV query for 3.23.0 returned 0 vulnerabilities.
  - `fd-lock` 4.0.4: MIT OR Apache-2.0; latest 4.0.4; crates.io updated 2025-03-10; 42M total / 8.8M recent downloads; OSV query for 4.0.4 returned 0 vulnerabilities. Because maintenance freshness is weaker than the other candidates, reuse it only as consolidation of an already-present dependency, not as a reason to broaden locking abstractions.
  - `schemars`: MIT
  - `pulldown-cmark`: MIT
  - `slug`: MIT/Apache-2.0
- Existing frontend candidates:
  - `dompurify` 3.4.0: MPL-2.0 OR Apache-2.0; latest 3.4.1; npm modified 2026-04-21; 0 runtime dependencies, 1 optional dependency; OSV query for 3.4.0 returned 0 vulnerabilities.
  - `@jsonforms/core` / `@jsonforms/vue` 3.7.0: MIT; latest 3.7.0; npm modified 2026-03-05; `@jsonforms/vue` has 1 runtime dependency and 2 peer dependencies; OSV query for 3.7.0 returned 0 vulnerabilities.
  - `@vueuse/core` 14.2.1: MIT; latest 14.2.1; npm modified 2026-02-10; 3 runtime dependencies and 1 peer dependency; OSV query for 14.2.1 returned 0 vulnerabilities.
  - `@modelcontextprotocol/sdk` 1.29.0: MIT; latest 1.29.0; npm modified 2026-03-30; 17 runtime dependencies and 2 peer dependencies; OSV query for 1.29.0 returned 0 vulnerabilities. The dependency footprint is larger than the ideal threshold, so this recommendation is justified only because the SDK is already installed and replaces security-sensitive custom protocol code.
  - `@agentclientprotocol/sdk` 0.19.0 in workspace: Apache-2.0; latest 0.20.0; npm modified 2026-04-23; 0 runtime dependencies and 1 peer dependency; OSV query for 0.19.0 returned 0 vulnerabilities.

### Tier 1 - Replace Immediately

#### 1. Sanitize Mermaid SVG insertion with existing DOMPurify

- **Current Implementation**: `apps/desktop/src/composables/useMermaid.ts`, approximately 99 LOC; used by `ChatView.vue` and `KnowledgeNoteRenderer.vue`.
  - Custom `escapeHtml()` is used only for error text.
  - Successful Mermaid renders replace sanitized Markdown with:
    - `preBlock.outerHTML` set to a `mermaid-rendered` wrapper containing the raw Mermaid SVG.
  - This bypasses the DOMPurify sanitization already used before rendering Markdown.
- **Recommended Replacement**: existing `dompurify` 3.4.0 for Mermaid SVG output; prefer DOM APIs for error insertion instead of `insertAdjacentHTML`.
- **License**: Apache-2.0 option available from `dompurify` lock metadata.
- **Benefits**:
  - Security improvement: closes a sanitizer bypass around SVG/HTML injection.
  - Code reduction: modest, approximately 15-30 LOC.
  - Maintenance savings: one sanitizer policy for Markdown and Mermaid output.
- **Supply Chain**:
  - Already installed dependency.
  - No new transitive dependency introduced.
  - License satisfies Apache-2.0 requirement.
  - DOMPurify is a widely used security-focused sanitizer; keep it updated because it is security-sensitive.
- **Migration Effort**: Low.
  - Wrap Mermaid `svg` with `DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true, svgFilters: true } })` or a tighter allowlist.
  - Replace error HTML insertion with `document.createElement()` and `textContent`.
- **Action**:
  1. Import DOMPurify in `useMermaid.ts`.
  2. Sanitize `svg` before assigning to DOM.
  3. Avoid `outerHTML`/`insertAdjacentHTML` where possible; construct elements directly.
  4. Add tests with malicious Mermaid labels/links to verify sanitizer behavior.

#### 2. Replace backend MCP stdio JSON-RPC/process transport with `rmcp` client transport

- **Current Implementation**:
  - `crates/maenifold-core/src/mcp/stdio_session.rs`, approximately 298 LOC
  - `crates/maenifold-core/src/api/mcp_discovery/stdio.rs`, approximately 279 LOC
  - Total: approximately 577 LOC of custom JSON-RPC framing, stdio line reading, stderr capture, child process groups, cancellation/timeout, and MCP initialize/list/call/resource operations.
- **Recommended Replacement**: existing `rmcp` 1.5.0 client transport, especially the already-used `rmcp::transport::TokioChildProcess` / `ServiceExt` pattern visible in `crates/maenifold-core/src/workflow/pipeline/external_mcp_transport.rs`.
- **License**: Apache-2.0.
- **Benefits**:
  - Estimated code reduction: 350-500 LOC.
  - Security improvement: deletes bespoke protocol framing and process-lifecycle code.
  - Consistency: repo already uses `rmcp` for MCP servers and external MCP pipeline transport.
  - Fewer custom timeout/error-path bugs in gateway/discovery code.
- **Supply Chain**:
  - Already in workspace dependencies.
  - No new direct dependency.
  - Feature flags already include MCP server/client transport capabilities in `maenifold-core`.
  - Latest crate metadata: Apache-2.0, latest 1.5.0, updated 2026-04-16, 8.4M total downloads, 5.6M recent downloads, OSV returned 0 vulnerabilities for 1.5.0.
- **Migration Effort**: Medium to High.
  - Preserve gateway behavior:
    - stderr tail inclusion
    - startup timeout
    - per-call timeout
    - protocol-version compatibility
    - session poisoning after timeout
    - tool/resource routing semantics
  - `stdio_pool.rs` depends on the current `StdioSession` API, so adapter boundaries should be introduced first.
- **Action**:
  1. Create a small trait for gateway sessions: `tools()`, `call_tool()`, `read_resource()`, `shutdown()`, `is_usable()`.
  2. Add an `rmcp`-backed implementation beside current `StdioSession`.
  3. Port discovery code first because it is narrower than pooled gateway sessions.
  4. After parity tests pass, remove hand-rolled request/response line parser and duplicate process-group termination helpers.

#### 3. Replace frontend hand-rolled MCP/ACP JSON-RPC clients with installed protocol SDKs where feature parity exists

- **Current Implementation**:
  - `apps/desktop/src/lib/acp-client-core.ts`, approximately 357 LOC
  - `apps/desktop/src/lib/acp-rpc.ts`, approximately 72 LOC
  - `apps/desktop/src/composables/useMcpClient.ts`, approximately 404 LOC
  - `apps/desktop/src/composables/mcpTransport.ts`, approximately 191 LOC
  - Total: approximately 1,024 LOC of custom WebSocket JSON-RPC, request maps, timeouts, SSE parsing, reconnect logic, MCP Streamable HTTP session headers, and response discrimination.
- **Recommended Replacement**:
  - Existing `@agentclientprotocol/sdk` 0.19.0 for ACP message/client types and any available client transport helpers.
  - Existing `@modelcontextprotocol/sdk` 1.29.0 for MCP Streamable HTTP client/session handling if browser-compatible APIs satisfy Electron renderer needs.
- **License**:
  - `@agentclientprotocol/sdk`: Apache-2.0
  - `@modelcontextprotocol/sdk`: MIT
- **Benefits**:
  - Estimated code reduction: 500-900 LOC if SDKs cover transport/session needs.
  - Security improvement: fewer bespoke protocol parsers and timeout/reconnect edge cases.
  - Maintenance improvement: protocol version changes tracked upstream instead of duplicated locally.
- **Supply Chain**:
  - Both SDKs are already present in `apps/desktop/package-lock.json`.
  - No new dependency required.
  - License-compatible.
  - `@agentclientprotocol/sdk` has a small footprint: 0 runtime dependencies and 1 peer dependency; OSV returned 0 vulnerabilities for installed 0.19.0.
  - `@modelcontextprotocol/sdk` is heavier: 17 runtime dependencies and 2 peer dependencies; OSV returned 0 vulnerabilities for 1.29.0. Use it only where it deletes more custom protocol/security surface than it adds.
  - Browser/Electron renderer compatibility must be proven by the first migration step before deleting local transport behavior.
- **Migration Effort**: High.
  - The current client integrates with UI traffic logging, permission prompts, stderr relay events, reconnect behavior, and stateless MCP tests.
  - SDK browser/Electron renderer compatibility must be confirmed.
- **Action**:
  1. Spike MCP replacement first behind `useMcpClient`'s existing public API.
  2. Preserve the existing composable return shape so stores/components do not churn.
  3. Confirm SDK supports:
     - Streamable HTTP
     - `Mcp-Session-Id`
     - SSE notifications
     - browser/Electron renderer fetch
     - explicit disconnect/delete
  4. Only then attempt ACP WebSocket bridge replacement or partial extraction.
  5. If SDK transport cannot fully replace custom logic, still delete `acp-rpc.ts` by using SDK JSON-RPC/message types and validators.

### Tier 2 - Strong Candidates

#### 1. Replace custom gate atomic write and flock implementation with existing `tempfile` and `fd-lock`

- **Current Implementation**: `crates/maenifold-gate/src/task_file.rs`, approximately 236 LOC.
  - Custom atomic temp filename generation via `AtomicU64`, timestamp, PID.
  - Manual open/write/sync/rename/sync-directory loop.
  - Manual Unix `libc::flock` lock/unlock with no-op non-Unix fallback.
- **Recommended Replacement**:
  - `tempfile::NamedTempFile::new_in(parent)?.persist(path)` for atomic write, matching the pattern already used in `crates/maenifold-core/src/markdown/storage.rs`.
  - `fd_lock::RwLock` for RAII file locks, already used in pipeline lease code.
- **License**:
  - `tempfile`: MIT OR Apache-2.0
  - `fd-lock`: MIT OR Apache-2.0
- **Benefits**:
  - Estimated code reduction: 80-140 LOC.
  - Removes unsafe `libc::flock` calls from this module.
  - Aligns gate locking with existing pipeline locking semantics.
  - Cross-platform behavior becomes library-defined instead of custom no-op.
- **Supply Chain**:
  - Both crates are already in workspace dependencies.
  - No new dependency.
  - `tempfile` is current and high-adoption: latest 3.27.0, updated 2026-03-11, 538M total downloads, OSV returned 0 vulnerabilities for installed 3.23.0.
  - `fd-lock` is high-adoption but less fresh: latest 4.0.4, updated 2025-03-10, 42M total downloads, OSV returned 0 vulnerabilities for 4.0.4. Treat this as consolidation of an already-present crate, not a green light for broader new locking use.
- **Migration Effort**: Low to Medium.
  - Verify `fd-lock` blocking semantics match current `TaskFileLock::acquire`.
  - Preserve directory sync behavior if required for durability.
- **Action**:
  1. Replace `write_atomically()` internals with `NamedTempFile`.
  2. Replace `TaskFileLock.file` with an `fd_lock` guard type.
  3. Keep existing concurrency tests and add one non-Unix behavior expectation if the project supports Windows.

#### 2. Collapse custom schema form complexity/fallback layer into JSON Forms renderers where possible

- **Current Implementation**:
  - `apps/desktop/src/lib/schema-form.ts`, approximately 282 LOC
  - `apps/desktop/src/lib/schema-form-simple.ts`, approximately 214 LOC
  - `apps/desktop/src/components/SchemaForm.vue`, approximately 278 LOC
  - `apps/desktop/src/components/SchemaFormField.vue`, approximately 111 LOC
  - `apps/desktop/src/components/SchemaFormRawEditor.vue`, approximately 41 LOC
  - Total: approximately 926 LOC.
- **Recommended Replacement**: existing `@jsonforms/vue`, `@jsonforms/core`, and `@jsonforms/vue-vanilla` renderers with a project-specific renderer set only for truly custom fields.
- **License**: MIT.
- **Benefits**:
  - Estimated code reduction: 250-600 LOC depending on how much simple-field UX must remain.
  - Reduces custom JSON Schema traversal/complexity scoring.
  - Centralizes validation/error rendering in JSON Forms/Ajv instead of custom `computeSimpleErrors`.
- **Supply Chain**:
  - JSON Forms packages are already installed.
  - No new dependency.
  - Latest npm metadata for `@jsonforms/vue` 3.7.0: MIT, modified 2026-03-05, 1 runtime dependency, 2 peer dependencies, OSV returned 0 vulnerabilities.
- **Migration Effort**: Medium.
  - Existing complexity guardrails may encode deliberate UX/safety behavior:
    - external `$ref` rejection
    - schema size limits
    - unsupported keyword fallback
    - raw editor fallback
  - Preserve safety checks even if rendering moves to JSON Forms.
- **Action**:
  1. Keep `hasExternalRef` and size guard initially.
  2. Replace simple object renderer path with JSON Forms vanilla renderer.
  3. Move custom labels/help text into JSON Forms UI schema rather than custom field models.
  4. Delete `schema-form-simple.ts` once parity tests pass.

### Tier 3 - Consider

#### 1. Replace localStorage helper/cache boilerplate with `@vueuse/core` `useStorage`

- **Current Implementation**:
  - `apps/desktop/src/lib/local-storage.ts`, approximately 51 LOC
  - `apps/desktop/src/lib/agent-probe-cache.ts`, approximately 94 LOC
  - Additional usage scattered through stores/composables.
- **Recommended Replacement**: existing `@vueuse/core` `useStorage` / `useLocalStorage`, with small schema migration wrapper for versioned data.
- **License**: MIT.
- **Benefits**:
  - Estimated code reduction: 50-100 LOC initially.
  - Better Vue reactivity and SSR/window-availability handling.
  - Avoids repeating parse/stringify fallback behavior.
- **Supply Chain**:
  - Already installed.
  - No new dependency.
  - Latest npm metadata for `@vueuse/core` 14.2.1: MIT, modified 2026-02-10, 3 runtime dependencies, 1 peer dependency, OSV returned 0 vulnerabilities.
- **Migration Effort**: Low.
  - Current helpers are simple and stable; apply while editing related stores.
- **Action**:
  1. Pilot on agent probe cache.
  2. Keep version normalization logic.
  3. Remove `local-storage.ts` after all call sites migrate.

#### 2. Consolidate Markdown renderers to one pipeline

- **Current Implementation**:
  - `apps/desktop/src/lib/markdown.ts`, approximately 33 LOC, configures singleton `marked` + `highlight.js`.
  - `apps/desktop/src/composables/useMarkdownIt.ts`, approximately 103 LOC, configures `markdown-it` + Shiki + WikiLink plugin.
  - Multiple components render Markdown with `marked.parse()` and DOMPurify, while knowledge notes use `markdown-it`.
- **Recommended Replacement**: standardize on the existing `markdown-it` + Shiki pipeline, or standardize on `marked` if WikiLink rendering is made a `marked` extension. Prefer deleting one renderer stack.
- **License**:
  - `markdown-it`: MIT
  - Shiki packages: MIT
  - `marked`: MIT
  - `highlight.js`: BSD-3-Clause, which does not meet the strict "MIT or Apache-2.0 only" preference for new recommendations; because it is already present, treat removal as a supply-chain simplification.
- **Benefits**:
  - Estimated code reduction: 30-120 LOC plus one fewer markdown/highlighting dependency path if `marked`/`highlight.js` become unused.
  - Consistent sanitizer policy and WikiLink rendering behavior.
  - Less bundle weight and less review surface around `v-html`.
- **Supply Chain**:
  - Uses existing packages.
  - Potential to remove `marked`, `marked-highlight`, and `highlight.js` if no longer needed.
- **Migration Effort**: Low to Medium.
  - Update components currently calling `marked.parse()`.
  - Async Shiki initialization may require component lifecycle adjustments.
- **Action**:
  1. Inventory all `marked.parse()` call sites.
  2. Move shared sanitized rendering into one composable.
  3. Delete `configureMarkdown()` and dependencies after call sites migrate.

### Rejected Opportunities

#### WikiLink extraction and normalization

- **Current Implementation**:
  - `crates/maenifold-core/src/markdown/wikilinks.rs`
  - `crates/maenifold-core/src/repair/wikilinks.rs`
- **Why not replace**:
  - This is core [[maenifold]] business logic, not generic Markdown parsing.
  - It deliberately combines `pulldown-cmark` offset parsing with custom WikiLink rejection rules, code-block exclusion, concept normalization, and graph-routing semantics.
  - Generic wiki-link plugins would likely lose parity around malformed/nested brackets and graph hygiene.
- **Decision**: Keep custom logic, but continue using `pulldown-cmark` as the parser substrate.

#### Frontmatter split/render with new frontmatter crate

- **Current Implementation**:
  - `crates/maenifold-core/src/markdown/storage.rs`
- **Why not replace**:
  - Current code already uses `serde_yaml` for parsing and has project-specific behavior:
    - BOM stripping
    - malformed YAML warning queue
    - deterministic frontmatter key ordering
    - checksum behavior
    - atomic writes
  - Adding a new frontmatter crate would add supply-chain risk for limited LOC reduction.
- **Decision**: Keep for now. Consider only if `serde_yaml` deprecation becomes blocking.

#### Path traversal and memory URI validation

- **Current Implementation**:
  - `crates/maenifold-core/src/util/paths.rs`
  - `crates/maenifold-core/src/memory/support.rs`
- **Why not replace**:
  - Security-sensitive and tailored to `memory://` semantics.
  - Uses canonicalization to prevent symlink escape from configured roots.
  - A generic path-cleaning crate could obscure threat-model details.
- **Decision**: Keep custom, add tests rather than replace.

#### Graph traversal, community analysis, pipeline state machines

- **Current Implementation**:
  - `crates/maenifold-core/src/graph/`
  - `crates/maenifold-core/src/workflow/pipeline/`
  - `crates/maenifold-gate/src/task_status.rs`
- **Why not replace**:
  - Domain-specific behavior.
  - Existing use of `petgraph` already delegates generic graph primitives.
  - Pipeline/gate state transitions encode product semantics.
- **Decision**: Keep. Refactor for clarity if needed, but do not outsource.

#### Launchd/systemd scheduling templates

- **Current Implementation**:
  - `crates/maenifold-core/src/schedule/launchd.rs`
  - `crates/maenifold-core/src/schedule/systemd.rs`
- **Why not replace**:
  - OS-specific integration with small surface area.
  - Generic scheduler crates would not eliminate platform template validation and could add heavy dependencies.
- **Decision**: Keep.

### Critical Findings

1. **Mermaid SVG sanitizer bypass**
   - `useMermaid.ts` injects Mermaid-rendered SVG via `outerHTML` after Markdown has already been sanitized elsewhere.
   - This is security-sensitive because SVG is an HTML-capable attack surface.
   - Fix with existing DOMPurify before broad UI work.

2. **Duplicated protocol/client implementations increase security review burden**
   - Backend and frontend both contain hand-rolled JSON-RPC/MCP/ACP request routing, parsing, timeout, and reconnect/session logic.
   - These are not inherently vulnerable from local inspection, but the threat model is unclear and the maintenance burden is high.
   - Prefer upstream SDKs already in the dependency graph.

3. **Custom process spawning handles user-configured commands**
   - Rust code generally uses `Command::new(...).args(...)`, which avoids shell interpolation.
   - Still, MCP/ACP agent command configuration is effectively local code execution and should remain explicitly consented and documented.
   - Keep command provenance and UI warnings visible.

4. **`serde_yaml` is marked deprecated in local cargo metadata**
   - Current use is functional and wrapped in project-specific behavior.
   - Do not add a replacement casually, but track this as dependency hygiene.
   - Any replacement must preserve YAML frontmatter compatibility and license constraints.

### Quality-control checklist

- [x] Verified recommended replacement licenses through local metadata and public package registry metadata.
- [x] Checked maintenance freshness through public registry metadata.
  - `fd-lock` is the weak point: latest 4.0.4 was published/updated on 2025-03-10. Its recommendation is limited to consolidating an already-present dependency and removing local unsafe locking code.
- [x] Checked package-version vulnerability reports through OSV queries for the recommended installed versions; all queried package/version pairs returned 0 vulnerabilities.
- [x] Validated dependency depth aligns with supply-chain security principles by preferring already-present dependencies.
- [x] Confirmed feature parity where the replacement is a narrow implementation swap; protocol SDK replacements are explicitly scoped to prove parity before deleting local behavior.
- [x] Estimated migration effort and breaking-change risk.
- [x] Identified security-sensitive custom code that should be prioritized.
- [x] Cross-checked for existing usage of recommended libraries to avoid adding duplicate capability.
- [x] Avoided recommending micro-dependencies for trivial helpers.
- [x] Preferred language standard library or existing dependencies before new dependencies.

### Context7 / docs verification note

Context7 lookup was attempted for DOMPurify, JSON Forms, VueUse, the MCP TypeScript SDK, the ACP TypeScript SDK, `rmcp`, `tempfile`, and `fd-lock`, but Context7 returned "Monthly quota exceeded" for each request. The recommendations in this report are therefore **not Context7-backed**. They are grounded in current package metadata, OSV/registry checks, docs.rs, and first-party project documentation fetched directly.

Important feasibility qualifiers:

- `@modelcontextprotocol/sdk` replacement work must target installed v1.x APIs. The main repository README currently describes v2 as pre-alpha and says v1.x remains recommended for production.
- JSON Forms Vue support exists for Vue 3, but official docs describe the integration as preview. Treat the schema-form replacement as a spike, not a guaranteed drop-in.
- `fd-lock` provides advisory locks for cooperating processes; it should consolidate existing file locking, not be treated as an access-control boundary.
- Mermaid is already at the latest observed version while the transitive `uuid` remains below the fixed major. Do not assume a Mermaid upgrade alone fixes the advisory until upstream releases or an override is compatibility-tested.

### Grounding

- Current repository manifests: `Cargo.toml`, crate-level `Cargo.toml` files, and `apps/desktop/package.json`.
- Current architecture docs: `docs/acp-client-architecture.md`, `docs/UAT-PLAN.md`.
- Maenifold context considered: `memory://research/rust-code-quality-tooling-inventory`, `memory://build/research/maenifold-rs/current-code-sanity-check-for-maenifoldrs-20260330`, `memory://sprints/codebase-audit-crate-integration-opportunities`.
