# TODO

## Active backlog

- REL-001: Release v1.0.3 tasks (**Complete**)
- OPENCODE-001: OpenCode Plugin active work
- MAENIFOLDPY-001: Python port tasks
- DIST-001: Windows MSI installer
- CLEANUP-001: Active thinking sessions (**Complete**)
- COMMUNITY-001: Community detection (Louvain)
- HOOKS-001: Claude Code session start hook redesign (**Complete**)
- SITE-001: Site rebuild ("Warm Restraint")
- COV-001: Test coverage sprint (FR-17.x)
- HSM-001: Hierarchical State Machines — Workflow Supervisor (FR-10.x)
- APP-001: MCP Apps UI — Sync & MemoryStatus (FR-19.x)
- APP-002: MCP Apps UI — Workflow/HSM Visualizer (FR-20.x)
- APP-003: MCP Apps UI — Sequential Thinking Tree (FR-21.x)
- GOOSE-001: Goose Extension (FR-22.x)

## Sprint: Hierarchical State Machines (T-HSM-001)

Wave 1 — Implementation (serial):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HSM-001.1 | SWE: Implement supervisor state machine — add `phase`/`activeSubmachineType`/`activeSubmachineSessionId` frontmatter to Start(), add `submachineSessionId` param to Workflow(), implement register path and gate check in Continue() | FR-10.1, FR-10.3, FR-10.4 | **Complete** |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HSM-001.2 | Blue-team: Integration tests — workflow enters waiting, blocks while submachine active, unblocks on completion, backward compat, frontmatter persistence | FR-10.1-10.4 | **Complete** |
| T-HSM-001.3 | Red-team: Adversarial review — invalid sessionIds, abandoned status, missing files, response persistence. HIGH-001 + MEDIUM-001 remediated. | Security | **Complete** |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HSM-001.4 | Blue-team: Verify coverage held, all FR-10.x requirements met — 814/814 tests, line 77.77%, branch 67.22%, method 92.92%, 0 warnings | FR-10.1-10.6 | **Complete** |

## Sprint: MCP Apps UI (T-APP-001)

Wave 1 — Implementation (serial — resource registration + tool wiring are tightly coupled):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-001.1 | SWE: Register ui:// resources — create `AppResources.cs` with `[McpServerResource]` for `ui://maenifold/sync` and `ui://maenifold/memory-status`, MIME `text/html;profile=mcp-app`, embedded HTML templates with SEP-1865 lifecycle | FR-19.1, FR-19.6 | Pending |
| T-APP-001.2 | SWE: Wire Sync tool — add `[McpMeta]` annotation, change return type to include structuredContent (filesProcessed, conceptMentions, conceptRelations, orphanedConcepts, communities, modularity, vectors, vectorReady), preserve CLI backward compat | FR-19.2, FR-19.4, FR-19.9 | Pending |
| T-APP-001.3 | SWE: Wire MemoryStatus tool — add `[McpMeta]` annotation, change return type to include structuredContent (files.total, files.folders, files.sizeBytes, files.sizeMB, graph.concepts, graph.relations, graph.mentions, graph.dbSizeBytes, graph.dbSizeMB), preserve CLI backward compat | FR-19.3, FR-19.5, FR-19.9 | Pending |

