# Backlog

## CCPLUGIN-002: Add Ralph loop plugin docs, commands, and hooks

**Status**: Done
**Priority**: Medium
**Created**: 2026-01-23

### Problem

Claude Code plugin support needs a documented Ralph loop workflow, commands, and hooks, with agent docs aligned and prompt guidance published alongside usage docs.

### Fix

Add Ralph loop docs/commands/hooks/scripts, update agent descriptions, add DevAssist AI PRD, and publish PROMPT guidance in both asset and docs usage trees.

### Tasks

1. [x] Add DevAssist AI PRD (`docs/agent/PRD.md`)
2. [x] Add Ralph loop commands/hooks/scripts (`integrations/claude-code/plugin/commands/`, `integrations/claude-code/plugin/hooks/`, `integrations/claude-code/plugin/scripts/`)
3. [x] Update Claude Code plugin agent docs (`integrations/claude-code/plugin/agents/`)
4. [x] Update PROMPT guidance in `src/assets/usage/` and `docs/usage/`

### Acceptance Criteria

- [x] Ralph loop docs, commands, hooks, and scripts are present and documented
- [x] Agent docs reference new workflow where applicable
- [x] PROMPT guidance is in sync across assets and docs

## CCSKILL-001: Enforce step-by-step workflow adherence in Claude Code skill docs

**Status**: Done
**Priority**: High
**Created**: 2026-01-16

### Problem

Agents may start a structured workflow, read the first step, then jump directly to a conclusion and abandon the workflow. This defeats the purpose of structured methodologies.

### Fix

Update the `Workflow` skill instructions to explicitly require step-by-step execution, iterative continuation with `sessionId + response`, and completion via `status='completed'` + `conclusion`.

### Tasks

1. [x] Update `integrations/claude-code/plugin/skills/workflow/SKILL.md`
2. [x] Update `integrations/claude-code/vscode/skills/workflow/SKILL.md`
3. [x] Add CHANGELOG.md entry under [Unreleased]

### Acceptance Criteria

- [x] Skill docs explicitly forbid skipping steps and premature conclusions
- [x] Skill docs describe the required start/continue/complete loop
- [x] Plugin + VS Code skill copies remain consistent

## REL-001: Publish v1.0.3 release to GitHub

**Status**: Inactive
**Priority**: Critical
**Created**: 2026-01-11
**Blocks**: NPM-001

### Problem

Current published version is v1.0.2 (2025-11-19). Significant changes in [Unreleased] need to be released before NPM distribution work can proceed.

### Changes for v1.0.3

**Changed:**
- GRAPH-001: BuildContext extracts H2 section containing concept mention
- HOOKS-002: Enhanced graph_rag.sh with depth=2, score filtering, frequency ranking
- EVAL-001e: Refactored SessionStart hook to v2 (JSON output, portable CLI, CWD context)
- SequentialThinking enforces thoughtNumber=0 for new sessions

**Fixed:**
- GRAPH-002: Prevent WikiLink extraction from code blocks (Markdig AST parsing)
- DOCS-001: Synced tool documentation between src/assets/usage/ and docs/usage/
- SequentialThinkingToolsTests: Fixed 14 failing tests

**Security:**
- SEC-001: JSON deserialization depth limits (MaxDepth=32) via SafeJson utility

**Added:**
- TEST-001: 49 new tests (252 total, 242 active)

**Removed:**
- DEPR-002: 7 unused Config.cs properties
- AddMissingH1: Deprecated migration tool

### Tasks

1. [ ] Bump version in `src/Maenifold.csproj` from 1.0.2 to 1.0.3
2. [ ] Update CHANGELOG.md: Move [Unreleased] to [1.0.3] with date
3. [ ] Run full test suite: `dotnet test`
4. [ ] Build release binaries for all 4 platforms (framework-dependent)
5. [ ] Create GitHub Release with tag v1.0.3
6. [ ] Upload platform archives to release
7. [ ] Verify Git LFS checkout includes actual ONNX model (not 133-byte pointer)

### Files

- `src/Maenifold.csproj` - Version bump
- `CHANGELOG.md` - Release notes
- `.github/workflows/` - Release workflow (if restored)

### Acceptance Criteria

- [ ] All 252 tests pass
- [ ] v1.0.3 tag created on main branch
- [ ] GitHub Release created with 4 platform archives
- [ ] Each archive contains functional binary + 86MB ONNX model
- [ ] Release notes match CHANGELOG.md

