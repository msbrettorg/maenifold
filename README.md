<p align="center">
  <img src="docs/branding/maenifold-logo.svg" alt="maenifold">
</p>

<p align="center">
  Domain expertise that compounds. Open. Local. Yours.
</p>

<p align="center">
  <a href="https://github.com/msbrettorg/maenifold/releases/latest"><img src="https://img.shields.io/github/v/release/msbrettorg/maenifold?style=flat-square" alt="Latest Release"></a>
  <a href="https://github.com/msbrettorg/maenifold/blob/main/LICENSE"><img src="https://img.shields.io/github/license/msbrettorg/maenifold?style=flat-square" alt="MIT License"></a>
</p>

---

Context engineering infrastructure for AI agents. Agents think in chains of thought — maenifold captures the important bits as `[[WikiLinks]]`, builds a graph of just those concepts and how they relate, and feeds it back into the context window. The filler is stripped. The signal compounds. Every AI tool on your machine shares one graph.

## Quick Start

```bash
# Homebrew (macOS/Linux)
brew install msbrettorg/tap/maenifold

# Manual — download from GitHub Releases
# https://github.com/msbrettorg/maenifold/releases/latest
```

**CLI**

```bash
maenifold --tool WriteMemory --payload '{"title":"Auth Decision","content":"Using [[OAuth2]] for [[authentication]]"}'
maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid"}'
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'
```

**MCP** (Claude Code, Claude Desktop, Codex, etc.)

```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

Both interfaces have full feature parity. CLI filters intermediate results and preserves context ([why this matters](https://www.anthropic.com/engineering/code-execution-with-mcp)). MCP auto-syncs the graph during interactive sessions.

## How It Works

**`[[WikiLinks]]` are the primitive.** Each one is a compressed semantic unit — `[[authentication]]`, `[[commitment-discounts]]`, `[[null-reference-exception]]` — carrying meaning in its name alone. When agents tag concepts in their reasoning, those tags become graph nodes. Co-occurring WikiLinks become edges. Structure emerges from use.

**Memory is for humans.** Readable markdown with full prose, citations, and context. Open a file, read it, audit what your agents know.

**The graph is for agents.** A navigable structure of concept names and relationships — the semantic skeleton of everything the machine has learned, stripped of filler. Community detection clusters reasoning domains. Decay weights surface what's fresh. At session start, the graph is injected into the context window as a concept map.

**The graph IS the context window.** Not a database the agent queries and hopes for the best. The compressed, clustered, decay-weighted concept map *is* what primes every session. Agents traverse deeper only when they need the full document.

**One graph. Every agent.** Claude Code, VS Code, Copilot, cron jobs — any MCP client connects to the same local binary. What one agent learns, every agent knows. Knowledge compounds across clients, sessions, domains, and time.

### Capabilities

- **Hybrid search** — semantic vectors + full-text with RRF fusion
- **Sequential thinking** — multi-step reasoning with revision, branching, persistence
- **39 workflows** — deductive reasoning to multi-agent sprints
- **Memory lifecycle** — decay, consolidation, repair modeled on cognitive neuroscience
- **16 roles, 7 thinking colors, 12 perspectives** — composable cognitive assets
- **Community detection** — Louvain algorithm identifies reasoning domains during sync
- **Decay weighting** — ACT-R power-law recency bias across search, context, and similarity

Six layers: WikiLinks → Graph → Search → Session State → Reasoning → Orchestration.

Three proof domains — [FinOps](integrations/opencode/skills/finops-toolkit/README.md), software engineering, and EDA — zero overlap, same infrastructure.

**See it in action**: [6 parallel agents analyzed this brand statement](docs/demo-artifacts/brand-analysis-workflow.md) using Six Thinking Hats, Strategic Thinking, Lateral Thinking, CRTA, Design Thinking, and Socratic Dialogue — all running simultaneously through maenifold's own workflow engine.

## Platforms

| Platform | Binary | Notes |
|----------|--------|-------|
| macOS | osx-arm64, osx-x64 | Apple Silicon or Intel; Homebrew recommended |
| Linux | linux-x64, linux-arm64 | x64 or ARM64 |
| Windows | win-x64 | x64 only |

Self-contained (.NET 9.0 bundled). Vector embeddings via ONNX (bundled). No external dependencies.

## Documentation

- **[Complete Guide](docs/README.md)** — Architecture, philosophy, detailed examples
- **[Bootstrap Guide](docs/BOOTSTRAP.md)** — From empty graph to domain expertise
- **[Scripting Guide](integrations/claude-code/plugin-maenifold/skills/maenifold/references/SCRIPTING.md)** — CLI patterns, Graph-RAG, HYDE, FLARE
- **[Context Engineering](docs/research/context-engineering.md)** — Theoretical foundations
- **[Security Model](docs/SECURITY_MODEL.md)** — STRIDE analysis and data flow

## Skills

| Skill | What You Get |
|-------|--------------|
| **[Maenifold](integrations/claude-code/plugin-maenifold/skills/maenifold/)** | 25+ tools, 6 composable layers, sequential thinking, 39 workflows |
| **[Product Manager](integrations/claude-code/plugin-product-team/skills/product-manager/)** | Multi-agent orchestration, graph context injection, quality gates, sprint traceability |

## Integrations

| Integration | Purpose |
|-------------|---------|
| **[Claude Code](integrations/claude-code/plugin-maenifold/)** | MCP server, graph-of-thought hooks, skill auto-loading |
| **[FinOps Toolkit](integrations/opencode/skills/finops-toolkit/)** | Azure cost management agents, KQL query catalog |
| **[OpenCode](integrations/opencode/)** | WikiLink-aware compaction, session persistence |

## License

MIT. Contributions welcome at [github.com/msbrettorg/maenifold](https://github.com/msbrettorg/maenifold).

---

[![Stargazers over time](https://starchart.cc/msbrettorg/maenifold.svg?variant=adaptive)](https://starchart.cc/msbrettorg/maenifold)
