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
- DIST-001: Windows MSI installer with PATH integration

**Removed:**
- DEPR-002: 7 unused Config.cs properties
- AddMissingH1: Deprecated migration tool

### Tasks

1. [ ] Bump version in `src/Maenifold.csproj` from 1.0.2 to 1.0.3
2. [ ] Update CHANGELOG.md: Move [Unreleased] to [1.0.3] with date
3. [ ] Run full test suite: `dotnet test`
4. [ ] Create WiX MSI configuration (installer/, wix.json, PATH integration)
5. [ ] Update release.yml to build MSI on Windows
6. [ ] Create PR dev → main
7. [ ] After merge, tag v1.0.3 on main and push (triggers release workflow)
8. [ ] Verify GitHub Release created with 6 artifacts (5 archives + MSI)
9. [ ] Verify Homebrew formula auto-updated

### Acceptance Criteria

- [ ] All tests pass
- [ ] v1.0.3 tag created on main branch
- [ ] GitHub Release created with 6 artifacts (osx-arm64, osx-x64, linux-x64, linux-arm64, win-x64, win-x64.msi)
- [ ] MSI installer adds maenifold to system PATH
- [ ] MSI uninstall cleanly removes PATH entry
- [ ] Homebrew formula updated automatically via repository dispatch
- [ ] `brew upgrade maenifold` works after release

---

## OPENCODE-001: OpenCode Plugin for Maenifold

**Status**: Active
**Priority**: High
**Created**: 2026-01-27

### Problem

OpenCode lacks native maenifold integration. Need a TypeScript plugin that provides:
- Session start context injection (graph context from SearchMemories/RecentActivity)
- Compaction preservation (save session knowledge via WriteMemory)
- Concept augmentation (enrich tool inputs containing `[[WikiLinks]]` with BuildContext)

### Completed Tasks

- [x] T-OC-1: Research OpenCode plugin API and hook surface
- [x] T-OC-2: Research session.compacted event capture pattern
- [x] T-OC-3: Design plugin architecture (SPEC.md)
- [x] T-OC-4: Implement plugin (`~/.config/opencode/plugins/maenifold.ts`)
- [x] T-OC-5: Blue-team defensive review (1 HIGH, 7 MEDIUM, 11 LOW findings)
- [x] T-OC-6: Red-team security assessment (1 CRITICAL, 3 HIGH, 3 MEDIUM)
- [x] T-OC-6.1: Fix Unicode homoglyph bypass (NFKC normalization)
- [x] T-OC-6.2: Fix symlink path traversal (realpathSync validation)
- [x] T-OC-6.3: Fix type guard (extractMessageText implementation)
- [x] T-OC-7: Verify security remediations (CONDITIONAL PASS)

### Outstanding Tasks

1. [ ] T-OC-8: Implement compaction summary persistence with security mitigations
   - Treat summary as untrusted data (schema separation, fenced reinjection)
   - Aggressive sanitization (NFKC, control char stripping)
   - Strict `[[wikilinks]]` allowlisting to prevent graph corruption
   - See: memory://thinking/sequential/2026/01/28/session-1769625876165
2. [ ] T-OC-9: Add secret/PII scrubbing before persistence
3. [ ] T-OC-10: Add project isolation (namespace by project.id)
4. [ ] T-OC-11: Add retention controls (TTL, opt-out toggle)
5. [ ] T-OC-12: Integration testing with live OpenCode session
6. [ ] T-OC-13: Documentation and README

### Acceptance Criteria

- [ ] Plugin loads successfully in OpenCode
- [ ] Session start injects graph context
- [ ] Compaction summaries persist to SequentialThinking (with sanitization)
- [ ] `[[WikiLinks]]` in prompts trigger BuildContext enrichment
- [ ] All security mitigations verified by red-team

### Related Memory

- memory://tech/integrations/opencode-plugin-for-maenifold
- memory://tech/integrations/opencode-plugin-api-reference
- memory://tech/integrations/opencode-sessioncompacted-event-summary-capture-pattern
- memory://thinking/sequential/2026/01/28/session-1769625876165 (security analysis)

---

## MAENIFOLDPY-001: Python Port Quality Baseline

**Status**: Active
**Priority**: Medium
**Created**: 2026-01-28

### Problem

The maenifold-py repository has type checking issues that need resolution before further development.

### Findings (2026-01-28)

- All pytest tests pass
- mypy reports 51 errors across 16 files
- Key issues:
  - MCP `repair_concepts` kwarg mismatch (`create_wiki_links` vs `create_wikilinks`)
  - ConfigMeta metaclass typing pattern needs adjustment
  - SafeJson generic signature issues
  - Watcher tools bytes/str mismatches
  - Missing PyYAML type stubs

### Tasks

1. [ ] T-PY-006: Fix MCP `repair_concepts` kwarg mismatch
2. [ ] T-PY-007: Establish clean mypy baseline (fix errors or configure ignores)
3. [ ] T-PY-008: Add types-PyYAML to dev dependencies
4. [ ] T-PY-009: Update TODO.md to remove stale C# references

### Acceptance Criteria

- [ ] `mypy src` reports 0 errors (or documented ignores)
- [ ] All pytest tests continue to pass
- [ ] MCP server repair_concepts works correctly

### Related Memory

- memory://tech/maenifoldpy-repo-evaluation-notes-20260128

---

## DIST-001: Windows MSI installer

**Status**: Merged into REL-001
**Priority**: High
**Created**: 2026-01-27

### Problem

Windows users must manually download and extract release archives. An MSI installer would provide a better experience.

### Resolution

Merged into REL-001 (v1.0.3 release). See REL-001 tasks 4-5 for implementation.

---

## CLEANUP-001: Close Active Thinking Sessions

**Status**: Pending
**Priority**: Low
**Created**: 2026-01-28

### Problem

Several SequentialThinking sessions from recent activity are still marked as "active" but appear abandoned.

### Sessions to Review

1. `session-1769578881329` (Jan 28, 10:33) - Compaction log, 4 thoughts, active
2. `session-1769561775163` (Jan 28, 07:53) - Security review, 1 thought, active
3. `session-1769558002091` (Jan 28, 07:00) - Persist test, 1 thought, active

### Tasks

1. [ ] Review each session for any valuable insights to persist
2. [ ] Close or cancel abandoned sessions
3. [ ] Document any learnings in appropriate memory:// notes

### Acceptance Criteria

- [ ] No orphaned active sessions older than 24 hours
