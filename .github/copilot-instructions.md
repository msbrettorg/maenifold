# Copilot Instructions for maenifold

These instructions are for AI coding agents (including GitHub Copilot) working in this repo.

## 1. Always Re-read the Ground Truth
- Never work from memory; **re-open key docs on every session**:
  - `README.md` and `docs/README.md` for the big-picture architecture and cognitive stack.
  - `CONTRIBUTING.md`, `docs/WHAT_WE_DONT_DO.md`, `docs/TESTING_PHILOSOPHY.md`, `docs/SECURITY_PHILOSOPHY.md` for project philosophy.
- Treat these design principles as **inviolable, not bugs**:
  - **NO FAKE AI** – do not add retries, background “auto-fix”, or hidden fallbacks. Let errors propagate clearly.
  - **NO UNNECESSARY ABSTRACTIONS** – no DI containers, no interfaces for single implementations, no layering for its own sake.
  - **NO FAKE TESTS** – no mocks/stubs in new tests; always hit real SQLite and the real filesystem.
  - **NO FAKE SECURITY** – do not add path guards, quotas, or sandboxing; the only “security” is parameterized SQL.

## 2. Mental Model of the System
- This is a **.NET 9 C# MCP server + CLI** that turns markdown + WikiLinks into a **SQLite-backed knowledge graph**.
- Two entry modes (see `src/Program.cs`):
  - `maenifold --mcp` – stdio JSON-RPC server (tools annotated with `[McpServerTool]`).
  - `maenifold --tool <Name> --payload '<json>'` – CLI tool runner wired via `ToolRegistry`.
- Major namespaces (under `src/Tools/`):
  - `MemoryTools` – file CRUD and URI handling (`memory://...`).
  - `GraphTools` – SQLite graph + FTS5 sync/visualization.
  - `MemorySearchTools` – full-text, vector, and hybrid RRF search.
  - `SequentialThinkingTools` – long-running reasoning sessions.
  - `WorkflowTools` – YAML workflows + orchestration.
  - `MaintenanceTools` – concept repair and corruption analysis.
  - `SystemTools` – config, diagnostics, status.
- Core infrastructure lives in `src/Utils/` (config, markdown IO, vectors, `GraphDatabase`). **Do not introduce new infra layers; extend these directly.**

## 3. Project-specific Conventions
- **WikiLinks are the backbone**:
  - Always use `[[concept-name]]` (double brackets, normalized to lowercase-with-hyphens, usually singular).
  - Every new memory write/edit MUST include at least one WikiLink.
  - Never mechanically merge class names (e.g. `VectorTools`) with generic concepts (e.g. `[[tool]]`). Use `MaintenanceTools.AnalyzeConceptCorruption` first, then `RepairConcepts` with `dryRun: true`.
- **Memory URIs**:
  - Logical form: `memory://folder/subfolder/name` → `~/maenifold/memory/folder/subfolder/name.md`.
  - Keep URIs stable; when moving files, use the `MoveMemory` tool rather than ad‑hoc file moves.
- **File format**:
  - Always include YAML frontmatter with `checksum`. Use existing helpers instead of hand-writing checksums.
  - Content is plain markdown; keep it Obsidian-friendly.
- **Tools**:
  - Tools are **static methods** with `[McpServerTool]` attributes and are also registered in `ToolRegistry` with a `ToolDescriptor`.
  - When adding a tool, update both the partial class and `ToolRegistry`, and add a usage doc under `src/assets/usage/tools/`.

## 4. Working Effectively in This Codebase
- **Before changing behavior**, locate the existing pattern:
  - Search in `src/Tools/` for an analogous tool (e.g., how `SearchMemories`, `WriteMemory`, or `BuildContext` is implemented).
  - For graph/DB changes, read `src/Utils/GraphDatabase.cs` and follow the existing schema + trigger patterns.
