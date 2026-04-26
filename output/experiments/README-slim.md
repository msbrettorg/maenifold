# maenifold

**Domain expertise that compounds. Open. Local. Yours.**

**Persistent hierarchical state machines executing across ephemeral agents on a shared cognitive manifold, where each HSM transition produces graph artifacts that reshape the manifold for all future transitions by all threads — human and agent alike.**

Context engineering infrastructure for AI agents. Point it at any domain's literature, and it builds specialized experts that live on your machine, work offline, and get smarter with every use.

---

## Product Overview

maenifold is a local-first product composed of a shared markdown memory corpus, a graph-and-vector index, a long-running local daemon, an MCP tool surface for agents, and a packaged Electron desktop console for operators. The same knowledge base, workflows, and local configuration power the CLI, desktop app, and MCP-integrated clients.

### Runtime Surfaces

| Surface | Entry point | What it is for |
|---------|-------------|----------------|
| **Desktop app** | Packaged Electron app or `npm run dev:electron` | Human operator console for agents (with inline MCP Apps), pipelines, gate tasks, knowledge exploration, workflow authoring, and local settings. |
| **MCP server** | `maenifold --mcp` | Native tool surface for Codex, Claude Code, Continue, Cline, and other MCP-capable clients. |
| **HTTP daemon** | `maenifold --http` | Long-running local control plane used by the desktop app and any HTTP-based integrations. |
| **CLI** | `maenifold --tool ... --payload ...` | One-shot automation, scripting, debugging, and reproducible direct tool invocation. |
| **Cognitive assets** | `assets/` plus memory-root assets | Reusable workflows, roles, perspectives, colors, and prompt scaffolds that shape reasoning. |

### Persistent State

| Artifact | Backing store | Notes |
|----------|---------------|-------|
| **Markdown memory corpus** | `memory/` | Source of truth for notes, decisions, research, and thinking traces. |
| **Graph and vector cache** | `memory.db` | SQLite cache for embeddings, graph structure, and search acceleration. Regenerable from memory files. |
| **Workflow and role library** | `assets/` | JSON and markdown assets for orchestration, roles, perspectives, and packaged guidance. |
| **Desktop preferences** | Local application storage | Device-local UI preferences such as text size, telemetry choice, and last working directory. |
| **Daemon-backed configuration** | Local config files plus daemon API | Agent definitions, MCP server definitions, runtime tuning, and embedding model state. |

### Operator Workflows

| Workflow | Primary surface | Outcome |
|----------|-----------------|---------|
| **Start or resume an agent session** | Home, Agents | Create ACP-backed conversations, attach files, inspect tool calls, and continue prior work. |
| **Design and run a pipeline** | Pipelines | Compose prompt, workflow, tool, and sequential-thinking nodes into a DAG and execute it. |
| **Track delivery work** | Gate | Browse task files, filter by process or state, and inspect stage history and progress. |
| **Explore notes and concept structure** | Knowledge | Search the memory corpus, read notes, follow WikiLinks, and inspect concept communities. |
| **Author workflow assets** | Workflows | Create, import, edit, validate, and save reusable workflow definitions. |
| **Configure the local runtime** | Settings | Manage agents, MCP servers, runtime tuning, embedding models, and desktop-local preferences. |
| **Proxy external MCP servers** | MCP Tool Gateway | Expose stdio MCP servers through maenifold's Streamable HTTP endpoint with lifecycle management and crash recovery. |

---


## Theoretical Foundations

### Philosophical Foundations

