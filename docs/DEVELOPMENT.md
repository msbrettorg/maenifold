# Development Guide

Development setup and contribution guidelines for maenifold.

## Prerequisites

- **.NET 9.0 SDK** - https://dotnet.microsoft.com/download
- **Node.js 18+** - https://nodejs.org/
- **Git** - https://git-scm.com/

Optional:
- **Visual Studio 2022**, **VS Code**, or **JetBrains Rider**
- **Claude Code**, **Continue**, or **Codex** for MCP testing

## Initial Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/ma-collective/maenifold.git
   cd maenifold
   ```

2. **Restore dependencies:**
   ```bash
   dotnet restore src/Maenifold.csproj
   ```

3. **Build the project:**
   ```bash
   dotnet build src/Maenifold.csproj -c Debug
   ```

4. **Run tests:**
   ```bash
   dotnet test
   ```

## Project Structure

```
maenifold/
├── src/                          # Main source code
│   ├── Tools/                    # MCP tool implementations
│   │   ├── MemoryTools.*.cs     # Memory operations (Read, Write, Edit, etc.)
│   │   ├── GraphTools.cs        # Graph operations (BuildContext, Visualize, etc.)
│   │   ├── SystemTools.cs       # System operations (GetConfig, GetHelp, etc.)
│   │   ├── ThinkingTools.cs     # Thinking operations (SequentialThinking, Workflow)
│   │   ├── MaintenanceTools.cs  # Maintenance operations (RepairConcepts, etc.)
│   │   └── ToolRegistry.cs      # Centralized tool dispatch
│   ├── Utils/                    # Utility classes
│   │   ├── MarkdownReader.cs    # Markdown file parsing
│   │   ├── MarkdownWriter.cs    # Markdown file writing
│   │   ├── PayloadReader.cs     # JSON payload extraction
│   │   └── VectorTools.cs       # Vector embeddings (ONNX)
│   ├── Program.cs                # Entry point (CLI/MCP modes)
│   └── assets/                   # Static assets
│       ├── models/              # ONNX models (all-MiniLM-L6-v2)
│       ├── usage/tools/         # Tool documentation markdown
│       └── workflows/           # Workflow YAML definitions (47 workflows)
├── tests/                        # NUnit tests
│   └── Maenifold.Tests/
├── docs/                         # Documentation
│   ├── README.md                # Architecture and philosophy
│   ├── DEVELOPMENT.md           # This file
│   ├── branding/                # Brand assets (SVG logos)
│   └── demo-artifacts/          # Hero demo artifacts
├── scripts/                      # NPM distribution scripts
│   ├── maenifold.js             # Platform detection wrapper
│   └── postinstall.js           # NPM post-install script
└── bin/                          # Build outputs (platform-specific)
```

## Development Workflow

### Building

**Debug build:**
```bash
dotnet build src/Maenifold.csproj -c Debug
```

**Release build:**
```bash
dotnet build src/Maenifold.csproj -c Release
```

**Platform-specific builds:**
```bash
npm run build:linux    # Linux x64
npm run build:osx      # macOS ARM64
npm run build:osx-x64  # macOS Intel
npm run build:win      # Windows x64
npm run build:all      # All platforms
```

### Testing

**Run all tests:**
```bash
dotnet test
```

**Run specific test:**
```bash
dotnet test --filter "FullyQualifiedName~MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"
```

**Run with coverage:**
```bash
dotnet test --collect:"XPlat Code Coverage"
```

### Running Locally

**CLI mode:**
```bash
src/bin/Debug/net9.0/Maenifold --tool MemoryStatus --payload '{}'
```

**MCP mode (stdio):**
```bash
src/bin/Debug/net9.0/Maenifold --mcp
```

Then send MCP JSON-RPC messages via stdin.

### Testing MCP Integration

**With Claude Code:**

Add to `~/.claude/config.json`:
```json
{
  "mcpServers": {
    "maenifold-dev": {
      "command": "/absolute/path/to/maenifold/src/bin/Debug/net9.0/Maenifold",
      "args": ["--mcp"],
      "env": {
        "maenifold_ROOT": "~/maenifold-test"
      }
    }
  }
}
```

**With Codex:**

Add to `~/.config/codex/config.toml`:
```toml
[mcp_servers.maenifold-dev]
type = "stdio"
command = "/absolute/path/to/maenifold/src/bin/Debug/net9.0/Maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600