- **When adding a feature**:
  - Prefer small, explicit methods in the relevant partial class instead of new layers.
  - Thread config via `Config` (in `src/Utils/Config.cs`); do not introduce new global state or hard-coded paths.
  - If the feature interacts with the graph or memory, ensure `Sync` behavior remains predictable and explicit.
- **When adding tests**:
  - Follow `tests/Maenifold.Tests/*` patterns, especially `MemoryToolsTests`.
  - Use `Config.TestMemoryPath` and real SQLite DBs; do not mock the DB or filesystem.
  - Use the `Run ma-core tests` VS Code task (Release, `--no-build`) as the default test workflow.

## 5. Integration & External Surfaces
- **MCP clients**: The public surface is tools exposed via MCP; keep tool signatures simple, JSON-friendly, and stable.
- **CLI**: Assume scripts and other agents call `--tool` with a JSON payload; avoid breaking payload shapes.
- **Docs as contract**:
  - `docs/integrations/claude-code/README.md` and other integration docs describe expected tool behavior—treat them as part of the API.
  - When you change a tool that appears in docs, update the relevant doc in the same PR.

## 6. Safe Change Patterns
- For **graph/knowledge changes**:
  - Run `Sync` tools as described in docs after structural changes.
  - Use `AnalyzeConceptCorruption` + `RepairConcepts` **with `dryRun: true` first**, and read the diff-like output.
- For **memory semantics** (write/move/edit):
  - Study the bug found in the demo around extension loss in move operations (`docs/demo-artifacts/part1-pm-lite/*`) and avoid re‑introducing similar issues.
  - Preserve checksums and frontmatter invariants when editing.

## 7. When in Doubt
- Re-read `CONTRIBUTING.md` and `docs/MA_MANIFESTO.md` to realign with the Ma (間) Protocol.
- Prefer small, explicit code over “clever” abstractions.
- Surface full error information to the caller; let the orchestrating agent decide how to react.
# Copilot Instructions for maenifold

IMPORTANT: NEVER INFER CURRENT STATE. ALWAYS QUERY FOR THE ABSOLUTE LATEST INFORMATION. CHANGES HAPPEN OUT OF BAND FREQUENTLY.

## Overview

**maenifold** is a Model Context Protocol (MCP) server built in C# (.NET 9) that provides persistent knowledge graphs with WikiLinks for AI agents. It enables test-time reasoning through memory, graph navigation, and structured thinking workflows. The core philosophy is the **Ma (間) Protocol** - creating space for AI intelligence to emerge by resisting unnecessary abstractions.

## Architecture

### Dual-Mode Entry Point
- **MCP Server Mode**: `maenifold --mcp` (stdio-based JSON-RPC)
- **CLI Mode**: `maenifold --tool <name> --payload '<json>'`
- Entry point: `src/Program.cs` - minimal bootstrap, delegates to tool implementations

### Tool Organization (src/Tools/)
Tools are organized by domain with partial classes for logical separation:
- **MemoryTools** (partial): `Write.cs`, `Operations.cs`, `Helpers.cs` - memory file CRUD
- **GraphTools**: Sync, BuildContext, Visualize - SQLite-backed knowledge graph
- **MemorySearchTools** (partial): `Text.cs`, `Vector.cs`, `Fusion.cs` - hybrid search
- **SequentialThinkingTools**: Structured reasoning sessions with [[concepts]]
- **WorkflowTools** (partial): `Core.cs`, `Management.cs`, `Runner.cs` - YAML workflows
- **MaintenanceTools**: RepairConcepts, AnalyzeConceptCorruption
- **SystemTools**: GetConfig, GetHelp, MemoryStatus
- **ToolRegistry**: Central dispatcher mapping tool names to implementations

**Important**: Tools are registered via `[McpServerTool]` attributes for MCP mode, but CLI mode uses `ToolRegistry` for direct invocation.

