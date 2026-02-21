# maenifold

**Domain expertise that compounds. Open. Local. Yours.**

Context engineering infrastructure for AI agents. Point it at any domain's literature, and it builds specialized experts that live on your machine, work offline, and get smarter with every use.

---

## Table of Contents

- [Theoretical Foundations](#theoretical-foundations)
- [How It Works](#how-it-works)
  - [Sequential Thinking](#sequential-thinking)
  - [Orchestrated Workflows](#orchestrated-workflows)
  - [Hybrid RRF Search](#hybrid-rrf-search)
  - [Lazy Graph Construction](#lazy-graph-construction)
  - [Memory Lifecycle](#memory-lifecycle)
- [Quick Start](#quick-start)
  - [For VSCode Users](#for-vscode-users)
  - [For Developers](#for-developers)
- [The Cognitive Stack](#the-cognitive-stack)
  - [Memory Layer](#memory-layer)
  - [Graph Layer](#graph-layer)
  - [Symbolic Communication](#symbolic-communication)
  - [Reasoning Layer](#reasoning-layer)
- [Cognitive Assets](#cognitive-assets)
- [Key Capabilities](#key-capabilities)
- [Example Usage](#example-usage)
- [Real-World Impact](#real-world-impact)
- [Testing & Validation](#testing--validation)
- [Technical Specifications](#technical-specifications)
- [Technical Principles](#technical-principles)
- [Configuration](#configuration)
- [Skills](#skills)
- [Project Structure](#project-structure)
- [License & Attribution](#license--attribution)

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

### Reasoning

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking)** | State machine: `active` → `completed`/`cancelled`/`abandoned`. Multi-step reasoning with revision, branching, and concurrent multi-agent branches | Extends MCP server with `sessionId`, `branchId`, `parentWorkflowId`; can be registered as a Workflow submachine |
| **Assumption Ledger** | Track beliefs and their validation status | `AssumptionLedger` with confidence levels, evidence links |

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

Concepts can be excluded from the graph via `.wikilink-filter.json` — a simple blocklist checked at write time that prevents hub or ephemeral concepts from polluting the graph structure.

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

## Quick Start

### For VSCode Users

1. **Install maenifold**:
```bash
# macOS/Linux
brew install msbrettorg/tap/maenifold

# Windows - download from GitHub Releases
# https://github.com/msbrettorg/maenifold/releases/latest
```

2. **Configure your AI assistant**:

#### Claude Code
```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

#### Continue.dev
Add to `~/.continue/config.json`:
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": { "MAENIFOLD_ROOT": "~/maenifold" }
    }
  }
}
```

#### Cline
Add to VSCode settings (`Cmd+,` → Extensions → Cline):
```json
{
  "cline.mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": { "MAENIFOLD_ROOT": "~/maenifold" }
    }
  }
}
```

#### Codex
Add to `~/.codex/config.toml`:
```toml
[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
env = { MAENIFOLD_ROOT = "~/maenifold" }
```

**Multi-agent orchestration** requires MCP clients with agent dispatch:
- **Claude Code** — `claude-code` CLI with Task tool
- **Codex** — Purpose-built orchestrator
- **aishell** — Open-source with agent management

3. **Start using it**:
- Single agent: "Write a memory about our project architecture"
- Multi-agent: "Use agentic-dev workflow to implement authentication"
- Graph query: "Show me how our concepts connect"

### For Developers

**CLI** (macOS/Linux):
```bash
maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'
maenifold --tool Sync --payload '{}'
```

**CLI** (Windows PowerShell):
```powershell
maenifold --tool WriteMemory --payload '{\"title\":\"Auth Decision\",\"content\":\"Using [[OAuth2]] for [[authentication]]\"}'
```

**MCP vs CLI**:

| Interface | Advantage | Best for |
|-----------|-----------|----------|
| **MCP** | Auto-sync watcher keeps graph current | Interactive sessions, simple queries |
| **CLI** | Filter intermediate results, preserve context ([why this matters](https://www.anthropic.com/engineering/code-execution-with-mcp)) | Complex workflows, scripting, [Graph-RAG patterns](SCRIPTING.md) |

See [SCRIPTING.md](SCRIPTING.md) for advanced patterns and [BOOTSTRAP.md](BOOTSTRAP.md) for the full journey from empty graph to domain expertise.

---

## The Cognitive Stack

### Memory Layer

Every piece of knowledge lives as a markdown file with a unique URI:
- `memory://decisions/api-design` — Architectural decisions
- `memory://thinking/session-12345` — Sequential thinking sessions
- `memory://research/rag-comparison` — Research notes

All files are human-readable, Obsidian-compatible, and persist across sessions.

**Two-Stage Memory Model**: Following the hippocampal-neocortical distinction:
- **Episodic** (`memory://thinking/`): Fast encoding, rapid decay — raw reasoning traces
- **Semantic** (`memory://research/`, `memory://decisions/`): Slow consolidation, stable knowledge
- **Immortal**: Validated assumptions and core architecture — exempt from decay

The memory-consolidation workflow transfers high-value episodic memories to semantic storage, mirroring biological memory consolidation during sleep.

### Graph Layer

Automatic graph construction from WikiLinks with:
- **384-dimensional embeddings** for semantic similarity
- **Edge weights** that strengthen with repeated mentions
- **Concept clustering** revealing emergent patterns
- **Community detection** via Louvain algorithm identifies reasoning domains
- **Decay weighting** (ACT-R power-law) biases concept rankings by recency
- **Incremental sync** keeping the graph current

### Symbolic Communication

WikiLinks serve as an inter-agent communication protocol — **concept-as-protocol**:

```markdown
# Agent prompt with symbolic references
"Fix the [[authentication]] bug in [[session-management]]"
```

When an agent writes `[[concept]]`, the system:
1. Extracts WikiLinks via regex
2. Calls `BuildContext` + `FindSimilarConcepts` for each
3. Injects graph neighborhood as enriched context
4. Receiving agent gets structured knowledge without manual curation

**Information Density**: A 14-byte `[[authentication]]` dereferences to megabytes of structured knowledge — far exceeding what fits in a prompt.

**Storage vs Retrieval Strength** (New Theory of Disuse):
- **WikiLinks** = storage strength (pointers persist, never decay)
- **Decay weight** = retrieval strength (access recency determines surfacing priority)

Forgetting is not loss of the pointer — it's reduced priority for retrieval. The knowledge remains; only its accessibility fades.

### Reasoning Layer

Where test-time computation happens:
- **Sequential Thinking**: State machine (`active` → `completed`/`cancelled`/`abandoned`) with revision, branching, and multi-agent sessions
- **Workflow Orchestration**: Supervisor state machine managing 35+ methodologies; registers SequentialThinking as submachines, gates step advancement until completion
- **Assumption Ledger**: Traceable skepticism — capture, validate, and track assumptions
- **Multi-agent Coordination**: Wave-based execution with parallel agent dispatch
- **Intelligent Method Selection**: Meta-cognitive system for optimal reasoning approach
- **RTM Validation**: Requirements traceability for systematic development
- **Quality Control**: Stop conditions, validation gates, and anti-slop controls

**Context Window Economics**: The PM (blue hat) uses sequential thinking to preserve expensive context while dispatching fresh agents for implementation. This allows complex projects without context exhaustion.

---

## Cognitive Assets

| Asset | Count | Examples |
|-------|-------|----------|
| **Workflows** | 35+ | Deductive, design thinking, agentic sprints, game theory, research think tank |
| **Roles** | 16 | Architect, PM, red-team, blue-team, researcher, writer, FinOps practitioner |
| **Thinking Colors** | 7 | DeBono's Six Hats + Gray (skeptical inquiry) |
| **Perspectives** | 12 | Native language modes for culturally-aware reasoning |

---

## Key Capabilities

- **Test-time Adaptive Reasoning**: Sequential thinking with revision, branching, and multi-agent collaboration
- **Intelligent Workflow Selection**: Meta-cognitive system that analyzes problems and selects optimal reasoning approaches
- **35+ Distinct Methodologies**: Complete taxonomy from deductive reasoning to design thinking
- **Hybrid RRF Search**: Semantic + full-text fusion for optimal retrieval
- **Lazy Graph Construction**: No schema, no ontology — structure emerges from WikiLink usage
- **Neuroscience-Grounded Decay**: ACT-R power-law forgetting with tiered memory classes
- **Autonomous Sleep Cycles**: Five-specialist maintenance runs unattended
- **Symbolic Inter-Agent Protocol**: WikiLinks dereference to graph context
- **Hierarchical State Machines**: Workflows supervise SequentialThinking submachines — gate step advancement, self-heal on missing sessions
- **Quality-Gated Orchestration**: Multi-agent coordination with validation waves and RTM compliance
- **Complete Transparency**: Every thought, revision, and decision visible in markdown files
- **Multi-day Persistence**: Sessions maintain state across restarts
- **Community Detection**: Louvain algorithm identifies reasoning domains during sync, enabling graph-of-thought priming
- **Session Abandonment Detection**: 30-minute inactivity threshold marks active sessions as abandoned during sync
- **Graph-of-Thought Priming**: Hook system queries community index to inject clustered concept maps at session start and task dispatch

---

## Demo Artifacts

Real orchestration sessions demonstrating the system in use:

- **[PM-lite Protocol](demo-artifacts/part1-pm-lite/)** — 12 agents across 4 waves, 85% test success, critical bug found and fixed (28 min)
- **[Agentic-SLC Workflow](demo-artifacts/part2-agentic-slc/)** — 13 agents, 3 issues fixed via TDD pipeline (40 min)
- **[Brand Analysis](demo-artifacts/brand-analysis-workflow.md)** — 6 parallel workflows analyzed maenifold's own brand positioning
- **[Sprint Impact Analysis](demo-artifacts/sprint-knowledge-graph-impact-analysis.md)** — Knowledge graph growth from a real sprint

See the [demo artifacts README](demo-artifacts/README.md) for the full timeline and directory structure.

---

## Testing & Validation

### The Hero Demo

The [comprehensive E2E test](demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md) orchestrated **12 specialized agents** across 4 waves:

- **Found and fixed a critical bug**: Move operations were losing file extensions — mocks would never catch this
- **Discovered parameter inconsistencies**: minScore filtering wasn't working — only real queries revealed this
- **Validated actual performance**: Real operations against real data, not synthetic benchmarks
- **85% success rate**: Honest assessment, not 100% fake passes

### Test Philosophy

Following the **NO FAKE TESTS** principle:
- Real SQLite, not mocks
- Real file operations, not stubs
- Real vector embeddings, not fixtures
- Real multi-agent coordination, not simulations

---

## Technical Specifications

| Spec | Value |
|------|-------|
| Language | C# / .NET 9.0 |
| Vectors | 384-dim (all-MiniLM-L6-v2 via ONNX) |
| Search | Reciprocal Rank Fusion (k=60) |
| Database | SQLite + [sqlite-vec](https://github.com/asg017/sqlite-vec) (bundled) |
| Memory Cycle | Configurable interval; decay params expressed as cycle multiples |
| Decay Model | ACT-R power-law (d=0.5); calibrated to memory cycle |
| Memory Tiers | Sequential (7d grace) / Workflows (14d) / Semantic (28d) / Half-life 30d |
| Maintenance | 5 workflows (repair/hub-detection/consolidation/epistemic/status) in serialized dependency order |

*Decay affects search ranking only. Files are never deleted.*

*Local-only, single-user, no authentication. Memory files (markdown) are the source of truth; the SQLite database is a regenerable cache (`maenifold --tool Sync`). Version control memory with git to track changes alongside code. Set `MAENIFOLD_ROOT` to store memory in your repository for atomic commits of code + reasoning.*

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

## Configuration

**Custom data directory:**
```bash
export MAENIFOLD_ROOT=~/my-knowledge-base
```

**Claude Desktop** — Same MCP config, different location:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

---

## Skills

| Skill | What You Get |
|-------|--------------|
| **[Maenifold](../integrations/claude-code/plugin-maenifold/skills/maenifold/references/README.md)** | 25+ tools, 6 composable layers, sequential thinking, 35+ workflows |
| **[Product Manager](../integrations/claude-code/plugin-product-team/skills/product-manager/README.md)** | Multi-agent orchestration, graph context injection, quality gates, sprint traceability |

---

## Integrations

| Integration | What It Does |
|-------------|-------------|
| **[Claude Code Plugin](../integrations/claude-code/plugin-maenifold/)** | MCP server, graph-of-thought hooks, skill auto-loading |
| **[OpenCode Plugins](../integrations/opencode/)** | WikiLink-aware compaction, session persistence for OpenCode CLI |

---

## Project Structure

```
~/maenifold/
├── memory/           # Your markdown memories with WikiLinks
├── memory.db         # Knowledge graph and vector embeddings
└── assets/           # Workflows, roles, colors, perspectives
```

---

## License & Attribution

**License**: MIT

Sequential Thinking implementation inspired by [MCP Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking) — enhanced with lazy graph construction and multi-agent collaboration.

---

*Domain expertise that compounds.*
