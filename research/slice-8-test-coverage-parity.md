# Parity Audit Slice 8: Test Coverage Parity

**Date**: 2025-07-10  
**Traces**: T-1.4, T-1.5, T-1.6, T-2.3, T-2.4, T-3.2, T-9.6 | FR-1 through FR-6, NFR-1  
**Sequential thinking**: session-1773104046670-38000 / branch: audit-slice-8-tests

---

## Executive Summary

The Rust port has **1,191 tests** vs C#'s **1,023 tests** — a net surplus of **+168 tests**. Over 90% of C# test areas have exact or better Rust coverage. Five deficit areas exist with a combined ~38 missing tests, but none are blocking. Six oracle parity tests are ignored with documented T-10.19 reasons; Sprint 1–3 tool parity test files remain pending (already tracked as T-1.6, T-2.4, T-3.2).

---

## Grand Totals

| Metric | C# | Rust |
|--------|-----|------|
| Test files | 71 (excl. obj/) | ~85 (core + CLI + compat) |
| Test methods | 1,023 | 1,191 (1,177 external + 14 inline) |
| Performance tests | 28 | 28 |
| Oracle parity tests | N/A | 25 (6 ignored) |
| Ignored tests | 0 | 7 (6 parity + 1 perf) |

---

## Per-Area Comparison

### ✅ Full Parity or Rust Exceeds

```
Test Area: Adopt (FR-1)
C# test count: 12 (AdoptToolsTests.cs)
Rust test count: 12 (adopt_tools_tests=5 + adopt_boundary=7)
Gap: None
Severity: N/A

Test Area: AnalyzeConceptCorruption (FR-2)
C# test count: 19 (ConceptAnalyzerTests.cs)
Rust test count: 19 (concept_analysis_boundary.rs)
Gap: None
Severity: N/A

Test Area: RepairConcepts (FR-6)
C# test count: 28 (ConceptRepairToolTests.cs)
Rust test count: 28 (repair_concepts_boundary.rs)
Gap: None
Severity: N/A

Test Area: UpdateAssets (FR-3)
C# test count: 17 (AssetManagerTests.cs)
Rust test count: 23 (asset_manager_tests=17 + update_assets_boundary=6)
Gap: None — Rust exceeds by +6
Severity: N/A

Test Area: RecentActivity (FR-4)
C# test count: 35 (RecentActivityPipelineTests.cs)
Rust test count: 45 (pipeline=35 + boundary=8 + tool=2)
Gap: None — Rust exceeds by +10
Severity: N/A

Test Area: RunFullBenchmark (FR-5)
C# test count: 0 (no dedicated test file)
Rust test count: 7 (benchmark_boundary.rs)
Gap: None — Rust-only coverage
Severity: N/A

Test Area: Markdown I/O
C# test count: 107 (boundary=39 + adversarial=32 + reader=10 + sync=7 + writer=19)
Rust test count: 107 (boundary=39 + adversarial=32 + rw=36)
Gap: None — exact aggregate match
Severity: N/A

Test Area: SequentialThinking
C# test count: 17 (SequentialThinkingToolsTests.cs)
Rust test count: 17 (within session_tests.rs, verified 1:1 function name match)
Gap: None
Severity: N/A

Test Area: AssumptionLedger
C# test count: 44 (AssumptionLedgerTests=12 + ValidationTests=32)
Rust test count: 44 (within session_tests.rs, verified 1:1 function name match)
Gap: None
Severity: N/A

Test Area: Workflow
C# test count: 60 (WorkflowTools=22 + Operations=22 + Supervisor=16)
Rust test count: 86 (workflow_tests=44 + supervisor=36 + session_workflow_boundary=6)
Gap: None — Rust exceeds by +26
Severity: N/A

Test Area: Memory Tools
C# test count: 56 (MemoryTools=15 + EditMemory=18 + MemoryPath=14 + ListMemories=9)
Rust test count: 71 (memory_tools=42 + memory_path=14 + memory_boundary_paths=15)
Gap: None — Rust exceeds by +15
Severity: N/A

Test Area: Community Detection
C# test count: 34 (Detection=18 + Coverage=9 + Runner=7)
Rust test count: 34 (community_tests.rs)
Gap: None
Severity: N/A

Test Area: Search Tools
C# test count: 3 (SearchToolsTests.cs)
Rust test count: 3 (search_tools_tests.rs)
Gap: None
Severity: N/A

Test Area: Vector/Sync Diagnostics
C# test count: 46 (VectorSearch=3 + VectorTools=19 + VecSyncDiag=12 + DegradedDiag=7 + MemSearchDiag=5)
Rust test count: 48 (vector_search=3 + vector_tools=19 + vec_sync_diag=9 + vec_sync_boundary=5 + degraded_diag=7 + mem_search_diag=5)
Gap: Marginal rebalancing — VecSyncDiag C#=12 vs Rust=9+5=14 (different split)
Severity: N/A

Test Area: CLI Output/Security/Perf
C# test count: 51 (Output=20 + Security=29 + Perf=2)
Rust test count: 51 (output=20 + security=29 + perf=2)
Gap: None
Severity: N/A

Test Area: MCP Resources
C# test count: 33 (McpResourceToolsTests.cs)
Rust test count: 39 (mcp_resource=33 + mcp=6)
Gap: None — Rust exceeds by +6
Severity: N/A

Test Area: Sync Performance/Mtime/Unification
C# test count: 78 (SyncPerf=21 + SyncLeader=31 + SyncUnify=7 + SyncMtime=3 + IncrSync=8 + IncrUnify=5 + IncrMtime=3)
Rust test count: 77 (sync_perf=21 + sync_leader=17 + leader_boundary=7 + sync_unify=7 + sync_mtime=3 + sync_mtime_opt=3 + incr_sync=8 + incr_unify=5 + incr_boundary=6)
Gap: See SyncLeader deficit below
Severity: N/A (overall), important (SyncLeader subset)

Test Area: Utility Class
C# test count: 60 (UtilityClassTests.cs)
Rust test count: 60 (utility_tests.rs)
Gap: None
Severity: N/A

Test Area: All other 1:1 matches
(Frontmatter=18/18, SafeJson=4/4, ContentPreview=8/8, ConfigDecay=16/16,
 AccessBoosting=10/10, DateDetection=21/21, AssetHotLoading=4/4,
 AssetWatcher=10/10, ProximityWeight=22/22+8, FindSimilarPlateau=6/6,
 ConceptSyncEdge=5/5, WikiLinkRegex=6/6, WikiLinkFilter=6/6,
 WikiLinkFilterInteg=5/5, PathSecurity=6/6, DbMaintenance=7/7,
 TQualFsc2=1/1)
Gap: None
Severity: N/A
```

