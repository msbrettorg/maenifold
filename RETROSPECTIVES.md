# Maenifold Sprint Retrospectives

This document captures sprint retrospectives to compound learnings across development cycles.

---

## Sprint 2026-02-01: CLI JSON Output (P0)

**Sprint Duration**: 2026-02-01
**Outcome**: T-CLI-JSON-001.1-8 Complete
**Priority**: P0 (Blocking OpenClaw integrations)

### Executive Summary

Delivered CLI JSON output feature (FR-8.1 through FR-8.5) with comprehensive security hardening. All 49 tests passing. Feature unblocks OpenClaw SessionChannel and Memory Plugin integrations.

### What Worked Well

1. **TDD Approach**: Writing `CliJsonOutputTests.cs` before implementation ensured complete coverage
   - 24 functional tests covering all FR-8.x requirements
   - Structured `{ success, data, error }` response format validated

2. **Security-First Development**: `CliJsonSecurityTests.cs` (819 lines) covered 8 attack vectors:
   - JSON injection via content (quotes, braces, newlines)
   - Unicode/encoding attacks (BOM, null bytes, control chars)
   - Large payload DoS protection
   - Error handling information leakage prevention
   - Concurrency safety in OutputContext
   - ANSI escape code handling

3. **Atomic RTM Items**: T-CLI-JSON-001.1 through T-CLI-JSON-001.8 enabled precise tracking

### Issues Encountered

1. **PathTooLongException Leak**: SEC-EDGE-002 discovered unhandled exception exposing internal paths
   - **Resolution**: Added structured `PATH_TOO_LONG` error response
   - **Lesson**: Always test boundary conditions with security lens

2. **OutputContext Thread Safety**: Initial implementation had race condition potential
   - **Resolution**: Implemented `AsyncLocal<T>` pattern with `UseJson()`/`UseMarkdown()` scopes
   - **Lesson**: CLI tools may be called concurrently; design for it

### Metrics

| Metric | Value |
|--------|-------|
| Tests Written | 49 |
| Tests Passing | 49 (100%) |
| Security Vectors Covered | 8 |
| Components Modified | 4 (`Program.cs`, `OutputContext.cs`, `JsonToolResponse.cs`, `SafeJson.cs`) |

### Learnings for Future Sprints

1. **Security tests find real bugs**: SEC-EDGE-002 found a legitimate vulnerability
2. **Thread-local context pattern works well**: Reuse for other cross-cutting concerns
3. **NFR-8.1.4 (no ANSI codes) was easy to verify**: Just check `Does.Not.Contain("\u001b[")`

### Related PRD/RTM

- PRD: FR-8.1, FR-8.2, FR-8.3, FR-8.4, FR-8.5, NFR-8.1.1-8.1.4
- RTM: T-CLI-JSON-001.1 through T-CLI-JSON-001.8

---

## Sprint 2026-02-01: Embeddings Quality (T-QUAL-FSC2)

**Sprint Duration**: 2026-02-01
**Outcome**: T-QUAL-FSC2.1-4 Complete
**Priority**: P1

### Executive Summary

Resolved FindSimilarConcepts embedding plateau issue. System now rejects degenerate embeddings and similarity scores are properly bounded to <= 1.000.

### What Worked Well

1. **Diagnostic Tests First**: `TQualFsc2HardMeasurementsTests.cs` identified embedding collapse before fixing
2. **Tokenization Compatibility**: NFR-7.4.1 ensured model vocab/IDs match, preventing silent failures
3. **Plateau Detection**: System now detects hash duplicates and zero-distance counts

### Issues Encountered

1. **Embedding Collapse Root Cause**: Initially unclear why unrelated concepts had 1.000 similarity
   - **Resolution**: Added tokenization diagnostics, found vocab mismatch
   - **Lesson**: Always validate embedding pipeline end-to-end

### Metrics

| Metric | Value |
|--------|-------|
| Diagnostic Tests | 4 |
| Regression Tests | 3 |
| Similarity Bound | <= 1.000 enforced |

### Related PRD/RTM

- PRD: FR-7.4, NFR-7.4.1, NFR-7.4.2
- RTM: T-QUAL-FSC2.1 through T-QUAL-FSC2.4

---

## Backlog: Decay Weighting (T-GRAPH-DECAY-001 through 004)

**Status**: Planned (not started)
**Priority**: P1/P2

### Scope

- T-GRAPH-DECAY-001: Recency decay weighting for search rankings
- T-GRAPH-DECAY-002: Access-frequency boosting via ReadMemory
- T-GRAPH-DECAY-003: ListMemories decay metadata display
- T-GRAPH-DECAY-004: Assumption decay by epistemic status

### Pre-Sprint Notes

1. **Config Layer First**: Start with T-GRAPH-DECAY-001.2-4 (env vars, defaults) before search integration
2. **Tiered Grace Periods**: Sequential=7d, Workflows=14d, Other=14d per PRD NFR-7.5.1-7.5.2
3. **Access Boosting Scope**: Only ReadMemory updates `last_accessed`, not SearchMemories or BuildContext

### Dependencies

- Requires decay weight calculation utility
- Requires `last_accessed` timestamp in file metadata
- May require database schema update for timestamp storage

---

## Retrospective Template

Use this template for future retrospectives:

```markdown
## Sprint [DATE]: [Feature Name]

**Sprint Duration**: [Start] - [End]
**Outcome**: [RTM items delivered]
**Priority**: [P0/P1/P2]

### Executive Summary
[2-3 sentences summarizing what was delivered]

### What Worked Well
1. [Success pattern 1]
2. [Success pattern 2]

### Issues Encountered
1. **[Issue Name]**: [Description]
   - **Resolution**: [How it was fixed]
   - **Lesson**: [What to do differently next time]

### Metrics
| Metric | Value |
|--------|-------|
| Tests Written | X |
| Tests Passing | X |

### Learnings for Future Sprints
1. [Learning 1]
2. [Learning 2]

### Related PRD/RTM
- PRD: [FR-X.X, NFR-X.X]
- RTM: [T-XXX-001.X]
```

---

*Last updated: 2026-02-02*
*Related: [[PRD]] [[RTM]] [[TODO]] [[agentic-slc]]*