---

## NPM-001: Implement platform-specific NPM package distribution

**Status**: Inactive
**Priority**: High
**Created**: 2026-01-11

### Problem

Current distribution via GitHub Releases requires manual download. NPM distribution failed in v1.0.0-1.0.1 because optionalDependencies referenced non-existent platform packages. Binary size (~142MB per platform) makes single-package distribution impractical.

### Background

Framework-dependent build breakdown:
- ONNX model (all-MiniLM-L6-v2): 86 MB
- ONNX runtime native: 34 MB
- ONNX runtime managed: 14 MB
- SQLite + vec0 extensions: 2 MB
- Managed DLLs: 19 MB
- Assets: 0.7 MB
- **Total per platform: ~142 MB**

Self-contained adds ~60MB for .NET runtime. Native AOT not viable due to reflection usage in SqliteExtensions.cs and YamlDotNet dependency.

### Solution

Implement platform-specific NPM packages (pattern used by esbuild, swc, sharp, @biomejs/biome):

```
maenifold                       # ~5KB stub + platform detection
├── optionalDependencies:
│   ├── @maenifold/linux-x64    # ~142MB
│   ├── @maenifold/osx-arm64    # ~142MB
│   ├── @maenifold/osx-x64      # ~142MB
│   └── @maenifold/win-x64      # ~142MB
```

NPM only downloads the matching platform package. User installs ~142MB, not 568MB.

### Tasks

1. [ ] Create `@maenifold/linux-x64` package with binary + assets
2. [ ] Create `@maenifold/osx-arm64` package with binary + assets
3. [ ] Create `@maenifold/osx-x64` package with binary + assets
4. [ ] Create `@maenifold/win-x64` package with binary + assets
5. [ ] Create main `maenifold` package with:
   - Platform detection (os.platform + os.arch)
   - optionalDependencies for all platforms
   - JS stub that requires correct platform package
   - Exports binary path for MCP configuration
6. [ ] Create GitHub Actions workflow for coordinated publish:
   - Build all platform binaries
   - Publish platform packages first
   - Publish main package after all platform packages succeed
7. [ ] Update Claude Code plugin `.mcp.json` to use `maenifold` from PATH
8. [ ] Test installation on all 4 platforms
9. [ ] Update documentation with `npm install -g maenifold`

### Files

- `npm/maenifold/package.json` - Main package
- `npm/maenifold/index.js` - Platform detection + binary path export
- `npm/linux-x64/package.json` - Linux x64 platform package
- `npm/osx-arm64/package.json` - macOS ARM64 platform package
- `npm/osx-x64/package.json` - macOS x64 platform package
- `npm/win-x64/package.json` - Windows x64 platform package
- `.github/workflows/publish-npm.yml` - Coordinated publish workflow

### Acceptance Criteria

- [ ] `npm install -g maenifold` works on all 4 platforms
- [ ] Only platform-matching package downloaded (~142MB, not 568MB)
- [ ] `maenifold --version` works after install
- [ ] Binary in PATH for MCP server configuration
- [ ] Coordinated publish prevents broken optionalDependencies

### Reference

- v1.0.0-1.0.1 failure: optionalDependencies referenced non-existent packages
- Git LFS issue: ONNX model checked out as 133-byte pointer
- Prior work: `.github/workflows/release.yml` (deleted), `CHANGELOG.md` entries

---

## REFACTOR-001: Consolidate MemoryTools partial classes

**Status**: Done
**Priority**: Low
**Created**: 2026-01-12

### Problem

MemoryTools is split across 4 partial class files without justification. The main file `MemoryTools.cs` is empty (7 lines, just the class declaration). This is pure abstraction noise, not file-size management.

### Current State

| File | Lines | Content |
|------|-------|---------|
| `MemoryTools.cs` | 7 | Empty partial class declaration |
| `MemoryTools.Helpers.cs` | 172 | Private helpers |
| `MemoryTools.Operations.cs` | 307 | ReadMemory, EditMemory, DeleteMemory, MoveMemory, ExtractConceptsFromFile |
| `MemoryTools.Write.cs` | 97 | WriteMemory + `[McpServerToolType]` attribute |

**Total: ~583 lines** - easily one file.

### Tasks

1. [x] Merge all content into `MemoryTools.cs`
2. [x] Move `[McpServerToolType]` attribute to class declaration
3. [x] Delete `MemoryTools.Helpers.cs`
4. [x] Delete `MemoryTools.Operations.cs`
5. [x] Delete `MemoryTools.Write.cs`
6. [x] Run tests to verify no regressions

