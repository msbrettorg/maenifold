# Maenifold Skill for Claude Code

This skill provides Claude Code with access to maenifold's knowledge graph and reasoning infrastructure. It enables persistent memory, graph-based context building, structured workflows, and multi-session thinking continuity.

## Overview

The maenifold skill exposes 25+ MCP tools that transform ephemeral AI sessions into continuous collective intelligence:

- **Memory Tools**: Write, read, search, edit, and organize knowledge files with `[[WikiLink]]` graph integration
- **Graph Tools**: Build context, find similar concepts, visualize relationships, and sync the knowledge graph
- **Reasoning Tools**: Sequential thinking with branching, structured workflows, and assumption tracking
- **System Tools**: Adopt roles/perspectives, monitor activity, and manage configuration

Every `[[WikiLink]]` you create becomes a node in the graph. Knowledge compounds across sessions instead of resetting.

## Prerequisites

- **Claude Code** (CLI or VS Code extension)
- **maenifold binary** installed and accessible in PATH (or specify full path in configuration)
- **Write permissions** to the maenifold data directory (default: `~/maenifold`)

## Installation

### Method 1: Plugin Installation (Recommended)

Install the maenifold plugin directly through Claude Code:

```bash
claude plugin install msbrettorg/maenifold
```

This automatically:
- Downloads the maenifold plugin
- Configures MCP server settings
- Registers the skill for automatic invocation

### Method 2: Manual Installation

If you prefer manual setup or need custom configuration:

1. Clone or download the plugin files to your `.claude/plugins` directory
2. Configure the MCP server in your project's `.mcp.json` (see MCP Configuration below)
3. Copy skill files to your project or reference them from the plugin cache

## Binary Installation

The maenifold MCP server requires the maenifold binary. Choose your installation method:

### Homebrew (macOS/Linux)

```bash
brew install msbrettorg/tap/maenifold
```

This installs the binary and adds it to your PATH automatically.

### Windows MSI Installer

1. Download the latest `.msi` file from [GitHub Releases](https://github.com/msbrettorg/maenifold/releases/latest)
2. Run the installer
3. The binary installs to `C:\Program Files\Maenifold\` and is added to PATH automatically

After installation, verify with:
```powershell
maenifold --version
```

### Build from Source

Requires .NET 9.0 SDK.

```bash
git clone https://github.com/msbrettorg/maenifold.git
cd maenifold
dotnet build -c Release
```

The binary will be at `src/Maenifold/bin/Release/net9.0/maenifold` (or `maenifold.exe` on Windows).

## MCP Configuration

The maenifold skill requires the MCP server to be configured. Configuration varies by client.

### Claude Code

Create or update `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "type": "stdio"
    }
  }
}
```

If maenifold is not in PATH, specify the full path:

```json
{
  "mcpServers": {
    "maenifold": {
      "command": "/usr/local/bin/maenifold",
      "args": ["--mcp"],
      "type": "stdio"
    }
  }
}
```

### Claude Desktop

Edit the configuration file at:
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

Add the maenifold server:

```json
{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"]
    }
  }
}
```

On Windows with default MSI installation:

```json
{
  "mcpServers": {
    "maenifold": {
      "command": "C:\\Program Files\\Maenifold\\maenifold.exe",
      "args": ["--mcp"]
    }
  }
}
```

### Codex CLI

Edit `~/.codex/config.toml`:

```toml
[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
```

With custom data directory:

```toml
[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
env = { MAENIFOLD_ROOT = "~/my-knowledge-base" }
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MAENIFOLD_ROOT` | Root directory for memory files and database | `~/maenifold` |

Set the environment variable to use a custom location:

```bash
# Bash/Zsh
export MAENIFOLD_ROOT=~/my-knowledge-base

# PowerShell
$env:MAENIFOLD_ROOT = "$HOME\my-knowledge-base"
```

Or specify it in the MCP configuration:

```json
{
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "type": "stdio",
      "env": {
        "MAENIFOLD_ROOT": "~/my-knowledge-base"
      }
    }
  }
}
```

## Verification

After installation, verify the setup:

### 1. Check Binary Installation

```bash
maenifold --version
```

Expected output: Version number (e.g., `1.0.3`)

### 2. Test MCP Server

```bash
maenifold --mcp
```

The server should start and wait for JSON-RPC input. Press Ctrl+C to exit.

### 3. Test from Claude Code

Start Claude Code and try:

```
Write a test memory about [[installation]] verification
```

If successful, you should see confirmation that the memory was written.

### 4. Verify Skill Loading

In Claude Code, the skill should load automatically when you mention:
- "write memory", "read memory", "search memories"
- "build context", "find similar concepts"
- "sequential thinking", "run workflow"
- `[[WikiLinks]]`, `memory://`, or maenifold tools

