# Requirements Traceability Matrix (RTM)

## T-CLI-JSON-001: CLI JSON Output (P0 - Highest Priority)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-CLI-JSON-001.1 | FR-8.1 | CLI SHALL support `--json` flag for structured JSON output. | src/Program.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.2 | FR-8.2 | All MCP tools SHALL return JSON when `--json` flag present. | src/Tools/*.cs, src/Utils/OutputContext.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.3 | FR-8.3, NFR-8.1.3 | JSON SHALL include `success`, `data`, `error` fields. | src/Utils/JsonToolResponse.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.4 | FR-8.4 | Error responses SHALL include error codes and messages. | src/Utils/JsonToolResponse.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.5 | FR-8.5 | Omitting `--json` SHALL return markdown (backward compat). | src/Program.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.6 | NFR-8.1.1 | JSON output SHALL be valid UTF-8 parseable JSON. | src/Utils/SafeJson.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.7 | NFR-8.1.4 | JSON output SHALL NOT include ANSI escape codes. | src/Utils/OutputContext.cs | tests/Maenifold.Tests/CliJsonSecurityTests.cs | **Complete** |
| T-CLI-JSON-001.8 | Security | PathTooLongException SHALL return structured error (SEC-EDGE-002). | src/Tools/MemoryTools.cs | tests/Maenifold.Tests/CliJsonSecurityTests.cs | **Complete** |

**Blocks**: OpenClaw SessionChannel JSON migration, OpenClaw Memory Plugin

---

## T-QUAL-FSC2: FindSimilarConcepts plateau remediation

| T-ID | PRD FR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|--------|----------------------|--------------|---------|--------|
| T-QUAL-FSC2.1 | FR-7.4 | Tokenization MUST be compatible with model vocab/IDs; mismatches fail closed. | src/Utils/VectorTools.Onnx.cs, src/Utils/VectorTools.Embeddings.cs | tests/Maenifold.Tests/TQualFsc2TokenizerDiagnosticsTests.cs | Complete |
| T-QUAL-FSC2.2 | FR-7.4 | FindSimilarConcepts MUST reject degenerate embeddings and similarity plateaus. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | Complete |
| T-QUAL-FSC2.3 | FR-7.4 | Similarity output MUST be bounded to <= 1.000. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | Complete |
| T-QUAL-FSC2.4 | FR-7.4 | Diagnostics MUST identify embedding collapse (hash duplicates / zero-distance counts). | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | Complete |

