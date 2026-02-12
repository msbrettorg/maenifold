# Requirements Traceability Matrix (RTM)

## T-CLI-JSON-001: CLI JSON Output (P0 - Highest Priority)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-CLI-JSON-001.1 | FR-8.1 | CLI SHALL support `--json` flag for structured JSON output. | src/Program.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.2 | FR-8.2 | All MCP tools SHALL return JSON when `--json` flag present. | src/Tools/*.cs, src/Utils/OutputContext.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.3 | FR-8.3, NFR-8.1.3 | JSON SHALL include `success`, `data`, `error` fields. | src/Utils/JsonToolResponse.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.4 | FR-8.4 | Error responses SHALL include error codes and messages. | src/Utils/JsonToolResponse.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.5 | FR-8.5 | Omitting `--json` SHALL return markdown (backward compat). | src/Program.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.6 | NFR-8.1.1 | JSON output SHALL be valid UTF-8 parseable JSON. | src/Utils/SafeJson.cs | tests/Maenifold.Tests/CliJsonOutputTests.cs | **Complete** |
| T-CLI-JSON-001.7 | NFR-8.1.4 | JSON output SHALL NOT include ANSI escape codes. | src/Utils/OutputContext.cs | tests/Maenifold.Tests/CliJsonSecurityTests.cs | **Complete** |
| T-CLI-JSON-001.8 | Security | PathTooLongException SHALL return structured error (SEC-EDGE-002). | src/Tools/MemoryTools.cs | tests/Maenifold.Tests/CliJsonSecurityTests.cs | **Complete** |

**Blocks**: OpenClaw SessionChannel JSON migration, OpenClaw Memory Plugin

---

## T-QUAL-FSC2: FindSimilarConcepts plateau remediation

| T-ID | PRD FR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|--------|----------------------|--------------|---------|--------|
| T-QUAL-FSC2.1 | FR-7.4 | Tokenization MUST be compatible with model vocab/IDs; mismatches fail closed. | src/Utils/VectorTools.Onnx.cs, src/Utils/VectorTools.Embeddings.cs | tests/Maenifold.Tests/TQualFsc2TokenizerDiagnosticsTests.cs | Complete |
| T-QUAL-FSC2.2 | FR-7.4 | FindSimilarConcepts MUST reject degenerate embeddings and similarity plateaus. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | Complete |
| T-QUAL-FSC2.3 | FR-7.4 | Similarity output MUST be bounded to <= 1.000. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | Complete |
| T-QUAL-FSC2.4 | FR-7.4 | Diagnostics MUST identify embedding collapse (hash duplicates / zero-distance counts). | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | Complete |