### ⚠️ Areas with Deficits

```
Test Area: SessionCleanup
C# test count: 17 (SessionCleanupTests.cs)
Rust test count: 6 (session_cleanup_boundary.rs)
Gap: -11 tests. C# tests both HandleSessionCleanup (per-type: null, memory, 
     already-abandoned, stale workflow, stale sequential, recent active, missing 
     modified field, invalid date) and TryMarkAbandonedFromLastIndexed 
     (returns true/false, updates frontmatter, appends section, preserves content, 
     exact threshold boundary). Rust tests at sweep level only.
Severity: important — abandonment edge cases are safety-critical for data integrity

Test Area: GraphAnalyzer
C# test count: 24 (GraphAnalyzerTests.cs)
Rust test count: 16 (graph_analyzer_tests.rs)
Gap: -8 tests. Missing: RandomWalk_TerminatesAtDeadEnd, 
     RandomWalk_ProducesDifferentResults, RandomWalk_HasDefaultWalkLength,
     plus TestCase(0) edge cases for depth and maxNodes validation,
     and Visualize_WithIsolatedConcept boundary.
Severity: important — random walk is a user-facing feature

Test Area: SyncLeader Election
C# test count: 31 (SyncLeaderElectionTests.cs)
Rust test count: 24 (sync_leader=17 + leader_boundary=7)
Gap: -7 tests. Missing: FR30-5 (GetSyncRole defaults, ShouldParticipateInElection,
     ShouldForceLeader), FR30-6 (follower emits skip diagnostic), FR30-7 (skip run
     outcomes auditable), FR30-8 (follower skips file sync, leader processes, follower
     skips delete, auto non-leader skips via election), FR30-9 (follower zero DB
     connections, zero log entries, zero resource allocation), FR30-10 (leader conditional
     logging for no-op sync, file processing, unsynced delete).
Severity: important — leader election correctness affects data consistency

Test Area: ToolRegistry
C# test count: 20 (ToolRegistryTests.cs)
Rust test count: 13 (tool_registry_tests.rs)
Gap: -7 tests. C# uses [TestCase("WRITEMEMORY")] etc. for parametrized case
     variants. Rust covers same semantics with fewer test functions (collapsed
     asserts). Functional coverage equivalent; numeric count differs.
Severity: cosmetic — parametrized test style difference, not missing functionality

Test Area: DecayCalculator
C# test count: 37 (DecayCalculatorCoverageTests.cs)
Rust test count: 32 (decay_calculator_tests.rs)
Gap: -5 tests. Missing: Power-law decay edge cases — within grace returns 1.0,
     grace boundary returns 1.0, past grace calculates effective age, different
     from exponential, case-insensitive function string.
Severity: cosmetic — power-law is a minor code path variant

Test Area: VectorSyncDiagnostics (minor)
C# test count: 12 (FR25_2_AC1_AC2_VectorSyncDiagnosticsTests.cs)
Rust test count: 9 (vector_sync_diag_tests.rs) + 5 (vector_sync_boundary.rs) = 14
Gap: Net +2 but 3 specific AC tests redistributed: AC7 (failure diagnostics
     normalized reason codes), AC8 (verification warning fallback), AC9 (explicit
     top-level status) may be folded into different tests.
Severity: cosmetic — coverage exists but organized differently
```

