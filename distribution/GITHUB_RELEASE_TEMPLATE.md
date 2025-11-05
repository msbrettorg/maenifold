# GitHub Release Template

Use this template when creating GitHub releases.

## Release Title

```
v1.0.0
```

## Release Description

```markdown
# maenifold v1.0.0

Persistent Graph-of-Thoughts for AI agents and multi-agent systems.

## 🎉 Highlights

- Initial public release
- Persistent Graph-of-Thoughts implementation
- 47 orchestrated workflows for multi-agent coordination
- MCP protocol support for Claude, GPT, Qwen, Grok, and other LLMs
- Cross-platform self-contained binaries (no .NET runtime required)

## ✨ Features

### Primary Tools (Graph Builders)
- **SequentialThinking** - Multi-step reasoning with revision/branching
- **Workflow** - 47 orchestrated methodologies (think-tank, agentic-slc, etc.)
- **WriteMemory** - Create knowledge files with `[[WikiLinks]]`
- **AssumptionLedger** - Track and validate reasoning assumptions

### Memory & Graph Tools
- **SearchMemories** - Hybrid search (text + semantic)
- **BuildContext** - Traverse concept relationships
- **Visualize** - Generate Mermaid diagrams of concept networks
- **RecentActivity** - Query recent thinking sessions and memories

### Integration Modes
- **.NET Library** - Direct integration with Microsoft Agent Framework
- **MCP Protocol** - Cross-language support for Claude Code, Continue, Cline, Codex
- **CLI** - Script automation and testing

## 📦 Installation

### NPM (Recommended)
```bash
npm install -g @ma-collective/maenifold
```

### Homebrew (macOS/Linux)
```bash
brew tap ma-collective/tap
brew install maenifold
```

### WinGet (Windows)
```bash
winget install maenifold
```

### Direct Download
Download platform-specific binaries from the Assets section below.

## 🔧 Usage

**CLI mode:**
```bash
maenifold --tool WriteMemory --payload '{
  "title": "API Design",
  "content": "We chose [[REST]] over [[GraphQL]]..."
}'
```

**MCP mode (Claude Code):**
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": {"maenifold_ROOT": "~/maenifold"}
    }
  }
}
```

## 📊 Performance

- **Scale tested:** 1.1M+ graph relationships
- **Build time:** ~3-5 seconds (full Release build)
- **Test coverage:** 78 passing tests
- **Platform support:** Linux x64, macOS ARM64/x64, Windows x64

## 📚 Documentation

- [Complete Documentation](https://github.com/ma-collective/maenifold/blob/main/docs/README.md)
- [Development Guide](https://github.com/ma-collective/maenifold/blob/main/docs/DEVELOPMENT.md)
- [Distribution Guide](https://github.com/ma-collective/maenifold/blob/main/DISTRIBUTION.md)

## 🔬 Research Foundation

Based on 2024 research breakthroughs:
- [Graph of Thoughts](https://arxiv.org/abs/2308.09687) (AAAI 2024) - 62% better than Tree-of-Thoughts
- [Graph-RAG](https://www.microsoft.com/en-us/research/blog/graphrag-unlocking-llm-discovery-on-narrative-private-data/) (Microsoft Research)

## 🐛 Known Issues

None at this time.

## 🙏 Acknowledgments

- Microsoft Agent Framework team
- AAAI 2024 Graph of Thoughts researchers
- MCP protocol contributors

## 📝 Full Changelog

See [CHANGELOG.md](https://github.com/ma-collective/maenifold/blob/main/CHANGELOG.md)

---

## Platform-Specific Download Instructions

### Linux (x64)
```bash
wget https://github.com/ma-collective/maenifold/releases/download/v1.0.0/maenifold-linux-x64.tar.gz
tar -xzf maenifold-linux-x64.tar.gz
chmod +x maenifold
./maenifold --tool MemoryStatus --payload '{}'
```

### macOS (ARM64 - M1/M2/M3)
```bash
curl -LO https://github.com/ma-collective/maenifold/releases/download/v1.0.0/maenifold-osx-arm64.tar.gz
tar -xzf maenifold-osx-arm64.tar.gz
chmod +x maenifold
./maenifold --tool MemoryStatus --payload '{}'
```

### macOS (x64 - Intel)
```bash
curl -LO https://github.com/ma-collective/maenifold/releases/download/v1.0.0/maenifold-osx-x64.tar.gz
tar -xzf maenifold-osx-x64.tar.gz
chmod +x maenifold
./maenifold --tool MemoryStatus --payload '{}'
```

### Windows (x64)
```powershell
# Download maenifold-win-x64.zip from Assets below
# Extract the ZIP
# Run in PowerShell:
.\maenifold.exe --tool MemoryStatus --payload '{}'
```

## Verification

Verify downloads with SHA256 checksums from `SHA256SUMS` file in Assets.

```bash
# Linux/macOS
shasum -a 256 -c SHA256SUMS

# Windows PowerShell
Get-FileHash maenifold-win-x64.zip -Algorithm SHA256
```
```

## Release Assets to Upload

1. `maenifold-linux-x64.tar.gz`
2. `maenifold-osx-arm64.tar.gz`
3. `maenifold-osx-x64.tar.gz`
4. `maenifold-win-x64.zip`
5. `SHA256SUMS`

## Release Checklist

Before creating the release:

- [ ] Version updated in `package.json`
- [ ] Run `./scripts/release.sh 1.0.0`
- [ ] All tests passing (`dotnet test`)
- [ ] All platform binaries built successfully
- [ ] SHA256SUMS file generated
- [ ] Git tag created: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Git tag pushed: `git push origin v1.0.0`
- [ ] CHANGELOG.md updated with release notes

After creating the release:

- [ ] Upload all 5 files to GitHub release
- [ ] Mark as "Latest release" (if applicable)
- [ ] Publish to NPM: `npm publish --access public`
- [ ] Update Homebrew tap with new formula
- [ ] Submit WinGet PR (if updating existing package)
- [ ] Announce on GitHub Discussions/social media