## T-GRAPH-DECAY-001: Recency decay weighting

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-001.1 | FR-7.5 | Apply recency-based decay weighting to all search result rankings. | src/Tools/MemorySearchTools.Fusion.cs, src/Tools/GraphTools.cs, src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/GraphDecayWeightingTests.cs | **Complete** |
| T-GRAPH-DECAY-001.2 | NFR-7.5.1 | `memory/thinking/sequential/` grace period: 7 days (`MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.2a | NFR-7.5.1a | `memory/thinking/workflows/` grace period: 14 days (`MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.3 | NFR-7.5.2 | All other memory grace period: 14 days (`MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.4 | NFR-7.5.3 | Default half-life 30 days (`MAENIFOLD_DECAY_HALF_LIFE_DAYS`). | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ConfigDecayDefaultsTests.cs | **Complete** |
| T-GRAPH-DECAY-001.5 | NFR-7.5.4 | Decay affects ranking only; content remains fully retrievable via direct query. | src/Tools/MemoryTools.cs | tests/Maenifold.Tests/GraphDecayWeightingTests.cs | **Complete** |
| T-GRAPH-DECAY-001.6 | NFR-7.5.5 | Optional power-law decay (`R = a × t^(-b)`) via `MAENIFOLD_DECAY_FUNCTION` env var. | src/Utils/Config.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/DecayFunctionTests.cs | **Complete** |

## T-GRAPH-DECAY-002: Access-frequency boosting

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-002.1 | FR-7.6, NFR-7.6.1 | ReadMemory SHALL update `last_accessed` timestamp on every read. | src/Tools/MemoryTools.cs, src/Tools/GraphDatabase.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |
| T-GRAPH-DECAY-002.2 | NFR-7.6.2 | SearchMemories SHALL NOT update `last_accessed` for results. | src/Tools/MemorySearchTools.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |
| T-GRAPH-DECAY-002.3 | NFR-7.6.3 | BuildContext SHALL NOT update `last_accessed` for referenced files. | src/Tools/GraphTools.cs | tests/Maenifold.Tests/AccessBoostingTests.cs | **Complete** |

## T-GRAPH-DECAY-003: ListMemories decay metadata

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-003.1 | FR-7.7 | ListMemories SHALL display `created`, `last_accessed`, `decay_weight` for each file. | src/Tools/SystemTools.cs | tests/Maenifold.Tests/ListMemoriesMetadataTests.cs | **Complete** |
| T-GRAPH-DECAY-003.2 | NFR-7.7.1 | decay_weight SHALL use file's tier (sequential=7d, workflows=14d, other=14d grace). | src/Tools/SystemTools.cs, src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/ListMemoriesMetadataTests.cs | **Complete** |

## T-GRAPH-DECAY-004: Assumption decay by status

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-004.1 | FR-7.8, NFR-7.8.1 | `validated` assumptions SHALL NOT decay. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |
| T-GRAPH-DECAY-004.2 | NFR-7.8.2 | `active` and `refined` assumptions: 14d grace, 30d half-life. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |
| T-GRAPH-DECAY-004.3 | NFR-7.8.3 | `invalidated` assumptions: 7d grace, 14d half-life. | src/Utils/DecayCalculator.cs | tests/Maenifold.Tests/AssumptionDecayTests.cs | **Complete** |

## T-GRAPH-DECAY-005: Memory consolidation (Cognitive Sleep Cycle)

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GRAPH-DECAY-005.1 | FR-7.9 | System SHALL support periodic consolidation of high-value episodic content via Cognitive Sleep Cycle workflow. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |
| T-GRAPH-DECAY-005.2 | FR-7.9 | Consolidation SHALL distill episodic (thinking/) content into semantic (memory://) notes with WikiLinks. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |

## T-SLEEP-MULTI-001: Multi-agent sleep cycle architecture

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SLEEP-MULTI-001.1 | FR-7.10, NFR-7.10.2 | Sleep orchestrator SHALL dispatch 4 specialist agents in parallel. | assets/workflows/memory-cycle.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.2 | FR-7.10, NFR-7.10.1 | Consolidation workflow SHALL include tailored memory replay + consolidation + dream synthesis. | assets/workflows/memory-consolidation.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.3 | FR-7.10, NFR-7.10.1 | Decay workflow SHALL include tailored memory replay + decay processing. | assets/workflows/memory-decay.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.4 | FR-7.10, NFR-7.10.1 | Repair workflow SHALL include tailored memory replay + concept repair. | assets/workflows/memory-repair.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.5 | FR-7.10, NFR-7.10.1 | Epistemic workflow SHALL include tailored memory replay + assumption review. | assets/workflows/memory-epistemic.json | Manual workflow execution | **Complete** |
| T-SLEEP-MULTI-001.6 | NFR-7.10.3 | Each specialist workflow SHALL be executable independently of orchestrator. | assets/workflows/memory-*.json | Manual workflow execution | **Complete** |

## T-SLEEP-SAFETY-001: Tool access safety during sleep

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-SLEEP-SAFETY-001.1 | FR-7.11, NFR-7.11.1 | Consolidation workflow MAY use `read_memory` (intentional access boosting). | assets/workflows/memory-consolidation.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.2 | FR-7.11, NFR-7.11.2 | Decay workflow SHALL use `list_memories` NOT `read_memory`. | assets/workflows/memory-decay.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.3 | FR-7.11, NFR-7.11.3 | Repair workflow SHALL use graph tools NOT `read_memory`. | assets/workflows/memory-repair.json | Manual workflow execution | **Complete** |
| T-SLEEP-SAFETY-001.4 | FR-7.11, NFR-7.11.4 | Epistemic workflow SHALL use `assumption_ledger` NOT `read_memory`. | assets/workflows/memory-epistemic.json | Manual workflow execution | **Complete** |

## T-GOV-RETRO-001: Retrospectives log

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GOV-RETRO-001.1 | FR-9.1 | Repository SHALL include `RETROSPECTIVES.md`. | RETROSPECTIVES.md | N/A | **Complete** |
| T-GOV-RETRO-001.2 | FR-9.1 | RETROSPECTIVES.md SHALL contain retrospective template and sprint entries. | RETROSPECTIVES.md | N/A | **Complete** |

---

## T-DATE-001: Session path date detection fix (sprint-20260212)

### Requirements Traceability Matrix

MUST HAVE (Atomic Functionality):
| T-ID | Spec | Requirement (Atomic) | Component(s) | Test(s) | Commit | Status |
|------|------|----------------------|--------------|---------|--------|--------|
| RTM-001 | FR-1 | GetSessionPath SHALL strip session ID prefix and parse timestamp from first numeric segment after prefix. | src/Utils/MarkdownWriter.cs:52-59 | DateDetectionPathTests.cs (6 tests) | 255a0e4, c09d1df | **Complete** |
| RTM-002 | FR-1 | GetSessionPath SHALL produce date directory path matching the UTC date of the Unix timestamp (not 1970/01/01). | src/Utils/MarkdownWriter.cs:52-59 | DateDetectionPathTests.cs (6 tests) | 255a0e4, c09d1df | **Complete** |
| RTM-003 | FR-2 | IsValidSessionIdFormat SHALL validate the timestamp segment (not random suffix) is a valid long. | src/Tools/SequentialThinkingTools.cs:184-194 | DateDetectionValidationTests.cs (7 tests) | d864c2b, a5e1829 | **Complete** |
| RTM-004 | FR-3 | ExtractSessionId SHALL return session ID from last path segment, not segments[1]. | src/Tools/RecentActivity.Reader.cs:117-123 | DateDetectionValidationTests.cs (4 tests) | b64482c, a5e1829 | **Complete** |
| RTM-005 | NFR-1 | All human-readable timestamps (4 sites) SHALL include " UTC" suffix. | src/Tools/SequentialThinkingTools.cs:250, src/Tools/WorkflowOperations.Core.cs:51, src/Tools/WorkflowOperations.Management.cs:65,160 | DateDetectionTimestampTests.cs (2 tests) | d864c2b, b91008e, c312959 | **Complete** |
| RTM-006 | NFR-2 | ISO 8601 timestamps in FinalizeSession SHALL use CultureInfo.InvariantCulture. | src/Tools/SequentialThinkingTools.cs:303,323 | DateDetectionTimestampTests.cs (2 tests) | d864c2b, c312959 | **Complete** |

Note: RTM-001 and RTM-002 are atomically coupled — both implemented in commit 255a0e4 (same function, same lines).

MUST NOT HAVE:
| T-ID | Constraint |
|------|-----------|
| RTM-X01 | No backward compatibility shims for old 1970/01/01/ paths |
| RTM-X02 | No scope creep beyond the 6 changes in SPECS-sprint-20260212.md |
| RTM-X03 | No changes to SessionCleanup.cs or SystemTools.cs (adjacent scope) |
| RTM-X04 | No refactoring of unrelated code |

ESCAPE HATCHES:
- Non-atomic requirement discovered → STOP and re-decompose
- Git diff exceeds RTM scope → STOP immediately
- Existing 1970/01/01/ sessions: document migration approach but do not implement in this sprint

### Red Team Triage (Step 9 → Step 10)

| Finding | Disposition | Rationale |
|---------|-------------|-----------|
| C-001 (git): RTM-002 missing from commit 255a0e4 message | REMEDIATED | Added commit column to RTM; documented atomic coupling |
| C-002 (git/code): No test files | PLANNED | Wave 2 (Step 12) — tests are next workflow phase |
| C-003 (git): Out-of-scope commit e8d1c0e | REJECTED | Pre-existing commit (Feb 6) in branch lineage, not sprint scope |
| CRITICAL-001 (code): FormatException in GetSessionPath | REJECTED | Already handled by SessionExists catch block (MarkdownWriter.cs:64-72) |
| CRITICAL-003 (code): Path traversal via negative timestamps | REJECTED | Session IDs are system-generated (not user input); RTM-X02 scope constraint |
| HIGH-001 (code): Timestamp range validation | DEFERRED | Out of sprint scope (RTM-X02); system-generated IDs |
| HIGH-002 (code): ExtractSessionId empty array | REJECTED | False positive — String.Split('/') returns [""] not [] |
| MEDIUM-002 (code): Hardcoded format strings | DEFERRED | Out of sprint scope (RTM-X04 no refactoring) |