### Core Infrastructure (src/Utils/)
- **Config**: Environment-based configuration (`MAENIFOLD_ROOT`, defaults to `~/maenifold`)
- **MarkdownReader/Writer**: Frontmatter + content + checksum handling
- **VectorTools**: ONNX-based embeddings (all-MiniLM-L6-v2) for semantic search
- **GraphDatabase**: SQLite schema with FTS5, WAL mode, foreign keys

### Knowledge Graph Schema
SQLite tables (see `GraphDatabase.cs`):
- `concepts`: concept_name (PK), occurrence_count, first_seen
- `concept_mentions`: (concept_name, source_file) pairs with mention_count
- `concept_graph`: (concept_a, concept_b) co-occurrence edges
- `file_content` + `file_search` (FTS5): full-text search index
- `concept_embeddings`: ONNX vector embeddings for semantic similarity

Triggers automatically sync FTS5 with file_content on INSERT/UPDATE/DELETE.

### Ma (間) Protocol - Critical Design Decisions

**NO FAKE AI**: Do not add retry logic, fallback strategies, or "smart" error recovery. Errors must propagate to the LLM with complete information.

**NO UNNECESSARY ABSTRACTIONS**: 
- No interfaces for single implementations
- No dependency injection frameworks (direct instantiation)
- Partial classes instead of inheritance hierarchies

**NO FAKE TESTS**: 
- Real SQLite databases in tests (not mocks)
- Real file systems (use `Config.TestMemoryPath`)
- See `tests/Maenifold.Tests/MemoryToolsTests.cs` for examples

**NO FAKE SECURITY**:
- No path validation or resource limits (trust the user)
- Only real security: prepared statements for SQL
- This is a local personal knowledge system

These are **intentional design principles**, not bugs to fix. See `CONTRIBUTING.md` for details.

## Development Workflow

### Building
```bash
# Debug build
dotnet build src/Maenifold.csproj -c Debug

# Release build (all platforms)
npm run build:all  # Creates bin/linux-x64, bin/osx-arm64, etc.
```

### Testing
```bash
# All tests
dotnet test

# Specific test
dotnet test --filter "FullyQualifiedName~MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"

# From VS Code
Run task: "Run ma-core tests"
```

**Test Philosophy**: Use real databases and file systems. `Config.TestMemoryPath` provides isolated test directories. No mocks, no stubs.

### Local MCP Testing

**Claude Code** (`~/.claude/config.json`):
```json
{
  "mcpServers": {
    "maenifold-dev": {
      "command": "/absolute/path/to/maenifold/src/bin/Debug/net9.0/maenifold",
      "args": ["--mcp"],
      "env": { "MAENIFOLD_ROOT": "~/maenifold-test" }
    }
  }
}
```

**Codex** (`~/.config/codex/config.toml`):
```toml
[mcp_servers.maenifold-dev]
type = "stdio"
command = "/absolute/path/to/maenifold/src/bin/Debug/net9.0/maenifold"
args = ["--mcp"]
[mcp_servers.maenifold-dev.env]
MAENIFOLD_ROOT = "~/maenifold-test"
```

## Key Conventions

### WikiLinks & Concepts
- `[[concept-name]]` creates bidirectional graph edges
- Always use double brackets, never single
- Normalized to lowercase-with-hyphens: `[[Machine Learning]]` → `machine-learning`
- Use singular forms: `[[tool]]` not `[[tools]]` (unless referring to collection)
- Every memory write/edit MUST include at least one `[[concept]]`

### Memory URIs
Format: `memory://folder/subfolder/filename` (extension optional)
- Resolves to: `~/maenifold/memory/folder/subfolder/filename.md`
- Example: `memory://projects/maenifold/activeContext` → `~/maenifold/memory/projects/maenifold/activeContext.md`

### Frontmatter Structure
```yaml
---
title: "The Title"
tags: [tag1, tag2]
created: 2025-11-20T12:00:00Z
updated: 2025-11-20T14:30:00Z
checksum: abc123def456
---

# The Title

Content with [[WikiLinks]]...
```

