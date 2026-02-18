# Maenifold Skill for Claude Code

<!-- NOTE: Installation content is duplicated in skills/maenifold/references/README.md (canonical for AI agents). Keep both in sync. -->

This skill provides Claude Code with access to maenifold's knowledge graph and reasoning infrastructure. It enables persistent memory, graph-based context building, structured workflows, and multi-session thinking continuity.

## Overview

The maenifold skill exposes 25+ MCP tools that transform ephemeral AI sessions into continuous collective intelligence:

- **Memory Tools**: Write, read, search, edit, and organize knowledge files with `[[WikiLink]]` graph integration
- **Graph Tools**: Build context, find similar concepts, visualize relationships, and sync the knowledge graph
- **Community Detection**: Louvain algorithm identifies reasoning domain clusters during sync
- **Reasoning Tools**: Sequential thinking with branching, structured workflows, and assumption tracking
- **System Tools**: Adopt roles/perspectives, monitor activity, and manage configuration

Every `[[WikiLink]]` you create becomes a node in the graph. Knowledge compounds across sessions instead of resetting.

## Hooks

Three hooks integrate maenifold's knowledge graph into the Claude Code lifecycle:

| Hook | Event | What It Does |
|------|-------|-------------|
| **SessionStart** | Session begins | Queries community index for graph-of-thought priming — concepts grouped by Louvain-detected reasoning domains with thread metadata |
| **PreToolUse** (Task) | Task tool invoked | Same graph priming injected into subagent prompts as `## Graph of Thought (auto-injected)` section, enabling concept-as-protocol |
| **SubagentStop** | Subagent finishing | ConfessionReport gating — blocks stop unless transcript contains a ConfessionReport |

All hooks are implemented in a single script (`scripts/hooks.sh`) with 3 modes: `session_start`, `task_augment`, `subagent_stop`.

## Cognitive Architecture

Maenifold operates as a **6-layer composition architecture** where higher layers invoke lower layers. Complexity emerges from composition, not bloated tools.

### 6-Layer Stack

```mermaid
graph TB
    subgraph "Layer 6: Orchestration"
        Orchestration[Workflow<br/>Multi-step processes<br/>Nested workflows]
    end

    subgraph "Layer 5: Reasoning"
        Reasoning[Sequential Thinking<br/>Branching, revision<br/>Multi-session persistence]
    end

    subgraph "Layer 4: Persona"
        Persona[Adopt<br/>Roles, colors, perspectives<br/>Conditioned reasoning]
    end

    subgraph "Layer 3: Session"
        Session[Recent Activity<br/>Assumption Ledger<br/>State tracking]
    end

    subgraph "Layer 2: Memory + Graph"
        Memory[Write/Read/Search/Edit<br/>BuildContext, FindSimilar<br/>Persist & Query]
    end

    subgraph "Layer 1: Concepts"
        Concepts["WikiLinks<br/>Atomic units<br/>Graph nodes"]
    end

    Orchestration -->|invokes| Reasoning
    Reasoning -->|conditions| Persona
    Persona -->|tracks| Session
    Session -->|queries| Memory
    Memory -->|built from| Concepts

    style Orchestration fill:#0969DA
    style Reasoning fill:#0969DA
    style Persona fill:#0969DA
    style Session fill:#0969DA
    style Memory fill:#0969DA
    style Concepts fill:#0969DA
```

**Layer 1: Concepts** - Every `[[WikiLink]]` becomes a graph node. Foundation for all knowledge.

**Layer 2: Memory + Graph** - Tools persist (`writememory`) and query (`searchmemories`, `buildcontext`, `findsimilarconcepts`) knowledge.

**Layer 3: Session** - Track state across interactions (`recentactivity`, `assumptionledger`).

**Layer 4: Persona** - Condition reasoning through roles/colors/perspectives (`adopt`).

**Layer 5: Reasoning** - Enable revision, branching, multi-day persistence (`sequentialthinking`).

**Layer 6: Orchestration** - Compose all layers; workflows can nest workflows (`workflow`).

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

This automatically (if the plugin is available on the marketplace):
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

### GitHub Releases (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/msbrettorg/maenifold/releases/latest).

#### macOS

1. Download the appropriate archive:
   - **Apple Silicon (M1/M2/M3)**: `maenifold-osx-arm64.tar.gz`
   - **Intel**: `maenifold-osx-x64.tar.gz`

2. Extract to `~/maenifold/bin`:
   ```bash
   # Download (replace URL with correct architecture)
   curl -L -o maenifold.tar.gz https://github.com/msbrettorg/maenifold/releases/latest/download/maenifold-osx-arm64.tar.gz

   # Extract to bin directory
   mkdir -p ~/maenifold/bin
   tar -xzf maenifold.tar.gz -C ~/maenifold/bin --strip-components=1
   ```

3. Add to PATH:
   ```bash
   # Add to ~/.zshrc or ~/.bash_profile
   echo 'export PATH="$HOME/maenifold/bin:$PATH"' >> ~/.zshrc

   # Reload shell configuration
   source ~/.zshrc
   ```

4. Verify installation:
   ```bash
   maenifold --tool GetConfig --payload '{}'
   ```

#### Linux

1. Download `maenifold-linux-x64.tar.gz`