### Files

- `src/Tools/MemoryTools.cs` - Target (merge all content here)
- `src/Tools/MemoryTools.Helpers.cs` - Delete after merge
- `src/Tools/MemoryTools.Operations.cs` - Delete after merge
- `src/Tools/MemoryTools.Write.cs` - Delete after merge

### Acceptance Criteria

- [x] Single `MemoryTools.cs` file with all functionality
- [x] All MemoryTools tests pass (9/9) - 4 unrelated failures in MarkdownReaderAdversarialTests pre-existed
- [x] No behavior changes

### Notes

MemorySearchTools was evaluated but kept split - each file has distinct algorithmic responsibility (text search, vector search, fusion). MemoryTools split has no such justification.

---

## SEC-002: Fix URL-encoded path traversal bypass in SanitizeUserInput

**Status**: Done (with known limitation)
**Priority**: High
**Created**: 2026-01-12

### Problem

`SanitizeUserInput` (MemoryTools.cs:150) strips `..` but doesn't handle URL-encoded forms. Attacker can use `%2e%2e%2f` to bypass path traversal protection.

### Fix

Added `Uri.UnescapeDataString(input)` at line 138 to decode before sanitization.

### Files

- `src/Tools/MemoryTools.cs` - SanitizeUserInput method (line 138)

### Acceptance Criteria

- [x] URL-encoded `..` sequences are blocked (single-encoding)
- [x] Tests added for encoded bypass attempts (5 tests)
- [x] All existing tests pass (14/14)

### Known Limitation

Double-encoded attacks (`%252e%252e%252f`) only decode once to `%2e%2e%2f`, not to `../`. This is documented in test comments (lines 365-367). Risk is LOW because:
1. `Slugify()` strips non-alphanumeric chars from filename
2. `ValidatePathSecurity()` provides defense-in-depth
3. File system never receives encoded paths

See: `memory://security/fixes/sec002-urlencoded-path-traversal-fix-in-sanitizeuserinput`

---

## SLOP-001: Remove XSS blacklist from SanitizeUserInput

**Status**: Done
**Priority**: Low
**Created**: 2026-01-12

### Problem

Lines 163-172 of `SanitizeUserInput` check for XSS-related strings ("javascript", "vbscript", "onload", etc.) in file **titles**. This is security theater - titles aren't rendered as HTML. This is cargo-cult code.

### Fix

Deleted the blacklist check entirely.

### Files

- `src/Tools/MemoryTools.cs` - SanitizeUserInput method

### Acceptance Criteria

- [x] XSS blacklist removed
- [x] All existing tests pass (14/14)
- [x] Titles containing "javascript" etc. work normally (verified via CLI)

See: `memory://security/assessments/blue-team-quality-review-sec002-slop001-fixes`

---

## SIMP-001: Delete duplicate ConceptRepairer.cs

**Status**: Done
**Priority**: Medium
**Created**: 2026-01-13
**Est. Lines Saved**: ~277

### Problem

`src/Tools/ConceptRepairer.cs` is a duplicate of `src/Tools/ConceptRepairTool.cs`. Both implement the same concept repair functionality.

### Tasks

1. [ ] Verify ConceptRepairTool.cs is the canonical implementation
2. [ ] Delete ConceptRepairer.cs
3. [ ] Run tests to verify no regressions

### Files

- `src/Tools/ConceptRepairer.cs` - Delete
- `src/Tools/ConceptRepairTool.cs` - Keep (canonical)

### Acceptance Criteria

- [ ] Only one concept repair implementation exists
- [ ] All tests pass

---

## SIMP-002: Extract learn parameter boilerplate

**Status**: Done
**Priority**: Medium
**Created**: 2026-01-13
**Est. Lines Saved**: ~160

### Problem

Every MCP tool has the same 8-line `learn` parameter handling pattern repeated:

```csharp
if (learn)
{
    var toolName = nameof(ToolMethod).ToLowerInvariant();
    var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
    if (!File.Exists(helpPath))
        return $"ERROR: Help file not found for {nameof(ToolMethod)}";
    return File.ReadAllText(helpPath);
}
```

This is duplicated ~20 times across tools.

### Tasks

1. [ ] Create helper method `ToolHelpers.GetLearnContent(string toolName)`
2. [ ] Replace all 20 occurrences with single-line call
3. [ ] Run tests to verify no regressions

