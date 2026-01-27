# AGENTS.md

This file is for agentic coding assistants working in this repo.
Focus on .NET (core product) and the Next.js site under `site/`.

## Git Workflow

- **Development branch**: `dev` - all work happens here
- **Main branch**: `main` - production releases only
- **Workflow**: Work on `dev`, create PR to `main` for releases
- **Tags**: Release tags (e.g., `v1.0.3`) trigger automated builds via GitHub Actions

Do NOT merge directly to main. Always use a PR.

## Build, Lint, Test

### Core (.NET)
- Restore: `dotnet restore src/Maenifold.csproj`
- Build (Debug): `dotnet build src/Maenifold.csproj -c Debug`
- Build (Release): `dotnet build src/Maenifold.csproj -c Release`
- Publish (Release): `dotnet publish src/Maenifold.csproj -c Release --self-contained -o bin`

### CLI execution rules
- `dotnet run` is forbidden for Maenifold CLI usage
- Compile the CLI first, then run the built `maenifold` binary

### Tests (.NET, NUnit)
- Run all tests: `dotnet test`
- Run a single test (FullyQualifiedName filter):
  `dotnet test --filter "FullyQualifiedName~MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"`
- Coverage: `dotnet test --collect:"XPlat Code Coverage"`

### Site (Next.js)
- Dev server: `npm run dev` (from `site/`)
- Build: `npm run build` (from `site/`)
- Lint: `npm run lint` (from `site/`)

### Cross-platform build scripts (root `package.json`)
- `npm run build` (publish .NET release build to `bin/`)
- `npm run build:all` (all platform binaries)
- `npm run build:osx-arm64`, `npm run build:osx-x64`, `npm run build:linux-x64`, `npm run build:linux-arm64`, `npm run build:win-x64`, `npm run build:win-arm64`

## Code Style and Conventions

### General .NET style (from CONTRIBUTING + DEVELOPMENT)
- Follow .NET runtime coding guidelines
- Allman braces; 4 spaces; no tabs
- Public members: `PascalCase`
- Private fields: `_camelCase` with underscore
- Prefer language keywords (`int` not `Int32`)
- Use `nameof(...)` instead of string literals
- Keep tools small and composable; avoid unnecessary abstractions

### Analyzer/EditorConfig rules
- Treat warnings as errors in builds (`Directory.Build.props`)
- Nullable enabled, implicit usings enabled
- Code analysis:
  - `IDE0005` (remove unused usings) = warning
  - `IDE0052` (unused private members) = warning
  - `CA1812`, `CA2201` = warning
  - `CA1848` and `CA2254` = warning (logging templates)
  - `CA1031` (catch general exception) = disabled (intentional)
  - Prefer braces; prefer simple `using` statements
- Namespace style: `dotnet_style_namespace_match_folder = true`

### Logging and Console
- MCP mode forbids casual console usage; prefer structured logging
- Avoid hardcoded strings (CA1303 is suggestion)

### Error handling (Ma Protocol)
- Let errors surface; no retry logic or “smart” recovery
- Do not add fallback behavior that hides failures
- Prefer direct, explicit flow over abstractions

### Testing philosophy
- NO FAKE TESTS: do not add mocks/stubs/test doubles
- Use real SQLite and real file system paths
- Keep test artifacts in `test-outputs/` for debugging
- Integration over isolated unit tests

### Security philosophy
- NO FAKE SECURITY: avoid security theater for local tool
- Use prepared SQL statements; let OS/SQLite enforce boundaries
- Do not add user-restricting guardrails unless explicitly requested

## Naming and API conventions
- Tool names are `PascalCase` and mirrored in docs
- Tool documentation lives in `src/assets/usage/tools/*.md`
- Register tools in `src/Tools/ToolRegistry.cs`
- Prefer explicit parameter names and direct calls

## Project layout (high level)
- `src/` core .NET implementation
- `tests/` NUnit tests
- `docs/` documentation
- `site/` Next.js site
- `scripts/` NPM distribution helpers
- `integrations/` Claude Code plugins and shared resources
  - `agents/` shared agent definitions
  - `scripts/` shared hook scripts
  - `skills/` shared skill definitions
  - `claude-code/plugin-maenifold/` base plugin (MCP server, core hooks)
  - `claude-code/plugin-product-team/` opinionated workflow plugin

## Distribution

### macOS (Homebrew)
```bash
brew tap msbrettorg/tap
brew install maenifold
```

Tap repo: https://github.com/msbrettorg/homebrew-tap

### Windows
Download MSI from GitHub Releases (TODO: set up release workflow)

### From source
```bash
dotnet publish src/maenifold.csproj -c Release -r osx-arm64 --self-contained
```

## Agent-specific constraints
- This repo enforces a clean root directory (`Directory.Build.targets`)
  - Avoid dropping ad-hoc files in repo root
  - Keep new files in appropriate subdirectories

## Claude Code plugins

Two-layer plugin architecture:

**plugin-maenifold** (base):
- MCP server for maenifold tools
- Hooks: `SessionStart`, `PreCompact`
- Install: `claude plugin add /path/to/integrations/claude-code/plugin-maenifold`

**plugin-product-team** (opinionated):
- Requires plugin-maenifold installed first
- Agents: swe, researcher, red-team, blue-team
- Hooks: `PreToolUse` (Task), `SubagentStop`
- Install: `claude plugin add /path/to/integrations/claude-code/plugin-product-team`

Shared hook script: `integrations/scripts/hooks.sh`
- Modes: `session_start`, `task_augment`, `pre_compact`, `subagent_stop`

Scripting reference: `docs/SCRIPTING.md`

## Cursor/Copilot rules
- No `.cursor/rules/`, `.cursorrules`, or `.github/copilot-instructions.md` found

## Quick context for agents
- Language: C# (.NET 9), NUnit tests
- The system is a local MCP server and CLI
- Design philosophy: Ma Protocol (no fake AI/tests/security)

## Tips for single-test runs
- NUnit filters support `FullyQualifiedName~` substring matching
- Use class or test method names from `tests/Maenifold.Tests/*.cs`
