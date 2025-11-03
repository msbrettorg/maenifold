# Requirements Traceability Matrix - Sprint 20251103

## Sprint Scope
1. Codex-optimize 25 LLM-facing tool docs (~70% token reduction)
2. Add `learn` parameter to all MCP tools
3. Deprecate ListWorkflows tool (PRD)

## MUST HAVE Requirements (Atomic)

### R1: Codex Documentation Optimization

**RTM-001**: Rewrite `addmissingh1.md` per codex pattern
- File: `src/assets/usage/tools/addmissingh1.md`
- Target: ≥60% token reduction
- Test: Manual token count verification

**RTM-002**: Rewrite `analyzeconceptcorruption.md` per codex pattern
- File: `src/assets/usage/tools/analyzeconceptcorruption.md`
- Target: ≥60% token reduction

**RTM-003** through **RTM-025**: Remaining 23 docs
- Files: `assumptionledger.md`, `buildcontext.md`, `deletememory.md`, `editmemory.md`, `extractconceptsfromfile.md`, `findsimilarconcepts.md`, `getconfig.md`, `listmemories.md`, `memorystatus.md`, `movememory.md`, `readmemory.md`, `recentactivity.md`, `repairconcepts.md`, `runfullbenchmark.md`, `searchmemories.md`, `sequentialthinking.md`, `sync.md`, `updateassets.md`, `visualize.md`, `adopt.md`, `workflow.md`, `listworkflows.md`, `gethelp.md`
- Pattern: Title → Description → Parameters → Returns → Example → Constraints → Integration
- Target: ≥60% token reduction per file
- Test: Token count verification, preserve [[WikiLinks]]

### R2: Learn Parameter Implementation

**RTM-026**: Add `learn` parameter to GraphTools.cs
- File: `src/Tools/GraphTools.cs`
- Methods: `Sync()`, `BuildContext(...)`, `Visualize(...)`
- Implementation: Add `bool learn = false` as last parameter
- Test: Unit test `learn=true` returns help content

**RTM-027**: Add `learn` parameter to MemoryTools.Write.cs
- File: `src/Tools/MemoryTools.Write.cs`
- Method: `WriteMemory(...)`
- Implementation: Add `bool learn = false`, return `writememory.md` content when true

**RTM-028** through **RTM-049**: Remaining 14 tool files
- Files: `MemoryTools.Operations.cs` (5 methods), `MemorySearchTools.cs`, `VectorSearchTools.cs`, `SystemTools.cs` (5 methods), `ConceptRepairTool.cs` (2 methods), `SequentialThinkingTools.cs`, `WorkflowTools.Runner.cs`, `WorkflowTools.cs` (skip - deprecated), `RecentActivityTools.cs`, `AssumptionLedgerTools.cs`, `AdoptTools.cs` (async), `MaintenanceTools.cs`, `PerformanceBenchmark.cs`, `IncrementalSyncTools.cs` (2 methods)
- Pattern: `if (learn) { return File.ReadAllText(helpPath); }`
- Test: Integration tests per tool, build passes

### R3: ListWorkflows Deprecation PRD

**RTM-050**: Create PRD document
- File: `/docs/PRD-listworkflows-deprecation.md`
- Sections: Rationale, Implementation, Breaking Changes, Migration, Docs Cleanup, Testing, Rollout, Alternatives
- Test: Manual review, <500 lines per Ma Protocol

**RTM-051**: Update human-facing docs to match LLM-facing
- Files: `/docs/usage/tools/{listworkflows,workflow,gethelp}.md`
- Pattern: Match deprecation notice from `/src/assets/usage/tools/listworkflows.md`
- Test: Content consistency verification

## MUST NOT HAVE (Scope Boundaries)

**RTM-X01**: NO modifications to `/docs/usage/tools/` UNTIL R3
- Rationale: Human-facing docs separate from LLM-facing (R1/R2)
- Enforcement: File path verification in commits

**RTM-X02**: NO breaking changes without documentation
- Rationale: R2 is backward compatible (default `learn=false`)
- Enforcement: PR review, test coverage

**RTM-X03**: NO dotnet run usage for maenifold testing
- Rationale: PM Protocol termination rule
- Enforcement: Use `dotnet build --tool` or MCP protocol only

**RTM-X04**: NO sequential agent blocking
- Rationale: PM Protocol termination rule
- Enforcement: All Task tool calls in single message

**RTM-X05**: NO unverified agent work
- Rationale: PM Protocol - always dispatch verification agents
- Enforcement: Verification wave after each implementation wave

**RTM-X06**: NO MVPs or partial solutions
- Rationale: Simple, Lovable, Complete philosophy
- Enforcement: All 25 docs, all 24 tools, complete PRD required

## Agent Orchestration Rules

1. **Concurrent Waves:** Dispatch independent agents in parallel (never sequential)
2. **Agent Context:** Each agent receives full context + memory:// URIs
3. **Verification:** Never trust agent reports - dispatch verification agents
4. **RTM Traceability:** All code changes link to specific RTM items above
5. **Build Validation:** Must compile + pass tests before completion
6. **Codex-Specialist Pattern:** All doc agents adopt role + read README.md first

## Git Workflow

- **Branch:** `sprint-20251103-learn-param-codex-docs`
- **Baseline:** `sprint-baseline.txt` (captured)
- **Commit Strategy:** 
  - Commit 1: RTM-001 through RTM-025 (all docs)
  - Commit 2: RTM-026 through RTM-049 (learn parameter)
  - Commit 3: RTM-050 through RTM-051 (PRD + docs)
- **PR Target:** `main`

## Definition of Done

- [ ] All 25 docs rewritten with ≥60% token reduction (RTM-001 through RTM-025)
- [ ] All 24 tools have `learn` parameter working correctly (RTM-026 through RTM-049)
- [ ] PRD created + human docs updated (RTM-050, RTM-051)
- [ ] `dotnet build` succeeds with zero warnings
- [ ] `dotnet test` passes all tests
- [ ] All commits link to RTM items in messages
- [ ] No MUST NOT violations (RTM-X01 through RTM-X06)
