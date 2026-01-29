# Changelog

All notable changes to maenifold MCP Server will be documented in this file.

## [Unreleased]

## [1.0.3] - 2026-01-29

### Added
- **DIST-001**: Windows MSI installer with PATH integration (WiX v5)
- **CCPLUGIN-002**: Added DevAssist AI PRD and Ralph loop plugin docs, commands, hooks, and scripts:
  - Files added: `docs/agent/PRD.md`, `integrations/claude-code/plugin/README_RALPH.md`, `integrations/claude-code/plugin/commands/cancel-ralph.md`, `integrations/claude-code/plugin/commands/help-ralph.md`, `integrations/claude-code/plugin/commands/ralph-loop.md`, `integrations/claude-code/plugin/hooks/stop-hook.sh`, `integrations/claude-code/plugin/scripts/setup-ralph-loop.sh`
  - Agent docs updated: `integrations/claude-code/plugin/agents/blue-team.md`, `integrations/claude-code/plugin/agents/ma.md`, `integrations/claude-code/plugin/agents/red-team.md`, `integrations/claude-code/plugin/agents/researcher.md`, `integrations/claude-code/plugin/agents/swe.md`
  - PROMPT guidance synced: `src/assets/usage/PROMPT.md`, `docs/usage/PROMPT.md`

### Changed
- **GRAPH-001**: BuildContext now extracts the H2 section containing the concept mention instead of showing the file start:
  - Added `ExtractSectionWithConcept()` method that parses markdown sections and finds where `[[concept]]` is mentioned
  - Falls back to `CreateSmartPreview()` if concept not found in any section
  - Handles edge cases: H1-only files, multiple mentions (uses first), plain text matches, truncation, empty content
  - Files updated: `src/Tools/GraphTools.cs` (added 88 lines)
  - Tests added: `BuildContextExtractsSectionWithConceptTest` (integration test), `ExtractSectionWithConceptEdgeCasesTest` (unit test with 8 edge cases)
  - Total test count: 244 tests (was 243)

### Changed
- **HOOKS-002**: Enhanced `graph_rag.sh` hook to align with canonical patterns from `search-and-scripting.md`:
  - Multi-hop depth increased from 1→2 (§5.6: captures "extended network", ~40+ concepts vs ~20)
  - Repository context now filters by semantic score > 0.5 (§5.4: reduces low-relevance noise)
  - Task augmentation now frequency-ranks concepts (repeated concepts processed first, matching session_start behavior)
  - Files updated: `~/.claude/hooks/graph_rag.sh`, `docs/integrations/claude-code/hooks/graph_rag.sh`
- **EVAL-001e**: Refactored SessionStart hook (`~/.claude/hooks/session_start.sh`) from v1 to v2:
  - JSON output format with `hookSpecificOutput.additionalContext` (was plain text)
  - Portable CLI detection via PATH with fallback
  - Hub concepts ranked by frequency (not just recent)
  - FindSimilarConcepts expansion for semantic discovery
  - Active SequentialThinking session detection with warning
  - CWD-based repository context hints via SearchMemories
  - Single RecentActivity call (was called twice)
  - Graceful degradation when maenifold unavailable

### Removed
- **DEPR-002**: Removed 7 unused Config.cs properties that were defined but never referenced:
  - `MaxDebounceBatchSize` (`MAENIFOLD_MAX_BATCH_SIZE`)
  - `DefaultSearchLimit` (`MAENIFOLD_SEARCH_LIMIT`)
  - `DefaultContextDepth` (`MAENIFOLD_CONTEXT_DEPTH`)
  - `MaxContextEntities` (`MAENIFOLD_MAX_ENTITIES`)
  - `SqliteConnectionPoolSize` (`MAENIFOLD_DB_POOL_SIZE`)
  - `MaxConcurrentOperations` (`MAENIFOLD_MAX_CONCURRENT`)
  - `EnableDebugLogging` (`MAENIFOLD_DEBUG`)

### Fixed
- **GRAPH-003**: WikiLink regex rejects triple/quadruple bracket patterns (negative lookbehind/lookahead)
- **GRAPH-002**: Prevent WikiLink extraction from code blocks:
  - Modified `MarkdownReader.ExtractWikiLinks()` to use Markdig AST parsing
  - Skips fenced code blocks (```), indented code blocks, and inline code (`)
  - Prevents shell variables and code examples from polluting concept graph
  - Root cause: Simple regex matched `[[concepts]]` inside code blocks, creating malformed entries like `n-"$active-session"`
  - Added 10 test cases in `MarkdownReaderTests.cs`
  - Files updated: `src/Utils/MarkdownReader.cs` (~100 lines added)
