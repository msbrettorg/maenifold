# Ma Core Hero Demo - Test Matrix and Orchestration Plan

## Executive Overview

**Date**: January 21, 2025
**Orchestrator**: PM-lite Protocol (Blue Hat)
**Test Coverage**: 100% of Ma Core functionality
**Agent Count**: 12 specialized agents across 4 waves
**Coordination**: Shared sequential thinking session

## Agent Wave Architecture

### 🌊 Wave 1: Discovery and Analysis (3 Agents)
**Purpose**: Establish baseline and discover current state
**Duration**: ~5 minutes
**Parallelization**: Full concurrent execution

| Agent ID | Specialization | Test Assignments |
|----------|---------------|------------------|
| DISC-01 | System Auditor | Environment validation, dependency check, build verification |
| DISC-02 | Graph Analyst | Database inspection, concept count, relationship mapping |
| DISC-03 | Memory Explorer | Memory file structure, WikiLink patterns, content analysis |

### 🌊 Wave 2: Core Functionality Testing (4 Agents)
**Purpose**: Test all primary operations
**Duration**: ~10 minutes
**Parallelization**: Full concurrent execution

| Agent ID | Specialization | Test Assignments |
|----------|---------------|------------------|
| CORE-01 | CRUD Specialist | Create, Read, Update, Delete, Move operations |
| CORE-02 | Search Expert | Hybrid, Semantic, FullText search modes |
| CORE-03 | Graph Engineer | Sync, BuildContext, Visualize, concept repair |
| CORE-04 | Thinking Systems | Sequential thinking, workflows, session persistence |

### 🌊 Wave 3: Integration and Stress Testing (3 Agents)
**Purpose**: Complex scenarios and edge cases
**Duration**: ~8 minutes
**Parallelization**: Full concurrent execution

| Agent ID | Specialization | Test Assignments |
|----------|---------------|------------------|
| INTG-01 | CLI/MCP Validator | Parity testing, response format verification |
| INTG-02 | Performance Tester | Large-scale operations, timing benchmarks |
| INTG-03 | Edge Case Hunter | Error handling, boundary conditions, recovery |

### 🌊 Wave 4: Verification and Validation (2 Agents)
**Purpose**: Final verification and report compilation
**Duration**: ~5 minutes
**Parallelization**: Sequential for accuracy

| Agent ID | Specialization | Test Assignments |
|----------|---------------|------------------|
| VRFY-01 | Quality Auditor | Cross-check all test results, verify fixes |
| VRFY-02 | Report Compiler | Generate final report with metrics |

## Test Coverage Matrix

### Memory Operations (15 tests)
| Test ID | Test Name | Agent | Wave | Priority |
|---------|-----------|-------|------|----------|
| MEM-001 | Create memory with WikiLinks | CORE-01 | 2 | Critical |
| MEM-002 | Read existing memory | CORE-01 | 2 | Critical |
| MEM-003 | Edit memory content | CORE-01 | 2 | High |
| MEM-004 | Delete memory | CORE-01 | 2 | High |
| MEM-005 | Move memory (bug fix verification) | CORE-01 | 2 | Critical |
| MEM-006 | Extract concepts from file | CORE-01 | 2 | High |
| MEM-007 | Large file handling | INTG-02 | 3 | Medium |
| MEM-008 | Unicode content | INTG-03 | 3 | Low |
| MEM-009 | Nested directories | CORE-01 | 2 | High |
| MEM-010 | Invalid paths | INTG-03 | 3 | Medium |
| MEM-011 | Concurrent writes | INTG-02 | 3 | Medium |
| MEM-012 | Memory status check | DISC-01 | 1 | High |
| MEM-013 | Recent activity | DISC-03 | 1 | Medium |
| MEM-014 | File permissions | INTG-03 | 3 | Low |
| MEM-015 | Backup/restore | INTG-01 | 3 | Low |

### Search Operations (10 tests)
| Test ID | Test Name | Agent | Wave | Priority |
|---------|-----------|-------|------|----------|
| SRCH-001 | Hybrid search with RRF | CORE-02 | 2 | Critical |
| SRCH-002 | Semantic vector search | CORE-02 | 2 | Critical |
| SRCH-003 | Full-text search | CORE-02 | 2 | Critical |
| SRCH-004 | Similarity threshold | CORE-02 | 2 | High |
| SRCH-005 | Multi-word queries | CORE-02 | 2 | High |
| SRCH-006 | Empty results | INTG-03 | 3 | Medium |
| SRCH-007 | Large result sets | INTG-02 | 3 | Medium |
| SRCH-008 | Special characters | INTG-03 | 3 | Low |
| SRCH-009 | Performance benchmark | INTG-02 | 3 | High |
| SRCH-010 | Ranking accuracy | VRFY-01 | 4 | High |

