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

Context engineering infrastructure for AI agents. Point it at any domain's literature, and it builds specialized experts that live on your machine, work offline, and get smarter with every use.

Three proof domains — [FinOps](integrations/opencode/skills/finops-toolkit/README.md), software engineering, and EDA roles — zero overlap, same infrastructure.

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

Seed the graph. Keep the experts. Watch it compound.

- **`[[WikiLinks]]`** — lightweight concept identifiers, not payloads
- **Hybrid search** — semantic vectors + full-text with RRF fusion
- **Knowledge graph** — lazy construction, structure emerges from use
- **Sequential thinking** — multi-step reasoning with revision, branching, persistence
- **35+ workflows** — deductive reasoning to multi-agent sprints
- **Memory lifecycle** — decay, consolidation, repair modeled on cognitive neuroscience
- **16 roles, 7 thinking colors, 12 perspectives** — composable cognitive assets
- **Community detection** — Louvain algorithm identifies reasoning domains during sync
- **Decay weighting** — ACT-R power-law recency bias across search, context, and similarity
- **Graph-of-thought priming** — hook system injects clustered concept maps at session start

Six layers: WikiLinks → Graph → Search → Session State → Reasoning → Orchestration.

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
| **[Maenifold](integrations/claude-code/plugin-maenifold/skills/maenifold/)** | 25+ tools, 6 composable layers, sequential thinking, 35+ workflows |
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
