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
| T-GRAPH-DECAY-005.1 | FR-7.9 | System SHALL support periodic consolidation of high-value episodic content via Cognitive Sleep Cycle workflow. | assets/workflows/sleep-cycle.json | Manual workflow execution | **Complete** |
| T-GRAPH-DECAY-005.2 | FR-7.9 | Consolidation SHALL distill episodic (thinking/) content into semantic (memory://) notes with WikiLinks. | assets/workflows/sleep-cycle.json | Manual workflow execution | **Complete** |

## T-GOV-RETRO-001: Retrospectives log

| T-ID | PRD FR/NFR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|------------|----------------------|--------------|---------|--------|
| T-GOV-RETRO-001.1 | FR-9.1 | Repository SHALL include `RETROSPECTIVES.md`. | RETROSPECTIVES.md | N/A | **Complete** |
| T-GOV-RETRO-001.2 | FR-9.1 | RETROSPECTIVES.md SHALL contain retrospective template and sprint entries. | RETROSPECTIVES.md | N/A | **Complete** |