## Troubleshooting

### "Command not found" Error

- **Cause**: maenifold binary not in PATH
- **Solution**: Either add the installation directory to PATH or use the full path in MCP configuration

### "Permission denied" Error

- **Cause**: Cannot write to MAENIFOLD_ROOT directory
- **Solution**: Ensure the directory exists and you have write permissions:
  ```bash
  mkdir -p ~/maenifold
  chmod 755 ~/maenifold
  ```

### MCP Server Timeout

- **Cause**: Server taking too long to start
- **Solution**: Increase timeout in configuration:
  ```json
  {
    "mcpServers": {
      "maenifold": {
        "command": "maenifold",
        "args": ["--mcp"],
        "type": "stdio",
        "timeout": 120000
      }
    }
  }
  ```

### Database Lock Error

- **Cause**: Multiple processes accessing the same database
- **Solution**: Ensure only one MCP server instance is running per MAENIFOLD_ROOT. The system supports concurrent agents, but each must use the same server instance.

### Skill Not Loading

- **Cause**: Skill files not found or plugin not installed
- **Solution**:
  1. Run `claude plugin list` to verify installation
  2. Reinstall with `claude plugin install msbrettorg/maenifold`

### Empty Search Results

- **Cause**: Graph not synced after adding memories
- **Solution**: Run the sync tool:
  ```
  Sync the knowledge graph
  ```

## Client Comparison

| Feature | Claude Code | Claude Desktop | Codex CLI |
|---------|-------------|----------------|-----------|
| Config Format | JSON (`.mcp.json`) | JSON | TOML |
| Config Location | Project root | App Support folder | `~/.codex/` |
| Skill Support | Yes | No | Partial |
| Plugin Install | `claude plugin install` | Manual only | Manual only |
| Env Variables | In config or shell | In config | In config |
| Timeout Config | `timeout` field | Not configurable | `startup_timeout_sec`, `tool_timeout_sec` |

## Related Documentation

- [Main README](https://github.com/msbrettorg/maenifold/blob/main/README.md) - Project overview and quick start
- [Full Documentation](https://github.com/msbrettorg/maenifold/blob/main/docs/README.md) - Architecture and examples
- [GitHub Releases](https://github.com/msbrettorg/maenifold/releases) - Download binaries
- [SKILL.md](./SKILL.md) - Skill usage and tool reference

## Tool Reference

For detailed tool documentation, see the `references/` directory:

| Category | Tools |
|----------|-------|
| Memory | `writememory`, `readmemory`, `searchmemories`, `editmemory`, `deletememory`, `movememory`, `listmemories` |
| Graph | `buildcontext`, `findsimilarconcepts`, `visualize`, `sync`, `extractconceptsfromfile` |
| Repair | `analyzeconceptcorruption`, `repairconcepts` |
| Reasoning | `workflow`, `sequentialthinking`, `assumptionledger` |
| System | `adopt`, `getconfig`, `gethelp`, `memorystatus`, `recentactivity`, `updateassets` |