Checksum prevents stale edits (computed via SHA256 of full file content).

### Environment Variables
- `MAENIFOLD_ROOT`: Memory root directory (default: `~/maenifold`)
- `MAENIFOLD_AUTO_SYNC`: Enable incremental file watcher (default: true)
- `MAENIFOLD_DEBOUNCE_MS`: File change debounce (default: 150ms)
- `MAENIFOLD_SEARCH_LIMIT`: Default search results (default: 10)

See `src/Utils/Config.cs` for complete list.

## Common Patterns

### Adding a New Tool

1. Add method to appropriate `Tools/*.cs` class:
```csharp
[McpServerTool]
[Description("Brief description")]
public static string MyNewTool(
    [Description("Parameter description")] string param)
{
    // Implementation
    return "Success message";
}
```

2. Add descriptor to `ToolRegistry.cs`:
```csharp
add(new ToolDescriptor("MyNewTool", payload =>
{
    var param = PayloadReader.GetString(payload, "param");
    return MyNewTool(param);
}, new[] { "mynew" }, "Brief description"));
```

3. Create help file: `src/assets/usage/tools/MyNewTool.md`

### Graph Operations Workflow
Agents typically follow this pattern:
1. **Sync**: Rebuild graph from markdown files (`IncrementalSyncTools.Sync()`)
2. **Search**: Find relevant knowledge (`MemorySearchTools.SearchMemories(query, mode: "Hybrid")`)
3. **Build Context**: Traverse graph relationships (`GraphTools.BuildContext(conceptName, depth: 2)`)
4. **Read/Write**: Operate on memory files with `[[concepts]]`
5. **Sync**: Update graph indexes

### Concept Repair Pattern
Before consolidating concept variants:
```csharp
// 1. Analyze first (required)
MaintenanceTools.AnalyzeConceptCorruption("tool");

// 2. Dry run (required)
MaintenanceTools.RepairConcepts(
    conceptsToReplace: "tools,MCP tools,MCP Tools",
    canonicalConcept: "tool",
    dryRun: true);

// 3. Execute if safe
MaintenanceTools.RepairConcepts(..., dryRun: false);
```

**Warning**: Never merge class names (e.g., `VectorTools`) with generic concepts (e.g., `[[tool]]`). This destroys semantic meaning.

## Branch Strategy

- **`main`**: Production-ready, protected
- **`dev`**: Integration branch, protected
- **Feature branches**: `feature/*`, `fix/*`, `docs/*`

Always PR to `dev` first, then `dev` → `main` for releases.

Commit format: [Conventional Commits](https://www.conventionalcommits.org/)
- `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`

## Resources

- **Full architecture**: `docs/README.md`
- **Ma Protocol philosophy**: `CONTRIBUTING.md` (Design Philosophy section)
- **Development setup**: `docs/DEVELOPMENT.md`
- **Tool documentation**: `src/assets/usage/tools/*.md`
- **Demo artifacts**: `docs/demo-artifacts/` (25-agent production bug discovery)

## What NOT to Do

❌ Add path validation "for security" (violates NO FAKE SECURITY)  
❌ Add retry logic or error recovery (violates NO FAKE AI)  
❌ Create interfaces for single implementations (violates NO UNNECESSARY ABSTRACTIONS)  
❌ Mock databases or file systems in tests (violates NO FAKE TESTS)  
❌ Consolidate concepts without `AnalyzeConceptCorruption` + dry run first  
❌ Hardcode configuration values (use `Config` class with env var overrides)

## Quick Reference

**Build**: `dotnet build src/Maenifold.csproj`  
**Test**: `dotnet test`  
**Run CLI**: `maenifold --tool WriteMemory --payload '{"title":"Test","content":"[[concept]]"}'`  
**All platforms**: `npm run build:all`
