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

A 6-layer cognitive architecture for AI agents: `[[WikiLinks]]` → Graph → Hybrid Search → Session State → Reasoning → Orchestration. Each layer builds on the one below. Decay models prioritize recent and frequently-accessed content. Maintenance workflows consolidate episodic data into semantic memory.

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
| macOS | osx-arm64 | Apple Silicon; Homebrew recommended |
| Linux | linux-x64 | x64 only |
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

**Optional: MCP for AI Assistants**

To expose maenifold to Claude Code, Claude Desktop, or other MCP clients:

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
| **[Maenifold](integrations/skills/maenifold/README.md)** | 25+ tools, 6-layer cognitive stack, sequential thinking, 32 workflows |
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
| **New Theory of Disuse** | Bjork & Bjork | Storage vs retrieval strength |
| **Two-Stage Memory** | Cognitive neuroscience | Episodic → semantic consolidation |
| **Linguistic Relativity** | Weak form (Boroditsky et al.) | Language influences agent cognition |

### Memory System

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Two-Stage Model** | Fast episodic encoding → slow semantic consolidation | `memory://thinking/` (episodic) vs `memory://research/` (semantic) with different decay rates |
| **ACT-R Decay** | Memories fade without access following power-law | `DecayService` calculates retrieval strength: `base × time^(-rate)` |
| **Storage vs Retrieval** | Pointers persist; only accessibility fades | WikiLinks in files never deleted; decay weights affect search ranking |
| **Maintenance Cycles** | Scheduled graph hygiene for signal-to-noise ratio | 4 parallel workflows: `memory-consolidation`, `memory-decay`, `memory-repair`, `memory-epistemic` |
| **Consolidation** | CoT → long-term memory promotion + cross-domain linking | Episodic thinking sessions processed into semantic memory; `FindSimilarConcepts` surfaces connections |

### Symbolic Systems

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Concept-as-Protocol** | WikiLinks carry context between agents | `PreToolUse` hook extracts `[[concepts]]`, calls `BuildContext` + `FindSimilarConcepts`, injects results |
| **Lazy Graph** | No predefined schema; structure builds incrementally | `Sync` extracts WikiLinks from markdown via regex; co-occurrence creates edges |
| **Hybrid Search** | Best of semantic similarity + exact matching | `SearchMemories` runs both, fuses with Reciprocal Rank Fusion (k=60) |
| **Concept Repair** | Normalize WikiLink variants safely | `RepairConcepts` validates semantic similarity ≥0.7 before replacing |

### Reasoning

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Sequential Thinking** | Multi-step reasoning that persists and branches | `SequentialThinking` tool with `sessionId`, `branchId`, `revisesThought` parameters |
| **Assumption Ledger** | Track beliefs and their validation status | `AssumptionLedger` tool with `action=append/update/read`, confidence levels, evidence links |

### Workflow System

| Feature | What It Means | Implementation |
|---------|---------------|----------------|
| **Workflow Engine** | Structured multi-step methodology execution | `Workflow` tool with JSON step definitions, `toolHints`, guardrails, stop conditions |
| **Nested Composition** | Workflows invoke other workflows and tools | Workflows embed `SequentialThinking`; thinking sessions can trigger workflows |
| **Session Persistence** | Resume reasoning across days | Workflow state stored in `memory://workflow/`; session IDs enable continuation |

**Notable Workflows:**

| Workflow | What It Demonstrates | How |
|----------|---------------------|-----|
| **Workflow Dispatch** | Meta-cognitive methodology selection | Analyzes problem characteristics, queries graph for similar past problems, selects optimal workflow |
| **Research Think Tank** | Multi-agent collaborative reasoning | Defines roles (synthesizer, critic, explorer), structures knowledge construction phases |
| **Agentic SLC** | Quality-controlled development | Embeds anti-slop checks, requires RTM traceability, enforces ConfessionReports |
| **Constitutional Roles** | Constitutional AI for persona creation | Uses principles + examples to generate role definitions that constrain agent behavior |
| **Higher-Order Thinking** | Meta-cognitive reflection | Recursive self-analysis steps: examine reasoning, identify biases, refine approach |
| **Six Thinking Hats** | Structured perspective switching | Sequences through DeBono's colors with explicit transitions and synthesis |

*Multi-agent orchestration requires an MCP client with agent dispatch capability (Claude Code, Codex, aishell).*

### Cognitive Assets

| Asset | Count | Examples |
|-------|-------|----------|
| **Workflows** | 32 | Deductive, design thinking, agentic sprints, game theory |
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
| Decay Model | ACT-R power-law (30d half-life); affects ranking, never deletes |
| Memory Tiers | Grace periods: Episodic (7d) / Semantic (14d) / Immortal |
| Maintenance | 4 workflows (consolidation/decay/repair/epistemic) |
| Scale | 1.1M+ relationships tested |

*Decay affects search ranking only. Files are never deleted.*

*Local-only, single-user, no authentication. Memory files (markdown) are the source of truth; the SQLite database is a regenerable cache (`maenifold --tool Sync`). Version control memory with git to track changes alongside code. Set `MAENIFOLD_ROOT` to store memory in your repository for atomic commits of code + reasoning.*

## Documentation

- **[Bootstrap Guide](docs/BOOTSTRAP.md)** — From empty graph to domain expertise
- **[Scripting Guide](docs/SCRIPTING.md)** — CLI patterns, Graph-RAG, HYDE, FLARE
- **[Complete Guide](docs/README.md)** — Architecture, philosophy, detailed examples
- **[Maenifold Skill](integrations/skills/maenifold/README.md)** — Core tools and cognitive stack
- **[Product Manager Skill](integrations/skills/product-manager/README.md)** — Multi-agent orchestration
- **[Demo Artifacts](docs/demo-artifacts/README.md)** — 25-agent test with full logs

## Community

MIT License. Contributions welcome at [github.com/msbrettorg/maenifold](https://github.com/msbrettorg/maenifold).

---

[![Stargazers over time](https://starchart.cc/msbrettorg/maenifold.svg?variant=adaptive)](https://starchart.cc/msbrettorg/maenifold)
