# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## ⚠️ CRITICAL: Git Workflow

**Branch Policy (STRICTLY ENFORCED):**
- **NEVER commit directly to `main` or `dev`**
- **ALL work happens on feature/sprint branches off `dev`**
- **ALL merges use squash merge strategy**
- Only `dev` branch (or dependabot) can create PRs to `main`

**Local Development Workflow:**
1. Create feature/sprint branch from `dev` (e.g., `sprint-20251103-fix-dotnet-version`)
2. Make changes and commit to feature branch
3. When feature complete, **merge feature branch to local `dev`** (squash merge)
4. Delete feature branch after merge

**Push to Origin Workflow:**
1. Push `dev` branch to origin (`git push origin dev`)
2. Create PR from `dev` to `main` on GitHub
3. **Monitor PR for CI/build status** - ensure all checks pass
4. After PR merged (squash merge by maintainers), pull latest from origin:
   ```bash
   git checkout main && git pull origin main
   git checkout dev && git pull origin dev
   ```

**Never:**
- Commit directly to `main` or `dev` branches
- Push feature branches to origin
- Create PRs from feature branches (always merge to local `dev` first)
- Use regular merge (always squash merge to maintain clean history)

**This is non-negotiable. Session will be terminated for violations.**

## ⚠️ CRITICAL: Testing Maenifold

**Testing Priority:**
1. **PRIMARY: Use MCP tools** - `mcp__maenifold__*` tools in this session
2. **FALLBACK ONLY: Use release binary CLI** - `/Users/brett/src/ma-collective/maenifold/src/bin/Release/net9.0/Maenifold`

**NEVER use `dotnet run` - session will be terminated without warning.**

After code changes, always run `dotnet build` so CLI and MCP use the same latest build.

```bash
# ✅ CORRECT: Use MCP tools (PRIMARY METHOD)
mcp__maenifold__write_memory(...)

# ✅ CORRECT: Use release binary CLI (FALLBACK ONLY - when MCP unavailable or wrong build)
/Users/brett/src/ma-collective/maenifold/src/bin/Release/net9.0/maenifold --tool WriteMemory --payload '{...}'

# ❌ WRONG: Using dotnet run (TERMINATES SESSION)
dotnet run --project src/Maenifold.csproj -- --tool WriteMemory
```

## Project Overview

**maenifold** is a persistent knowledge graph system built with C# that transforms ephemeral AI sessions into continuous collective intelligence. It uses `[[WikiLinks]]` in markdown files to create a graph database with 384-dimensional vector embeddings, enabling semantic search and knowledge exploration across AI agent sessions.