---

## Oracle Parity Tests (tests/compat/)

| File | Tests | Passing | Ignored |
|------|-------|---------|---------|
| cli_json_parity.rs | 13 | 13 | 0 |
| search_graph_parity.rs | 4 | 3 | 1 (T-10.19: BuildContext ExpandedRelations) |
| session_workflow_parity.rs | 4 | 2 | 2 (T-10.19: Workflow + AssumptionLedger JSON envelope) |
| vector_sync_parity.rs | 4 | 1 | 3 (T-10.19: FindSimilar, Sync, ExtractConcepts JSON envelope) |
| **Total** | **25** | **19** | **6** |

**Missing parity test files** (ALREADY KNOWN — T-1.6, T-2.4, T-3.2):
- Adopt, AnalyzeConceptCorruption, UpdateAssets (Sprint 1 — T-1.6 pending)
- RecentActivity, RunFullBenchmark (Sprint 2 — T-2.4 pending)
- RepairConcepts (Sprint 3 — T-3.2 pending)

---

## Rust-Only Tests (no C# counterpart)

| File | Count | Purpose |
|------|-------|---------|
| dynamic_model_tests.rs | 9 | Sprint 9: ONNX model discovery, dimension detection |
| contracts.rs | 6 | Rust API contract verification |
| Various *_boundary.rs | ~40+ | Supplemental boundary tests beyond C# scope |
| mcp_tests.rs | 6 | MCP stdio transport (Rust-specific) |

---

## Edge Case Coverage Matrix

| Category | C# | Rust | Status |
|----------|-----|------|--------|
| Unicode / special chars | 32 (MarkdownAdversarial) | 32 (markdown_adversarial) | ✅ Parity |
| Empty/null inputs | Spread across files | Spread across files | ✅ Parity |
| Large file sync | 21 (SyncPerformance) | 21 (sync_perf) | ✅ Parity |
| Concurrent access / leader | 31 (SyncLeaderElection) | 24 (sync_leader + boundary) | ⚠️ -7 |
| Path traversal / security | 35 (PathSec=6 + CliSec=29) | 35 (path_sec=6 + cli_sec=29) | ✅ Parity |
| Decay edge cases | 73 (func=31 + calc=37 + bench=5) | 68 (func=31 + calc=32 + bench=5) | ⚠️ -5 |

---

## Performance / Benchmark Tests

Exact parity across all 3 perf test areas:

| Area | C# | Rust |
|------|-----|------|
| SyncPerformance | 21 | 21 |
| DecayBenchmark | 5 | 5 |
| CliJsonPerformance | 2 | 2 |
| **Total** | **28** | **28** |

---

## Recommendations

### Important (should add before GA)

1. **SessionCleanup**: Add 11 boundary tests covering per-session-type handling, TryMarkAbandoned sub-API, threshold boundary, frontmatter preservation. Maps to abandonment data safety. (~T-new)

2. **GraphAnalyzer**: Add 8 tests for RandomWalk termination, reproducibility, default walk length, and depth/maxNodes=0 edge cases. (~T-new)

3. **SyncLeader**: Add 7 integration tests for follower resource zero-allocation (FR30-9), conditional logging (FR30-10), and sync role config (FR30-5). (~T-new)

### Cosmetic (nice-to-have)

4. **DecayCalculator**: Add 5 power-law edge case tests. Low risk since exponential path is well-tested.

5. **ToolRegistry**: Consider adding parametrized test variants for parity. Currently functionally equivalent despite count difference.

---

## Limitations

- Test count comparison is mechanical (regex-based). A single Rust test function with 10 `assert!` calls covers as much ground as 10 C# `[Test]` methods.
- Some C# `[TestCase]` variants inflate the C# count for what is logically one test.
- Code coverage percentage was not measured — this audit counts test methods, not line/branch coverage.
- The 6 already-known ignored parity tests and the 6 already-known missing parity test files were excluded from new findings per instructions.
