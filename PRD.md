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
| 2.2 | 2026-02-15 | Brett | Added FR-13.x (community detection), FR-14.x (sync mtime optimization). Reordered sections. |
| 3.0 | 2026-02-15 | PM Agent | Added FR-15.x (site rebuild) — full replacement of marketing site with technical documentation site |

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

### 3.7 Community Detection (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-13.1 | System SHALL implement Louvain modularity optimization to detect concept communities from `concept_graph` edge weights. | **P1** |
| FR-13.2 | Community detection SHALL use `co_occurrence_count` as edge weight (file breadth: number of files containing both concepts). No semantic edge augmentation. | **P1** |
| FR-13.3 | Community assignments SHALL be persisted to a `concept_communities` table with community_id, modularity score, resolution parameter, and detection timestamp. | **P1** |
| FR-13.4 | Community detection SHALL run automatically during full Sync. | **P1** |
| FR-13.5 | A database file watcher with debounce SHALL trigger community recomputation after incremental sync activity settles. | **P1** |
| FR-13.6 | `BuildContext` SHALL use community membership to scope and rank related concepts when community data is available. | **P1** |
| FR-13.7 | System SHALL gracefully degrade when community data is absent or stale — `BuildContext` falls back to current behavior. | **P1** |
| FR-13.8 | Community detection SHALL support a configurable resolution parameter (gamma) controlling community granularity. | P2 |

### 3.9 OpenCode Plugin Integration (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-12.1 | Plugin SHALL inject FLARE-pattern graph context into the system prompt at session start: search by repo name + recent activity → extract concepts → BuildContext → inject into `output.system`. | **P1** |
| FR-12.2 | Plugin SHALL augment Task tool prompts with graph context extracted from `[[WikiLinks]]` in the prompt text (Concept-as-Protocol): extract WikiLinks → normalize → BuildContext for each → append context to `output.args.prompt`. | **P1** |
| FR-12.3 | Plugin SHALL inject WikiLink tagging guidelines into the compaction prompt so the LLM produces graph-friendly summaries. | **P1** |
| FR-12.4 | Plugin SHALL extract concepts and key decisions from the conversation during compaction and persist them to maenifold via WriteMemory. | **P1** |
| FR-12.5 | Plugin SHALL persist compaction summaries to maenifold via SequentialThinking, maintaining a per-project session chain across compactions. | **P1** |
| FR-12.6 | Plugin SHALL enforce ConfessionReport compliance on subagent task completion: after the task tool returns, inspect the output for "ConfessionReport"; if missing, send a follow-up prompt to the subagent session demanding one, read the response, and append the confession to the task output visible to the parent. | **P1** |

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
| NFR-13.6.1 | `BuildContext` community scoping SHALL NOT add latency when community data is absent (check is a single table lookup). | Required |
| NFR-13.8.1 | Default resolution parameter (gamma) SHALL be 1.0 with env override (`MAENIFOLD_LOUVAIN_GAMMA`). | Required |

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

### Community Detection Design (FR-13.x)

**Why structural-only (no semantic edge augmentation):** `co_occurrence_count` is file breadth — the number of distinct files containing both concepts. This is already a strong semantic signal. 50 co-occurrences in 1 file = weight 1. 1 co-occurrence in 50 files = weight 50. Concepts that co-occur across many files are genuinely related across the knowledge base, not just mentioned together in one document.

**DB watcher pattern:** The database file watcher uses the same `FileSystemWatcher` + debounce timer infrastructure as the existing incremental sync file watcher. When the DB file settles after a burst of writes, the debounce timer fires and triggers Louvain recomputation. The watcher skips its own writes to avoid feedback loops.

