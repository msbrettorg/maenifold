# TODO

## Active backlog

| ID | Description | PRD | Status |
|----|-------------|-----|--------|
| APP-001 | MCP Apps UI — Sync & MemoryStatus | FR-19.x | Pending |
| APP-002 | MCP Apps UI — Workflow/HSM Visualizer | FR-20.x | Pending |
| APP-003 | MCP Apps UI — Sequential Thinking Tree | FR-21.x | Pending |
| GOOSE-001 | Goose Extension | FR-22.x | Pending |
| MAENIFOLDPY-001 | Python port | — | Pending |
| DIST-001 | Windows MSI installer | — | Pending |

## Completed

| ID | Description | PRD | Retro |
|----|-------------|-----|-------|
| REL-001 | Release v1.0.3 | — | — |
| CLEANUP-001 | Session abandonment sweep | FR-14.6 | — |
| COMMUNITY-001 | Community detection (Louvain) | FR-13.x | Sprint 2026-02-16 |
| HOOKS-001 | Session start hook redesign | FR-16.x | Sprint 2026-02-16 |
| COV-001 | Test coverage | FR-17.x | — |
| HSM-001 | Hierarchical state machines | FR-10.x | — |
| SYNC-MTIME-001 | Sync mtime optimization | FR-14.x | — |
| WLFILTER-001 | WikiLink write filter & hub detection | FR-11.x | — |
| PROX-001/002/003 | Proximity-weighted co-occurrence + Louvain + NMI | FR-23.x | Sprint 2026-02-23/24 |
| META-001 | Frontmatter metadata standardization | FR-24.x | Sprint 2026-02-24 |
| FIX-001 | Bug fixes (test hang, YAML warning, filesystem fallback) | FR-24.6 | Sprint 2026-02-24 |
| SITE-001 | Site rebuild ("Warm Restraint") | FR-15.x | — |

## Cancelled

| ID | Description | PRD | Reason |
|----|-------------|-----|--------|
| OC-PLUGIN-001 | OpenCode Plugin Integration | FR-12.x | OpenCode removed |

---

## Sprint: MCP Apps UI — Sync & MemoryStatus (T-APP-001)

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

## Sprint: Goose Extension (T-GOOSE-001)

Wave 1 — Implementation (parallel — no file overlap):

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-GOOSE-001.1 | SWE: Implement ServerInstructions — create `src/Utils/ServerInstructions.cs` (static class, const string, <500 words), wire into `src/Program.cs` via `McpServerOptions.ServerInstructions`. Content: CLI-first, WikiLinks, graph-first retrieval, search-before-write, memory lineage, categorized tool summary. | FR-22.1, FR-22.2, FR-22.3 | Pending |
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

_Completed sprint details in `RETROSPECTIVES.md`. Task history prior to this cleanup preserved in git (`commons` branch, commit before cleanup)._