### Graph Operations (12 tests)
| Test ID | Test Name | Agent | Wave | Priority |
|---------|-----------|-------|------|----------|
| GRPH-001 | Sync WikiLinks to graph | CORE-03 | 2 | Critical |
| GRPH-002 | Build context from concept | CORE-03 | 2 | Critical |
| GRPH-003 | Visualize relationships | CORE-03 | 2 | High |
| GRPH-004 | Concept repair | CORE-03 | 2 | Critical |
| GRPH-005 | Find similar concepts | CORE-03 | 2 | High |
| GRPH-006 | Graph statistics | DISC-02 | 1 | High |
| GRPH-007 | Orphaned concepts | CORE-03 | 2 | Medium |
| GRPH-008 | Circular references | INTG-03 | 3 | Medium |
| GRPH-009 | Large graph traversal | INTG-02 | 3 | Medium |
| GRPH-010 | Relationship weights | DISC-02 | 1 | Medium |
| GRPH-011 | Database integrity | VRFY-01 | 4 | Critical |
| GRPH-012 | Incremental sync | CORE-03 | 2 | High |

### Thinking Systems (8 tests)
| Test ID | Test Name | Agent | Wave | Priority |
|---------|-----------|-------|------|----------|
| THINK-001 | Sequential thinking session | CORE-04 | 2 | Critical |
| THINK-002 | Thought revision | CORE-04 | 2 | High |
| THINK-003 | Session persistence | CORE-04 | 2 | High |
| THINK-004 | Workflow execution | CORE-04 | 2 | Critical |
| THINK-005 | Workflow queue | CORE-04 | 2 | High |
| THINK-006 | List workflows | DISC-03 | 1 | Medium |
| THINK-007 | Multi-step reasoning | INTG-01 | 3 | High |
| THINK-008 | Session recovery | INTG-03 | 3 | Medium |

### System Operations (5 tests)
| Test ID | Test Name | Agent | Wave | Priority |
|---------|-----------|-------|------|----------|
| SYS-001 | Environment validation | DISC-01 | 1 | Critical |
| SYS-002 | Configuration check | DISC-01 | 1 | High |
| SYS-003 | CLI execution | INTG-01 | 3 | Critical |
| SYS-004 | MCP server mode | INTG-01 | 3 | Critical |
| SYS-005 | Build verification | DISC-01 | 1 | Critical |

## Orchestration Protocol

### Phase 1: Initialization
1. Create shared sequential thinking session
2. Initialize test database and memory space
3. Deploy monitoring infrastructure
4. Begin orchestration log

### Phase 2: Wave Execution
1. **Wave 1 Launch** (T+0:00)
   - Deploy DISC-01, DISC-02, DISC-03 concurrently
   - Monitor via BashOutput for completion
   - Collect baseline metrics

2. **Wave 2 Launch** (T+0:05)
   - Deploy CORE-01, CORE-02, CORE-03, CORE-04 concurrently
   - Track test execution progress
   - Log any failures for immediate fix

3. **Wave 3 Launch** (T+0:15)
   - Deploy INTG-01, INTG-02, INTG-03 concurrently
   - Focus on edge cases and performance
   - Identify any critical issues

4. **Wave 4 Launch** (T+0:23)
   - Deploy VRFY-01 sequentially
   - Then VRFY-02 for final report
   - Compile all results

### Phase 3: Adaptation Protocol
- Monitor agent output in real-time
- Deploy fix agents for any critical bugs found
- Update test matrix with actual results
- Document all adaptations made

### Phase 4: Report Generation
1. Compile agent results
2. Add sequential thinking session
3. Generate metrics dashboard
4. Update E2E_TEST_REPORT.md

## Success Metrics

### Critical Path Tests (Must Pass)
- [ ] Build compiles without warnings
- [ ] Move operation preserves .md extension
- [ ] All search modes return results
- [ ] Graph sync completes without errors
- [ ] Sequential thinking persists
- [ ] CLI/MCP parity confirmed

### Performance Targets
- Read operations: < 100ms
- Write operations: < 500ms
- Search operations: < 150ms
- Graph traversal: < 200ms
- Sync operations: < 60s for 2500 files

### Quality Gates
- Zero build warnings
- 100% CLI/MCP parity
- No semantic damage in concept repair
- All critical tests passing
- Performance within targets

## Adaptive Orchestration Rules

1. **If critical test fails:**
   - Immediately dispatch coding-agent for fix
   - Re-run failed test after fix
   - Document fix in orchestration log

2. **If performance degrades:**
   - Dispatch performance analysis agent
   - Identify bottleneck
   - Document but continue testing

3. **If agent times out:**
   - Kill agent process
   - Re-dispatch with adjusted parameters
   - Log timeout incident

4. **If unexpected behavior:**
   - Dispatch investigation agent
   - Document finding
   - Determine if blocker or continue

## Coordination Mechanism

All agents will:
1. Read from shared sequential thinking session
2. Write their findings to designated memory locations
3. Update test results in real-time
4. Report completion via structured output

The PM (Blue Hat) will:
1. Monitor all agent outputs
2. Make real-time adjustments
3. Ensure quality gates are met
4. Compile final orchestration report

---

*This orchestration plan demonstrates the PM-lite protocol's ability to coordinate complex multi-agent testing while maintaining Ma Protocol principles of transparency and real system testing.*