- **CCSKILL-001**: Clarified Claude Code `Workflow` skill usage to prevent premature abandonment:
  - Explicitly requires step-by-step execution of structured workflows (no skipping)
  - Documents the required start/continue/complete loop using `sessionId + response`
  - Files updated: `integrations/claude-code/plugin/skills/workflow/SKILL.md`, `integrations/claude-code/vscode/skills/workflow/SKILL.md`
- **DOCS-001**: Synced tool documentation between `src/assets/usage/tools/` and `docs/usage/tools/`:
  - Fixed adopt.md to list all 16 built-in roles, 7 colors, 12 language perspectives
  - Added custom assets documentation ($MAENIFOLD_ROOT/assets/ for runtime assets)
  - Fixed MoveMemory overwrite behavior text (correctly states NO overwrite)
  - Added nested WikiLink warning to editmemory.md
  - Added depth limits (1-5 hops) to visualize.md
  - Removed dead environment variable references from getconfig.md

### Security
- **SEC-001**: Added JSON deserialization depth limits (MaxDepth=32) to prevent DoS attacks via deeply nested JSON payloads. New `SafeJson` utility provides consistent protection across all deserialization paths:
  - CLI payload parsing (Program.cs)
  - Asset loading (AssetManager.cs)
  - Workflow JSON parsing (WorkflowOperations.Core.cs, WorkflowOperations.Management.cs)
  - Graph file list parsing (GraphTools.cs, IncrementalSync.Database.cs, ConceptSync.cs)
  - Payload parameter extraction (PayloadReader.cs)
  - Asset file parsing (AssetResources.cs, AdoptTools.cs)

### Removed
- **AddMissingH1**: Deprecated and removed the one-time migration tool. All memory files now include H1 headers via WriteMemory, making this tool obsolete.

### Changed
- SequentialThinking docs and error text now explicitly enforce starting sessions at `thoughtNumber=0`, clarifying flows that previously failed when called at 1 without a session.

### Added
- **TEST-001**: Expanded test coverage with 49 new tests:
  - `SystemToolsTests.cs` (15 tests) - GetConfig, GetHelp, ListMemories, MemoryStatus, UpdateAssets
  - `AdoptToolsTests.cs` (15 tests) - role/color/perspective loading, error handling
  - `GraphAnalyzerTests.cs` (19 tests) - Visualize, RandomWalk, edge cases
- Test suite now has 252 tests total (242 active, 10 skipped benchmarks)

### Fixed
- **SequentialThinkingToolsTests**: Fixed 14 failing tests that incorrectly used `thoughtNumber=1` for new sessions instead of `thoughtNumber=0`.

## [1.0.2] - 2025-11-19

### Fixed
- **CRITICAL**: Removed broken optionalDependencies that prevented installation
  - v1.0.0 and v1.0.1 referenced non-existent platform packages
  - Now ships all binaries in single package with automatic platform detection
- Fixed VS Code install URL scheme on website (vscode:mcp/install)
- Fixed release archive filenames to include version number (e.g., maenifold-1.0.2-linux-x64.tar.gz)
- Added mandatory release validation (pre-commit + GitHub Actions)
  - Blocks commits with optionalDependencies
  - Validates VS Code URL schemes
  - Verifies package.json structure
- Removed automatic npm publishing from release workflow
  - npm publishing moved to separate manual workflow (publish-npm.yml)
  - Prevents accidental npm publishes on GitHub releases
- Fixed Git LFS checkout in release workflow
  - ONNX model file (90MB) was being checked out as LFS pointer (133 bytes)
  - Added `lfs: true` to checkout action to fetch actual model files
  - Previous releases (v1.0.2 initial) were non-functional due to missing model

### Changed
- Distribution model: single package with bundled binaries for all platforms
- Wrapper script automatically selects correct binary based on OS/arch
- **Platform Support**: Temporarily removed linux-arm64 and win-arm64 builds
  - Missing sqlite-vec native libraries (vec0) for these platforms
  - See [sqlite-vec#231](https://github.com/asg017/sqlite-vec/issues/231)
- Updated CLAUDE.md asset counts (32 workflows, 16 roles)

### Architecture
- No separate platform packages (maenifold-win32-x64, etc.)
- All binaries included in bin/ directory structure
- Cleaner, more reliable installation process
- SHA256 checksums now generated for all release archives
- README and LICENSE included in release archives
