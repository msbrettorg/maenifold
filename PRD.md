# Product Requirements Document: Maenifold

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-31 | PM Agent | Net-new PRD for T-QUAL-FSC2 (FindSimilarConcepts embedding quality) |
| 1.1 | 2026-02-01 | PM Agent | Added FR-7.6 (access boosting), tiered decay, NFR-7.5.3-7.5.6 per Moltbook research |
| 1.2 | 2026-02-01 | PM Agent | Added FR-7.7 (ListMemories decay metadata) |
| 1.3 | 2026-02-01 | PM Agent | Added FR-7.8 (assumption decay by status) |
| 1.4 | 2026-02-01 | PM Agent | Refined thinking tier: sequential=7d, workflows=14d per Moltbook gap analysis |
| 1.5 | 2026-02-01 | PM Agent | **P0**: Added FR-8.x (CLI JSON output) - blocks OpenClaw integrations |
| 1.6 | 2026-02-01 | PM Agent | Added FR-9.1 (retrospectives log) |
| 1.7 | 2026-02-01 | PM Agent | Added FR-9.2 (debug-only build/test requirement) |
| 1.8 | 2026-02-02 | PM Agent | Added FR-7.9 (consolidation), NFR-7.5.5 (power-law decay option) per research validation |
| 1.9 | 2026-02-02 | PM Agent | Added FR-7.10 (multi-agent sleep cycle), FR-7.11 (tool access safety), sub-workflow architecture |
| 2.0 | 2026-02-14 | Brett | Define hierarchical state machine architecture for Workflow + submachines (SequentialThinking, AssumptionLedger) |
| 2.1 | 2026-02-15 | PM Agent | Added FR-12.x (OpenCode plugin integration) — parity with Claude Code plugin hooks |
| 3.0 | 2026-02-15 | PM Agent | Added FR-15.x (site rebuild) — full replacement of marketing site with technical documentation site |
| 3.1 | 2026-02-15 | PM Agent | Replaced placeholder design notes with finalized design system (color palette, typography, layout) derived from Norbauer analysis |
| 3.3 | 2026-02-16 | PM Agent | Added §7.1.5 Aesthetic Rationale ("Warm Restraint") — user story and design reasoning backing FR-15.x |
| 3.4 | 2026-02-16 | PM Agent | Finalized FR-15.x gap resolutions: FR-15.31 (theme cascade), FR-15.35 (mobile scroll nav), §7.9 (system monospace, feTurbulence texture), NFR-15.1 (output: export), NFR-15.6 (build-time mmdc) |
| 3.5 | 2026-02-16 | PM Agent | Added FR-14.6 (session abandonment detection via DB metadata pre-pass) — backfill for shipped CLEANUP-001.1 implementation |
| 3.6 | 2026-02-16 | PM Agent | Added FR-16.x (Claude Code session start hook redesign) — pointer array loader with community-clustered output |
| 3.7 | 2026-02-16 | PM Agent | FR-16.x panel review fixes: output format example, hub selection change, includeContent ban, thread cap, timeout behavior, WikiLink validation |
| 4.0 | 2026-02-19 | PM Agent | Added FR-18.x (MCP SDK upgrade 0.4.0-preview.3 → 0.8.0-preview.1) |
| 4.1 | 2026-02-20 | Brett | Status audit — marked completed sections, updated FR-16.x with recency boost |

---

## Completion Status

| Section | Description | Priority | Status |
|---------|-------------|----------|--------|
| FR-8.x | CLI JSON Output | P0 | **Done** |
| FR-7.4-7.6 | Embeddings Quality / Decay / Access Boosting | P1 | **Done** |
| FR-7.7 | ListMemories Decay Metadata | P2 | **Done** |
| FR-7.8 | Assumption Decay by Status | P1 | **Done** |
| FR-7.9-7.11 | Cognitive Sleep Cycle / Tool Access Safety | P2 | **Done** (serialized, not parallel — design choice) |
| FR-9.x | Product Governance | P2 | **Done** |
| FR-10.x | Hierarchical State Machines | P0 | **Done** |
| FR-11.1-11.2 | WikiLink Write Filter | P1 | **Done** |
| FR-11.3-11.4 | Hub Detection / Sleep Phases | P2 | **Done** |
| FR-12.x | OpenCode Plugin Integration | P1 | **Cancelled** (OpenCode removed) |
| FR-13.x | Community Detection | P1 | **Done** |
| FR-14.x | Sync Mtime Optimization | P1 | **Done** |
| FR-15.x | Site Rebuild | P0/P1 | **Done** |
| FR-16.x | Session Start Hook Redesign | P1 | **Done** (recency-boosted SQL, bugs fixed) |
| FR-17.x | Test Coverage | P0/P1 | **Done** (line 77.75%, branch 67.48%, method 93.34%) |
| FR-18.x | MCP SDK Upgrade | P1 | **Done** (0.8.0-preview.1) |

---

## 1. Executive Summary

**Product Area**: Maenifold graph embeddings and semantic similarity
**Objective**: Eliminate similarity plateaus caused by embedding collapse, ensuring FindSimilarConcepts returns meaningful rankings for valid inputs.

---

## 2. Problem Statement

Current `FindSimilarConcepts` results can degenerate into a similarity plateau (many unrelated concepts at 1.000 similarity) when embeddings collapse to identical vectors. This undermines semantic search quality and user trust.

---

## 3. Functional Requirements

### 3.1 CLI JSON Output (P0 - Highest Priority)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-8.1 | CLI SHALL support a `--json` flag that outputs structured JSON instead of human-readable markdown. | **P0** |
| FR-8.2 | All MCP tools SHALL return structured JSON when `--json` flag is present. | **P0** |
| FR-8.3 | JSON output SHALL include `success`, `data`, and `error` fields for consistent parsing. | **P0** |
| FR-8.4 | Error responses SHALL include structured error codes and human-readable messages. | **P0** |
| FR-8.5 | Omitting `--json` flag SHALL return current markdown format (backward compatible). | **P0** |

**Rationale**: OpenClaw integrations (SessionChannel, Memory Plugin) currently parse CLI output using fragile regex on markdown. Structured JSON enables reliable programmatic access and blocks critical integration work.

**Blocking**:
- OpenClaw SessionChannel JSON migration (eliminates regex parsing)
- OpenClaw Memory Plugin (maenifold as memory provider)

### 3.2 Embeddings Quality

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-7.4 | System SHALL discover semantically similar concepts via embeddings and return meaningful rankings for valid inputs. | P1 |
| FR-7.5 | System SHALL apply recency-based decay weighting to all search result rankings with tiered grace periods and configurable half-life. | P1 |
| FR-7.6 | System SHALL reset decay clock (update `last_accessed` timestamp) when a memory is explicitly read via ReadMemory, boosting frequently-accessed content. | P1 |
| FR-7.7 | ListMemories SHALL display decay-relevant metadata (`created`, `last_accessed`, `decay_weight`) for each file. | P2 |
| FR-7.8 | AssumptionLedger assumptions SHALL decay based on status: `validated` assumptions are exempt; `active`, `refined`, and `invalidated` assumptions decay. | P1 |
| FR-7.9 | System SHALL support periodic consolidation of high-value episodic content (thinking sessions) into durable semantic memory via the Cognitive Sleep Cycle workflow. | P2 |
| FR-7.10 | Cognitive Sleep Cycle SHALL be implemented as a multi-agent orchestration with 4 parallel specialist workflows: consolidation, decay, repair, and epistemic. | P2 |
| FR-7.11 | Sleep cycle workflows SHALL preserve decay integrity by using observation-only tools (`recent_activity`, `list_memories`, `search_memories`) except where explicit access is semantically correct. | P2 |

### 3.3 Product Governance

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-9.1 | Repository SHALL include `RETROSPECTIVES.md` to capture sprint retrospectives. | P2 |
| FR-9.2 | During active sprints, builds and tests SHALL use Debug configuration only; Release build SHALL NOT be invoked. | P2 |

### 3.4 Hierarchical State Machines (Workflow Supervisor) (Sprint)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-10.1 | Workflow sessions SHALL function as a supervisor state machine that manages the lifecycle of submachines (e.g., SequentialThinking, AssumptionLedger). | **P0** |
| FR-10.2 | Each submachine SHALL remain directly invokable as a standalone tool (without a Workflow supervisor). | **P0** |
| FR-10.3 | Workflow supervisor state SHALL be explicitly persisted in the workflow session frontmatter (e.g., `phase`, `activeSubmachineType`, `activeSubmachineSessionId`). | **P0** |
| FR-10.4 | While a workflow session is waiting on an active submachine, it SHALL NOT advance `currentStep` or `currentWorkflow` until the submachine reaches a terminal state (`completed` or `cancelled`). | **P0** |
| FR-10.5 | The workflow tool SHALL continue to support serial queuing of multiple workflows within a single workflow session. | P1 |
| FR-10.6 | Parallel workflow execution SHALL be achieved by running multiple workflow sessions concurrently (one supervisor per session). | P1 |

### 3.5 WikiLink Write Filter & Hub Detection

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-11.1 | System SHALL provide a WikiLink filter engine that reads blocked concepts from a dotfile (`.wikilink-filter`) in the memory path. | P1 |
| FR-11.2 | WriteMemory and EditMemory SHALL reject content containing filtered WikiLinks with a descriptive error listing each blocked concept and its reason. | P1 |
| FR-11.3 | System SHALL provide a hub-detection workflow that discovers high-degree hub concepts, classifies them, and cleans them from existing files via RepairConcepts. | P2 |
| FR-11.4 | Cognitive Sleep Cycle SHALL run 5 serialized phases: repair → hub-detection → consolidation → epistemic → status. Each phase produces cleaner data for the next. | P2 |