**Key Technologies:**
- .NET 9.0 (C#)
- SQLite with vector extensions for graph storage
- ONNX Runtime (ML.OnnxRuntime) for embeddings (all-MiniLM-L6-v2)
- Model Context Protocol (MCP) server implementation
- YamlDotNet, Markdig for markdown/YAML processing

## The Ma Protocol Philosophy

**maenifold follows the Ma (間) Protocol** - a philosophy that creates space for intelligence to emerge by resisting unnecessary features and abstractions.

### Core Principles

**1. NO FAKE AI** - Every decision we make removes a decision the LLM could make
- Don't add retry logic or fallback strategies
- Don't implement "smart" error recovery
- Don't make decisions about what the user "probably meant"
- Let errors propagate to the LLM with complete information

**2. NO UNNECESSARY ABSTRACTIONS** - Abstractions create cognitive load. Simplicity creates understanding.
- No interfaces for single implementations
- No factory patterns for simple object creation
- No dependency injection frameworks
- Direct calls over indirection

**3. NO FAKE TESTS** - Mocks create false confidence. Real tests find real bugs.
- Use real SQLite databases (in-memory for speed, but real)
- Use real file systems (test outputs in `test-outputs/`, not temp directories)
- No mocks, no stubs - test real behavior
- Keep test artifacts for debugging

**4. NO FAKE SECURITY** - This is a local personal knowledge system. The user is the admin.
- Use prepared statements for SQL (the only security measure needed)
- No path validation or sanitization
- No artificial limits on file sizes or resources
- Trust the user to manage their own system

**Critical Reading:**
- `/docs/MA_MANIFESTO.md` - The philosophy of productive emptiness
- `/docs/WHAT_WE_DONT_DO.md` - Features we've intentionally rejected
- `/docs/TESTING_PHILOSOPHY.md` - Why we don't mock
- `/docs/SECURITY_PHILOSOPHY.md` - Security for personal systems

**Remember:** The absence IS the feature. Every feature we don't add creates space for the LLM to work more effectively.

## Build and Development Commands

### Building the Project

```bash
# Build for development
dotnet build

# Build release version
dotnet publish src/Maenifold.csproj -c Release --self-contained -o bin

# Build all platforms (uses npm scripts)
npm run build:all
```

### Running Tests

```bash
# Run all tests
dotnet test

# Run specific test file
dotnet test --filter "FullyQualifiedName~Maenifold.Tests.GraphToolsTests"

# Run tests with verbose output
dotnet test --logger "console;verbosity=detailed"
```

### Running the Application

```bash
# Run in MCP server mode (production)
maenifold --mcp

# Run in CLI mode for direct tool invocation
maenifold --tool WriteMemory --payload '{"title":"Test","content":"Content with [[WikiLink]]"}'

# Run from source (development mode)
dotnet run --project src/Maenifold.csproj -- --mcp
```

## Architecture Overview

### Core Pattern: Tool-Based MCP Server

The application is structured as an MCP (Model Context Protocol) server with two operation modes:

1. **MCP Server Mode** (`--mcp`): Persistent server handling tool calls via stdio transport
2. **CLI Mode** (`--tool --payload`): Direct tool invocation for scripting/automation

All functionality is exposed through **MCP Tools** decorated with `[McpServerTool]` attributes, auto-discovered via reflection:

```csharp
builder.Services
    .AddMcpServer()
    .WithStdioServerTransport()
    .WithToolsFromAssembly();  // Scans for [McpServerTool] attributes
```

### Directory Structure

```
src/
├── Program.cs                    # Entry point, mode routing (MCP vs CLI)
├── Tools/                        # MCP tool implementations (40 files)
│   ├── GraphTools.cs            # Core graph operations (Sync, BuildContext, Visualize)
│   ├── MemoryTools.cs           # CRUD for memory:// files (Write/Read/Edit/Delete)
│   ├── MemorySearchTools.cs     # Hybrid search (semantic + full-text fusion)
│   ├── SequentialThinkingTools.cs # Structured thinking sessions
│   ├── WorkflowTools.cs         # Methodology orchestration
│   └── ...                      # Additional specialized tools
├── Utils/                        # Core utilities (14 files)
│   ├── Config.cs                # Configuration, paths, environment vars
│   ├── MarkdownIO.cs            # Markdown parsing, [[WikiLink]] extraction
│   ├── VectorTools.*.cs         # ONNX embedding generation
│   └── ...
└── assets/                       # Cognitive assets
    ├── workflows/               # 32 JSON workflow definitions
    ├── roles/                   # 16 role definitions (PM, Architect, etc.)
    ├── colors/                  # 7 thinking hat colors
    └── perspectives/            # 12 linguistic perspectives
```

### Key Architectural Concepts

#### 1. Memory System (`memory://` URI scheme)

All knowledge is stored as markdown files under `~/maenifold/memory/` with:
- YAML frontmatter (title, tags, timestamps)
- Markdown content with `[[WikiLinks]]` for concept connections
- URI format: `memory://path/to/file` (auto-generated from title)

**WikiLink Normalization:** `[[Machine Learning]]` → `machine-learning` (lowercase-with-hyphens)

#### 2. Graph Database (SQLite)

Located at `~/maenifold/memory.db`:
- `Concepts` table: Normalized concept names with vector embeddings
- `ConceptMentions` table: Concept-to-file relationships with edge weights
- Full-text search (FTS5) index for text-based search
- Vector similarity for semantic search

**Connection String:** Pooling is **disabled** (`Pooling=false`) to prevent lock contention in long-running MCP server.

#### 3. WikiLink-to-Graph Pipeline

1. User writes markdown with `[[concepts]]`
2. `MemoryTools.WriteMemory` saves to disk
3. `Sync()` extracts `[[WikiLinks]]` from all files
4. Concepts normalized and inserted into graph database
5. Vector embeddings generated via ONNX (384 dimensions)
6. `BuildContext()` traverses graph to discover relationships

#### 4. Hybrid Search Architecture

`SearchMemories` uses **Reciprocal Rank Fusion (RRF)** to combine:
- **Semantic Search**: ONNX embeddings + cosine similarity
- **Full-Text Search**: SQLite FTS5 tokenization + BM25 ranking
- **Fusion Parameter**: `k=60` (configurable in `MemorySearchTools.Fusion.cs`)

This prevents the "embedding similarity trap" where semantically similar but contextually wrong results dominate.

#### 5. SequentialThinking & Workflow Systems

- **SequentialThinking**: Test-time adaptive reasoning with revision, branching, and automatic `[[concept]]` extraction
- **Workflows**: 28 pre-defined methodologies (deductive reasoning, design thinking, SDLC, etc.)
- All sessions stored as markdown in `memory://thinking/sequential/` and `memory://thinking/workflows/`
- Workflows can embed SequentialThinking for deep analysis steps

#### 6. Asset Management

Cognitive assets (workflows, roles, colors, perspectives) are JSON files in `src/assets/`. On first run, `AssetManager.InitializeAssets()` copies them to `~/maenifold/assets/` for user customization.

## Critical Implementation Patterns

### Environment Variables

```bash
MAENIFOLD_ROOT          # Root directory (default: ~/maenifold)
MAENIFOLD_DATABASE_PATH # Database location (default: ~/maenifold/memory.db)
MAENIFOLD_TEST_MEMORY   # Test isolation path (testing only)
```

### Configuration Paths (Utils/Config.cs)

- `Config.MaenifoldRoot` → `~/maenifold/`
- `Config.MemoryPath` → `~/maenifold/memory/`
- `Config.DatabasePath` → `~/maenifold/memory.db`
- `Config.ThinkingPath` → `~/maenifold/memory/thinking/`

**Critical:** Always use `Config.EnsureDirectories()` before file operations.

### Tool Implementation Pattern

```csharp
[McpServerTool, Description("Clear description of what this tool does and when to use it")]
public static ResultType ToolName(
    [Description("Parameter purpose")] string param1,
    [Description("Optional parameter")] int param2 = defaultValue)
{
    // Validate inputs (basic type checking only - trust the user)
    if (string.IsNullOrWhiteSpace(param1))
        throw new ArgumentException("param1 required", nameof(param1));

    // Perform operation
    var result = DoWork(param1, param2);

    // Return structured result (auto-serialized to JSON)
    return result;
}
```

### Security: ALWAYS Use Prepared Statements

```csharp
// ✅ CORRECT: Prepared statements
var command = connection.CreateCommand();
command.CommandText = "INSERT INTO Concepts (Name, Embedding) VALUES (@name, @embedding)";
command.Parameters.AddWithValue("@name", conceptName);
command.Parameters.AddWithValue("@embedding", embeddingBytes);
command.ExecuteNonQuery();

// ❌ NEVER: String concatenation
var query = $"INSERT INTO Concepts (Name) VALUES ('{conceptName}')";
```

### Testing Pattern

```csharp
public class MyToolTests : IDisposable
{
    private readonly string _testRoot;

    public MyToolTests()
    {
        // Real test directory in test-outputs/ (NOT temp)
        _testRoot = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "..", "..",
            "test-outputs",
            "memory"
        );
        Config.SetRootOverride(_testRoot);
        Config.EnsureDirectories();
    }

    [Fact]
    public void ToolName_WithRealDatabase_ProducesExpectedResult()
    {
        // Use REAL SQLite, REAL file system - NO mocks
        var result = MyTool.DoSomething(input);
        Assert.NotNull(result);
    }

    public void Dispose()
    {
        // Keep test artifacts for debugging - cleanup is periodic
        if (Directory.Exists(_testRoot))
            Directory.Delete(_testRoot, true);
    }
}
```

## Development Philosophy in Practice

When working on maenifold, ask yourself:

**Before adding a feature:**
- Does this remove a decision the LLM could make? (NO FAKE AI)
- Does this add abstraction without clear benefit? (NO UNNECESSARY ABSTRACTIONS)
- Am I optimizing without measurement? (NO PREMATURE OPTIMIZATION)

**When writing code:**
- Could this be simpler?
- Am I trusting the user?
- Is this adding real value or just complexity?

**When testing:**
- Am I testing real behavior with real systems?
- Do I need this mock or can I use the real thing?

**When handling errors:**
- Should I catch this error or let it propagate?
- Am I hiding information from the LLM?

**The Ma Protocol litmus test:** If you're hesitating to add something because it feels like "feature creep" or "just in case" code, you're probably right. Don't add it.

## Common Development Tasks

### Adding a New Tool

1. Create method in appropriate `Tools/*.cs` file
2. Add `[McpServerTool]` and `[Description]` attributes
3. Implement logic using existing utilities (`MarkdownIO`, `Config`, etc.)
4. Add tests in `tests/Maenifold.Tests/` (use real systems, no mocks)
5. Run `dotnet test` to verify
6. Tool auto-discovered on next build (no registration needed)

### Debugging Search Results

1. Check FTS index: `sqlite3 ~/maenifold/memory.db "SELECT * FROM MemoriesFts WHERE MemoriesFts MATCH 'query'"`
2. Check vector similarity: Use `FindSimilarConcepts` tool
3. Verify fusion: Set breakpoints in `MemorySearchTools.Fusion.cs`

### Why Certain Architectural Decisions Were Made

**No dependency injection framework?**
- Ma Protocol: NO UNNECESSARY ABSTRACTIONS
- Simple static methods with clear dependencies are easier to understand

**Connection pooling disabled?**
- Ma Protocol: NO PREMATURE OPTIMIZATION
- Measured actual problem (lock contention), applied targeted fix

**No IMemoryRepository interface?**
- Ma Protocol: NO UNNECESSARY ABSTRACTIONS
- One implementation (SQLite), direct calls are clearer

**Test artifacts kept?**
- Ma Protocol: NO FAKE TESTS
- Real debugging requires real evidence

**No path validation?**
- Ma Protocol: NO FAKE SECURITY
- Users manage their own file systems; OS provides real boundaries

**Errors propagate?**
- Ma Protocol: NO FAKE AI
- LLM needs complete error information to make decisions

## Performance Notes

- **Vector Operations**: Embeddings cached in `Concepts` table to avoid recomputation
- **FTS Index**: Rebuilt during `Sync()` - can be slow for large corpora (>10k files)
- **Graph Traversal**: `BuildContext()` uses recursive CTEs for efficient multi-hop queries
- **Connection Pooling**: Disabled due to lock contention in long-running MCP server

## Reference Links

- **C# MCP SDK**: https://github.com/modelcontextprotocol/csharp-sdk
- **MCP Protocol Spec**: https://modelcontextprotocol.io
- **Project Issues**: https://github.com/msbrettorg/maenifold/issues
- **Ma Collective Docs**: `/docs/` (MA_MANIFESTO.md, WHAT_WE_DONT_DO.md, TESTING_PHILOSOPHY.md, SECURITY_PHILOSOPHY.md)
- https://github.com/modelcontextprotocol/csharp-sdk/tree/main/samples/EverythingServer
- https://modelcontextprotocol.github.io/csharp-sdk/api/ModelContextProtocol.html
- smoke test the MCP server directly - not via the cli, dotnet run or any other mechanism. Only the MCP protocol is acceptable.