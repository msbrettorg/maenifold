# GitHub Release Template

Use this template when creating GitHub releases.

## Release Title

```
v1.0.0
```

## Release Description

```markdown
# maenifold v1.0.0

Context engineering infrastructure for AI agents.

## Highlights

- Persistent Graph-of-Thoughts implementation
- Orchestrated workflows for multi-agent coordination
- MCP protocol support for Claude, GPT, Qwen, Grok, and other LLMs
- Cross-platform self-contained binaries (no .NET runtime required)

## Features

### Primary Tools (Graph Builders)
- **SequentialThinking** - Multi-step reasoning with revision/branching
- **Workflow** - Orchestrated methodologies (think-tank, agentic-slc, etc.)
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

## Installation

### Homebrew (macOS/Linux) - Recommended
```bash
brew install msbrettorg/tap/maenifold
```

### .NET Tool
```bash
dotnet tool install --global Maenifold
```

### Direct Download
Download platform-specific binaries from the Assets section below.

## Usage

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
      "env": {"MAENIFOLD_ROOT": "~/maenifold"}
    }
  }
}
```

## Documentation

- [Complete Documentation](https://github.com/msbrettorg/maenifold/blob/main/docs/README.md)
- [Development Guide](https://github.com/msbrettorg/maenifold/blob/main/docs/DEVELOPMENT.md)
- [Distribution Guide](https://github.com/msbrettorg/maenifold/blob/main/DISTRIBUTION.md)

## Full Changelog

See [CHANGELOG.md](https://github.com/msbrettorg/maenifold/blob/main/CHANGELOG.md)

---

## Platform-Specific Download Instructions

### Linux (x64)
```bash
curl -LO https://github.com/msbrettorg/maenifold/releases/download/v1.0.0/maenifold-linux-x64.zip
unzip maenifold-linux-x64.zip
chmod +x maenifold
sudo mv maenifold /usr/local/bin/
```

### Linux (ARM64)
```bash
curl -LO https://github.com/msbrettorg/maenifold/releases/download/v1.0.0/maenifold-linux-arm64.zip
unzip maenifold-linux-arm64.zip
chmod +x maenifold
sudo mv maenifold /usr/local/bin/
```

### macOS (ARM64 - M1/M2/M3)
```bash
curl -LO https://github.com/msbrettorg/maenifold/releases/download/v1.0.0/maenifold-osx-arm64.zip
unzip maenifold-osx-arm64.zip
chmod +x maenifold
sudo mv maenifold /usr/local/bin/
```

### macOS (x64 - Intel)
```bash
curl -LO https://github.com/msbrettorg/maenifold/releases/download/v1.0.0/maenifold-osx-x64.zip
unzip maenifold-osx-x64.zip
chmod +x maenifold
sudo mv maenifold /usr/local/bin/
```

### Windows (x64)
```powershell
# Download maenifold-win-x64.zip from Assets below
# Extract the ZIP
# Add to PATH manually or run from extracted directory:
.\maenifold.exe --tool MemoryStatus --payload '{}'
```

## Release Assets

1. `maenifold-osx-arm64.zip`
2. `maenifold-osx-x64.zip`
3. `maenifold-linux-x64.zip`
4. `maenifold-linux-arm64.zip`
5. `maenifold-win-x64.zip`

## Release Checklist

Before creating the release:

- [ ] Version updated in `src/Maenifold.csproj`
- [ ] CHANGELOG.md updated with release notes
- [ ] All tests passing (`dotnet test`)
- [ ] PR merged to main
- [ ] Git tag created: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Git tag pushed: `git push origin v1.0.0`

After tag push (automated):

- [ ] GitHub Actions builds all platform binaries
- [ ] GitHub Release created automatically with all assets
- [ ] Homebrew formula updated via repository dispatch