### 3.6 Sync Mtime Optimization (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-14.1 | Sync SHALL compare filesystem mtime against `last_indexed` before reading file contents. If mtime is unchanged, skip the file without reading. | **P1** |
| FR-14.2 | If mtime differs, Sync SHALL read the file and compare content hash against `file_md5`. If hash is unchanged, update `last_indexed` and skip. | **P1** |
| FR-14.3 | If hash differs, Sync SHALL process the file (extract concepts, update graph, etc.). | **P1** |
| FR-14.4 | Each exit condition SHALL be an independent guard clause — no compound conditionals combining mtime and hash checks. | **P1** |
| FR-14.5 | Incremental sync SHALL use the same mtime → hash → process guard clause chain when processing file change events. | **P1** |
| FR-14.6 | Sync SHALL detect abandoned thinking sessions (sequential and workflow) whose `last_indexed` exceeds the abandonment threshold, even when the mtime guard would skip reading the file. Candidate selection SHALL use DB metadata only; only candidates are read from disk. | **P1** |

### 3.7 Community Detection (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-13.1 | System SHALL implement Louvain modularity optimization to detect concept communities from `concept_graph` edge weights. | **P1** |
| FR-13.2 | Community detection SHALL use `co_occurrence_count` as edge weight (file breadth: number of files containing both concepts). No semantic edge augmentation. | **P1** |
| FR-13.3 | Community assignments SHALL be persisted to a `concept_communities` table with community_id, modularity score, resolution parameter, and detection timestamp. | **P1** |
| FR-13.4 | Community detection SHALL run automatically during full Sync. | **P1** |
| FR-13.5 | A database file watcher with debounce SHALL trigger community recomputation after incremental sync activity settles. | **P1** |
| FR-13.6 | `BuildContext` SHALL include `community_id` on each `RelatedConcept` in the output when community data is available. | **P1** |
| FR-13.7 | `BuildContext` SHALL return a separate `CommunitySiblings` section containing concepts in the same community as the query concept that have no direct edge to it. | **P1** |
| FR-13.8 | Community siblings SHALL be scored by normalized weighted neighborhood overlap: sum of min(query_weight, sibling_weight) for shared neighbors, divided by sibling total degree. | **P1** |
| FR-13.9 | Community siblings SHALL be filtered by minimum thresholds (shared neighbors >= 3, normalized overlap >= 0.4) and capped at 10 results. | **P1** |
| FR-13.10 | System SHALL gracefully degrade when community data is absent — `BuildContext` omits `community_id` and `CommunitySiblings`, falls back to current behavior. | **P1** |
| FR-13.11 | Community detection SHALL support a configurable resolution parameter (gamma) controlling community granularity. Default 1.0. | P2 |

### 3.9 OpenCode Plugin Integration (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-12.1 | Plugin SHALL inject FLARE-pattern graph context into the system prompt at session start: search by repo name + recent activity → extract concepts → BuildContext → inject into `output.system`. | **P1** |
| FR-12.2 | Plugin SHALL augment Task tool prompts with graph context extracted from `[[WikiLinks]]` in the prompt text (Concept-as-Protocol): extract WikiLinks → normalize → BuildContext for each → append context to `output.args.prompt`. | **P1** |
| FR-12.3 | Plugin SHALL inject WikiLink tagging guidelines into the compaction prompt so the LLM produces graph-friendly summaries. | **P1** |
| FR-12.4 | Plugin SHALL extract concepts and key decisions from the conversation during compaction and persist them to maenifold via WriteMemory. | **P1** |
| FR-12.5 | Plugin SHALL persist compaction summaries to maenifold via SequentialThinking, maintaining a per-project session chain across compactions. | **P1** |
| FR-12.6 | Plugin SHALL enforce ConfessionReport compliance on subagent task completion: after the task tool returns, inspect the output for "ConfessionReport"; if missing, send a follow-up prompt to the subagent session demanding one, read the response, and append the confession to the task output visible to the parent. | **P1** |

### 3.10 Claude Code Session Start Hook Redesign (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-16.1 | Session start hook SHALL use RecentActivity with `filter="thinking"`, `limit=10`, `timespan="3.00:00:00"`, and `includeContent=false` to discover recent sequential-thinking sessions as seed material for graph context. The First field (truncated first 500 chars) contains WikiLinks without full file reads, providing seed concepts at zero file I/O cost. | **P1** |
| FR-16.2 | Session start hook SHALL extract one seed concept per thinking session (first WikiLink from each session's first thought), deduplicate, and use these as graph traversal entry points. | **P1** |
| FR-16.3 | Session start hook SHALL call BuildContext on each seed concept (`depth=1`, `maxEntities=3`, `includeContent=false`) to retrieve community-tagged relations and community siblings. | **P1** |
| FR-16.4 | Session start hook SHALL maintain a skip list of seen concepts (seed + direct relations + same-community siblings) after each BuildContext call, skipping any seed already in the skip list to maximize community diversity across calls. | **P1** |
| FR-16.5 | Session start hook SHALL group all discovered concepts by community cluster and format output as a community-clustered pointer array: the first seed concept that discovers each community serves as the cluster label, WikiLink members listed per cluster. | **P1** |
| FR-16.6 | Session start hook SHALL include a thread index section listing recent thinking sessions with session ID, status, and thought count from RecentActivity output. | P2 |
| FR-16.7 | Session start hook SHALL include an action footer directing the LLM to SequentialThinking for thread continuation and BuildContext for concept exploration. | P2 |
| FR-16.8 | Concept deduplication in hooks.sh SHALL use BSD-compatible awk syntax (fix current silent failure on macOS BSD awk at line 124). | **P1** |
| FR-16.9 | Session start hook SHALL NOT filter BuildContext results by co-occurrence count (remove broken filter at line 88 that drops entire concept blocks when any relation has low co-occurrence). | **P1** |
| FR-16.10 | BuildContext SHALL guard against NULL values when reading community_id from SQLite results — `IsDBNull` check before `GetString` in SqliteExtensions.cs (fix crash on ~10% of concepts with no community assignment). | **P1** |
| FR-16.11 | Session start hook SHALL validate extracted WikiLinks against normalized concept format (lowercase-with-hyphens, no dots, no path separators, ≥3 characters) before passing to CLI tools. Malformed WikiLinks SHALL be skipped. | **P1** |

---

## 4. Non-Functional Requirements

### 4.1 CLI JSON Output

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-8.1.1 | JSON output SHALL be valid UTF-8 encoded JSON parseable by standard libraries. | Required |
| NFR-8.1.2 | JSON output SHALL complete within the same time bounds as markdown output (no performance regression). | Required |
| NFR-8.1.3 | JSON error responses SHALL include error code, message, and optional details object. | Required |
| NFR-8.1.4 | JSON output SHALL NOT include ANSI escape codes or terminal formatting. | Required |

### 4.2 Embeddings Quality

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-7.4.1 | Tokenization MUST be compatible with the embedding model's vocab and IDs; mismatches SHALL fail closed. | Required |
| NFR-7.4.2 | Similarity output SHALL NOT exceed 1.000. | Required |
| NFR-7.5.1 | Default decay grace period for `memory/thinking/sequential/` SHALL be 7 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL`). | Required |
| NFR-7.5.1a | Default decay grace period for `memory/thinking/workflows/` SHALL be 14 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS`). | Required |
| NFR-7.5.2 | Default decay grace period for all other memory SHALL be 28 days with env override (`MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT`). | Required |
| NFR-7.5.3 | Default decay half-life SHALL be 30 days with env override (`MAENIFOLD_DECAY_HALF_LIFE_DAYS`). | Required |
| NFR-7.5.4 | Decay SHALL affect search ranking only; decayed content SHALL remain fully retrievable via direct query (no deletion). | Required |
| NFR-7.5.5 | System SHALL support both power-law decay (`R = a × t^(-b)`) and exponential decay, configurable via `MAENIFOLD_DECAY_FUNCTION` env var (`power-law` default per research validation, `exponential` option available). | Required |
| NFR-7.6.1 | ReadMemory SHALL update the file's `last_accessed` timestamp on every read. | Required |
| NFR-7.6.2 | SearchMemories SHALL NOT update `last_accessed` for files appearing in results. | Required |
| NFR-7.6.3 | BuildContext SHALL NOT update `last_accessed` for files referenced in context. | Required |
| NFR-7.7.1 | ListMemories decay_weight SHALL be computed using the file's tier (sequential=7d, workflows=14d, other=14d grace) and current age. | Required |
| NFR-7.8.1 | `validated` assumptions SHALL NOT decay (exempt from decay weighting). | Required |
| NFR-7.8.2 | `active` and `refined` assumptions SHALL use 14-day grace period and 30-day half-life. | Required |
| NFR-7.8.3 | `invalidated` assumptions SHALL use 7-day grace period and 14-day half-life (aggressive decay). | Required |
| NFR-7.10.1 | Each sleep specialist workflow SHALL include its own memory replay phase tailored to its task. | Required |
| NFR-7.10.2 | Sleep orchestrator SHALL dispatch all 4 specialist agents in a single parallel dispatch. | Required |
| NFR-7.10.3 | Specialist workflows SHALL be self-contained and executable independently of the orchestrator. | Required |
| NFR-7.11.1 | Consolidation workflow MAY use `read_memory` (access boosting is semantically correct for content being consolidated). | Required |
| NFR-7.11.2 | Decay workflow SHALL use `list_memories` for metadata inspection, NOT `read_memory`. | Required |
| NFR-7.11.3 | Repair workflow SHALL use graph tools (`analyze_concept_corruption`, `repair_concepts`, `sync`), NOT `read_memory`. | Required |
| NFR-7.11.4 | Epistemic workflow SHALL use `assumption_ledger`, NOT `read_memory`. | Required |

