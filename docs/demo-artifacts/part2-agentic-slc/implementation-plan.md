# Implementation Plan
## Sprint: 2025-01-21 Issue Fixes
## Session: workflow-1758474734596

## Task Decomposition & Agent Assignment Strategy

### Wave 1: SRCH-004 Implementation (3 Agents Parallel)

**Agent-SRCH-01: Method Signature Updates**
- Task 1: Add minScore parameter to SearchMemories (RTM-001, RTM-002)
- Task 2: Update all three search method signatures (RTM-003, RTM-004, RTM-005)
- Files: src/Tools/MemorySearchTools.cs

**Agent-SRCH-02: Filtering Implementation**
- Task 3: Implement hybrid search filtering (RTM-006)
- Task 4: Implement semantic search filtering (RTM-007)
- Task 5: Implement full-text normalization and filtering (RTM-008, RTM-009)
- Files: src/Tools/MemorySearchTools.Fusion.cs, MemorySearchTools.Vector.cs, MemorySearchTools.Text.cs

**Agent-SRCH-03: Test Creation**
- Task 6: Create SearchToolsTests.cs file
- Task 7: Implement minScore=0.0 test (RTM-010)
- Task 8: Implement minScore=1.0 test (RTM-011)
- Task 9: Implement mode consistency test (RTM-012)
- Files: tests/Maenifold.Tests/SearchToolsTests.cs

### Wave 2: MEM-009 Implementation (2 Agents Parallel)

**Agent-MEM-01: Path Validation Implementation**
- Task 10: Test current behavior with CLI (RTM-013)
- Task 11: Add path validation logic (RTM-015, RTM-016, RTM-017)
- Files: src/Tools/MemoryTools.Write.cs

**Agent-MEM-02: Test Creation**
- Task 12: Create MemoryPathTests.cs file
- Task 13: Test nested directory creation (RTM-018)
- Task 14: Test path traversal rejection (RTM-019)
- Task 15: Test cross-platform paths (RTM-020)
- Files: tests/Maenifold.Tests/MemoryPathTests.cs

### Wave 3: GRPH-009 Investigation & Implementation (2 Agents Sequential)

**Agent-GRPH-01: Performance Analysis**
- Task 16: Analyze current implementation (RTM-021)
- Task 17: Measure current performance baseline
- Task 18: Confirm if N+1 problem exists

**Agent-GRPH-02: CTE Implementation (IF NEEDED)**
- Task 19: Adapt CTE from GraphAnalyzer (RTM-022, RTM-023)
- Task 20: Add result limits (RTM-024)
- Task 21: Maintain output format (RTM-025)
- Task 22: Create performance test (RTM-026, RTM-027)
- Files: src/Tools/GraphTools.cs, tests/Maenifold.Tests/GraphPerformanceTests.cs

## Execution Strategy

### Parallel Execution Plan
```
TIME    WAVE 1 (SRCH)         WAVE 2 (MEM)          WAVE 3 (GRPH)
----    --------------         ------------          -------------
T+0     Agent-SRCH-01 ──┐      [waiting]             [waiting]
        Agent-SRCH-02 ──┤
        Agent-SRCH-03 ──┘
T+15    [complete]             Agent-MEM-01 ──┐      [waiting]
                               Agent-MEM-02 ──┘
T+25    [complete]             [complete]            Agent-GRPH-01
T+30    [complete]             [complete]            Agent-GRPH-02
```

### Dependencies
- Wave 1: No dependencies, can start immediately
- Wave 2: No dependencies, can start with/after Wave 1
- Wave 3: Sequential - GRPH-02 depends on GRPH-01 findings

### Task Distribution Summary
- **Total Agents**: 7
- **Parallel Agents**: 5 (SRCH x3 + MEM x2)
- **Sequential Agents**: 2 (GRPH investigation then implementation)
- **Total Tasks**: 22 atomic tasks
- **Estimated Duration**: 30-35 minutes

## Success Criteria Per Wave

### Wave 1 Success
- [ ] minScore parameter added with default 0.0
- [ ] All three search modes filter correctly
- [ ] All tests pass
- [ ] No breaking changes

### Wave 2 Success
- [ ] Path validation implemented
- [ ] Nested directories work correctly
- [ ] Security tests pass
- [ ] Cross-platform compatibility verified

### Wave 3 Success
- [ ] Performance issue investigated
- [ ] If needed, CTE implementation complete
- [ ] Performance improvement measured
- [ ] Output format unchanged

## Risk Mitigation
- Full-text score normalization may need iteration
- MEM-009 may not actually need fixing (test first)
- GRPH-009 only implement if performance issue confirmed

## Git Coordination
All agents work in branch: sprint-20250121-issue-fixes
No commits during implementation (PM handles at end)