2. Extract to `~/maenifold/bin`:
   ```bash
   # Download
   curl -L -o maenifold.tar.gz https://github.com/msbrettorg/maenifold/releases/latest/download/maenifold-linux-x64.tar.gz

   # Extract to bin directory
   mkdir -p ~/maenifold/bin
   tar -xzf maenifold.tar.gz -C ~/maenifold/bin --strip-components=1
   ```

3. Add to PATH:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   echo 'export PATH="$HOME/maenifold/bin:$PATH"' >> ~/.bashrc

   # Reload shell configuration
   source ~/.bashrc
   ```

4. Verify installation:
   ```bash
   maenifold --tool GetConfig --payload '{}'
   ```

#### Windows

1. Download `maenifold-win-x64.zip`

2. Extract to `%USERPROFILE%\maenifold\bin`:
   ```powershell
   # Download (adjust path to your Downloads folder)
   $downloadPath = "$env:USERPROFILE\Downloads\maenifold-win-x64.zip"

   # Create directory and extract
   New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\maenifold\bin"
   Expand-Archive -Path $downloadPath -DestinationPath "$env:USERPROFILE\maenifold\bin" -Force
   ```

3. Add to PATH:
   ```powershell
   # Add to user PATH
   $path = [Environment]::GetEnvironmentVariable("PATH", "User")
   $newPath = "$env:USERPROFILE\maenifold\bin"
   if ($path -notlike "*$newPath*") {
       [Environment]::SetEnvironmentVariable("PATH", "$path;$newPath", "User")
   }

   # Restart terminal for PATH change to take effect
   ```

4. Verify installation:
   ```powershell
   maenifold --tool GetConfig --payload '{}'
   ```

### Homebrew (Alternative for macOS/Linux)

If you prefer using Homebrew package manager:

```bash
brew install msbrettorg/tap/maenifold
```

This installs the binary and adds it to your PATH automatically. Note that the default installation location differs from the recommended `~/maenifold` directory.

### Build from Source

Requires .NET 9.0 SDK.

```bash
git clone https://github.com/msbrettorg/maenifold.git
cd maenifold
dotnet build src/Maenifold.csproj -c Release
```

The binary will be at `src/bin/Release/net9.0/maenifold` (or `maenifold.exe` on Windows). Copy to `~/maenifold/bin` and add to PATH as shown in the platform-specific instructions above:

```bash
# macOS/Linux
mkdir -p ~/maenifold/bin
cp src/bin/Release/net9.0/maenifold ~/maenifold/bin/
```

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

If maenifold is not in PATH, specify the full path to your installation:

**macOS/Linux**:
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "/Users/<username>/maenifold/bin/maenifold",
      "args": ["--mcp"],
      "type": "stdio"
    }
  }
}
```

**Windows**:
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "C:\\Users\\<username>\\maenifold\\bin\\maenifold.exe",
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

If maenifold is not added to PATH, use the full path:

**macOS/Linux**:
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "/Users/<username>/maenifold/bin/maenifold",
      "args": ["--mcp"]
    }
  }
}
```

**Windows**:
```json
{
  "mcpServers": {
    "maenifold": {
      "command": "C:\\Users\\<username>\\maenifold\\bin\\maenifold.exe",
      "args": ["--mcp"]
    }
  }
}
```

Replace `<username>` with your actual username, or use environment variable expansion if your client supports it.

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
maenifold --tool GetConfig --payload '{}'
```

Expected output: JSON configuration including `maenifoldRoot` path and version info

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

| Feature | Claude Code | Claude Desktop | Codex CLI (experimental, unverified) |
|---------|-------------|----------------|---------------------------------------|
| Config Format | JSON (`.mcp.json`) | JSON | TOML (unverified) |
| Config Location | Project root | App Support folder | `~/.codex/` (unverified) |
| Skill Support | Yes | No | Unknown |
| Plugin Install | `claude plugin install` | Manual only | Manual only |
| Env Variables | In config or shell | In config | In config (unverified) |
| Timeout Config | `timeout` field | Not configurable | `startup_timeout_sec`, `tool_timeout_sec` (unverified) |

## Related Documentation

- [Main README](https://github.com/msbrettorg/maenifold/blob/main/README.md) - Project overview and quick start
- [Full Documentation](https://github.com/msbrettorg/maenifold/blob/main/docs/README.md) - Architecture and examples
- [GitHub Releases](https://github.com/msbrettorg/maenifold/releases) - Download binaries
- [SKILL.md](./skills/maenifold/SKILL.md) - Skill usage and tool reference

## Tool Reference

For detailed tool documentation, see the [skills/maenifold/usage/](./skills/maenifold/usage/) directory:

| Category | Tools |
|----------|-------|
| Memory | `writememory`, `readmemory`, `searchmemories`, `editmemory`, `deletememory`, `movememory`, `listmemories` |
| Graph | `buildcontext`, `findsimilarconcepts`, `visualize`, `sync`, `extractconceptsfromfile` |
| Repair | `analyzeconceptcorruption`, `repairconcepts` |
| Reasoning | `workflow`, `sequentialthinking`, `assumptionledger` |
| System | `adopt`, `getconfig`, `gethelp`, `listassets`, `memorystatus`, `readmcpresource`, `recentactivity`, `updateassets` |
