# AGENTS.md

This file is for agentic coding assistants working in this repo.
Primary focus: core .NET CLI/MCP server (`src/`) and the Next.js site (`site/`).

## Ground Truth (read first)
- High-level architecture + philosophy: `README.md`, `docs/README.md`
- Dev commands and project layout: `docs/DEVELOPMENT.md`
- Non-negotiables: `CONTRIBUTING.md`, `docs/TESTING_PHILOSOPHY.md`
- Copilot-specific guidance (exists): `.github/copilot-instructions.md`

## Git Workflow (MANDATORY)
**Branch model for agent work**: `commons` -> `dev` -> PR to `main`.

If `CONTRIBUTING.md` conflicts with this agent workflow, follow `AGENTS.md`.

1. Do all work on `commons`.
2. Merge to `dev` only after tests pass: `git checkout dev && git merge commons`
3. Release via PR: `dev` -> `main`.

Hard rules:
- Do not commit directly to `dev` or `main`.
- Do not create feature branches for agent work.
- Do not merge directly to `main`.
- Release tags (e.g., `v1.0.3`) trigger CI builds.

## Build / Lint / Test
Project targets `net9.0` (see `src/Maenifold.csproj`).

### Build policy (CRITICAL)
- During sprints: Debug builds only.
- Ask permission before any Release `dotnet build -c Release` or `dotnet publish -c Release`.

### .NET (core)
```bash
dotnet restore src/Maenifold.csproj
dotnet build src/Maenifold.csproj -c Debug
```

### Tests (NUnit)
```bash
# All tests
dotnet test

# List discovered tests
dotnet test tests/Maenifold.Tests/Maenifold.Tests.csproj --list-tests

# Run a single test (substring match)
dotnet test tests/Maenifold.Tests/Maenifold.Tests.csproj --filter "FullyQualifiedName~MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"

# Run a single test (exact match)
dotnet test tests/Maenifold.Tests/Maenifold.Tests.csproj --filter "FullyQualifiedName=Maenifold.Tests.MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"

# After you already built: faster iteration
dotnet test tests/Maenifold.Tests/Maenifold.Tests.csproj --no-build --filter "FullyQualifiedName~MemoryToolsTests"
```

Optional test settings (see `test.runsettings`):
```bash
dotnet test -s test.runsettings
dotnet test -s test.runsettings --collect:"XPlat Code Coverage"
```

### CLI execution rules
- Do not use `dotnet run` to exercise the CLI.
- Build and run the produced binary:

```bash
./src/bin/Debug/net9.0/maenifold --tool MemoryStatus --payload '{}'
./src/bin/Debug/net9.0/maenifold --mcp
```

### Assets (workflows, tool docs, roles, colors)
Assets live in `src/assets/` and must be copied to `~/maenifold/assets/` for manual testing:

```bash
dotnet build src/Maenifold.csproj -c Debug
./src/bin/Debug/net9.0/maenifold update_assets dryRun=true
./src/bin/Debug/net9.0/maenifold update_assets dryRun=false
```

### Site (Next.js)
Run from `site/`:

```bash
npm install
npm run dev
npm run lint
npm run build
```

### Concept map app (Vite)

Run from `src/apps/concept-map/`: `npm install && npm run build && npm run copy`

## Code Style & Conventions
### Formatting
- C#: Allman braces; 4 spaces; no tabs (see `docs/DEVELOPMENT.md`).
- Use `.editorconfig` + analyzers; `TreatWarningsAsErrors=true` (see `Directory.Build.props`).
- Remove unused `using` directives (IDE0005 is a warning in `src/.editorconfig`).
- Prefer block-scoped namespaces (not file-scoped) to match existing code and `dotnet_style_namespace_match_folder = true`.

### Imports (C# usings)
- Keep `using` directives minimal; rely on `ImplicitUsings=enable` where it applies.
- Order usings consistently (typically `System.*` first, then external packages, then `Maenifold.*`).
- Avoid unused usings and unused private members; CI treats warnings as errors.

### Naming
- Public members and tool names: `PascalCase`.
- Private fields: `_camelCase`.
- Use `nameof(...)` instead of string literals when referencing identifiers.
- Prefer C# keywords over BCL types (`int`, `string`).

### Types, nullability, and collections
- Nullable is enabled; do not suppress nullability warnings without a reason.
- Prefer concrete types when it improves clarity/perf (see `src/.editorconfig` suggestions).

### Error handling (Ma Protocol)
- NO FAKE AI: do not add retries, background auto-fix, or hidden fallbacks.
- Let errors propagate with full information; don't swallow exceptions.
- NO UNNECESSARY ABSTRACTIONS: avoid interfaces/DI for single implementations.
- Prefer throwing real exceptions over returning magic "ERROR:" strings unless an existing tool contract already does so.

### Logging and console
- Follow logging analyzer warnings in `src/.editorconfig` (CA1848, CA2254).
- Avoid casual console writes in MCP-facing code; prefer existing patterns.

### Testing (NO FAKE TESTS)
- Do not add mocks/stubs/test doubles (see `docs/TESTING_PHILOSOPHY.md`).
- Use real SQLite (`Microsoft.Data.Sqlite`) and real filesystem paths.
- Leave useful artifacts for debugging (tests commonly write under `test-outputs/`).

### Tools and docs (public API surface)
- Tools are static methods with `[McpServerTool]` and also registered in `src/Tools/ToolRegistry.cs` (see `.github/copilot-instructions.md`).
- When adding/changing a tool, update its usage doc under `src/assets/usage/tools/`.

Checklist for tool changes:
- Add `[Description]` annotations for tool + parameters (MCP surface).
- Keep payload shapes stable; agents/scripts call `--tool <Name> --payload '<json>'`.
- Add/adjust real tests under `tests/Maenifold.Tests/`.

## Workspace Hygiene
- Keep repo root clean; `Directory.Build.targets` enforces allowed root files.
- Multi-agent rule: read a file before editing; make surgical changes; never delete unknown content.
- Avoid destructive git operations unless explicitly requested.

## MCP Apps References
- Getting Started: https://claude.com/docs/connectors/building/mcp-apps/getting-started
- Design Guidelines: https://claude.com/docs/connectors/building/mcp-apps/design-guidelines
- Cross-Compatibility: https://claude.com/docs/connectors/building/mcp-apps/cross-compatibility
- Troubleshooting: https://claude.com/docs/connectors/building/mcp-apps/troubleshooting
- SDK API: https://modelcontextprotocol.github.io/ext-apps/api/documents/Quickstart.html
- Examples: https://github.com/modelcontextprotocol/ext-apps/tree/main/examples

## Cursor / Copilot Rules
- Cursor: no `.cursor/rules/` or `.cursorrules` found.
- Copilot: rules exist in `.github/copilot-instructions.md`.
  If it conflicts with this file for agent workflow (branches / release builds), follow `AGENTS.md`.