[mcp_servers.maenifold-dev.env]
maenifold_ROOT = "~/maenifold-test"
```

## Code Style

Follow [.NET runtime coding guidelines](https://github.com/dotnet/runtime/blob/main/docs/coding-guidelines/coding-style.md):

- **Allman braces** - each brace on new line
- **Four spaces** for indentation (no tabs)
- **Private fields**: `_camelCase` with underscore prefix
- **Public members**: `PascalCase`
- **Use `nameof(...)` instead of string literals**
- **Language keywords** over BCL types (`int` not `Int32`)

### Example

```csharp
public static string WriteMemory(
    string title,
    string content,
    string? folder = null)
{
    var _basePath = GetBasePath();

    if (string.IsNullOrWhiteSpace(title))
    {
        return "ERROR: Title cannot be empty";
    }

    var sluggedTitle = MarkdownIO.Slugify(title);
    // ... implementation
}
```

## Adding New Tools

1. **Add method to appropriate class in `src/Tools/`:**
   ```csharp
   [McpServerTool]
   [Description("Brief description of what this tool does")]
   public static string MyNewTool(
       [Description("Parameter description")] string param)
   {
       // Implementation
       return "Result in markdown format";
   }
   ```

2. **Register in `ToolRegistry.cs`:**
   ```csharp
   add(new ToolDescriptor("MyNewTool", payload =>
   {
       var param = PayloadReader.GetString(payload, "param");
       return MyTools.MyNewTool(param);
   }, new[] { "mynewtool", "mnt" }, "Brief description"));
   ```

3. **Add documentation in `src/assets/usage/tools/mynewtool.md`:**
   ```markdown
   # MyNewTool

   Brief description.

   ## Parameters
   - `param` (string, required): Parameter description

   ## Returns
   Result description

   ## Example
   ```json
   {
     "tool": "MyNewTool",
     "param": "value"
   }
   ```
   ```

4. **Add tests in `tests/Maenifold.Tests/`**

5. **Run tests and build:**
   ```bash
   dotnet test
   dotnet build -c Release
   ```

## Debugging

### VS Code

Launch configuration (`.vscode/launch.json`):
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "CLI Mode",
      "type": "coreclr",
      "request": "launch",
      "preLaunchTask": "build",
      "program": "${workspaceFolder}/src/bin/Debug/net9.0/Maenifold.dll",
      "args": ["--tool", "MemoryStatus", "--payload", "{}"],
      "cwd": "${workspaceFolder}",
      "stopAtEntry": false,
      "console": "internalConsole"
    }
  ]
}
```

### Visual Studio / Rider

Set command line arguments in project properties:
```
--tool MemoryStatus --payload {}
```

## Contributing

1. **Fork the repository**
2. **Create a feature branch:** `git checkout -b feature/my-feature`
3. **Make changes and add tests**
4. **Run tests:** `dotnet test`
5. **Commit changes:** `git commit -m "feat: add my feature"`
6. **Push to fork:** `git push origin feature/my-feature`
7. **Create Pull Request**

### Commit Message Format

Follow conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation only
- `chore:` Maintenance tasks
- `refactor:` Code restructuring
- `test:` Adding tests
- `perf:` Performance improvements

## Build Enforcement

`Directory.Build.targets` enforces code quality:
- **Local development:** Warnings for violations
- **CI (when `CI=true`):** Errors for violations

Violations checked:
- Root directory must not contain: `*.py`, `poetry.lock`, `pyproject.toml`, `requirements.txt`
- Ensures .NET-first codebase

## Environment Variables

- `maenifold_ROOT`: Base path for memory storage (default: `~/maenifold`)
- `CI`: Set to `true` in CI to enforce stricter build rules

## Resources

- **Research Papers:**
  - [Graph of Thoughts (AAAI 2024)](https://arxiv.org/abs/2308.09687)
  - [Graph-RAG (Microsoft Research)](https://www.microsoft.com/en-us/research/blog/graphrag-unlocking-llm-discovery-on-narrative-private-data/)

- **Related Projects:**
  - [Microsoft Agent Framework](https://github.com/microsoft/agent-framework)
  - [MCP Protocol](https://modelcontextprotocol.io)

- **Community:**
  - GitHub Issues: https://github.com/ma-collective/maenifold/issues
  - Discussions: https://github.com/ma-collective/maenifold/discussions
