# AGENTS.md

This file is for agentic coding assistants working in this repo.
Focus on .NET (core product) and the Next.js site under `site/`.

## Git Workflow (MANDATORY)

**Branch model**: `commons` → `dev` → PR to `main`.

1. **Do all work on `commons`.** Commit early and often with T-* identifiers.
2. **Merge `commons` into `dev`** when work is complete and tests pass: `git checkout dev && git merge commons`
3. **Create a PR from `dev` to `main`** for releases.

**Hard rules:**
- Do NOT commit directly to `dev` or `main`. All work happens on `commons`.
- Do NOT create feature branches. `commons` is the working branch.
- Do NOT merge directly to `main`. Always use a PR.
- **Tags**: Release tags (e.g., `v1.0.3`) trigger automated builds via GitHub Actions.

## Build, Lint, Test

### Build Policy (CRITICAL)

**During sprints**: Use Debug build only.
```bash
dotnet build src/Maenifold.csproj           # Debug is default
dotnet test                                  # Tests use Debug
```

**After sprint signoff**: Release build (with permission).
```bash
dotnet build src/Maenifold.csproj -c Release
dotnet publish src/Maenifold.csproj -c Release --self-contained -o bin
```

**Why?** All agents actively use the maenifold release build as their cognitive substrate. Breaking the release build breaks the agents' ability to think, remember, and reason. Don't break the substrate!

**Rule**: ASK PERMISSION before running `dotnet build -c Release` or `dotnet publish -c Release`. The release build replaces the installed binary that agents depend on.

### Core (.NET)
- Restore: `dotnet restore src/Maenifold.csproj`
- Build (Debug): `dotnet build src/Maenifold.csproj -c Debug`
- Build (Release): `dotnet build src/Maenifold.csproj -c Release` **(ask permission first!)**
- Publish (Release): `dotnet publish src/Maenifold.csproj -c Release --self-contained -o bin`

### CLI execution rules
- `dotnet run` is forbidden for Maenifold CLI usage
- Compile the CLI first, then run the built `maenifold` binary

### Asset propagation for testing

Workflow JSON files and other assets live in `src/assets/` and must be copied to `~/maenifold/assets/` for testing:

```bash
# 1. Build the CLI (Debug)
dotnet build src/Maenifold.csproj -c Debug

# 2. Preview asset changes (dry-run)
./src/bin/Debug/net9.0/maenifold update_assets dryRun=true

# 3. Apply asset changes
./src/bin/Debug/net9.0/maenifold update_assets dryRun=false
```

**Source → Target mapping:**
- `src/assets/workflows/*.json` → `~/maenifold/assets/workflows/`
- `src/assets/usage/tools/*.md` → `~/maenifold/assets/usage/tools/`
- `src/assets/roles/*.json` → `~/maenifold/assets/roles/`
- `src/assets/colors/*.json` → `~/maenifold/assets/colors/`

**When to propagate:**
- After creating/modifying workflow JSON files
- After updating tool usage documentation
- Before manual testing of workflows

### Tests (.NET, NUnit)
- Run all tests: `dotnet test`
- Run a single test (FullyQualifiedName filter):
  `dotnet test --filter "FullyQualifiedName~MemoryToolsTests.WriteMemoryCreatesFileWithFrontmatter"`
- Coverage: `dotnet test --collect:"XPlat Code Coverage"`

### Site (Next.js)
- Dev server: `npm run dev` (from `site/`)
- Lint: `npm run lint` (from `site/`)

### Cross-platform build scripts (root `package.json`)

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
- `scripts/` Build helpers
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

### Multi-agent environment
This repo has multiple agents working concurrently. Other agents (and the human) may have uncommitted or recently committed changes in any file. Before editing a file:
- **Read the file first.** Do not assume you know its current contents.
- **Make surgical edits.** Do not rewrite entire files. Use targeted replacements that preserve surrounding content you did not write.
- **PRD.md, RTM.md, TODO.md are especially dangerous.** These are actively maintained by the Product Manager and are the source of truth for requirements, traceability, and backlog. Do not add, remove, or modify entries in these files unless your task explicitly requires it and references a T-* identifier. If you find gaps or ambiguities, ask — do not assume.
- **Never discard unrecognized content.** If a file contains sections or entries you didn't expect, leave them intact. Another agent or the human put them there for a reason.

### Clean root directory
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