### 4.3 WikiLink Write Filter

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-11.1.1 | Filter file SHALL be a dotfile (`.wikilink-filter`) invisible to graph indexing (sync watches `*.md` only). | Required |
| NFR-11.1.2 | Filter SHALL use mtime-based caching with thread-safe lock; stat call on each invocation. | Required |
| NFR-11.1.3 | Missing filter file SHALL return empty results (no error, all concepts pass). | Required |
| NFR-11.2.1 | Filtered concepts SHALL cause a hard error (block, don't mutate content). | Required |
| NFR-11.2.2 | Error message SHALL list each blocked concept with its reason from the filter file. | Required |
| NFR-11.2.3 | JSON mode SHALL return structured error with `FILTERED_WIKILINKS` error code. | Required |
| NFR-11.3.1 | Hub-detection workflow SHALL use `repair_concepts` with `dryRun=true` before applying changes. | Required |
| NFR-11.3.2 | Hub-detection workflow SHALL NOT use `read_memory` (preserve decay integrity). | Required |
| NFR-11.4.1 | Sleep orchestrator SHALL run 5 specialist phases in dependency order: repair → hub-detection → consolidation → epistemic → status. | Required |

---

## 5. Design Notes

### Permanent Knowledge (No Decay)

Content that should never decay is stored outside the `memory://` system:

| Content Type | Location | Rationale |
|--------------|----------|-----------|
| Procedural knowledge | Skills, system prompts | Injected at session start; not subject to retrieval ranking |
| Core identity / values | AGENTS.md, system prompts | Session context, not searched |
| Architecture decisions | Code, docs, AGENTS.md | Source of truth is the repo itself |
| Security lessons | AGENTS.md, code comments | Critical guidance lives with the code it protects |

This separation means `memory://` only contains knowledge that *should* decay — ephemeral session thinking and evolving notes. Permanent knowledge doesn't need a "pinned" flag because it's architecturally separate.

### Thinking Tiers (NFR-7.5.1, NFR-7.5.1a)

Thinking content is tiered by type, not treated uniformly:

### Workflow + Submachines as State Machines (FR-10.x)

Workflow, SequentialThinking, and AssumptionLedger are treated as explicit state machines with persisted state in frontmatter + append-only markdown.

- **Supervisor model**: Workflow session is the top-level supervisor state machine; it may enter a waiting phase while a submachine runs.
- **Submachine model**: SequentialThinking and AssumptionLedger remain independent tools with their own persisted session/file state, and can be run standalone.
- **Serial vs parallel**: Within a workflow session, the workflow queue is serial; parallelism comes from running multiple workflow sessions concurrently.

| Path | Grace Period | Rationale |
|------|--------------|-----------|
| `thinking/sequential/` | 7 days | Episodic reasoning tied to specific tasks; valuable while task is active |
| `thinking/workflows/` | 14 days | Structured multi-step processes; longer-lived than single reasoning sessions |

**Principle**: SequentialThinking sessions are task-bound episodic memory (what reasoning happened during a specific investigation). Workflows are closer to procedural artifacts (how to accomplish multi-step processes). Different cognitive types warrant different decay curves, per Moltbook community research.

### Access Boosting Scope (FR-7.6)

Only **ReadMemory** updates `last_accessed`. SearchMemories and BuildContext do not.

| Tool | Updates `last_accessed`? | Rationale |
|------|--------------------------|-----------|
| ReadMemory | ✅ Yes | Explicit, intentional access — user deliberately requested this content |
| SearchMemories | ❌ No | Appearing in search results ≠ being read; would boost 10 files when user reads 1 |
| BuildContext | ❌ No | Automated context enrichment runs frequently; would grant accidental immortality to hub concepts |

**Principle**: Access boosting rewards *deliberate use*, not *passive appearance*. A file should only reset its decay clock when someone actually reads it, not when it shows up in a list or gets pulled into context automatically.

### Assumption Decay by Status (FR-7.8)

Assumptions decay based on their epistemic status, creating pressure to validate or discard:

| Status | Decay Behavior | Rationale |
|--------|----------------|-----------|
| `validated` | ❌ No decay | Confirmed knowledge; treat as permanent |
| `active` | 14d grace, 30d half-life | Untested hypotheses should surface pressure to validate |
| `refined` | 14d grace, 30d half-life | Superseded by newer version; original should fade |
| `invalidated` | 7d grace, 14d half-life | Historical record only; aggressive decay to deprioritize |

**Principle**: Epistemic hygiene through decay. Active assumptions that linger without validation naturally lose retrieval priority, creating implicit pressure to either validate them (granting immortality) or let them fade (implicit invalidation).

### Decay Function Shape (NFR-7.5.5)

Research shows power-law decay (`R = a × t^(-b)`) better fits human memory than exponential decay (`R = e^(-t/τ)`):

| Function | Behavior | Research Basis |
|----------|----------|----------------|
| Exponential | Constant half-life; aggressive on old content | Simple; widely implemented |
| Power Law | Memory halves when time *quadruples*; slower long-term decay | Wixted & Ebbesen (1991); ACT-R d=0.5 |

Default changed to power-law based on research validation (Wixted & Ebbesen 1991, ACT-R d=0.5). Exponential option remains available via env var for deployments preferring simpler decay curves.

### Memory Consolidation (FR-7.9)

Richards & Frankland (2017) established that memory's goal is "to optimize decision-making, not information transmission through time." This requires active consolidation—transferring valuable episodic experience into durable semantic knowledge.

The Cognitive Sleep Cycle workflow (`/assets/workflows/memory-cycle.json`) implements consolidation through:
1. **Hippocampal Replay**: Review recent activity, score significance
2. **Slow-Wave Consolidation**: Distill high-value episodic → semantic via WikiLinks
3. **Synaptic Pruning**: Apply decay weights, flag severely decayed content

Without consolidation, agents either lose valuable experience (aggressive decay) or drown in accumulated episodes (no decay). Periodic consolidation maintains the balance.

### Multi-Agent Sleep Cycle Architecture (FR-7.10)

The Cognitive Sleep Cycle uses a hub-and-spoke orchestration pattern:

```
memory-cycle.json (Orchestrator)
├── Dispatch parallel agents ──┬── memory-consolidation.json
│                              ├── memory-decay.json
│                              ├── memory-repair.json
│                              └── memory-epistemic.json
├── Review all outputs
└── Wake Preparation
```

**Why multi-agent?** The sleep cycle phases have different dependencies:
- **Consolidation → Dream Synthesis**: Sequential (dream needs consolidated concepts)
- **Decay Processing**: Independent (just reads timestamps)
- **Concept Repair**: Independent (operates on graph structure)
- **Assumption Review**: Independent (operates on ledger)

Phases 2, 3, and 4 can run in parallel while consolidation handles the critical path.

**Self-contained workflows**: Each specialist workflow includes its own memory replay phase tailored to its task:
- Consolidation replay focuses on high-significance events
- Decay replay focuses on access patterns
- Repair replay focuses on concept usage frequency
- Epistemic replay focuses on assumptions touched

This eliminates shared dependencies and allows workflows to be run independently for testing or targeted maintenance.

### Tool Access Safety During Sleep (FR-7.11)

Sleep maintenance must not inadvertently extend content lifetime by triggering access boosting:

| Workflow | Allowed Tools | Forbidden | Rationale |
|----------|---------------|-----------|-----------|
| Consolidation | `read_memory` ✅ | — | Reading to consolidate IS intentional access |
| Decay | `list_memories`, `recent_activity` | `read_memory` | Observation only; preserve decay state |
| Repair | `analyze_concept_corruption`, `repair_concepts`, `sync` | `read_memory` | Graph operations don't need file content |
| Epistemic | `assumption_ledger`, `search_memories` | `read_memory` | Ledger is separate from memory files |

**Principle**: If you're maintaining something, don't reset its decay clock just by looking at it. Only consolidation (which explicitly promotes content to durable storage) should trigger access boosting.

### 4.4 Sync Mtime Optimization

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-14.1.1 | Mtime check SHALL use a single `stat()` call per file (no file read). | Required |
| NFR-14.2.1 | `last_indexed` update on hash-match SHALL NOT trigger full reprocessing (timestamp-only UPDATE). | Required |
| NFR-14.4.1 | Guard clause ordering SHALL follow: mtime check → hash check → process. Each is an independent `if` + early exit. | Required |

### 4.5 Community Detection

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-13.1.1 | Louvain SHALL operate in-memory on the full graph loaded from `concept_graph`. No external dependencies. | Required |
| NFR-13.1.2 | Louvain SHALL support deterministic execution via optional seed parameter (for testing). | Required |
| NFR-13.2.1 | Edge weights are file breadth (`co_occurrence_count` = number of files containing both concepts). Algorithm SHALL use these weights directly. | Required |
| NFR-13.3.1 | `concept_communities` table SHALL use `concept_name` as primary key with FK to `concepts` table. | Required |
| NFR-13.5.1 | DB file watcher debounce interval SHALL be configurable (default: 2000ms). | Required |
| NFR-13.5.2 | DB file watcher SHALL reuse the existing `FileSystemWatcher` + debounce timer pattern from incremental sync. | Required |
| NFR-13.5.3 | DB file watcher SHALL NOT trigger during its own write (avoid recomputation loop). | Required |
| NFR-13.6.1 | `community_id` lookup SHALL be a single indexed query against `concept_communities`. | Required |
| NFR-13.7.1 | Community siblings SHALL be returned as a separate section in `BuildContextResult`, not mixed into `DirectRelations` or `ExpandedRelations`. | Required |
| NFR-13.8.1 | Sibling scoring SHALL use the normalized weighted overlap formula validated against the production graph (1541 nodes, 60738 edges). | Required |
| NFR-13.9.1 | Sibling thresholds (min shared neighbors, min normalized overlap, max results) SHALL be configurable via env vars with defaults of 3, 0.4, and 10. | Required |
| NFR-13.10.1 | Absent community data SHALL NOT cause errors — `BuildContext` omits community fields silently. | Required |
| NFR-13.11.1 | Default resolution parameter (gamma) SHALL be 1.0 with env override (`MAENIFOLD_LOUVAIN_GAMMA`). | Required |

### 4.7 OpenCode Plugin

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-12.1.1 | Session start context injection SHALL complete within 10 seconds (CLI timeout per concept: 5s). | Required |
| NFR-12.1.2 | Session start SHALL use a token budget (default 4000 tokens, ~500 per concept) to cap injected context size. | Required |
| NFR-12.1.3 | Session start SHALL skip concepts with 0 relations or weak co-occurrence (1-2 files). | Required |
| NFR-12.2.1 | Task augmentation SHALL use a token budget (default 8000 tokens, ~1000 per concept). | Required |
| NFR-12.2.2 | Task augmentation SHALL normalize WikiLinks to lowercase-with-hyphens before lookup. | Required |
| NFR-12.2.3 | Task augmentation SHALL only fire for the `task` tool (not other tools). | Required |
| NFR-12.3.1 | WikiLink tagging guidelines SHALL include banned terms list, normalization rules, and anti-patterns. | Required |
| NFR-12.4.1 | Concept extraction SHALL use first H2 section (problem) + last H2 section (conclusion) to skip intermediary noise. | Required |
| NFR-12.4.2 | WriteMemory SHALL store to `sessions/compaction` folder with timestamped title. | Required |
| NFR-12.5.1 | SequentialThinking persistence SHALL cap summary text at 32K chars and total payload at 50K chars. | Required |
| NFR-12.5.2 | SequentialThinking persistence SHALL track per-project session/thought state in memory (resets on OpenCode restart). | Required |
| NFR-12.5.3 | SequentialThinking persistence SHALL sanitize text (NFKC normalize, strip control chars). | Required |
| NFR-12.5.4 | SequentialThinking persistence SHALL use `--json` flag for structured CLI output parsing. | Required |
| NFR-12.6.1 | ConfessionReport enforcement SHALL use `tool.execute.after` hook on the `task` tool (awaited, blocking). | Required |
| NFR-12.6.2 | ConfessionReport enforcement SHALL send follow-up prompts to the subagent session via `client.session.prompt()` (max 3 attempts). | Required |
| NFR-12.6.3 | ConfessionReport enforcement SHALL append the confession text to `output.output` so the parent LLM sees it. | Required |
| NFR-12.6.4 | ConfessionReport enforcement SHALL log warnings (not throw) if max attempts exceeded. | Required |
| NFR-12.6.5 | The ConfessionReport prompt text SHALL match the Claude Code version verbatim. | Required |
| NFR-12.7.1 | All CLI calls SHALL use `Bun.spawn` with configurable timeout (default 30s for persistence, 5s for context building). | Required |
| NFR-12.7.2 | Plugin SHALL degrade gracefully if `maenifold` CLI is not found (log warning, skip hook behavior). | Required |
| NFR-12.7.3 | Plugin SHALL be a single unified file (`integrations/opencode/plugins/maenifold.ts`) replacing the existing `compaction.ts` and `persistence.ts`. | Required |

### 4.8 Claude Code Session Start Hook

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-16.1.1 | Session start total execution SHALL complete within 5 seconds. | Required |
| NFR-16.1.2 | Session start output SHALL target 150-350 tokens (pointer array, not prose). When output exceeds 350 tokens, prune thread index first (reduce to 3 sessions, then omit entirely) before reducing pointer array content. | Required |
| NFR-16.1.3 | Session start SHALL make no more than 8 CLI calls total (1 RecentActivity + up to 6 BuildContext + 1 project search fallback). | Required |
| NFR-16.1.4 | Session start SHALL use pure bash + CLI tools only (no Python, no direct SQL, no external dependencies beyond maenifold CLI). | Required |
| NFR-16.1.5 | All bash constructs in hooks.sh SHALL work on macOS BSD userland (no GNU-only utilities). | Required |
| NFR-16.1.6 | When no recent thinking sessions exist, session start SHALL fall back to project concept search (SearchMemories by repo name) and still format output as a pointer array. | Required |
| NFR-16.1.7 | Session start hook SHALL NOT use `includeContent=true` for any CLI call. Measured cost: 2300 tokens per BuildContext call with content vs 145 without — 16x cost for 4 marginal concepts vs 15 from community siblings. | Required |
| NFR-16.1.8 | Thread index SHALL display at most 5 recent sessions. Sessions beyond 5 SHALL be omitted. | Required |
| NFR-16.1.9 | If a BuildContext call times out or returns an error, session start SHALL skip that seed and continue with remaining seeds. Partial output is acceptable. The hook SHALL NOT retry failed calls. | Required |
| NFR-16.1.10 | Session start output SHALL achieve ≥ 3 distinct community clusters when the graph contains ≥ 3 communities with recently-active concepts. When fewer communities are available, output all available clusters without error. | Required |

### Community Detection Design (FR-13.x)

**Why structural-only (no semantic edge augmentation):** `co_occurrence_count` is file breadth — the number of distinct files containing both concepts. This is already a strong semantic signal. 50 co-occurrences in 1 file = weight 1. 1 co-occurrence in 50 files = weight 50. Concepts that co-occur across many files are genuinely related across the knowledge base, not just mentioned together in one document.

**DB watcher pattern:** The database file watcher uses the same `FileSystemWatcher` + debounce timer infrastructure as the existing incremental sync file watcher. When the DB file settles after a burst of writes, the debounce timer fires and triggers Louvain recomputation. The watcher skips its own writes to avoid feedback loops.

**Louvain algorithm:** Two-phase iterative modularity optimization (Blondel et al. 2008). Phase 1: local node moving to maximize modularity gain. Phase 2: aggregate communities into super-nodes. Repeat until convergence. O(n log n) in practice.

**Community siblings scoring (validated on production graph):** Siblings are same-community concepts with no direct edge to the query concept. Scored by normalized weighted neighborhood overlap — for each shared neighbor, take the minimum of the query's edge weight and the sibling's edge weight to that neighbor, sum those, then divide by the sibling's total weighted degree. This penalizes hub-like nodes and rewards concepts whose connections are concentrated in the query's neighborhood. Thresholds (min 3 shared neighbors, min 0.4 normalized overlap) eliminate noise. Validated on 1541-node graph: `kql` siblings are all FinOps-relevant (`cost-optimization`, `report-architecture`, `savings-rate`); noisier for catch-all communities (maenifold internals) but thresholds contain it. Gamma=1.0 produces 10 communities with modularity 0.67.

**Community siblings are additive, not filtering:** Direct relations and expanded relations are returned as-is regardless of community membership. 42% of `build-context`'s direct neighbors are cross-community. Siblings are a separate output section — they surface concepts edge traversal misses, they don't gate what edges return.

### OpenCode Plugin Hook Mapping (FR-12.x)

The Claude Code plugin implements 4 hook behaviors via a bash script. The OpenCode plugin maps these to native TypeScript hooks:

| Claude Code Hook | Behavior | OpenCode Hook | Mechanism (confirmed from source) |
|-----------------|----------|---------------|-----------------------------------|
| `SessionStart` | FLARE: repo search + recency → BuildContext → inject ~4K tokens | `experimental.chat.system.transform` | Mutate `output.system` array. Hook is awaited via `Plugin.trigger()`. Fires before every LLM call — guard to run once per session. |
| `PreToolUse` (Task) | Concept-as-Protocol: extract `[[WikiLinks]]` → BuildContext → augment prompt | `tool.execute.before` (tool=`task`) | Mutate `output.args.prompt`. Hook is awaited. Only fires for `task` tool. |
| `PreCompact` | Extract concepts + decisions from conversation → WriteMemory | `experimental.session.compacting` | Mutate `output.context` array (tagging guidelines). Additionally read session messages via `client.session.messages()`, extract concepts, call `maenifold --tool WriteMemory` via CLI. |
| `SubagentStop` | Enforce ConfessionReport — block until subagent confesses | `tool.execute.after` (tool=`task`) | Hook is awaited via `Plugin.trigger()`. Inspect `output.output` for "ConfessionReport". If missing: get subagent session ID from `output.metadata.sessionId`, call `client.session.prompt()` to demand confession (blocks until subagent responds), read response, append to `output.output`. Parent sees complete result including confession. Max 3 attempts. |
| *(new)* Compaction persistence | Persist compaction summary to SequentialThinking chain | `event` (type=`session.compacted`) | Fire-and-forget (event hooks are NOT awaited). Extract summary from session messages, sanitize/cap, call `maenifold --tool SequentialThinking` via CLI. Track per-project session state in memory. |

**Key architectural facts** (verified from OpenCode source at `github.com/anomalyco/opencode`):

1. **`Plugin.trigger()` awaits hooks sequentially** — `await fn(input, output)` in a `for` loop. Hooks can block and mutate the shared `output` object by reference.
2. **`event` hooks are NOT awaited** — `hook["event"]?.({ event: input })` without `await`. Fire-and-forget. Cannot block.
3. **`tool.execute.after` result mutation propagates to parent** — the `result` object passed to the hook is the same reference returned to the AI SDK. Mutations to `output.output` are visible to the parent LLM.
4. **`output.metadata.sessionId`** in `tool.execute.after` for the `task` tool contains the subagent session ID (set in `task.ts`).
5. **`client.session.prompt()`** can send a follow-up message to an idle session and blocks until the LLM responds. The subagent session is idle by the time `tool.execute.after` fires.

### Session Start Hook Design (FR-16.x)

**Why "pointer array loader" not knowledge retrieval:** At session start, no user query exists — traditional RAG is inapplicable. The hook loads retrieval handles (WikiLinks) into the LLM's working memory. Each WikiLink is simultaneously vocabulary priming AND a concept-as-protocol retrieval handle. Per the symbolic communication research (`docs/research/symbolic-communication-in-ai-systems.md` §5.5), a single 14-byte WikiLink pointer resolves to unbounded information through concept-as-protocol resolution. Information comes later via on-demand BuildContext/SearchMemories calls triggered by WikiLinks in prompts. Density beats volume.

**Why community clusters:** Flat WikiLink lists provide no structural orientation. Community clusters give the LLM a topological map — it sees neighborhoods of related work, not a bag of concepts. With 8 communities in the production graph, the hook surfaces 3-5 clusters in ~200 tokens, covering the active concept landscape. Concepts within a cluster are structurally related (high modularity), so the cluster label serves as a semantic anchor.

**Why smart skip list:** Frequency-ranked seed concepts tend to cluster in the dominant community ("single-community trap"). After each BuildContext call, the skip list absorbs the seed + its direct relations + same-community siblings. The next seed that isn't in the skip list necessarily comes from a different community. This maximizes conceptual diversity per CLI call — 6 calls can cover 5+ distinct communities instead of returning redundant views of the same cluster.

**Why 150-350 tokens (not 4000):** The previous 4000-token budget was designed for prose content injection. Pointer arrays are orders of magnitude more information-dense. Panel experiments measured ~15 concepts per BuildContext call (5 direct + 10 siblings) at ~145 tokens. Community-clustered formatting compresses further — 5 clusters at ~40 tokens each = ~200 tokens covering 30+ unique concepts. Token budget breakdown: ~120-200 for pointer array, ~20-50 for thread index, ~10-30 for action footer.

**Why maxEntities=3 (temporary):** The `maxEntities=3` parameter in FR-16.3 is a temporary workaround for the NULL crash (FR-16.10). With default maxEntities=5+, ~10% of concepts crash BuildContext due to missing `IsDBNull` guard. Once FR-16.10 is fixed and verified stable, maxEntities should increase to 5 (the default) for richer context. The skip list (FR-16.4) controls token budget, not maxEntities.

**Why RecentActivity(thinking) for seeds:** Thinking sessions are the highest-signal source of active work context. Each session's first thought contains WikiLinks that anchor the session's topic. Using these as seeds ensures the pointer array reflects what the user has actually been working on, not just what exists in the graph. The `filter="thinking"` parameter restricts to sequential-thinking sessions (not memory writes or searches), and `includeContent=false` avoids token bloat.

**Canonical output format:** The session start hook output follows this structure:

```
# Active Work

**[[cost-optimization]]**
[[kql]] [[savings-rate]] [[commitment]]

**[[community-detection]]**
[[louvain-algorithm]] [[modularity]] [[concept-graph]]

**[[session-management]]**
[[sequential-thinking]] [[workflow]] [[session-cleanup]]

Threads: session-1771260150747 (active, 12t) | session-1771175726538 (completed, 8t)

Continue: sequential_thinking with session ID. Explore: build_context on any [[WikiLink]].
```

Format rules: bold seed concept as cluster label, WikiLink members on next line (space-separated), blank line between clusters, thread index as compact one-liner with `(status, Nt)` format, action footer as single sentence. Clusters ordered by recency (most recent seed first).

**Three bugs being fixed:**
1. **BSD awk (line 124):** `awk 'NF && !seen[$0]++'` uses associative array syntax that fails silently on macOS BSD awk — CONCEPTS is always empty. Fix: `awk 'NF { if ($0 in seen) next; seen[$0]=1; print }'`.
2. **Co-occurs filter (line 88):** `grep -qE "co-occurs in [1-2] files" && continue` matches if ANY relation in the BuildContext output has low co-occurrence, dropping the entire concept block even when the concept has strong relations. Fix: remove entirely — weak relations are a valid signal in a young graph.
3. **NULL crash (SqliteExtensions.cs:194):** `reader.GetString(2)` without `IsDBNull` check crashes on ~10% of concepts that have no community assignment (e.g., disconnected components or recently added concepts not yet assigned by Louvain). Fix: add `IsDBNull` guard, return null community_id for unassigned concepts.

---

## 6. Out of Scope

- Changing the embedding model itself.
- Introducing new ranking algorithms beyond cosine similarity.
- Per-file decay rate configuration (all files in a tier use the same rate).
- Explicit "pinned" or "no-decay" flags within `memory://` (use external sources instead).
- OpenCode MCP server configuration (already handled via `opencode.json`, not part of this plugin).
- Semantic edge augmentation for community detection (ArchRAG-style KNN, cosine reweighting). Structural weights are sufficient; revisit if community quality warrants it.
- Community-colored Mermaid visualization (follow-up to Visualize tool).
- Hierarchical community output (dendrogram). Single-level partition only.

---

## 7. Site Rebuild (FR-15.x)

### 7.1 Problem Statement

The current site (`site/`) is a marketing blog that fails to communicate what maenifold is. It leads with animated particle effects, gradient meshes, and aspirational taglines ("Never lose context", "break the conversation boundary") instead of product substance. It contains incorrect installation instructions (`npm install -g` instead of `brew install`), stale numbers (30 workflows vs actual 35+), and buries useful content behind decorative UI. The site's visual identity contradicts the product's Ma philosophy — productive emptiness, restraint, absence as feature.

The site must be rebuilt from scratch as a technical documentation site that matches the README's tone: direct, information-dense, developer-first.

### 7.1.5 Aesthetic Rationale: Warm Restraint

The design system has one governing idea: **warm restraint**. Everything flows from that.

**The problem it solves.** The current site looks like every AI-generated marketing page on the internet. Purple-to-cyan gradients, glassmorphism cards, particle animations, "Never lose context" taglines. It's aesthetic stock photography — visually busy, informationally empty. A developer lands on it and immediately pattern-matches to "AI slop" and leaves. The product is a CLI tool for developers. The site should feel like the product: precise, opinionated, utilitarian.

**The reference: Norbauer & Co.** [Norbauer](https://norbauer.com) makes luxury keyboard cases. Their site was studied because they solve the same problem in a different domain: how do you market a technical product to an audience that hates marketing? Three principles were extracted:

1. **Desaturation = sophistication.** Norbauer never uses saturated color. Everything is pulled back — warm grays, muted metallics, faint undertones. Saturated color reads as cheap. The site's accent (`#6AABB3`) is a desaturated teal — not cyan, not blue, not bright. It sits quietly until you need it.
2. **Warm undertones prevent clinical coldness.** Pure `#000000` and `#FFFFFF` read as clinical or AI-generated. The site's dark bg (`#141218`) has a faint purple undertone. The text (`#E8E0DB`) has a stone warmth. Light mode bg (`#F5F0EC`) is warm cream, not paper white. This is the difference between a room lit by fluorescents and a room lit by late afternoon sun.
3. **Grain texture is the single biggest differentiator.** Flat solid colors are the signature of AI-generated design. A 3-5% noise overlay (SVG `feTurbulence`) breaks the digital sterility. It's nearly invisible consciously but registers subconsciously as "this was designed by a human."

**The palette logic.** One accent color. Not two, not three. One desaturated teal (`#6AABB3`) used for every interactive element — links, focus rings, keyword syntax highlighting. This constraint forces discipline. When everything is the accent, nothing is.

**Syntax highlighting as identity.** For a CLI tool site, code blocks are the most important visual element. The custom Shiki theme uses 8 token colors, all desaturated, all sharing the warm undertone family. The code block should feel like a coherent composition, not a Christmas tree.

**The typography logic.** System fonts for body, system monospace for code. Zero font files shipped. Zero load time. Developers see the typeface they already use in their terminal. No display font. No serif. No custom sans-serif. The absence of a distinctive font IS the typographic identity — the same way the absence of decoration is the site's visual identity. Line-height 1.75 — generous, Norbauer-inspired. The extra breathing room makes dense technical content scannable instead of claustrophobic.

**The layout logic.** 72ch prose width — the typographic ideal for sustained reading. 900px code width — wider, because CLI commands shouldn't wrap. Single column, centered, no sidebar. Five pages don't need navigation chrome. 80px section gaps — generous negative space between sections. The space isn't empty; it's load-bearing. It separates concerns visually the way blank lines separate functions in code.

**What's absent (Ma philosophy).** The site's identity is defined as much by what's missing as what's present: no animations (`prefers-reduced-motion` has nothing to reduce), no gradient backgrounds, no canvas elements, no component libraries, no hamburger menus, no custom fonts, no client-side JavaScript for content, no search, no analytics, no telemetry, no newsletter, no social links. Every absence is a decision. The site is 200 lines of custom CSS max (NFR-15.7), compared to the current 435 lines of animation keyframes alone.

**The emotional target.** A developer should land on this site and feel: "These people respect my time." Not impressed, not wowed, not entertained. *Respected.* The site tells you what maenifold is, how to install it, and how to use it. Then it gets out of the way. That restraint — the confidence to leave things unsaid — is what warm restraint means.

### 7.2 Design Principles

| Principle | Rationale |
|-----------|-----------|
| **README is the gold standard** | The root README.md and docs/README.md already nail the tone. The site should read like them, not like a SaaS landing page. |
| **Ma philosophy applies to the site** | No decorative animations, no gradient meshes, no particle effects. Absence is the feature. |
| **Dark by default** | Primary audience is developers in terminals. Dark mode is the primary experience; light mode is the alternative. |
| **Monospace-forward** | CLI tool → code blocks are primary content. Typography should reflect this. |
| **Mermaid over fake graphs** | The plugin READMEs contain real architecture diagrams in Mermaid. Render those instead of decorative canvas animations. |
| **Information density over visual spectacle** | Every pixel should communicate product substance. If it doesn't explain what maenifold does, it doesn't belong. |
| **Correct over pretty** | Install commands, version numbers, workflow counts, platform support — all must match the actual product. |

### 7.3 Functional Requirements — Site Structure

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-15.1 | Site SHALL consist of exactly 5 routes: `/` (home), `/docs` (architecture + how it works), `/plugins` (Claude Code plugin setup), `/tools` (tool reference), `/workflows` (workflow catalog). | **P0** |
| FR-15.2 | Home page SHALL lead with the product description from README.md line 16: "Context engineering infrastructure for AI agents." — not a tagline or marketing copy. | **P0** |
| FR-15.3 | Home page SHALL display correct installation commands: `brew install msbrettorg/tap/maenifold` for macOS/Linux, GitHub Releases link for Windows. | **P0** |
| FR-15.4 | Home page SHALL display the MCP configuration JSON block from README.md with a copy-to-clipboard button. | **P0** |
| FR-15.5 | Home page SHALL display 3 CLI examples (WriteMemory, SearchMemories, BuildContext) from README.md with copy-to-clipboard buttons. | **P0** |
| FR-15.6 | Home page SHALL display the 6-layer cognitive stack diagram from plugin-maenifold README.md, rendered as a Mermaid diagram (not a static image). | **P1** |
| FR-15.7 | Home page SHALL display the platform support table from README.md (macOS, Linux, Windows with binary variants). | **P0** |
| FR-15.8 | Home page SHALL link to: Docs, Plugins, Tools, Workflows, GitHub repository. No other navigation. | **P0** |
| FR-15.9 | `/docs` page SHALL present content from docs/README.md: theoretical foundations, how it works (sequential thinking, workflows, hybrid search, lazy graph, memory lifecycle), the cognitive stack, cognitive assets, key capabilities, technical specifications. | **P1** |
| FR-15.10 | `/plugins` page SHALL present content from both plugin READMEs: plugin-maenifold (base MCP server, hook system, skill, installation, MCP config for Claude Code/Desktop/Codex/Continue/Cline) and plugin-product-team (multi-agent orchestration, wave-based execution). | **P1** |
| FR-15.11 | `/tools` page SHALL be a data-driven catalog generated from `src/assets/usage/tools/*.md` files, listing all tools with descriptions and usage documentation. | **P1** |
| FR-15.12 | `/workflows` page SHALL be a data-driven catalog generated from `src/assets/workflows/*.json` files, listing all workflows with descriptions, step counts, and metadata. | **P1** |
| FR-15.13 | All Mermaid diagram blocks in source content SHALL be rendered as interactive SVG diagrams on the site. | **P1** |
| FR-15.14 | The real knowledge graph screenshot (`docs/branding/graph.jpeg`) SHALL appear once on the home page, captioned, not as a decorative background. | **P2** |

### 7.4 Functional Requirements — Content Accuracy

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-15.20 | All installation commands SHALL match the current README.md exactly. No `npm install` instructions. | **P0** |
| FR-15.21 | Workflow count SHALL be dynamically derived from the workflow JSON files at build time, not hardcoded. | **P1** |
| FR-15.22 | Tool count SHALL be dynamically derived from the tool usage markdown files at build time, not hardcoded. | **P1** |
| FR-15.23 | Version number displayed in the footer SHALL be dynamically derived from the latest git tag or package.json at build time, not hardcoded. | **P1** |
| FR-15.24 | Platform support table SHALL match README.md exactly (osx-arm64, osx-x64, linux-x64, linux-arm64, win-x64). | **P0** |

### 7.5 Functional Requirements — Interaction

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-15.30 | All code blocks SHALL have a copy-to-clipboard button. | **P0** |
| FR-15.31 | Site SHALL detect system theme preference via `prefers-color-scheme`. User toggle SHALL override and persist to localStorage. When no system preference and no stored preference exist, dark mode SHALL be the fallback. No flash of unstyled content. Three-tier cascade: localStorage (explicit choice) → system preference → dark fallback. | **P0** |
| FR-15.32 | Navigation SHALL be a flat horizontal bar: logo, page links (Docs, Plugins, Tools, Workflows), dark/light toggle, GitHub link. No dropdowns. | **P0** |
| FR-15.35 | Navigation SHALL use horizontal scroll (`overflow-x: auto`) on mobile viewports. No hamburger menu, no off-canvas drawer. All navigation links SHALL remain visible without user interaction. | **P0** |
| FR-15.33 | Site SHALL be fully navigable via keyboard (WCAG 2.4.1 skip link, focus indicators). | **P1** |
| FR-15.34 | Site SHALL respect `prefers-reduced-motion` (though there should be nothing to reduce). | **P2** |

### 7.6 Non-Functional Requirements — Site

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-15.1 | Site SHALL deploy as static HTML to GitHub Pages via `next build` with `output: 'export'` in `next.config.ts` (output directory: `out/`). | Required |
| NFR-15.2 | Site SHALL use Next.js (current: 16), Tailwind CSS (current: 4), React (current: 19). No framework change. | Required |
| NFR-15.3 | Site SHALL NOT include any canvas-based animations, gradient mesh backgrounds, particle effects, or decorative motion. | Required |
| NFR-15.4 | Site SHALL NOT include `@headlessui/react` or any dropdown/modal component library. | Required |
| NFR-15.5 | Site SHALL include `shiki` for syntax-highlighted code blocks. | Required |
| NFR-15.6 | Mermaid diagrams SHALL be pre-rendered to inline SVG at build time via `@mermaid-js/mermaid-cli` (`mmdc`). No Mermaid JavaScript SHALL be shipped to the client. Mermaid theme SHALL use the design system palette (teal nodes `#6AABB3`, warm dark background `#1E1B22`, stone-white text `#E8E0DB`). | Required |
| NFR-15.7 | Total CSS for the site SHALL be under 200 lines (excluding Tailwind utilities). Custom animation CSS is prohibited. | Required |
| NFR-15.8 | Lighthouse performance score SHALL be 95+ on mobile. | Target |
| NFR-15.9 | Site SHALL load no JavaScript for users with JS disabled (static HTML with CSS-only dark mode fallback). | Target |
| NFR-15.10 | All page content SHALL be server-rendered at build time (no client-side data fetching). | Required |

### 7.7 What Gets Deleted

The following files/components from the current site SHALL be removed entirely (not refactored):

| File | Reason |
|------|--------|
| `app/components/AnimatedGraph.tsx` | 270-line decorative canvas animation |
| `app/components/AnimatedText.tsx` | Decorative text animation |
| `app/components/GlassCard.tsx` + `GlassCard.css` | Glassmorphism marketing component |
| `app/components/RippleButton.tsx` | Decorative button effect |
| `app/components/NetworkBackground.tsx` | Decorative background |
| `app/use-cases/` (all 5 files) | Marketing use-case pages — content lives in docs/BOOTSTRAP.md |
| `app/docs/philosophy/` | Philosophy lives in docs/MA_MANIFESTO.md, linked from /docs |
| `app/docs/vscode/` | Covered by /plugins page |
| `app/assets/roles/` | Roles are a tool catalog item, not a standalone page |
| `app/assets/colors-perspectives/` | Perspectives are a tool catalog item, not a standalone page |
| `app/start/` | Stale "Quick Start" page with incorrect `npm install -g` instructions — not in FR-15.1 route spec |
| All custom animation CSS in `globals.css` | ~400 lines of keyframes, dissolve effects, node pulses |

### 7.8 Content Source Mapping

| Site Page | Primary Content Source | Secondary Sources |
|-----------|----------------------|-------------------|
| `/` (Home) | `README.md` (root) | `integrations/claude-code/plugin-maenifold/README.md` (6-layer diagram) |
| `/docs` | `docs/README.md` | `docs/BOOTSTRAP.md`, `docs/context-engineering.md`, `docs/SECURITY_MODEL.md` (linked, not inlined) |
| `/plugins` | `integrations/claude-code/plugin-maenifold/README.md` | `integrations/claude-code/plugin-product-team/README.md` |
| `/tools` | `src/assets/usage/tools/*.md` | Tool registry in `src/Tools/ToolRegistry.cs` |
| `/workflows` | `src/assets/workflows/*.json` | Workflow descriptions from JSON metadata |

### 7.8.1 Shared Markdown Rendering Pipeline

All content pages (`/docs`, `/plugins`, `/tools`) consume raw markdown from source files. A single shared pipeline (`site/lib/markdown.ts`) SHALL handle: remark/rehype parsing, Mermaid block extraction (handing off to mmdc for build-time SVG rendering per NFR-15.6), Shiki code highlighting with the custom warm-restraint theme (NFR-15.5), and relative link resolution. This avoids reimplementing rendering logic per page and ensures consistent output. The pipeline is a Wave 1 dependency (T-SITE-001.2a) that unblocks all Wave 2 page tasks.

### 7.9 Design System

Full design system specification: `memory://decisions/site-rebuild/site-design-system`

Design research (Norbauer & Co. analysis): `memory://research/site-rebuild/norbauer-*`

Thinking session: `session-1771175726538-13390`

#### Color Palette

**Philosophy**: Warm restraint. One accent color. No pure white in dark mode, no pure black in light mode. Warm undertones throughout — inspired by Norbauer & Co.'s desaturated editorial palette, adapted for a dark-first developer tool.

**Dark Mode (Primary)**:

| Token | Hex | Usage |
|-------|-----|-------|
| `--bg` | `#141218` | Page background — warm near-black, faint purple undertone |
| `--bg-surface` | `#1E1B22` | Elevated surfaces: code blocks, nav, cards |
| `--bg-hover` | `#2A2630` | Hover/active states |
| `--border` | `#2A2630` | Subtle borders and dividers |
| `--text` | `#E8E0DB` | Primary text — warm stone-white |
| `--text-secondary` | `#9A938E` | Secondary text, captions, metadata |
| `--accent` | `#6AABB3` | Links, interactive elements, focus rings — desaturated warm teal |
| `--accent-hover` | `#8AC0C6` | Hover state for accent |
| `--accent-muted` | `#2A3A3D` | Very low opacity accent for subtle backgrounds |

**Light Mode (Alternative)**:

| Token | Hex | Usage |
|-------|-----|-------|
| `--bg` | `#F5F0EC` | Warm off-white |
| `--bg-surface` | `#FFFFFF` | Elevated surfaces |
| `--bg-hover` | `#EDE7E2` | Hover states |
| `--border` | `#DDD6D0` | Borders |
| `--text` | `#1E1B22` | Primary text |
| `--text-secondary` | `#7A746F` | Secondary text |
| `--accent` | `#3A8A94` | Darker teal for light backgrounds |
| `--accent-hover` | `#2A7A84` | Hover |

**Custom Syntax Highlighting Theme** (Shiki):

| Token | Hex | Usage |
|-------|-----|-------|
| `--code-bg` | `#18151C` | Code block background |
| `--code-text` | `#D4CEC8` | Default code text |
| `--code-keyword` | `#6AABB3` | Keywords — accent teal |
| `--code-string` | `#D4A76A` | Strings — warm amber |
| `--code-comment` | `#6A645F` | Comments — muted warm gray |
| `--code-property` | `#C4878A` | Properties — muted rose |
| `--code-number` | `#8AAA7A` | Numbers — soft sage |
| `--code-function` | `#B8A0C8` | Functions — muted lavender |
| `--code-punctuation` | `#7A746F` | Brackets, commas |

**Accessibility**: `--text` on `--bg` = ~13.5:1 (AAA). `--accent` on `--bg` = ~6.2:1 (AA). All combinations pass WCAG AA minimum.

#### Typography

- **Body**: System font stack (`-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif`). No custom sans-serif. Zero load time.
- **Code**: System monospace stack (`ui-monospace, 'Cascadia Code', 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace`). Zero font files shipped. Developers see their terminal's monospace font.
- **No serif. No display font. No custom fonts.**
- **Body**: 16px, weight 400, line-height 1.75 (generous, Norbauer-inspired)
- **H1**: 32px, weight 600. **H2**: 24px, weight 600. **H3**: 20px, weight 600.
- **Code**: 14px, line-height 1.6.

#### Layout

- **Prose max-width**: 72ch (~720-760px). The typographic ideal for reading.
- **Code/table max-width**: 900px. Wider to avoid wrapping CLI commands.
- **Section gap**: 80px (5rem). **Subsection gap**: 48px (3rem). **Paragraph gap**: 24px (1.5rem).
- **Single column, centered. No sidebar. No three-column.**

#### Texture

Subtle CSS noise overlay on `--bg` at ~3-5% opacity via inline SVG `feTurbulence` data URI (~200 bytes). `position: fixed`, `pointer-events: none`, composites once on paint. Zero external requests. Breaks digital sterility. Learned from Norbauer — grain texture is the single biggest differentiator from AI-generated flat design.

#### The Graph Screenshot

`docs/branding/graph.jpeg` — a real Obsidian graph view. Appears once on the home page as a captioned figure, not as a decorative background.

#### Mermaid Diagrams

The plugin READMEs contain 8+ Mermaid diagrams showing real architecture. These are the site's primary visual content. Override Mermaid theme to use our palette (teal nodes, warm dark backgrounds, stone-white text).

#### Footer

Logo (small), version (from git tag), brand statement ("Domain expertise that compounds."), MIT License link, GitHub link. No sitemap, no social links, no newsletter.

#### What We Do NOT Use

Canvas animations, gradient backgrounds, particle effects, glassmorphism, custom illustrations, serif fonts, more than one accent color, pure white (#FFFFFF) in dark mode, pure black (#000000) in light mode, component libraries.

### 7.10 Out of Scope

- Blog or changelog page (use GitHub Releases)
- Search functionality (5 pages don't need search)
- Analytics or telemetry (Ma philosophy: no telemetry)
- Comments or community features (use GitHub Discussions)
- i18n / localization
- CMS or content management (content is markdown in the repo)
- Custom illustrations or icons beyond the existing logo SVG

---

## 8. Test Coverage (FR-17.x)

### 8.1 Problem Statement

The codebase has 508 passing tests but no coverage tracking infrastructure and significant blind spots. A coverage report (2026-02-18) revealed:

- **Line coverage**: 65.1% (4,488 / 6,892)
- **Branch coverage**: 56.6% (1,522 / 2,687)
- **Method coverage**: 76.4% (376 / 492)

Nine classes have **0% coverage** — meaning no test touches them at all. These include the entire RecentActivity pipeline (3 classes), ToolRegistry (the tool dispatch table), and McpResourceTools. Two more classes sit under 25%. Coverage infrastructure (`coverlet.msbuild`) has been added but no thresholds enforce regression prevention.

### 8.2 Baseline (2026-02-18)

Source: `tests/Maenifold.Tests/coverage/html/Summary.txt`

**0% coverage (untested):**

| Class | Lines | Notes |
|-------|-------|-------|
| `RecentActivityTools` | 0% | Activity query tool — orchestrates Reader + Formatter |
| `RecentActivityFormatter` | 0% | Formats activity output for display |
| `RecentActivityReader` | 5.4% | Reads activity from DB — barely touched |
| `ConceptAnalyzer` | 0% | Graph analysis utility |
| `McpResourceTools` | 0% | MCP resource read handler |
| `ToolDescriptor` | 0% | Tool metadata model |
| `ToolRegistry` | 0% | Tool dispatch table |
| `PayloadReader` | 0% | CLI payload parsing |
| `PerformanceBenchmark` | 0% | Benchmark utility (not product code) |
| `Program` | 0% | Entry point (hard to unit test) |

**Under 60% (significant gaps):**

| Class | Coverage | Notes |
|-------|----------|-------|
| `IncrementalSyncTools` | 23.9% | Core infrastructure — file watcher + incremental processing |
| `AssetUpdateResult` | 35.7% | Asset update result model |
| `CultureInvariantHelpers` | 44.4% | Culture-safe string helpers |
| `StringBuilderExtensions` | 44.4% | StringBuilder utilities |
| `StringExtensions` | 50% | String utilities |
| `AssetManager` | 54.3% | Asset file management |
| `SessionCleanup` | 56.6% | Abandoned session detection |
| `AssumptionLedgerValidation` | 57.5% | Assumption input validation |
| `TimeZoneConverter` | 57.8% | Timezone conversion utilities |
| `WorkflowTools` | 58.9% | Workflow execution engine |

### 8.3 Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-17.1 | `dotnet test` SHALL automatically collect and display code coverage metrics (line, branch, method) in the build output via `coverlet.msbuild`. | **P0** |
| FR-17.2 | CI build workflow SHALL display a coverage summary table in the GitHub Actions job summary for every build on `dev`, `main`, and PRs. | **P0** |
| FR-17.3 | The RecentActivity pipeline (`RecentActivityTools`, `RecentActivityReader`, `RecentActivityFormatter`) SHALL have integration tests covering the primary query and formatting paths. | **P1** |
| FR-17.4 | `ToolRegistry` SHALL have tests verifying tool registration, lookup, and dispatch for at least the 5 most-used tools. | **P1** |
| FR-17.5 | `IncrementalSyncTools` SHALL have tests covering file change event processing, debounce behavior, and the mtime/hash guard clause chain. | **P1** |
| FR-17.6 | `WorkflowTools` SHALL have tests covering workflow session creation, step advancement, status transitions, and serial workflow queuing. | **P1** |
| FR-17.7 | `SessionCleanup` SHALL have tests covering abandonment detection, threshold logic, and the DB metadata pre-pass. | **P1** |
| FR-17.8 | `AssetManager` SHALL have tests covering asset discovery, copy, dry-run mode, and source-target mapping. | **P1** |
| FR-17.9 | `AssumptionLedgerValidation` SHALL have tests covering all validation rules and edge cases for assumption input. | **P2** |
| FR-17.10 | `McpResourceTools` SHALL have tests covering resource URI resolution and content retrieval. | **P2** |
| FR-17.11 | Utility classes (`TimeZoneConverter`, `CultureInvariantHelpers`, `StringExtensions`, `StringBuilderExtensions`) SHALL have targeted tests for uncovered branches. | **P2** |
| FR-17.12 | `ConceptAnalyzer` SHALL have tests covering graph analysis operations. | **P2** |

### 8.4 Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-17.1 | Line coverage SHALL reach 75% (from 65.1% baseline). | Required |
| NFR-17.2 | Branch coverage SHALL reach 65% (from 56.6% baseline). | Required |
| NFR-17.3 | Method coverage SHALL reach 85% (from 76.4% baseline). | Required |
| NFR-17.4 | Coverage thresholds SHALL be enforced in the test project via `coverlet.msbuild` — `dotnet test` SHALL fail if coverage drops below thresholds. | Required |
| NFR-17.5 | All new tests SHALL use real SQLite and real filesystem paths per testing philosophy. No mocks, no stubs. | Required |
| NFR-17.6 | Coverage reports (Cobertura XML) SHALL be generated automatically on every `dotnet test` run via csproj configuration. | Required |
| NFR-17.7 | `Program` and `PerformanceBenchmark` are excluded from coverage targets — entry point and benchmark utility respectively. | Accepted |

### 8.5 Design Notes

**Testing philosophy alignment**: All tests use real infrastructure per CLAUDE.md — real SQLite databases, real filesystem operations, real tool execution. This means coverage numbers represent genuine behavioral verification, not mock-inflated metrics. A 75% target with real tests is more valuable than 95% with mocks.

**Priority rationale**: P1 classes are either core infrastructure (IncrementalSyncTools, WorkflowTools, SessionCleanup) or tool dispatch infrastructure (ToolRegistry, RecentActivity pipeline). These have the highest blast radius if they break silently. P2 classes are utilities and secondary tools where failures are more contained.

**What we skip**: `Program` (entry point — integration tested via CLI), `PerformanceBenchmark` (diagnostic utility, not product behavior). `PayloadReader` and `ToolDescriptor` are simple data models that get exercised transitively through higher-level tests — explicit tests add noise without value.

### 8.6 Out of Scope

- Coverage targets for the Next.js site (`site/`) — static HTML generation, verified by `next build`
- Coverage gates on PR merge (CI displays coverage but does not block PRs on thresholds)
- Per-class minimum coverage requirements (aggregate targets only)
- HTML coverage report generation in CI (Cobertura XML + summary table is sufficient)

## 9. MCP SDK Upgrade (FR-18.x)

### 9.1 Problem Statement

Maenifold depends on `ModelContextProtocol` NuGet package version `0.4.0-preview.3` (released 2025-10-14). The latest version is `0.8.0-preview.1` (released 2026-02-05) — four releases behind. The upgrade spans breaking changes in protocol type mutability (v0.4.1), removed obsolete APIs (v0.5.0), and sealed protocol types (v0.8.0). While maenifold's usage patterns (attribute-driven tools, builder pattern, stdio transport) survive largely intact, staying on an outdated preview risks accumulating migration debt and missing security/correctness fixes.

Source: [NuGet Gallery — ModelContextProtocol](https://www.nuget.org/packages/ModelContextProtocol/), [GitHub Releases](https://github.com/modelcontextprotocol/csharp-sdk/releases)

### 9.2 Current State

- **Package**: `ModelContextProtocol` version `0.4.0-preview.3` in `src/Maenifold.csproj`
- **Target framework**: `net9.0` (supported by all SDK versions through 0.8.0)
- **SDK surface used**: 14 `[McpServerToolType]` classes, 26 `[McpServerTool]` methods, 1 `[McpServerResourceType]` class, 5 `[McpServerResource]` methods, 8 `McpException` throws, 1 `SendNotificationAsync` call, builder pattern (`AddMcpServer().WithStdioServerTransport().WithToolsFromAssembly().WithResourcesFromAssembly()`)
- **Files touching SDK**: 24 total (15 tool files, 4 test files, 1 utility, 1 entry point, 3 resource/notification)

Source: Codebase audit of `src/` and `tests/` directories

### 9.3 Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-18.1 | `ModelContextProtocol` package reference SHALL be updated from `0.4.0-preview.3` to `0.8.0-preview.1` in `src/Maenifold.csproj`. | **P1** |
| FR-18.2 | All 26 `[McpServerTool]` methods SHALL compile and function identically after upgrade — no tool regressions. | **P1** |
| FR-18.3 | All 5 `[McpServerResource]` methods SHALL compile and function identically after upgrade — no resource regressions. | **P1** |
| FR-18.4 | `SendNotificationAsync` in `AssetWatcherTools` SHALL continue to send `notifications/resources/list_changed` notifications after upgrade. | **P1** |
| FR-18.5 | Server builder pattern in `Program.cs` (`AddMcpServer().WithStdioServerTransport().WithToolsFromAssembly().WithResourcesFromAssembly()`) SHALL function without changes or with minimal adaptation. | **P1** |
| FR-18.6 | If `AddMcpServer()` requires `IMcpTaskStore` registration (v0.7.0 change), register `InMemoryMcpTaskStore` or equivalent default. | **P1** |
| FR-18.7 | Tool metadata annotations (`Destructive`, `Idempotent`, `ReadOnly`) SHOULD be added to applicable `[McpServerTool]` attributes where semantics are clear. | P2 |
| FR-18.8 | Source generator for XML-to-Description (v0.4.1 feature) SHOULD be evaluated — if adopted, replace manual `[Description("...")]` attributes with `///` XML doc comments on tool methods. | P2 |

### 9.4 Non-Functional Requirements

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-18.1 | All 801+ existing tests SHALL pass after upgrade with no test modifications beyond SDK API changes. | Required |
| NFR-18.2 | Coverage thresholds (line >= 75%, branch >= 65%, method >= 85%) SHALL not regress. | Required |
| NFR-18.3 | Debug build (`dotnet build -c Debug`) SHALL compile with zero warnings from SDK migration. | Required |
| NFR-18.4 | Upgrade SHALL be validated via red-team audit of any new attack surface introduced by SDK changes. | Required |

### 9.5 Breaking Changes to Address

Source: [v0.4.1-preview.1](https://github.com/modelcontextprotocol/csharp-sdk/releases/tag/v0.4.1-preview.1), [v0.5.0-preview.1](https://github.com/modelcontextprotocol/csharp-sdk/releases/tag/v0.5.0-preview.1), [v0.8.0-preview.1](https://github.com/modelcontextprotocol/csharp-sdk/releases/tag/v0.8.0-preview.1)

| Version | Change | Impact on Maenifold |
|---------|--------|---------------------|
| v0.4.1 | Protocol type properties: `init` → `set`, `required` keyword added | Low — may cause compile errors if maenifold constructs protocol types without all required properties |
| v0.4.1 | Collection types: `IReadOnlyList<T>` → `IList<T>`, `IReadOnlyDictionary` → `IDictionary` | Low — only affects code referencing old read-only interfaces on protocol types |
| v0.5.0 | Removed: `McpServerFactory`, `McpClientFactory`, `IMcpEndpoint`, `IMcpClient`, `IMcpServer` | None — maenifold does not use any of these |
| v0.5.0 | Removed: `Enumerate*Async` methods (replaced by `List*Async`) | None — client-side methods not used by maenifold |
| v0.5.0 | `CancellationToken token` → `CancellationToken cancellationToken` parameter rename | None — only affects named parameter usage |
| v0.7.0 | `IMcpTaskStore` may be required by `AddMcpServer()` | Medium — needs verification; may require registering default store |
| v0.8.0 | All protocol types in Protocol namespace sealed | None — maenifold does not subclass protocol types |

### 9.6 Migration Procedure

1. Update `src/Maenifold.csproj`: change `Version="0.4.0-preview.3"` to `Version="0.8.0-preview.1"`
2. `dotnet restore src/Maenifold.csproj`
3. `dotnet build src/Maenifold.csproj -c Debug` — fix any compile errors
4. Address `IMcpTaskStore` if `AddMcpServer()` throws at runtime
5. `dotnet test` — verify 801+ tests pass, coverage thresholds met
6. Manual smoke test: `maenifold --mcp` with a real MCP client

### 9.7 New Capabilities Unlocked (P2 — future sprints)

| Capability | SDK Version | Description |
|------------|-------------|-------------|
| Tool metadata | v0.7.0 | `Destructive`, `Idempotent`, `ReadOnly`, `OpenWorld` properties on `[McpServerTool]` |
| XML-to-Description source generator | v0.4.1 | Auto-generates `[Description]` from `///` XML doc comments |
| Task support | v0.7.0 | Long-running operations via `TaskSupport` on tools (experimental) |
| Message filters | v0.8.0 | `AddIncomingMessageFilter` / `AddOutgoingMessageFilter` for JSON-RPC interception |
| Request filters | v0.8.0 | Per-handler filters: `AddCallToolFilter`, `AddListToolsFilter`, `AddReadResourceFilter` |
| Icon support | v0.7.0 | `IconSource` property on `[McpServerTool]` and `[McpServerResource]` |
| Custom JsonSerializerOptions | v0.7.0 | Per-tool serialization configuration |
| OpenTelemetry alignment | v0.6.0 | MCP semantic conventions for observability |

### 9.8 Out of Scope

- Adopting Streamable HTTP transport (stdio is sufficient for local MCP server)
- Implementing MCP task support for long-running operations (future sprint)
- Adding message/request filters (no current use case)
- Upgrading to net10.0 target framework (net9.0 is current and supported)
- Migrating from `McpException` to a different error pattern
