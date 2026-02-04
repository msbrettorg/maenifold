<p align="center">
  <img src="docs/branding/maenifold-logo.svg" alt="maenifold">
</p>

<p align="center">
  Your AI is ephemeral. Your knowledge shouldn't be.
</p>

<p align="center">
  <a href="https://github.com/msbrettorg/maenifold/releases/latest"><img src="https://img.shields.io/github/v/release/msbrettorg/maenifold?style=flat-square" alt="Latest Release"></a>
  <a href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE"><img src="https://img.shields.io/github/license/msbrettorg/maenifold?style=flat-square" alt="MIT License"></a>
</p>

## What is maenifold?

A cognitive middle layer between raw input and AI reasoning. Six composable layers: `[[WikiLinks]]` → Graph → Hybrid Search → Session State → Reasoning → Orchestration. Decay models prioritize recent and frequently-accessed content. Maintenance workflows consolidate episodic data into semantic memory.

## Quick Start

**Install**

```bash
# Homebrew (macOS/Linux) — handles PATH automatically
brew install msbrettorg/tap/maenifold

# Manual — download from GitHub Releases, extract, add to PATH or symlink
# https://github.com/msbrettorg/maenifold/releases/latest
```

**Requirements**

| Platform | Binary | Notes |
|----------|--------|-------|
| macOS | osx-arm64, osx-x64 | Apple Silicon or Intel; Homebrew recommended |
| Linux | linux-x64, linux-arm64 | x64 or ARM64 |
| Windows | win-x64 | x64 only; use PowerShell syntax below |

Binaries are self-contained (.NET 9.0 bundled). Vector embeddings use ONNX runtime (bundled). No external dependencies.

**Use the CLI**

*macOS/Linux (bash):*
```bash
maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'
maenifold --tool Sync --payload '{}'
```

*Windows (PowerShell):*
```powershell
maenifold --tool WriteMemory --payload '{\"title\":\"Auth Decision\",\"content\":\"Using [[OAuth2]] for [[authentication]]\"}'
maenifold --tool SearchMemories --payload '{\"query\":\"authentication\",\"mode\":\"Hybrid\"}'
maenifold --tool BuildContext --payload '{\"conceptName\":\"authentication\",\"depth\":2}'
maenifold --tool Sync --payload '{}'
```

The CLI is the primary interface. See [SCRIPTING.md](docs/SCRIPTING.md) for advanced patterns.

**Build domain expertise** — See [Bootstrap Guide](docs/BOOTSTRAP.md) for the full journey: research workflows, custom roles, custom workflows, and maintenance.

**MCP vs CLI**

Both interfaces have full feature parity. Choose based on your use case:

| Interface | Advantage | Best for |
|-----------|-----------|----------|
| **MCP** | Auto-sync watcher keeps graph current | Interactive sessions, simple queries |
| **CLI** | Filter intermediate results, preserve context ([why this matters](https://www.anthropic.com/engineering/code-execution-with-mcp)) | Complex workflows, scripting, [Graph-RAG patterns](docs/SCRIPTING.md) |

**MCP config** (for Claude Code, Claude Desktop, etc.):

```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

## Skills

| Skill | What You Get |
|-------|--------------|
| **[Maenifold](integrations/skills/maenifold/README.md)** | 25+ tools, 6-layer composable layers, sequential thinking, 35+ workflows |
| **[Product Manager](integrations/skills/product-manager/README.md)** | maenifold-aware agents, graph context injection, quality gates, sprint traceability |

## Core Mechanisms

*Layered from theory → memory → symbols → reasoning → orchestration → assets*

### Philosophical Foundations

| Concept | Origin | Application |
|---------|--------|-------------|
| **[Ma (間)](docs/MA_MANIFESTO.md)** | Japanese aesthetics | The space between things as the thing itself ([what we don't do](docs/WHAT_WE_DONT_DO.md)) |

### Theoretical Foundations

| Concept | Origin | Application |
|---------|--------|-------------|
| **[ACT-R](docs/research/decay-in-ai-memory-systems.md)** | Anderson (CMU); Wixted & Ebbesen (1991) | Power-law inspired decay ([exponential approximation](docs/research/decay-in-ai-memory-systems.md#54-the-act-r-connection)) |
| **[New Theory of Disuse](docs/research/decay-in-ai-memory-systems.md#23-the-spacing-effect-and-retrieval-strengthening)** | Bjork & Bjork | Storage vs retrieval strength |
| **[Two-Stage Memory](docs/research/decay-in-ai-memory-systems.md#32-sleep-and-memory-consolidation)** | Cognitive neuroscience | Episodic → semantic consolidation |
| **[Linguistic Relativity](https://lera.ucsd.edu/papers/linguistic-relativity.pdf)** | Weak form (Boroditsky, 2003) | Perspectives change the linguistic frame of LLM reasoning |
| **[ConfessionReport](docs/research/confession-reports.md)** | OpenAI (Barak et al., 2025) | Inference-time honesty enforcement via hooks + adversarial audit |
| **[Context Engineering](docs/context-engineering.md)** | Anthropic (2025) | Just-in-time retrieval, compaction, structured notes, sub-agent architectures |

### [Memory System](docs/research/memory-system.md)

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Two-Stage Model](https://doi.org/10.1152/physrev.00032.2012)** | Fast episodic encoding → slow semantic consolidation | `memory://thinking/` (episodic) vs `memory://research/` (semantic) |
| **[ACT-R Decay](https://doi.org/10.4324/9781315805696)** | Memories fade without access following power-law | `DecayCalculator`: `base × time^(-0.5)` |
| **[Storage vs Retrieval](https://www.researchgate.net/publication/281322665)** | Pointers persist; only accessibility fades | WikiLinks never deleted; decay affects ranking |
| **[Maintenance Cycles](https://doi.org/10.1016/j.neuron.2013.12.025)** | Periodic graph hygiene | 4 workflows: consolidation, decay, repair, epistemic |
| **[Consolidation](https://doi.org/10.1038/nrn2762)** | Thinking sessions → semantic memory | Replay via `RecentActivity`, distill to `WriteMemory`, link via `FindSimilarConcepts` |

### Symbolic Systems

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Concept-as-Protocol](docs/context-engineering.md)** | WikiLinks carry context between agents | `PreToolUse` hook extracts `[[concepts]]`, injects graph context |
| **[Lazy Graph](https://en.wikipedia.org/wiki/Zettelkasten)** | No predefined schema; structure emerges | `Sync` extracts WikiLinks; co-occurrence creates edges |
| **[Hybrid Search](https://dl.acm.org/doi/10.1145/1571941.1572114)** | Semantic similarity + exact matching | `SearchMemories` fuses with Reciprocal Rank Fusion (k=60) |
| **Concept Repair** | Normalize WikiLink variants safely | `RepairConcepts` validates similarity ≥0.7 before replacing |

### Reasoning

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **[Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking)** | Multi-step reasoning with persistence, graph integration, multi-agent branching | Extends MCP server with `sessionId`, `branchId`, `parentWorkflowId` |
| **Assumption Ledger** | Track beliefs and their validation status | `AssumptionLedger` with confidence levels, evidence links |

### Workflow System

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Workflow Engine** | Structured multi-step methodology execution | `Workflow` tool with JSON steps, `toolHints`, guardrails |
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

### Cognitive Assets

| Asset | Count | Examples |
|-------|-------|----------|
| **Workflows** | 35+ | Deductive, design thinking, agentic sprints, game theory |
| **Roles** | 16 | Architect, PM, red-team, blue-team, researcher |
| **Thinking Colors** | 7 | DeBono's Six Hats + Gray (skeptical inquiry) |
| **Perspectives** | 12 | Native language modes for culturally-aware reasoning |

## Configuration

**Claude Desktop** — Same MCP config, different location:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

**Custom data directory:**
```bash
export MAENIFOLD_ROOT=~/my-knowledge-base
```

## Technical Specifications

| Spec | Value |
|------|-------|
| Language | C# / .NET 9.0 |
| Vectors | 384-dim (all-MiniLM-L6-v2 via ONNX) |
| Search | Reciprocal Rank Fusion (k=60) |
| Database | SQLite + [sqlite-vec](https://github.com/asg017/sqlite-vec) (bundled) |
| Decay Model | ACT-R power-law (d=0.5: memory halves when time quadruples); affects ranking, never deletes |
| Memory Tiers | Grace periods: Episodic (7d) / Semantic (14d) / Immortal |
| Maintenance | 4 workflows (consolidation/decay/repair/epistemic) |
| Scale | > 1M relationships tested |

*Decay affects search ranking only. Files are never deleted.*

*Local-only, single-user, no authentication. Memory files (markdown) are the source of truth; the SQLite database is a regenerable cache (`maenifold --tool Sync`). Version control memory with git to track changes alongside code. Set `MAENIFOLD_ROOT` to store memory in your repository for atomic commits of code + reasoning.*

## Documentation

- **[Bootstrap Guide](docs/BOOTSTRAP.md)** — From empty graph to domain expertise
- **[Scripting Guide](docs/SCRIPTING.md)** — CLI patterns, Graph-RAG, HYDE, FLARE
- **[Complete Guide](docs/README.md)** — Architecture, philosophy, detailed examples
- **[Maenifold Skill](integrations/skills/maenifold/README.md)** — Core tools and composable layers
- **[Product Manager Skill](integrations/skills/product-manager/README.md)** — Multi-agent orchestration

## Community

MIT License. Contributions welcome at [github.com/msbrettorg/maenifold](https://github.com/msbrettorg/maenifold).

---

[![Stargazers over time](https://starchart.cc/msbrettorg/maenifold.svg?variant=adaptive)](https://starchart.cc/msbrettorg/maenifold)