## T-GRAPH-DECAY-001: Recency decay weighting

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-001.1 | FR-7.5 | Apply recency-based decay weighting to all search result rankings. | src/Tools/MemorySearchTools.Fusion.cs, src/Tools/GraphTools.cs, src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/GraphDecayWeightingTests.cs | **Complete** |
| T-GRAPH-DECAY-001.2 | NFR-7.5.1 | `memory/thinking/sequential/` grace period: 7 days (`MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.2a | NFR-7.5.1a | `memory/thinking/workflows/` grace period: 14 days (`MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.3 | NFR-7.5.2 | All other memory grace period: 14 days (`MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.4 | NFR-7.5.3 | Default half-life 30 days (`MAENIFOLD_DECAY_HALF_LIFE_DAYS`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.5 | NFR-7.5.4 | Decay affects ranking only; content remains fully retrievable via direct query. | src/Tools/MemoryTools.cs | tests/Maenifold.Tests/GraphDecayWeightingTests.cs | **Complete** |
| T-GRAPH-DECAY-001.6 | NFR-7.5.5 | Optional power-law decay (`R = a × t^(-b)`) via `MAENIFOLD_DECAY_FUNCTION` env var. | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/DecayFunctionTests.cs | **Complete** |

## T-GRAPH-DECAY-002: Access-frequency boosting

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-002.1 | FR-7.6, NFR-7.6.1 | ReadMemory SHALL update `last_accessed` timestamp on every read. | src/Tools/MemoryTools.cs, src/Tools/GraphDatabase.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |
| T-GRAPH-DECAY-002.2 | NFR-7.6.2 | SearchMemories SHALL NOT update `last_accessed` for results. | src/Tools/MemorySearchTools.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |
| T-GRAPH-DECAY-002.3 | NFR-7.6.3 | BuildContext SHALL NOT update `last_accessed` for referenced files. | src/Tools/GraphTools.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |

## T-GRAPH-DECAY-003: ListMemories decay metadata

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-003.1 | FR-7.7 | ListMemories SHALL display `created`, `last_accessed`, `decay_weight` for each file. | src/Tools/SystemTools.cs | tests/Maenifold.Tests/ListMemoriesMetadataTests.cs | **Complete** |
| T-GRAPH-DECAY-003.2 | NFR-7.7.1 | decay_weight SHALL use file's tier (sequential=7d, workflows=14d, other=14d grace). | src/Tools/SystemTools.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ListMemoriesMetadataTests.cs | **Complete** |

## T-GRAPH-DECAY-004: Assumption decay by status

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-004.1 | FR-7.8, NFR-7.8.1 | `validated` assumptions SHALL NOT decay. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |
| T-GRAPH-DECAY-004.2 | NFR-7.8.2 | `active` and `refined` assumptions: 14d grace, 30d half-life. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |
| T-GRAPH-DECAY-004.3 | NFR-7.8.3 | `invalidated` assumptions: 7d grace, 14d half-life. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |

## T-GRAPH-DECAY-005: Memory consolidation (Cognitive Sleep Cycle)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-005.1 | FR-7.9 | System SHALL support periodic consolidation of high-value episodic content via Cognitive Sleep Cycle workflow. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |
| T-GRAPH-DECAY-005.2 | FR-7.9 | Consolidation SHALL distill episodic (thinking/) content into semantic (memory://) notes with WikiLinks. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |

## T-SLEEP-MULTI-001: Multi-agent sleep cycle architecture

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SLEEP-MULTI-001.1 | FR-7.10, NFR-7.10.2 | Sleep orchestrator SHALL run 5 specialist phases in serialized dependency order. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.2 | FR-7.10, NFR-7.10.1 | Consolidation workflow SHALL include tailored memory replay + consolidation + dream synthesis. | assets/workflows/memory-consolidation.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.3 | FR-7.10, NFR-7.10.1 | Status workflow SHALL include tailored memory replay + health reporting. | assets/workflows/memory-status.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.4 | FR-7.10, NFR-7.10.1 | Repair workflow SHALL include tailored memory replay + concept repair. | assets/workflows/memory-repair.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.5 | FR-7.10, NFR-7.10.1 | Epistemic workflow SHALL include tailored memory replay + assumption review. | assets/workflows/memory-epistemic.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.6 | NFR-7.10.3 | Each specialist workflow SHALL be executable independently of orchestrator. | assets/workflows/memory-*.json | Manual workflow execution | **Complete** |

## T-SLEEP-SAFETY-001: Tool access safety during sleep

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SLEEP-SAFETY-001.1 | FR-7.11, NFR-7.11.1 | Consolidation workflow MAY use `read_memory` (intentional access boosting). | assets/workflows/memory-consolidation.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.2 | FR-7.11, NFR-7.11.2 | Status workflow SHALL use `list_memories` NOT `read_memory`. | assets/workflows/memory-status.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.3 | FR-7.11, NFR-7.11.3 | Repair workflow SHALL use graph tools NOT `read_memory`. | assets/workflows/memory-repair.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.4 | FR-7.11, NFR-7.11.4 | Epistemic workflow SHALL use `assumption_ledger` NOT `read_memory`. | assets/workflows/memory-epistemic.json | Manual workflow execution | **Complete** |

## T-GOV-RETRO-001: Retrospectives log

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GOV-RETRO-001.1 | FR-9.1 | Repository SHALL include `RETROSPECTIVES.md`. | RETROSPECTIVES.md | N/A | **Complete** |
| T-GOV-RETRO-001.2 | FR-9.1 | RETROSPECTIVES.md SHALL contain retrospective template and sprint entries. | RETROSPECTIVES.md | N/A | **Complete** |

---

## T-DATE-001: Session path date detection fix (sprint-20260212)

### Requirements Traceability Matrix

MUST HAVE (Atomic Functionality):
| T-ID | Spec | Requirement (Atomic) | Component(s) | Test(s) | Commit | Status |
|------|------|----------------------|--------------|---------|--------|--------|
| RTM-001 | FR-1 | GetSessionPath SHALL strip session ID prefix and parse timestamp from first numeric segment after prefix. | src/Utils/MarkdownWriter.cs:52-59 | DateDetectionPathTests.cs (6 tests) | 255a0e4, c09d1df | **Complete** |
| RTM-002 | FR-1 | GetSessionPath SHALL produce date directory path matching the UTC date of the Unix timestamp (not 1970/01/01). | src/Utils/MarkdownWriter.cs:52-59 | DateDetectionPathTests.cs (6 tests) | 255a0e4, c09d1df | **Complete** |
| RTM-003 | FR-2 | IsValidSessionIdFormat SHALL validate the timestamp segment (not random suffix) is a valid long. | src/Tools/SequentialThinkingTools.cs:184-194 | DateDetectionValidationTests.cs (7 tests) | d864c2b, a5e1829 | **Complete** |
| RTM-004 | FR-3 | ExtractSessionId SHALL return session ID from last path segment, not segments[1]. | src/Tools/RecentActivity.Reader.cs:117-123 | DateDetectionValidationTests.cs (4 tests) | b64482c, a5e1829 | **Complete** |
| RTM-005 | NFR-1 | All human-readable timestamps (4 sites) SHALL include " UTC" suffix. | src/Tools/SequentialThinkingTools.cs:250, src/Tools/WorkflowOperations.Core.cs:51, src/Tools/WorkflowOperations.Management.cs:65,160 | DateDetectionTimestampTests.cs (2 tests) | d864c2b, b91008e, c312959 | **Complete** |
| RTM-006 | NFR-2 | ISO 8601 timestamps in FinalizeSession SHALL use CultureInfo.InvariantCulture. | src/Tools/SequentialThinkingTools.cs:303,323 | DateDetectionTimestampTests.cs (2 tests) | d864c2b, c312959 | **Complete** |

Note: RTM-001 and RTM-002 are atomically coupled — both implemented in commit 255a0e4 (same function, same lines).

MUST NOT HAVE:
| T-ID | Constraint |
|------|-----------|
| RTM-X01 | No backward compatibility shims for old 1970/01/01/ paths |
| RTM-X02 | No scope creep beyond the 6 changes in SPECS-sprint-20260212.md |
| RTM-X03 | No changes to SessionCleanup.cs or SystemTools.cs (adjacent scope) |
| RTM-X04 | No refactoring of unrelated code |

ESCAPE HATCHES:
- Non-atomic requirement discovered → STOP and re-decompose
- Git diff exceeds RTM scope → STOP immediately
- Existing 1970/01/01/ sessions: document migration approach but do not implement in this sprint

### Red Team Triage (Step 9 → Step 10)

| Finding | Disposition | Rationale |
|---------|-------------|-----------|
| C-001 (git): RTM-002 missing from commit 255a0e4 message | REMEDIATED | Added commit column to RTM; documented atomic coupling |
| C-002 (git/code): No test files | PLANNED | Wave 2 (Step 12) — tests are next workflow phase |
| C-003 (git): Out-of-scope commit e8d1c0e | REJECTED | Pre-existing commit (Feb 6) in branch lineage, not sprint scope |
| CRITICAL-001 (code): FormatException in GetSessionPath | REJECTED | Already handled by SessionExists catch block (MarkdownWriter.cs:64-72) |
| CRITICAL-003 (code): Path traversal via negative timestamps | REJECTED | Session IDs are system-generated (not user input); RTM-X02 scope constraint |
| HIGH-001 (code): Timestamp range validation | DEFERRED | Out of sprint scope (RTM-X02); system-generated IDs |
| HIGH-002 (code): ExtractSessionId empty array | REJECTED | False positive — String.Split('/') returns [""] not [] |
| MEDIUM-002 (code): Hardcoded format strings | DEFERRED | Out of sprint scope (RTM-X04 no refactoring) |

---

## T-WLFILTER-001: WikiLink Write Filter & Hub Detection

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-WLFILTER-001.1 | FR-11.1, NFR-11.1.1-3 | WikiLinkFilter SHALL read `.wikilink-filter` dotfile with mtime caching and thread-safe lock. | src/Utils/WikiLinkFilter.cs | tests/Maenifold.Tests/WikiLinkFilterTests.cs | **Complete** |
| T-WLFILTER-001.2 | FR-11.3, NFR-11.3.1-2 | Hub-detection workflow SHALL detect high-degree concepts and clean via RepairConcepts. | src/assets/workflows/memory-hub-detection.json | Manual workflow execution | **Complete** |
| T-WLFILTER-001.3 | FR-11.2, NFR-11.2.1-3 | WriteMemory SHALL reject content with filtered WikiLinks (hard error, not mutation). | src/Tools/MemoryTools.cs (WriteMemory) | tests/Maenifold.Tests/WikiLinkFilterIntegrationTests.cs | **Complete** |
| T-WLFILTER-001.4 | FR-11.2, NFR-11.2.1-3 | EditMemory SHALL reject content with filtered WikiLinks (same pattern as WriteMemory). | src/Tools/MemoryTools.cs (EditMemory) | tests/Maenifold.Tests/WikiLinkFilterIntegrationTests.cs | **Complete** |
| T-WLFILTER-001.5 | FR-11.4, NFR-11.4.1 | Sleep cycle SHALL run 5 serialized phases: repair → hub-detection → consolidation → epistemic → status. | src/assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |
| T-WLFILTER-001.6 | TC-11.1 | WikiLinkFilter unit tests: parse, cache, missing file, comments, case-insensitive. | tests/Maenifold.Tests/WikiLinkFilterTests.cs | 6 tests passing | **Complete** |
| T-WLFILTER-001.7 | TC-11.2-4 | Integration tests: WriteMemory/EditMemory blocked by filter, JSON mode error code. | tests/Maenifold.Tests/WikiLinkFilterIntegrationTests.cs | 5 tests passing | **Complete** |
| T-WLFILTER-001.8 | Security | Red-team audit: path traversal, TOCTOU, thread-safety, error injection, traceability. | All changed files | ConfessionReport | **Complete** |

---

## T-OC-PLUGIN-001: OpenCode Plugin Integration (sprint-20260215)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-OC-PLUGIN-001.1 | FR-12.1, NFR-12.1.1-3 | Plugin SHALL inject FLARE-pattern graph context into system prompt at session start via `experimental.chat.system.transform`. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.2 | FR-12.2, NFR-12.2.1-3 | Plugin SHALL augment Task tool prompts with graph context from `[[WikiLinks]]` via `tool.execute.before`. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.3 | FR-12.3, NFR-12.3.1 | Plugin SHALL inject WikiLink tagging guidelines into compaction prompt via `experimental.session.compacting`. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.4 | FR-12.4, NFR-12.4.1-2 | Plugin SHALL extract concepts/decisions from conversation during compaction and persist via WriteMemory CLI call. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.5 | FR-12.5, NFR-12.5.1-4 | Plugin SHALL persist compaction summaries to SequentialThinking via CLI, maintaining per-project session chain. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.6 | FR-12.6, NFR-12.6.1-5 | Plugin SHALL enforce ConfessionReport on task completion via `tool.execute.after`: inspect output, send follow-up prompt to subagent if missing, append confession to parent-visible output. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.7 | NFR-12.7.1-3 | Plugin SHALL be a single unified file with graceful degradation, configurable timeouts, and CLI discovery. | integrations/opencode/plugins/maenifold.ts | Red-team + blue-team review | Pending |
| T-OC-PLUGIN-001.8 | Security | Red-team audit: input validation, CLI injection, timeout handling, error propagation, traceability. | integrations/opencode/plugins/maenifold.ts | ConfessionReport | Pending |


---

## T-SYNC-MTIME-001: Sync mtime optimization (sprint-20260215)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SYNC-MTIME-001.1 | FR-14.1, NFR-14.1.1 | Sync SHALL compare filesystem mtime against `last_indexed` before reading file contents; if unchanged, skip without reading. | src/Tools/ConceptSync.cs | tests/Maenifold.Tests/SyncMtimeOptimizationTests.cs | **Complete** |
| T-SYNC-MTIME-001.2 | FR-14.2, NFR-14.2.1 | If mtime differs, Sync SHALL read file and compare content hash against `file_md5`; if hash unchanged, update `last_indexed` only and skip processing. | src/Tools/ConceptSync.cs | tests/Maenifold.Tests/SyncMtimeOptimizationTests.cs | **Complete** |
| T-SYNC-MTIME-001.3 | FR-14.3 | If hash differs, Sync SHALL process the file (extract concepts, update graph, etc.). | src/Tools/ConceptSync.cs | tests/Maenifold.Tests/SyncMtimeOptimizationTests.cs | **Complete** |
| T-SYNC-MTIME-001.4 | FR-14.4, NFR-14.4.1 | Guard clause ordering SHALL be: mtime check → hash check → process; each is an independent `if` + early exit (no compound mtime/hash conditionals). | src/Tools/ConceptSync.cs | tests/Maenifold.Tests/SyncMtimeOptimizationTests.cs | **Complete** |
| T-SYNC-MTIME-001.5 | FR-14.5 | Incremental sync SHALL use the same mtime → hash → process guard clause chain for file change events. | src/Tools/IncrementalSyncTools.Processing.cs, src/Tools/IncrementalSync.Database.cs | tests/Maenifold.Tests/IncrementalSyncMtimeOptimizationTests.cs | **Complete** |
| T-SYNC-MTIME-001.6 | Security | Red-team audit: verify no silent data loss, no watcher loops, and no bypass of required processing when content changes. | All changed files | ConfessionReport (session-1771260150747-29876) | **Complete** |

---

## T-CLEANUP-001.1: Session Abandonment Sweep (sprint-20260216)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-CLEANUP-001.1 | FR-14.6 | Sync SHALL detect abandoned thinking sessions via DB metadata pre-pass, even when the mtime guard would skip reading the file. | src/Tools/ConceptSync.cs, src/Tools/SessionCleanup.cs | tests/Maenifold.Tests/SessionAbandonmentSweepTests.cs | **Complete** |

---

## T-SITE-001: Site Rebuild (sprint-20260216)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SITE-001.1 | NFR-15.3, NFR-15.4, NFR-15.7 | Delete all decorative components (AnimatedGraph, AnimatedText, GlassCard, RippleButton, NetworkBackground), animation CSS, `@headlessui/react`, use-case pages, and stale route pages. | site/app/components/*, site/app/globals.css, site/app/use-cases/*, site/package.json | Build succeeds with zero deleted imports | Pending |
| T-SITE-001.2 | §7.9 (palette, typography, texture) | Implement design system foundation: CSS custom properties (dark/light palette), system font stacks, feTurbulence noise overlay, Tailwind v4 `@theme` config. | site/app/globals.css, site/tailwind.config.ts | Visual inspection; CSS < 200 lines (NFR-15.7) | Pending |
| T-SITE-001.3 | FR-15.31, FR-15.33 | Implement layout shell: `layout.tsx` with `class="dark"` default, inline `<head>` theme script (localStorage → prefers-color-scheme → dark fallback), skip link (WCAG 2.4.1). | site/app/layout.tsx | No FOUC on first load; theme persists across navigation | Pending |
| T-SITE-001.4 | FR-15.32, FR-15.35 | Implement Header: flat horizontal nav (logo, Docs, Plugins, Tools, Workflows, toggle, GitHub), horizontal scroll on mobile, no dropdowns, no hamburger. | site/app/components/Header.tsx | All links visible on 375px viewport without interaction | Pending |
| T-SITE-001.5 | §7.9, FR-15.23 | Implement Footer: logo (small), version from git tag (FR-15.23), "Domain expertise that compounds.", MIT link, GitHub link. | site/app/components/Footer.tsx | Version renders dynamically | Pending |
| T-SITE-001.6 | FR-15.2-15.8, FR-15.14, FR-15.20, FR-15.24, FR-15.30 | Implement Home page: product description, install commands, MCP config JSON, 3 CLI examples, 6-layer Mermaid diagram, platform table, graph screenshot, copy-to-clipboard on all code blocks. | site/app/page.tsx | All content matches README.md; copy buttons functional | Pending |
| T-SITE-001.7 | FR-15.9 | Implement `/docs` page: render docs/README.md content (theoretical foundations, how it works, cognitive stack, capabilities, tech specs). | site/app/docs/page.tsx | Content renders from source markdown | Pending |
| T-SITE-001.8 | FR-15.10 | Implement `/plugins` page: render plugin-maenifold README + plugin-product-team README content (installation, MCP config, hook system, multi-agent orchestration). | site/app/plugins/page.tsx | Content renders from source markdown; Mermaid diagrams render as SVG | Pending |
| T-SITE-001.9 | FR-15.11, FR-15.21, FR-15.22 | Implement `/tools` page: data-driven catalog from `src/assets/usage/tools/*.md`, dynamic tool count at build time. | site/app/tools/page.tsx | Tool count matches filesystem; all tools listed | Pending |
| T-SITE-001.10 | FR-15.12, FR-15.21 | Implement `/workflows` page: data-driven catalog from `src/assets/workflows/*.json`, dynamic workflow count at build time. | site/app/workflows/page.tsx | Workflow count matches filesystem; all workflows listed | Pending |
| T-SITE-001.11 | NFR-15.5 | Implement Shiki syntax highlighting with custom warm-restraint theme (8 token colors from §7.9). | site/lib/shiki.ts, site/app/components/CodeBlock.tsx | Code blocks render with correct token colors | Pending |
| T-SITE-001.12 | NFR-15.6, FR-15.13 | Implement build-time Mermaid rendering via `@mermaid-js/mermaid-cli` with design system theme. | site/lib/mermaid.ts, site/next.config.ts | Mermaid blocks render as inline SVG with palette colors; zero client JS | Pending |
| T-SITE-001.13 | FR-15.30 | Implement CopyButton component for all code blocks. | site/app/components/CopyButton.tsx | Click copies content to clipboard; visual feedback | Pending |
| T-SITE-001.14 | NFR-15.8, FR-15.33 | Lighthouse audit + keyboard navigation verification (skip link, focus indicators, tab order). | All pages | Lighthouse 95+ mobile; all interactive elements reachable via keyboard | Pending |
| T-SITE-001.15 | Security | Red-team audit: XSS in markdown rendering, content injection, build pipeline integrity, dependency audit. | All site files | ConfessionReport | Pending |
| T-SITE-001.16 | Coverage | Blue-team: verify all FR-15.x requirements met, content accuracy against README, design system compliance. | All site files | ConfessionReport | Pending |
