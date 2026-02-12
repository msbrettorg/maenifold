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

## Sprint 2026-02-02: Decay Weighting (T-GRAPH-DECAY-001 through 004)

**Sprint Duration**: 2026-02-02
**Outcome**: T-GRAPH-DECAY-001.1-6, 002.1-3, 003.1-2, 004.1-3 Complete
**Priority**: P1

### Executive Summary

Delivered recency-based decay weighting for all search rankings. Content naturally decays based on age, with tiered grace periods (sequential=7d, workflows=14d, default=14d) and configurable half-life (default 30d). ReadMemory updates `last_accessed` to boost frequently-accessed content. ListMemories displays decay metadata. Assumption decay varies by epistemic status.

### What Worked Well

1. **Parallel Agent Execution**: Ran 4+ agents simultaneously throughout sprint
   - 4 researchers in parallel for initial codebase mapping
   - 6 blue-team agents writing tests concurrently
   - 5 SWE agents implementing tool integration in parallel

2. **TDD with Blue-Team First**: Tests defined requirements precisely
   - DecayFunctionTests (31 tests) covered all edge cases
   - AccessBoostingTests (10 tests) verified access patterns
   - Red-team found only 1 real gap (env vars) because tests were thorough

3. **Database Schema Migration**: `last_accessed` column added cleanly
   - Used SQLite `ALTER TABLE` with error handling for idempotency
   - `ON CONFLICT DO UPDATE` preserves timestamps during re-sync

### Issues Encountered

1. **Missing ConfigDecayDefaultsTests**: Blue-team claimed to write but file wasn't created
   - **Resolution**: Discovered during red-team review, SWE wrote tests with implementation
   - **Lesson**: Verify test files exist before marking complete

2. **CLI JSON Output Regression**: JSON support was accidentally removed from tools
   - **Resolution**: SWE restored `OutputContext.IsJsonMode` checks
   - **Lesson**: Regression tests should run after every change

3. **Test Database Schema**: Tests failing with "no such column: last_accessed"
   - **Resolution**: Ensured `AddMissingColumns()` runs for test databases
   - **Lesson**: Database migrations must work for both production and test DBs

### Metrics

| Metric | Value |
|--------|-------|
| Tests Added | 91 (ConfigDecayDefaults=16, DecayFunction=31, AccessBoosting=10, GraphDecay=8, AssumptionDecay=17, ListMemories=9) |
| Total Tests | 429 passing, 11 skipped |
| Components Modified | 9 (Config, DecayCalculator, GraphDatabase, MemoryTools, MemorySearchTools, MemorySearchTools.Fusion, GraphTools, VectorSearchTools, SystemTools) |
| Agents Used | 15+ (4 researchers, 6 blue-team, 5+ SWE, 1 red-team) |

### Learnings for Future Sprints

1. **Verify file creation**: Check test files exist before marking blue-team tasks complete
2. **Run full test suite after each agent**: Catch regressions early
3. **Database migrations need test coverage**: Test databases use fresh schemas
4. **Red-team finds real gaps**: H-1 (env vars) was a legitimate PRD compliance issue

### Related PRD/RTM

- PRD: FR-7.5, FR-7.6, FR-7.7, FR-7.8, NFR-7.5.1-7.5.5, NFR-7.6.1-7.6.3, NFR-7.7.1, NFR-7.8.1-7.8.3
- RTM: T-GRAPH-DECAY-001.1-6, T-GRAPH-DECAY-002.1-3, T-GRAPH-DECAY-003.1-2, T-GRAPH-DECAY-004.1-3

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

## Sprint 2026-02-12: Date Detection Fix (T-DATE-001)

**Sprint Duration**: 2026-02-12
**Outcome**: RTM-001 through RTM-006 Complete
**Priority**: P1 (User-reported bug)

### Executive Summary

Fixed date detection bug in `MarkdownWriter.GetSessionPath` where `LastIndexOf('-')` parsed the random suffix instead of the Unix timestamp from session IDs (`session-{ts}-{random}`), storing all sessions under `1970/01/01/`. Also fixed downstream issues: `ExtractSessionId` returning year instead of filename, `IsValidSessionIdFormat` validating wrong segment, UTC suffix missing from 4 timestamp sites, and `InvariantCulture` missing from 2 ISO 8601 sites.

### What Worked Well

1. **Parallel SWE Dispatch**: 4 SWE agents implemented 6 RTM items simultaneously across 5 source files with zero merge conflicts
   - Each agent got a non-overlapping file scope (MarkdownWriter, SequentialThinkingTools, WorkflowOperations, RecentActivity.Reader)
   - All 4 completed within ~60s, reducing wall clock from ~4min serial to ~1min parallel

2. **3-File Test Split**: Splitting 21 tests across 3 files (PathTests, ValidationTests, TimestampTests) enabled parallel SWE dispatch for test writing
   - No file conflicts between test agents
   - Each file covers exactly 2 RTM items

3. **Red Team Found Real Regression**: WorkflowOperationsTests used `test-session-{ts}` format incompatible with prefix-aware parsing. Without red team audit, 18 pre-existing tests would have remained broken.

4. **RTM Traceability Discipline**: Every commit references RTM items. Red team triage table in RTM.md documents disposition of every finding.

### Issues Encountered

1. **WorkflowOperationsTests Regression (18 failures)**: Test helper `CreateTestSession` used invalid `test-session-{ts}` format
   - **Resolution**: Changed to production format `workflow-{ts}` (commit a435e20)
   - **Lesson**: Test helpers must use production-valid formats, not test-only conventions that accidentally work

2. **Build Policy Blocked Sprint**: `sprint-baseline.txt` in project root triggered `Directory.Build.targets` disallowed-files error
   - **Resolution**: Moved to `docs/sprint-baseline.txt` (commit 7ccf24f)
   - **Lesson**: Keep sprint artifacts in `docs/` from the start

3. **SWE Agents Skipped Traceability Reads**: All 4 SWE agents confessed to not reading PRD/RTM/TODO before implementation
   - **Resolution**: Tasks were self-contained enough that this didn't cause defects, but protocol was violated
   - **Lesson**: Include explicit "read these files first" in SWE task prompts, or accept that well-scoped tasks don't need it

### Metrics

| Metric | Value |
|--------|-------|
| Tests Written | 21 (6 path + 11 validation + 4 timestamp) |
| Tests Passing | 450 (full suite, 0 failures) |
| Source Files Modified | 5 |
| Sprint Commits | 11 |
| Net Lines Changed | +535 / -22 |
| Agents Dispatched | 11 (4 SWE impl + 3 SWE test + 2 red-team + 1 blue-team + 1 SWE fix) |

### Learnings for Future Sprints

1. **Test helpers are attack surface**: Red team should audit test setup code, not just production code
2. **Prefix-aware parsing is fragile**: Any session ID format that deviates from `{prefix}-{timestamp}[-{suffix}]` will break. Document the contract.
3. **3-file test split enables parallelism**: When RTM items map to independent functions, split test files by RTM pair for parallel agent dispatch
4. **Red team triage table is valuable**: Documenting ACCEPT/REJECT/DEFER for every finding creates auditable decision trail

### Related PRD/RTM

- SPECS: SPECS-sprint-20260212.md (FR-1, FR-2, FR-3, NFR-1, NFR-2)
- RTM: RTM-001 through RTM-006 (all Complete)

---

*Last updated: 2026-02-12*
*Related: [[PRD]] [[RTM]] [[TODO]] [[agentic-slc]]*