### Files

- `src/Tools/*.cs` - All tool files with learn parameter
- `src/Utils/ToolHelpers.cs` - New helper (or add to existing utils)

### Acceptance Criteria

- [ ] Single implementation of learn logic
- [ ] All tools use helper method
- [ ] All tests pass

---

## SIMP-003: Move tool catalog out of error message

**Status**: Blocked (Invalid)
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~50

### Problem

`src/Tools/ToolRegistry.cs` contains a 50-line tool catalog embedded in an error message string. This makes maintenance difficult and bloats the error handling code.

### Tasks

1. [ ] Move tool catalog to config file or resource
2. [ ] Load catalog dynamically in error handler
3. [ ] Run tests to verify no regressions

### Files

- `src/Tools/ToolRegistry.cs` - Remove inline catalog
- `src/assets/tool-catalog.json` or similar - New catalog location

### Acceptance Criteria

- [ ] Tool catalog externalized
- [ ] Error messages still include catalog when needed
- [ ] All tests pass

---

## SIMP-004: Deduplicate SQL INSERT patterns in GraphTools

**Status**: Done
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~60

### Problem

`src/Tools/GraphTools.cs` (lines 51-123) has repetitive SQL INSERT OR IGNORE patterns that could be consolidated into a helper method.

### Tasks

1. [ ] Identify all duplicate INSERT patterns
2. [ ] Create helper method for parameterized inserts
3. [ ] Replace duplicates with helper calls
4. [ ] Run tests to verify no regressions

### Files

- `src/Tools/GraphTools.cs` - Lines 51-123

### Acceptance Criteria

- [ ] Single INSERT helper method
- [ ] All duplicate patterns replaced
- [ ] All tests pass

---

## SIMP-005: Simplify YAML parsing fallbacks in MarkdownReader

**Status**: Blocked (Needs tests first)
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~40

### Problem

`src/Utils/MarkdownReader.cs` (lines 38-78) has multiple defensive fallback strategies for YAML parsing. Some may be unnecessary if input is already validated upstream.

### Tasks

1. [ ] Analyze which fallback paths are actually exercised
2. [ ] Remove unused fallback strategies
3. [ ] Add tests for remaining edge cases
4. [ ] Run tests to verify no regressions

### Files

- `src/Utils/MarkdownReader.cs` - Lines 38-78

### Acceptance Criteria

- [ ] Unnecessary fallbacks removed
- [ ] Valid edge cases still handled
- [ ] All tests pass

---

## SIMP-006: Use string interpolation in SequentialThinkingTools

**Status**: Done
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~35

### Problem

`src/Tools/SequentialThinkingTools.cs` has verbose section-building code that could use cleaner string interpolation patterns.

### Tasks

1. [ ] Identify verbose StringBuilder/concatenation patterns
2. [ ] Replace with string interpolation or template strings
3. [ ] Run tests to verify no regressions

### Files

- `src/Tools/SequentialThinkingTools.cs`

### Acceptance Criteria

- [ ] Cleaner string building code
- [ ] Same output format
- [ ] All tests pass

---

## SIMP-007: Remove redundant null checks in IncrementalSync

**Status**: Blocked (No redundancy found)
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~12

### Problem

`src/Tools/IncrementalSync.Database.cs` (lines 42-54) has redundant null checks after values have already been validated.

### Tasks

1. [ ] Identify null checks that follow prior validation
2. [ ] Remove redundant checks
3. [ ] Run tests to verify no regressions

### Files

- `src/Tools/IncrementalSync.Database.cs` - Lines 42-54

### Acceptance Criteria

- [ ] No redundant null checks
- [ ] All tests pass

---

## SIMP-008: Reduce over-defensive try/catch in VectorTools

**Status**: Done
**Priority**: Low
**Created**: 2026-01-13
**Est. Lines Saved**: ~10

### Problem

`src/Utils/VectorTools.Embeddings.cs` has over-defensive try/catch blocks that may swallow useful error information or handle impossible cases.

### Tasks

1. [ ] Identify try/catch blocks with overly broad exception handling
2. [ ] Narrow exception handling or remove unnecessary blocks
3. [ ] Run tests to verify no regressions

### Files

- `src/Utils/VectorTools.Embeddings.cs`

### Acceptance Criteria

- [ ] Appropriate exception handling
- [ ] Error information preserved
- [ ] All tests pass

---
