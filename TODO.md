# TODO

## REL-001: Release v1.0.3

**Status**: Active
**Priority**: High

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
4. [ ] Create PR dev → main
5. [ ] After merge, tag v1.0.3 on main and push (triggers release workflow)
6. [ ] Verify GitHub Release created with 5 platform archives
7. [ ] Verify Homebrew formula auto-updated

### Acceptance Criteria

- [ ] All 252 tests pass
- [ ] v1.0.3 tag created on main branch
- [ ] GitHub Release created with 5 platform archives (osx-arm64, osx-x64, linux-x64, linux-arm64, win-x64)
- [ ] Homebrew formula updated automatically via repository dispatch
- [ ] `brew upgrade maenifold` works after release

---

## DIST-001: Windows MSI installer

**Status**: Inactive
**Priority**: Medium
**Created**: 2026-01-27

### Problem

Windows users must manually download and extract release archives. An MSI installer would provide a better experience.

### Tasks

1. [ ] Research WiX or similar MSI tooling
2. [ ] Create MSI build configuration
3. [ ] Add MSI to release workflow
4. [ ] Update documentation with Windows install instructions

### Acceptance Criteria

- [ ] MSI installer published with each release
- [ ] Installer adds maenifold to PATH
- [ ] Uninstall works cleanly
