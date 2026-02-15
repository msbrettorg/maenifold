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

### 3.6 OpenCode Plugin Integration (P1)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-12.1 | Plugin SHALL inject FLARE-pattern graph context into the system prompt at session start: search by repo name + recent activity → extract concepts → BuildContext → inject into `output.system`. | **P1** |
| FR-12.2 | Plugin SHALL augment Task tool prompts with graph context extracted from `[[WikiLinks]]` in the prompt text (Concept-as-Protocol): extract WikiLinks → normalize → BuildContext for each → append context to `output.args.prompt`. | **P1** |
| FR-12.3 | Plugin SHALL inject WikiLink tagging guidelines into the compaction prompt so the LLM produces graph-friendly summaries. | **P1** |
| FR-12.4 | Plugin SHALL extract concepts and key decisions from the conversation during compaction and persist them to maenifold via WriteMemory. | **P1** |
| FR-12.5 | Plugin SHALL persist compaction summaries to maenifold via SequentialThinking, maintaining a per-project session chain across compactions. | **P1** |
| FR-12.6 | Plugin SHALL enforce ConfessionReport compliance on subagent task completion: after the task tool returns, inspect the output for "ConfessionReport"; if missing, send a follow-up prompt to the subagent session demanding one, read the response, and append the confession to the task output visible to the parent. | **P1** |

**Rationale**: The Claude Code plugin (`integrations/claude-code/plugin-maenifold/`) provides 4 hook behaviors (session_start, task_augment, pre_compact, subagent_stop) plus an MCP server. The OpenCode plugin must reach behavioral parity using OpenCode's native plugin API (`@opencode-ai/plugin`). OpenCode already provides maenifold as an MCP server via `opencode.json` config, so the plugin only needs the hook behaviors.

**Reference implementation**: `integrations/claude-code/plugin-maenifold/scripts/hooks.sh`

---

## 4. Non-Functional Requirements

*(existing sections 4.1–4.3 unchanged)*

### 4.4 OpenCode Plugin

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

---

## 5. Design Notes

*(existing design notes unchanged)*

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