Wave 1 — HTML Templates (parallel with T-APP-001.2/3 after T-APP-001.1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-001.4 | SWE: Sync dashboard HTML — read-only metrics display, host theming, SEP-1865 lifecycle, embedded C# string constant, < 5KB | FR-19.6, FR-19.7, FR-19.8 | Pending |
| T-APP-001.5 | SWE: MemoryStatus dashboard HTML — read-only metrics display, host theming, SEP-1865 lifecycle, embedded C# string constant, < 5KB | FR-19.6, FR-19.7, FR-19.8 | Pending |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-001.6 | Blue-team: Tests — structuredContent data shapes, CLI backward compat, learn param, resource registration, existing tests pass, coverage thresholds held | FR-19.1-19.9 | Pending |
| T-APP-001.7 | Red-team: Adversarial review — XSS in HTML templates, CSP, postMessage origin, structuredContent injection, CLI regression audit | Security | Pending |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-001.8 | Blue-team: Verify all FR-19.x requirements met, tests pass, coverage held, 0 warnings | FR-19.1-19.9 | Pending |

## Sprint: MCP Apps UI — Workflow/HSM Visualizer (T-APP-002)

Wave 1 — Implementation (serial — resource registration + tool wiring are tightly coupled):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-002.1 | SWE: Register `ui://maenifold/workflow` resource — add `[McpServerResource]` to `AppResources.cs`, MIME `text/html;profile=mcp-app`, embedded HTML template with SEP-1865 lifecycle and read-only HSM state dashboard | FR-20.1, FR-20.7, FR-20.8, FR-20.9 | Pending |
| T-APP-002.2 | SWE: Wire Workflow tool — add `[McpMeta]` annotation, change return type to include structuredContent (session state, step list from workflow JSON, submachine state fields), preserve CLI backward compat | FR-20.2, FR-20.3, FR-20.4, FR-20.5, FR-20.6, FR-20.10 | Pending |
| T-APP-002.4 | SWE: Implement `WorkflowStatus` app-only tool (`visibility: ["app"]`) returning current session state | FR-20.11 | Pending |
| T-APP-002.5 | SWE: Add self-polling to Workflow HTML template (poll `WorkflowStatus` via `tools/call`, 3s interval, terminal state stop; visibility-gated: IntersectionObserver + visibilityState) | FR-20.12, FR-20.13, FR-20.14, FR-20.15, FR-20.16 | Pending |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-002.3 | Blue-team: Tests — structuredContent shape (session state, step titles, submachine fields), CLI backward compat, learn param, existing Workflow tests pass, coverage thresholds held | FR-20.1-20.16 | Pending |
| T-APP-002.6 | Red-team: Adversarial review — XSS in step titles, structuredContent injection, CLI regression audit | Security | Pending |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-002.7 | Blue-team: Verify all FR-20.x requirements met, tests pass, coverage held, 0 warnings | FR-20.1-20.16 | Pending |

## Sprint: MCP Apps UI — Sequential Thinking Tree (T-APP-003)

Wave 1 — Implementation (serial — parser must land before tool wiring):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-003.1 | SWE: Register `ui://maenifold/sequential-thinking` resource — add `[McpServerResource]` to `AppResources.cs`, MIME `text/html;profile=mcp-app`, embedded HTML template with SEP-1865 lifecycle and collapsible tree dashboard | FR-21.1, FR-21.8, FR-21.9, FR-21.10, FR-21.11 | Pending |
| T-APP-003.2 | SWE: Implement `ThoughtParser.cs` — extract thought number, branchId, branchFromThought, agentId, revision target from markdown headings; handle trunk, branches, revisions, malformed headings gracefully | NFR-21.3, NFR-21.4 | Pending |
| T-APP-003.3 | SWE: Wire SequentialThinking tool — add `[McpMeta]` annotation, change return type to include structuredContent (session metadata, thought nodes, branch metadata, conclusion), preserve CLI backward compat | FR-21.2, FR-21.3, FR-21.4, FR-21.5, FR-21.6, FR-21.7, FR-21.12 | Pending |
| T-APP-003.5 | SWE: Implement `ThinkingStatus` app-only tool (`visibility: ["app"]`) returning current thought tree | FR-21.13 | Pending |
| T-APP-003.6 | SWE: Add self-polling to SequentialThinking HTML template (poll `ThinkingStatus` via `tools/call`, 3s interval, terminal state stop; visibility-gated: IntersectionObserver + visibilityState) | FR-21.14, FR-21.15, FR-21.16, FR-21.17, FR-21.18 | Pending |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-003.4 | Blue-team: Tests — thought parser (trunk, branches, revisions, malformed headings), structuredContent shape, CLI backward compat, learn param, existing SequentialThinking tests pass, coverage thresholds held | FR-21.1-21.18 | Pending |
| T-APP-003.7 | Red-team: Adversarial review — XSS in thought content/WikiLinks, malformed headings, large sessions (50+ thoughts), CLI regression audit | Security | Pending |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-APP-003.8 | Blue-team: Verify all FR-21.x requirements met, tests pass, coverage held, 0 warnings | FR-21.1-21.18 | Pending |

## Sprint: Session Abandonment Sweep (T-CLEANUP-001.1)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-CLEANUP-001.1 | SWE: Implement DB-driven abandoned session sweep in ConceptSync pre-pass + SessionCleanup.TryMarkAbandonedFromLastIndexed | FR-14.6 | **Complete** |

## Sprint: Community Detection (T-COMMUNITY-001)

Wave 1 — Foundation (serial):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.1 | SWE: Add concept_communities table schema with FK to concepts (`GraphDatabase.cs`) | FR-13.3 | **Complete** |
| T-COMMUNITY-001.2 | SWE: Implement Louvain algorithm — in-memory, deterministic seed, configurable gamma (`CommunityDetection.cs` new) | FR-13.1, FR-13.2, FR-13.11 | **Complete** |
| T-COMMUNITY-001.3 | SWE: Add config — MAENIFOLD_LOUVAIN_GAMMA, sibling thresholds, watcher debounce (`Config.cs`) | FR-13.11, NFR-13.5.1, NFR-13.9.1 | **Complete** |
| T-COMMUNITY-001.4 | Blue-team: Unit tests for Louvain — deterministic seed, known topology, empty graph, disconnected components (11 tests) | FR-13.1, FR-13.2 | **Complete** |

Wave 2 — Integration (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.5 | SWE: Hook community detection into full Sync — run after concept extraction, persist results (`ConceptSync.cs`) | FR-13.4 | **Complete** |
| T-COMMUNITY-001.6 | SWE: DB file watcher with debounce for community recomputation — reuse FileSystemWatcher pattern, skip own writes (`IncrementalSync.CommunityWatcher.cs` new) | FR-13.5 | **Complete** |
| T-COMMUNITY-001.7 | SWE: Add community_id to RelatedConcept in BuildContext via indexed query (`GraphTools.cs`, `BuildContextResult.cs`) | FR-13.6 | **Complete** |
| T-COMMUNITY-001.8 | SWE: Add CommunitySiblings section to BuildContext — normalized weighted overlap, thresholds, cap 10 (`GraphTools.cs`, `BuildContextResult.cs`) | FR-13.7, FR-13.8, FR-13.9 | **Complete** |
| T-COMMUNITY-001.9 | SWE: Graceful degradation — omit community fields silently when no data (`GraphTools.cs`) | FR-13.10 | **Complete** |

Wave 3 — Verification:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COMMUNITY-001.10 | Blue-team: Integration tests — sync hook, BuildContext enrichment, siblings, degradation (6 tests) | FR-13.4–13.10 | **Complete** |
| T-COMMUNITY-001.11 | Red-team: Adversarial review — found FAIL-001 (non-atomic write + missing guard), remediated | Security | **Complete** |
| T-COMMUNITY-001.12 | Blue-team: NFR compliance — 12/12 PASS | NFR-13.x | **Complete** |

## Sprint: Claude Code Session Start Hook Redesign (T-HOOKS-001)

Wave 1 — Bug Fixes (parallel):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HOOKS-001.1 | SWE: Fix BSD awk dedup (line 124) + remove broken co-occurs filter (line 88) in hooks.sh | FR-16.8, FR-16.9 | **Complete** |
| T-HOOKS-001.2 | SWE: Add IsDBNull guard in SqliteExtensions.cs for community_id NULL crash | FR-16.10 | **Complete** |

Wave 2 — Implementation (after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HOOKS-001.3 | SWE: Rewrite session_start mode — RecentActivity seeds (limit=10, 3d), WikiLink validation, BuildContext community expansion with skip list, pointer array output per canonical format (bold seed labels, thread index capped at 5, action footer), fallback to SearchMemories, skip failed calls without retry | FR-16.1–16.7, FR-16.11 | **Complete** |

Wave 3 — Verification (parallel after Wave 2):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-HOOKS-001.4 | Blue-team: Verify canonical output format, token budget (150-350 with pruning), community diversity (3+ when available), BSD compat, fallback behavior, execution (<5s), thread cap (5), no includeContent, WikiLink validation | FR-16.1–16.7, FR-16.11 | **Complete** |
| T-HOOKS-001.5 | Red-team: Adversarial review (CLI injection, timeout handling, jq injection, error propagation, traceability) — REPO_NAME injection remediated | Security | **Complete** |

## Sprint: Site Rebuild (T-SITE-001)

Wave 0 — Spike (blocks Wave 2):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.0 | SWE: Validate mmdc + Next.js `output: export` pipeline — install `@mermaid-js/mermaid-cli`, render a test Mermaid diagram to inline SVG, verify it survives `next build` with static export. Report findings. | T-SITE-001.0 | Pending |

Wave 1 — Foundation (serial dependency):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.1 | SWE: Delete all decorative components, animation CSS, `@headlessui/react`, use-case pages, stale routes (`app/start/`) | T-SITE-001.1 | Pending |
| T-SITE-001.2 | SWE: Implement design system foundation (CSS vars, palette, system fonts, feTurbulence noise, Tailwind v4 @theme) | T-SITE-001.2 | Pending |
| T-SITE-001.2a | SWE: Build shared markdown rendering pipeline (`site/lib/markdown.ts`) — remark/rehype parsing, Mermaid block extraction for build-time mmdc, Shiki integration, relative link resolution. Single `renderMarkdown()` export consumed by all page tasks. | T-SITE-001.2a | Pending |
| T-SITE-001.3 | SWE: Implement layout shell (dark default, theme cascade script, skip link) | T-SITE-001.3 | Pending |

Wave 2 — Components + Pages (7 parallel tasks after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.4 | SWE: Implement Header (flat nav, horizontal scroll mobile, no hamburger) | T-SITE-001.4 | Pending |
| T-SITE-001.5 | SWE: Implement Footer (logo, dynamic version, brand statement, MIT, GitHub) | T-SITE-001.5 | Pending |
| T-SITE-001.6 | SWE: Implement Home page (product desc, install, MCP config, CLI examples, platform table, graph screenshot) | T-SITE-001.6 | Pending |
| T-SITE-001.7 | SWE: Implement `/docs` page (render docs/README.md) | T-SITE-001.7 | Pending |
| T-SITE-001.8 | SWE: Implement `/plugins` page (render both plugin READMEs with Mermaid) | T-SITE-001.8 | Pending |
| T-SITE-001.9 | SWE: Implement `/tools` page (data-driven catalog from tools/*.md) | T-SITE-001.9 | Pending |
| T-SITE-001.10 | SWE: Implement `/workflows` page (data-driven catalog from workflows/*.json) | T-SITE-001.10 | Pending |

Wave 2 — Shared infrastructure (parallel with pages):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.11 | SWE: Implement Shiki syntax highlighting with custom warm-restraint theme | T-SITE-001.11 | Pending |
| T-SITE-001.12 | SWE: Implement build-time Mermaid rendering via mmdc with design system theme | T-SITE-001.12 | Pending |
| T-SITE-001.13 | SWE: Implement CopyButton component | T-SITE-001.13 | Pending |

Wave 3 — Verification:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SITE-001.14 | Blue-team: Lighthouse audit + keyboard nav verification | T-SITE-001.14 | Pending |
| T-SITE-001.15 | Red-team: Security audit (XSS, injection, build pipeline, deps) | T-SITE-001.15 | Pending |
| T-SITE-001.16 | Blue-team: Full FR-15.x compliance verification + content accuracy | T-SITE-001.16 | Pending |

## Sprint: Sync Mtime Optimization (T-SYNC-MTIME-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SYNC-MTIME-001.1 | Blue-team: add full Sync mtime/hash guard clause tests | TC-14.1-14.4 | **Complete** |
| T-SYNC-MTIME-001.2 | SWE: implement full Sync mtime → hash → process chain (no pre-read, independent guards) | FR-14.1-14.4 | **Complete** |
| T-SYNC-MTIME-001.3 | Blue-team: add incremental Sync mtime/hash guard clause tests | TC-14.5 | **Complete** |
| T-SYNC-MTIME-001.4 | SWE: implement incremental Sync mtime → hash → process chain (store filesystem mtime in last_indexed; maintain file_md5) | FR-14.5 | **Complete** |
| T-SYNC-MTIME-001.5 | Red-team audit | Security | **Complete** |
| T-SYNC-MTIME-001.6 | Blue-team verify coverage held under attack | Security | **Complete** |

## Sprint: WikiLink Write Filter & Hub Detection (T-WLFILTER-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-WLFILTER-001.1 | Implement WikiLinkFilter.cs filter engine | FR-11.1 | **Complete** |
| T-WLFILTER-001.2 | Create memory-hub-detection.json workflow | FR-11.3 | **Complete** |
| T-WLFILTER-001.3 | Integrate WikiLinkFilter into WriteMemory | FR-11.2 | **Complete** |
| T-WLFILTER-001.4 | Integrate WikiLinkFilter into EditMemory | FR-11.2 | **Complete** |
| T-WLFILTER-001.5 | Add hub-detection as 5th specialist to memory-cycle.json | FR-11.4 | **Complete** |
| T-WLFILTER-001.6 | Write WikiLinkFilter unit tests | TC-11.1 | **Complete** |
| T-WLFILTER-001.7 | Write WriteMemory/EditMemory filter integration tests | TC-11.2-4 | **Complete** |
| T-WLFILTER-001.8 | Red-team audit | Security | **Complete** |

## Sprint: OpenCode Plugin Integration (T-OC-PLUGIN-001)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-OC-PLUGIN-001.1 | Implement FLARE session start context injection (`experimental.chat.system.transform`) | FR-12.1 | Pending |
| T-OC-PLUGIN-001.2 | Implement Task augmentation with WikiLink graph context (`tool.execute.before`) | FR-12.2 | Pending |
| T-OC-PLUGIN-001.3 | Implement WikiLink tagging guidelines injection into compaction (`experimental.session.compacting`) | FR-12.3 | Pending |
| T-OC-PLUGIN-001.4 | Implement concept/decision extraction + WriteMemory persistence during compaction | FR-12.4 | Pending |
| T-OC-PLUGIN-001.5 | Implement SequentialThinking compaction persistence (`event` on `session.compacted`) | FR-12.5 | Pending |
| T-OC-PLUGIN-001.6 | Implement ConfessionReport enforcement (`tool.execute.after` on `task`) | FR-12.6 | Pending |
| T-OC-PLUGIN-001.7 | Implement unified plugin scaffold: CLI discovery, timeouts, graceful degradation, logging | FR-12.7 | Pending |
| T-OC-PLUGIN-001.8 | Red-team audit | Security | Pending |

## Sprint: Test Coverage (T-COV-001)

Wave 0 — Infrastructure (complete):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COV-001.0a | SWE: Add coverlet.msbuild to test project, configure auto-collection (CollectCoverage, CoverletOutputFormat, CoverletOutput) | FR-17.1, NFR-17.6 | **Complete** |
| T-COV-001.0b | SWE: Add coverage summary step to CI build workflow (GitHub Actions job summary) | FR-17.2 | **Complete** |

Wave 1 — P1 Coverage (6 parallel SWE tasks):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COV-001.1 | SWE: Write integration tests for RecentActivity pipeline — RecentActivityTools query dispatch, RecentActivityReader DB queries with time filtering, RecentActivityFormatter output formatting. Use real SQLite. | FR-17.3 | **Complete** |
| T-COV-001.2 | SWE: Write tests for ToolRegistry — tool registration, lookup by name (case-insensitive), dispatch to correct handler, unknown tool error. | FR-17.4 | **Complete** |
| T-COV-001.3 | SWE: Write tests for IncrementalSyncTools — file change event processing, debounce coalescing, mtime/hash guard chain, watcher start/stop lifecycle. Use real SQLite + real temp files. | FR-17.5 | **Complete** |
| T-COV-001.4 | SWE: Write tests for WorkflowTools — workflow session creation, step advancement, status transitions (active→paused→completed), serial queuing, conclusion with WikiLinks. Use real SQLite. | FR-17.6 | **Complete** |
| T-COV-001.5 | SWE: Write tests for SessionCleanup — abandonment detection, threshold logic (age-based), DB metadata pre-pass, cleanup of abandoned sessions. Use real SQLite. | FR-17.7 | **Complete** |
| T-COV-001.6 | SWE: Write tests for AssetManager — asset discovery from source dirs, file copy, dry-run mode (no writes), source-target mapping for workflows/tools/roles/colors. Use real temp dirs. | FR-17.8 | **Complete** |

Wave 2 — P2 Coverage (4 parallel SWE tasks):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COV-001.7 | SWE: Write tests for AssumptionLedgerValidation — all validation rules (required fields, enum values, concept format), edge cases (empty strings, null values). | FR-17.9 | **Complete** |
| T-COV-001.8 | SWE: Write tests for McpResourceTools — resource URI resolution, content retrieval for valid URIs, error handling for invalid URIs. Use real SQLite. | FR-17.10 | **Complete** |
| T-COV-001.9 | SWE: Write targeted tests for utility classes — TimeZoneConverter (DST, UTC, invalid zones), CultureInvariantHelpers (numeric formatting), StringExtensions (uncovered branches), StringBuilderExtensions (append patterns). | FR-17.11 | **Complete** |
| T-COV-001.10 | SWE: Write tests for ConceptAnalyzer — graph analysis operations, concept extraction, relationship analysis. Use real SQLite. | FR-17.12 | **Complete** |

Wave 3 — Threshold Enforcement + Verification:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-COV-001.11 | SWE: Add coverlet threshold properties to Maenifold.Tests.csproj — ThresholdType=line,branch,method; Threshold values per NFR-17.1-3 (75/65/85). Verify `dotnet test` fails below threshold. | NFR-17.4 | **Complete** |
| T-COV-001.12 | Blue-team: Verify coverage targets met — run dotnet test, confirm line ≥ 75%, branch ≥ 65%, method ≥ 85%. Verify threshold enforcement rejects regressions. | NFR-17.1-3 | **Complete** |
| T-COV-001.13 | Red-team: Audit test quality — verify no fake tests (mocks/stubs), real infrastructure usage, meaningful assertions, no coverage-padding tests. | NFR-17.5 | **Complete** |

## Sprint: Goose Extension (T-GOOSE-001)

Wave 1 — Implementation (parallel — no file overlap):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-GOOSE-001.1 | SWE: Implement ServerInstructions — create `src/Utils/ServerInstructions.cs` (static class, const string, <500 words), wire into `src/Program.cs` via `McpServerOptions.ServerInstructions`. Content: CLI-first, WikiLinks, graph-first retrieval, search-before-write, memory lineage, categorized tool summary. | FR-22.1, FR-22.2, FR-22.3 |  Pending |
|  |  AC-1: `ServerInstructions.Text` is a non-null const string under 500 words |  |  |
|  |  AC-2: `Program.cs` passes `ServerInstructions.Text` to `McpServerOptions.ServerInstructions` |  |  |
|  |  AC-3: Text includes "maenifold --tool" CLI pattern |  |  |
|  |  AC-4: Text includes "[[WikiLink]]" convention |  |  |
|  |  AC-5: Text includes categorized tool summary (Memory, Graph, Reasoning, System) |  |  |
| T-GOOSE-001.2 | SWE: Write full Goose recipe — `integrations/goose/recipes/maenifold.yaml`. Version, title, description, extension config (stdio, `maenifold --mcp`, timeout 600), instructions (CLI-first, WikiLinks, graph navigation chain, scripting patterns), activities (search, build context, write memory, status). | FR-22.4 | Pending |
|  |  AC-1: YAML has required fields: version, title, description, instructions, extensions, activities |  |  |
|  |  AC-2: Extension config specifies cmd=maenifold, args=["--mcp"], type=stdio |  |  |
|  |  AC-3: Instructions include CLI-first execution examples |  |  |
|  |  AC-4: Activities include at least 3 suggested prompts |  |  |
| T-GOOSE-001.3 | SWE: Write minimal Goose recipe — `integrations/goose/recipes/maenifold-minimal.yaml`. Extension config + core conventions only. | FR-22.5 | Pending |
|  |  AC-1: YAML has required fields: version, title, description, instructions, extensions |  |  |
|  |  AC-2: Instructions under 100 words |  |  |
| T-GOOSE-001.4 | SWE: Write config snippet — `integrations/goose/config-snippet.yaml`. Copy-paste entry for `~/.config/goose/config.yaml`. | FR-22.6 | Pending |
|  |  AC-1: Valid YAML with extensions.maenifold block |  |  |
| T-GOOSE-001.5 | SWE: Write README — `integrations/goose/README.md`. Prerequisites, installation methods (config + recipe), disable built-in memory, competitive comparison table, CLI-first usage, key patterns, SCRIPTING.md link. | FR-22.7 | Pending |
|  |  AC-1: Includes competitive comparison table (Goose built-in vs MCP reference vs Maenifold) |  |  |
|  |  AC-2: Includes instructions to disable built-in memory extension |  |  |
|  |  AC-3: Includes at least 4 CLI usage examples |  |  |
| T-GOOSE-001.6 | SWE: Update SCRIPTING.md section 4 — add Goose integration pattern alongside Claude Code. | FR-22.8 | Pending |
|  |  AC-1: Section 4 mentions Goose with recipe-based distribution |  |  |
|  |  AC-2: Section 4 explains CLI-first via developer extension |  |  |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-GOOSE-001.7 | Blue-team: Build verification — `dotnet build -c Debug` 0 warnings, `dotnet test` all pass, CLI `--tool MemoryStatus` works, `--mcp` starts without error. | NFR-22.5, NFR-22.6 | Pending |
| T-GOOSE-001.8 | Red-team: Review ServerInstructions for information leakage, review recipe YAML for injection vectors, verify CLI-first emphasis consistency across all files, check for contradictions between ServerInstructions and recipe instructions. | Security | Pending |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-GOOSE-001.9 | Blue-team: All FR-22.x requirements met. Build/tests pass. 0 warnings. | FR-22.1-22.8, NFR-22.1-22.6 | Pending |

---

## Sprint: Proximity-Weighted Co-occurrence (T-PROX-001)

Wave 1 — Implementation (parallel where marked):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-001.1 | SWE: Add `ExtractWikiLinksWithSections` to MarkdownReader.cs + MarkdownIO.cs facade | FR-23.1 | **Complete** |
|  | AC-1: Multi-section doc → concepts tagged with correct H2 section indices |  |  |
|  | AC-2: Flat doc (no H2) → all concepts at sectionIndex=0 |  |  |
|  | AC-3: Preamble content → sectionIndex=0 |  |  |
|  | AC-4: Concept in multiple sections → appears with each section's index |  |  |
| T-PROX-001.2 | SWE: Update `BuildGraphEdges` + add `ComputeProximityWeight` in ConceptSync.cs. Update `ProcessFile` call site. | FR-23.1, FR-23.2, FR-23.3 | **Complete** |
|  | AC-1: Same-section pair → weight 1.0 stored in source_files dict |  |  |
|  | AC-2: Cross-section pair → weight 0.5 |  |  |
|  | AC-3: Flat doc → all pairs weight 0.5 |  |  |
|  | AC-4: Concept in preamble + named section, paired with section concept → max 1.0 wins |  |  |
|  | AC-5: source_files serialized as `{"uri": weight}` dict, not array |  |  |
| T-PROX-001.3 | SWE: Update `ComputeEdgeDecayWeight` in CommunityDetection.cs — read dict, multiply proximity × decay | FR-23.4 | **Complete** |
|  | AC-1: Effective weight = proximity × decay for each source file |  |  |
|  | AC-2: Remove silent `catch { return rawCount; }` — throw on corrupt data |  |  |
| T-PROX-001.4 | SWE: Update `BuildContext` in GraphTools.cs — read dict, apply proximity × decay per file | FR-23.4 | **Complete** |
|  | AC-1: avgDecayWeight uses `proximity × decay` per file |  |  |
|  | AC-2: Strong-proximity concept ranks higher than weak-proximity |  |  |
| T-PROX-001.5 | SWE: Update `RemoveFile` in ConceptSync.cs — deserialize dict, remove by key | FR-23.1, FR-23.5 | **Complete** |
|  | AC-1: After RemoveFile, removed doc's weight absent from edge |  |  |
|  | AC-2: Remaining docs' weights preserved |  |  |

Wave 2 — Verification (parallel after Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-001.6 | Blue-team: Tests — all AC-* criteria above | FR-23.1-23.5 | **Complete** |
| T-PROX-001.7 | Red-team: Adversarial review — M1 null guard, M2 stale weight, S4 test format. All remediated. | Security | **Complete** |

Wave 3 — Close:

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-001.8 | Blue-team: All FR-23.x met, 832/832 tests pass, coverage 78.01%/67.97%/93.4%, 0 warnings | FR-23.1-23.5 | **Complete** |

---

## Sprint: T-PROX-001 Bug Fixes (T-PROX-002)

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-002.1 | Fix: H2 heading WikiLinks silently dropped by ExtractWikiLinksWithSections | FR-23.1 | **Complete** |
|  | AC-1: WikiLink in H2 heading text extracted with correct sectionIndex |  |  |
|  | AC-2: Existing tests unaffected (no regression) |  |  |
| T-PROX-002.2 | Fix: Empty source_files → weight 0.0 (not 1.0) in BuildContext | FR-23.4 | **Complete** |
|  | AC-1: BuildContext calls ComputeEdgeDecayWeight instead of inline formula |  | Deferred → T-PROX-003 |
|  | AC-2: Empty source_files → weight 0.0 (not 1.0) |  |  |
|  | AC-3: Existing proximity tests still pass |  |  |
| T-PROX-002.3 | Blue-team: Tests for both fixes (FR23_1_AC5, FR23_4_AC3) | FR-23.1, FR-23.4 | **Complete** |
| T-PROX-002.4 | Red-team: Audit — MEDIUM: AC-1 inline formula deferred; LOW: annotation + RTM gaps | Security | **Complete** |
| T-PROX-002.5 | Verify: 834/834 tests pass, 0 warnings, coverage 78.02%/67.95%/93.4% | FR-23.1-23.5 | **Complete** |

---

## Sprint: Proximity-Weighted Louvain (T-PROX-003)

Wave 1 — Parallel (GraphTools + blue-team tests):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-003.1 | SWE: GraphTools.cs — replace inline proximity×decay formula with CommunityDetection.ComputeEdgeDecayWeight call (deferred AC-1 from T-PROX-002.2) | FR-23.4 | **Complete** |
|  | AC-1: BuildContext calls CommunityDetection.ComputeEdgeDecayWeight(files) with null guard |  |  |
|  | AC-2: fileDict still parsed once for Files property (no double JSON parse) |  |  |
|  | AC-3: All FR23 tests pass |  |  |
| T-PROX-003.4 | Blue-team: Write tests for R1b (proximity Louvain) and R2 (NMI). InsertEdge audit: 2 helpers write '{}', both test files will break. | FR-23.6, FR-23.7 | **Complete** |
|  | AC-1: RunLouvain with proximity-weighted source_files → community structure reflects weights |  |  |
|  | AC-2: RunLouvain with empty source_files → edge weight 0.0 |  |  |
|  | AC-3: NMI computation: identical clusterings → NMI = 1.0 |  |  |
|  | AC-4: NMI computation: completely different clusterings → NMI < 1.0 |  |  |
|  | AC-5: RunAndPersist logs warning when NMI below threshold |  |  |
|  | AC-6: Run `dotnet test --list-tests` to verify exact test count |  |  |
|  | AC-7: Audit all InsertEdge helpers for source_files format compatibility |  |  |

Wave 2 — RunLouvain proximity wiring (serial, depends on Wave 1):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-003.2 | SWE: CommunityDetection.cs — RunLouvain reads source_files, uses sum(proximity) as edge weight; update InsertEdge helpers in CommunityDetectionTests.cs + CommunityDetectionCoverageTests.cs | FR-23.6 | **Complete** |
|  | AC-1: SQL reads source_files column from concept_graph |  |  |
|  | AC-2: Edge weight = sum of proximity values from source_files dict (NOT co_occurrence_count) |  |  |
|  | AC-3: Empty source_files → edge weight 0.0 |  |  |
|  | AC-4: Type change: (int neighbor, int weight) → (int neighbor, double weight) in adj, LouvainIterative, ComputeModularity |  |  |
|  | AC-5: InsertEdge helpers write source_files dict with N entries of proximity 1.0 (preserving old int weight semantics) |  |  |
|  | AC-6: All existing Louvain tests pass with updated helpers |  |  |

Wave 3 — NMI community diff (serial, depends on Wave 2):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-003.3 | SWE: CommunityDetection.cs — ComputeNMI implementation + RunAndPersist drift warning. Fixed AC-4 test data (label-swap → genuinely different partition). | FR-23.7 | **Complete** |
|  | AC-1: Before writing, read old community assignments from concept_communities |  |  |
|  | AC-2: Compute NMI between old and new assignments |  |  |
|  | AC-3: NMI < threshold → warning logged |  |  |
|  | AC-4: First run (no old data) → skip comparison, no warning |  |  |

Wave 4 — Verification (parallel, after all implementation):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-PROX-003.5 | Red-team: Audit — LOW: NMI threshold hardcoded (remediated → Config env-var). INFORMATIONAL: RTM FR reference, double JSON parse, AC-1 weight ratio. | Security | **Complete** |
| T-PROX-003.6 | Blue-team: 839/839 pass, 0 warnings, coverage 78.23%/68.15%/93.41%, 5 new tests (FR23_6 × 2, FR23_7 × 3) | FR-23.6, FR-23.7 | **Complete** |
| T-PROX-003.7 | GetDecayWeightForFile: change fallback from DateTime.UtcNow to File.GetLastWriteTimeUtc when no `created` frontmatter. Affects 211/277 files. | FR-23.4, FR-23.6 | **Complete** |
|  | AC-1: Files without `created` frontmatter use filesystem last-modified time as decay baseline | | |
|  | AC-2: Files with `created` frontmatter are unaffected (existing path unchanged) | | |
|  | AC-3: Non-existent files (test URIs) still return decay 1.0 via catch block | | |

---

## T-META-001: Frontmatter Metadata Standardization

Single write path mandate: every file flushed to memory:// goes through `WriteMarkdown`. The writer manages the envelope (`created`, `modified`). Tools manage content and semantic metadata. Tools NEVER set timestamps.

Wave 1 — Single write path (serial, MarkdownWriter.cs only):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-META-001.1 | WriteMarkdown: if frontmatter has no `created`, add it (now). Always set `modified` (now). Timestamps via TimeZoneConverter.GetUtcNowIso(). Null frontmatter → create dict. | FR-24.1 | Planned |
|  | AC-1: New file → frontmatter has `created` and `modified` | | |
|  | AC-2: Rewrite with `created` present → `created` preserved, `modified` updated | | |
|  | AC-3: Rewrite without `created` → `created` added, `modified` updated | | |
|  | AC-4: Null frontmatter → dict created with `created` and `modified` | | |
| T-META-001.2 | AppendH2Section: replace raw `File.WriteAllText` with read → append to content → WriteMarkdown. AppendH2Section knows nothing about YAML — it reads what's there, appends content, writes through the single path. | FR-24.2 | Planned |
|  | AC-1: Appending a section updates `modified` (via WriteMarkdown) | | |
|  | AC-2: All existing frontmatter fields preserved | | |
|  | AC-3: Existing content preserved verbatim (no whitespace changes except appended section) | | |

Wave 2 — Delete manual timestamps from tools (parallel, each task = one file):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-META-001.3 | MemoryTools.cs: delete `["created"] = now` (line 263), `["modified"] = now` (line 264), `["modified"] =` in EditMemory (line 509) and MoveMemory (line 630). Writer handles all. | FR-24.3 | Planned |
|  | AC-1: WriteMemory produces files with `created` and `modified` (via writer, not tool) | | |
|  | AC-2: EditMemory updates `modified` (via writer, not tool) | | |
|  | AC-3: MoveMemory updates `modified` (via writer, not tool) | | |
| T-META-001.4 | AssumptionLedgerTools.cs: delete `["created"] =` (line 150) and `["updated"] =` (line 228). Writer handles timestamps. | FR-24.3 | Planned |
|  | AC-1: New assumption has `created` and `modified` (via writer) | | |
|  | AC-2: Updated assumption has `modified` updated (via writer, no `updated` key) | | |
| T-META-001.5 | ConceptRepairTool.cs: replace raw `File.WriteAllText` (line 338) with MarkdownIO read/write. | FR-24.4 | Planned |
|  | AC-1: Repaired files get `modified` updated | | |
|  | AC-2: Existing frontmatter preserved | | |
|  | AC-3: WikiLink replacements identical to current behavior | | |

Wave 3 — Verification (parallel):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-META-001.6 | Blue-team: tests for FR-24.x. Verify sequential thinking, workflow, assumptions all get timestamps for free. Test naming: FR24_{x}_AC{z}_Description | FR-24.1–4 | Planned |
| T-META-001.7 | Red-team: audit — traceability, edge cases, timestamp format, no manual timestamps left in tools, `last_accessed` boundary (DB only, ReadMemory only) | Security, FR-24.5 | Planned |

---

_See `RETROSPECTIVES.md` for completed work and historical notes._