| Concept | Origin | Application |
|---------|--------|-------------|
| **[Ma (間)](MA_MANIFESTO.md)** | Japanese aesthetics | The space between things as the thing itself ([what we don't do](WHAT_WE_DONT_DO.md)) |

### Research Foundations

| Concept | Origin | Application |
|---------|--------|-------------|
| **[Context Engineering](research/context-engineering.md)** | Anthropic (2025) | Attention budget management: just-in-time retrieval, compaction, decay, structured notes, sub-agents |
| **[ACT-R](research/decay-in-ai-memory-systems.md)** | Anderson (CMU); Wixted & Ebbesen (1991) | Power-law inspired decay ([exponential approximation](research/decay-in-ai-memory-systems.md#54-the-act-r-connection)) |
| **[New Theory of Disuse](research/decay-in-ai-memory-systems.md#23-the-spacing-effect-and-retrieval-strengthening)** | Bjork & Bjork | Storage vs retrieval strength |
| **[Two-Stage Memory](research/decay-in-ai-memory-systems.md#32-sleep-and-memory-consolidation)** | Cognitive neuroscience | Episodic → semantic consolidation |
| **[Linguistic Relativity](https://lera.ucsd.edu/papers/linguistic-relativity.pdf)** | Weak form (Boroditsky, 2003) | Perspectives change the linguistic frame of LLM reasoning |
| **[ConfessionReport](research/confession-reports.md)** | OpenAI (Barak et al., 2025) | Inference-time honesty enforcement via hooks + adversarial audit |

### [Memory System](research/memory-system.md)

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Two-Stage Model](https://doi.org/10.1152/physrev.00032.2012)** | Fast episodic encoding → slow semantic consolidation | `memory://thinking/` (episodic) vs `memory://research/` (semantic) |
| **[ACT-R Decay](https://doi.org/10.4324/9781315805696)** | Memories fade without access following power-law | `DecayCalculator`: `base × time^(-0.5)` |
| **[Storage vs Retrieval](https://www.researchgate.net/publication/281322665)** | Pointers persist; only accessibility fades | WikiLinks never deleted; decay affects ranking |
| **[Maintenance Cycles](https://doi.org/10.1016/j.neuron.2013.12.025)** | Periodic graph hygiene | 5 workflows in serialized dependency order: repair, hub-detection, consolidation, epistemic, status |
| **[Consolidation](https://doi.org/10.1038/nrn2762)** | Thinking sessions → semantic memory | Replay via `RecentActivity`, distill to `WriteMemory`, link via `FindSimilarConcepts` |

### Symbolic Systems

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Concept-as-Protocol](context-engineering.md)** | WikiLinks carry context between agents | `PreToolUse` hook extracts `[[concepts]]`, injects graph context |
| **[Lazy Graph](https://en.wikipedia.org/wiki/Zettelkasten)** | No predefined schema; structure emerges | `Sync` extracts WikiLinks; co-occurrence creates edges |
| **[Hybrid Search](https://dl.acm.org/doi/10.1145/1571941.1572114)** | Semantic similarity + exact matching | `SearchMemories` fuses with Reciprocal Rank Fusion (k=60) |
| **Concept Repair** | Normalize WikiLink variants safely | `RepairConcepts` validates similarity ≥0.7 before replacing |
| **WikiLink Filtering** | Write-time concept blocklist | `.wikilink-filter.json` prevents hub concepts from graph pollution |
| **MCP Resources** | Live-queryable graph state via URI scheme | `graph://` URIs for corpus snapshots, communities, concepts, and recent activity with subscribe/notify |

### Reasoning

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking)** | State machine: `active` → `completed`/`cancelled`/`abandoned`. Multi-step reasoning with revision, branching, and concurrent multi-agent branches | Extends MCP server with `sessionId`, `branchId`, `parentWorkflowId`; can be registered as a Workflow submachine |
| **Assumption Ledger** | Track beliefs and their validation status | `AssumptionLedger` with confidence levels, evidence links |
| **Pipeline Execution** | DAG-ordered execution with conditional routing, per-node retry, timeout enforcement, and session resume | `Pipeline` tool with `EdgeType::Conditional`, `backon` retry, `sessionRef` for ACP session reconnection |

### Workflow System

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Workflow Engine** | Structured multi-step methodology execution | `Workflow` tool with JSON steps, `toolHints`, guardrails |
| **Supervisor/Submachine** | Workflows manage SequentialThinking lifecycles | `submachineSessionId` registers child; `phase` gates step advancement |
| **Nested Composition** | Workflows invoke other workflows and tools | Workflows embed `SequentialThinking`; bidirectional linking |
| **Session Persistence** | Resume reasoning across days | State in `memory://workflow/`; session IDs enable continuation |

**Notable Workflows:**

| Workflow | What It Demonstrates | How |
|----------|---------------------|-----|
| **Workflow Dispatch** | Meta-cognitive methodology selection | Analyzes problem characteristics, queries graph for similar past problems, selects optimal workflow |
| **Research Think Tank** | Multi-agent collaborative reasoning | Defines roles (synthesizer, critic, explorer), structures knowledge construction phases |
| **Agentic SLC** | Quality-controlled development | Embeds anti-slop checks, requires RTM traceability, enforces ConfessionReports |
| **Constitutional Roles** | Constitutional AI for persona creation | Uses principles + examples to generate role definitions that constrain agent behavior |
| **Higher-Order Thinking** | Meta-cognitive reflection | Recursive self-analysis steps: examine reasoning, identify biases, refine approach |
| **Six Thinking Hats** | Structured perspective switching | Sequences through DeBono's colors with explicit transitions and synthesis |

*Multi-agent orchestration requires an MCP client with agent dispatch (Claude Code, Codex, aishell) or CLI scripting with subprocess spawning.*

---


## How It Works

### Sequential Thinking

AI agents can engage in multi-hour reasoning sessions with full revision capability:

```markdown
# Thinking Session: Architecture Design
Thought 1: Analyzing requirements...
Thought 2: Wait, I need to reconsider the scaling approach from Thought 1...
Thought 3: Branching to explore microservices vs monolithic...
```

Each session creates a `memory://thinking/session-{id}` file tracking the complete reasoning chain. Sessions support:
- **Revision**: Reconsider and update previous thoughts
- **Branching**: Explore alternative reasoning paths
- **Persistence**: Continue across days or weeks
- **Multi-agent**: Share sessions between agents

### Orchestrated Workflows

Pre-built workflows embed sequential thinking at critical decision points:

- **Discovery Wave**: Parallel agents explore the problem space
- **Validation Wave**: Test assumptions and verify approaches
- **Implementation Wave**: Execute with confidence

**Intelligent Workflow Selection**: The `workflow-dispatch` meta-system analyzes problem characteristics, researches historical context, assesses cognitive requirements, and automatically selects optimal reasoning methodologies.

**The PM Pattern**: A primary agent (usually Sonnet) acts as the blue-hat orchestrator, using sequential thinking to maintain project context. It dispatches ephemeral sub-agents for specific tasks — these agents can burn through their context windows on implementation details while the PM preserves the overall vision. All thinking persists to `memory://` for continuity.

Workflows maintain state across days, enabling true long-running projects.

### Hybrid RRF Search

Combines semantic vectors with full-text search using Reciprocal Rank Fusion:

- **Semantic search** finds conceptually related content
- **Full-text search** ensures exact terms aren't missed
- **RRF fusion** optimally blends both result sets (k=60)
- **Returns context** with relevance scores for transparency

### Lazy Graph Construction

The knowledge graph builds itself through natural use:

```markdown
# API Design Decision
We chose [[REST]] over [[GraphQL]] for our [[public API]] due to
[[caching]] requirements and [[client simplicity]].
```

Every `[[WikiLink]]` becomes a node. Every mention strengthens edges. Patterns emerge without planning.

Concepts can be excluded from the graph via `.wikilink-filter.json` — a blocklist in the memory root checked at write time that prevents hub or ephemeral concepts from polluting the graph structure.

#### `.wikilink-filter.json` Format

The file is a JSON object mapping concept keys to human-readable reasons:

```json
{
  "software-tool": "God node with 7,438 edges — routes to nothing",
  "workflowstep*": "Code symbol prefix — catches WorkflowStepTimeline, WorkflowStepCard, etc."
}
```

**Exact match**: Keys without a trailing `*` are normalized via `normalize_concept` (lowercase, spaces/underscores/slashes → hyphens, deduplicated). `"AgentPool"` normalizes to `agentpool` and blocks `[[AgentPool]]`.

**Prefix match**: Keys ending with `*` use lowercase-only normalization (no hyphen splitting). `"workflowstep*"` matches any concept starting with `workflowstep` — catching `workflowsteptimeline`, `workflowstepinspector`, `workflowstepcard` in one entry. Because CamelCase normalizes to concatenated lowercase (not hyphenated), `"workflowstep*"` won't collide with `[[workflow-step]]`.

The filter is mtime-cached — edits take effect on the next write operation without restarting the server. Symlinked filter files are rejected (policy must be rooted in memory).

### Memory Lifecycle

Knowledge follows a lifecycle grounded in cognitive neuroscience:

**Decay**: Memories fade without access, following ACT-R's power-law of forgetting:
```
retrieval_strength = base_strength × (time_since_access)^(-decay_rate)
```

**Tiered Memory Classes**:

| Tier | Grace Period | Half-Life | Examples |
|------|-------------|-----------|----------|
| Sequential | 7 days | 30 days | `memory://thinking/sequential/` sessions |
| Workflows | 14 days | 30 days | `memory://thinking/workflow/` sessions |
| Semantic | 28 days | 30 days | `memory://research/`, `memory://decisions/` |
| Immortal | ∞ | ∞ | Validated assumptions, core architecture |

**Sleep Cycles**: Periodic maintenance runs five specialist workflows in serialized dependency order:
- **Repair**: Normalizes WikiLink variants, cleans orphaned concepts
- **Hub Detection**: Identifies and filters generic hub concepts from the graph
- **Consolidation**: Replays high-significance episodic memories, promotes to semantic
- **Epistemic**: Reviews assumptions, validates or invalidates based on evidence
- **Status**: Analyzes decay patterns, reports memory health

*Decay affects search ranking only. Files are never deleted.*

---


## Cognitive Assets

| Asset | Examples |
|-------|----------|
| **Workflows** | Deductive, design thinking, agentic sprints, game theory, research think tank |
| **Roles** | Architect, PM, red-team, blue-team, researcher, writer, FinOps practitioner |
| **Thinking Colors** | DeBono's Six Hats + Gray and Pink |
| **Perspectives** | Native language modes for culturally-aware reasoning |

---


## Technical Principles

### Transparency First
Every operation is inspectable — thinking sessions show all revisions, search results include scores, graph relationships are queryable SQL, all content is readable markdown.

### Lazy Evaluation
Nothing is pre-computed or pre-structured — graph builds from WikiLink usage, embeddings generate on demand, relationships emerge from repetition, structure follows function.

### Composable Tools
Each tool does one thing well — `WriteMemory` creates markdown, `SequentialThinking` reasons iteratively, `BuildContext` traverses the graph, `Workflow` orchestrates tool composition.

### Files as Source of Truth
Not a black box — direct file access for debugging, git-compatible for version control, Obsidian-compatible for human editing, standard markdown for portability.

---