**Louvain algorithm:** Two-phase iterative modularity optimization (Blondel et al. 2008). Phase 1: local node moving to maximize modularity gain. Phase 2: aggregate communities into super-nodes. Repeat until convergence. O(n log n) in practice.

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
| FR-15.10 | `/plugins` page SHALL present content from both plugin READMEs: plugin-maenifold (base MCP server, skill, installation, MCP config for Claude Code/Desktop/Codex/Continue/Cline) and plugin-product-team (multi-agent orchestration, hook system, wave-based execution). | **P1** |
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
| FR-15.31 | Site SHALL support dark mode (default) and light mode via toggle. User preference SHALL persist in localStorage. | **P0** |
| FR-15.32 | Navigation SHALL be a flat horizontal bar: logo, page links (Docs, Plugins, Tools, Workflows), dark/light toggle, GitHub link. No dropdowns. | **P0** |
| FR-15.33 | Site SHALL be fully navigable via keyboard (WCAG 2.4.1 skip link, focus indicators). | **P1** |
| FR-15.34 | Site SHALL respect `prefers-reduced-motion` (though there should be nothing to reduce). | **P2** |

### 7.6 Non-Functional Requirements — Site

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-15.1 | Site SHALL deploy as static HTML to GitHub Pages via `next build` + `next export` (output: `out/`). | Required |
| NFR-15.2 | Site SHALL use Next.js (current: 16), Tailwind CSS (current: 4), React (current: 19). No framework change. | Required |
| NFR-15.3 | Site SHALL NOT include any canvas-based animations, gradient mesh backgrounds, particle effects, or decorative motion. | Required |
| NFR-15.4 | Site SHALL NOT include `@headlessui/react` or any dropdown/modal component library. | Required |
| NFR-15.5 | Site SHALL include `shiki` for syntax-highlighted code blocks. | Required |
| NFR-15.6 | Site SHALL include a Mermaid rendering solution (client-side or build-time pre-render). | Required |
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
| All custom animation CSS in `globals.css` | ~400 lines of keyframes, dissolve effects, node pulses |

### 7.8 Content Source Mapping

| Site Page | Primary Content Source | Secondary Sources |
|-----------|----------------------|-------------------|
| `/` (Home) | `README.md` (root) | `integrations/claude-code/plugin-maenifold/README.md` (6-layer diagram) |
| `/docs` | `docs/README.md` | `docs/BOOTSTRAP.md`, `docs/context-engineering.md`, `docs/SECURITY_MODEL.md` (linked, not inlined) |
| `/plugins` | `integrations/claude-code/plugin-maenifold/README.md` | `integrations/claude-code/plugin-product-team/README.md` |
| `/tools` | `src/assets/usage/tools/*.md` | Tool registry in `src/Tools/ToolRegistry.cs` |
| `/workflows` | `src/assets/workflows/*.json` | Workflow descriptions from JSON metadata |

### 7.9 Design Notes

**Typography**: Use a system font stack for body text and a monospace font (e.g., `JetBrains Mono`, `Fira Code`, or system monospace) for code. The site should feel like well-formatted technical documentation — think Stripe's API docs or SQLite's documentation, not a startup landing page.

**Color palette**: Dark background (slate-900/950), blue accent (#3B82F6 or similar) for links and interactive elements, white/slate text. Minimal palette — 3 colors maximum.

**Layout**: Single-column, max-width 768px for prose content. Full-width for tables and diagrams. No sidebar navigation — pages are short enough to scroll.

**The graph screenshot**: `docs/branding/graph.jpeg` is a real Obsidian graph view of the maenifold knowledge graph. It should appear on the home page as a captioned figure — "A real knowledge graph built by maenifold" — not as a hero background or decorative element.

**Mermaid diagrams**: The plugin READMEs contain 8+ Mermaid diagrams showing real architecture (6-layer stack, chain pattern, HYDE pattern, sequential thinking branching, knowledge graph growth, concept-to-node mapping, context compounding, wave-based orchestration, traceability chain, hook sequence). These are the site's primary visual content. They communicate architecture better than any custom illustration could.

### 7.10 Out of Scope

- Blog or changelog page (use GitHub Releases)
- Search functionality (5 pages don't need search)
- Analytics or telemetry (Ma philosophy: no telemetry)
- Comments or community features (use GitHub Discussions)
- i18n / localization
- CMS or content management (content is markdown in the repo)
- Custom illustrations or icons beyond the existing logo SVG
