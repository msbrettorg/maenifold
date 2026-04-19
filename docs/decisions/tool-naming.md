# ADR: MCP Tool Naming

- Status: Accepted
- Date: 2026-04-18
- Scope: `crates/maenifold-core/src/registry.rs`

## Problem statement

The MCP tool registry currently exposes canonical PascalCase names and accepts lowercase/snake_case aliases for compatibility. We need a clear naming decision that:

1. Preserves a stable public surface for clients.
2. Avoids churn from decomposing compound tool names into smaller fragmented tools.
3. Documents the 29 canonical tools and their migration/alias behavior.

Without a documented decision, naming changes risk breaking integrations and increasing maintenance overhead.

## Options considered

### Option A — Keep PascalCase canonical names (chosen)

- Canonical tool names remain PascalCase.
- Lowercase/snake_case aliases remain temporary compatibility paths.
- Compound tool names are preserved where they represent one coherent operation.

### Option B — Switch canonical names to lowercase/snake_case

- Make lowercase/snake_case the primary API.
- Keep PascalCase as compatibility aliases.
- Requires broad external/client updates and documentation rewrite.

### Option C — Decompose compound tools into multiple smaller tools

- Split tools like `BuildContext`, `RunFullBenchmark`, `AnalyzeConceptCorruption`, etc.
- Increases tool count and coordination overhead.
- Pushes orchestration complexity to clients.

## Decision

We adopt **Option A**:

1. **Keep PascalCase canonical tool names.**
2. **Keep compound tools** when they capture a single domain action.
3. **Reject decomposition** of existing compound tools.
4. Maintain aliases for compatibility with a **6-month deprecation window**.

## 29-tool renaming table (legacy alias → canonical)

| # | Legacy name | Canonical name |
|---|---|---|
| 1 | `writememory` | `WriteMemory` |
| 2 | `readmemory` | `ReadMemory` |
| 3 | `editmemory` | `EditMemory` |
| 4 | `deletememory` | `DeleteMemory` |
| 5 | `movememory` | `MoveMemory` |
| 6 | `searchmemories` | `SearchMemories` |
| 7 | `extractconceptsfromfile` | `ExtractConceptsFromFile` |
| 8 | `sync` | `Sync` |
| 9 | `buildcontext` | `BuildContext` |
| 10 | `visualize` | `Visualize` |
| 11 | `sequentialthinking` | `SequentialThinking` |
| 12 | `workflow` | `Workflow` |
| 13 | `pipeline` | `Pipeline` |
| 14 | `recentactivity` | `RecentActivity` |
| 15 | `memorystatus` | `MemoryStatus` |
| 16 | `getconfig` | `GetConfig` |
| 17 | `gethelp` | `GetHelp` |
| 18 | `listmemories` | `ListMemories` |
| 19 | `updateassets` | `UpdateAssets` |
| 20 | `repairconcepts` | `RepairConcepts` |
| 21 | `analyzeconceptcorruption` | `AnalyzeConceptCorruption` |
| 22 | `findsimilarconcepts` | `FindSimilarConcepts` |
| 23 | `runfullbenchmark` | `RunFullBenchmark` |
| 24 | `adopt` | `Adopt` |
| 25 | `assumptionledger` | `AssumptionLedger` |
| 26 | `listassets` | `ListAssets` |
| 27 | `readmcpresource` | `ReadMcpResource` |
| 28 | `listsessions` | `ListSessions` |
| 29 | `acpexec` | `AcpExec` |

## Alias table and 6-month deprecation policy

Deprecation window: **6 months from ADR acceptance date** (target removal review: **2026-10-18**).

| Alias | Canonical | Deprecation target |
|---|---|---|
| `writememory` | `WriteMemory` | 2026-10-18 |
| `readmemory` | `ReadMemory` | 2026-10-18 |
| `editmemory` | `EditMemory` | 2026-10-18 |
| `deletememory` | `DeleteMemory` | 2026-10-18 |
| `movememory` | `MoveMemory` | 2026-10-18 |
| `searchmemories` | `SearchMemories` | 2026-10-18 |
| `extractconceptsfromfile` | `ExtractConceptsFromFile` | 2026-10-18 |
| `sync` | `Sync` | 2026-10-18 |
| `buildcontext` | `BuildContext` | 2026-10-18 |
| `visualize` | `Visualize` | 2026-10-18 |
| `sequentialthinking` | `SequentialThinking` | 2026-10-18 |
| `workflow` | `Workflow` | 2026-10-18 |
| `pipeline` | `Pipeline` | 2026-10-18 |
| `recentactivity` | `RecentActivity` | 2026-10-18 |
| `memorystatus` | `MemoryStatus` | 2026-10-18 |
| `getconfig` | `GetConfig` | 2026-10-18 |
| `gethelp` | `GetHelp` | 2026-10-18 |
| `listmemories` | `ListMemories` | 2026-10-18 |
| `updateassets` | `UpdateAssets` | 2026-10-18 |
| `repairconcepts` | `RepairConcepts` | 2026-10-18 |
| `analyzeconceptcorruption` | `AnalyzeConceptCorruption` | 2026-10-18 |
| `findsimilarconcepts` | `FindSimilarConcepts` | 2026-10-18 |
| `runfullbenchmark` | `RunFullBenchmark` | 2026-10-18 |
| `adopt` | `Adopt` | 2026-10-18 |
| `assumptionledger` | `AssumptionLedger` | 2026-10-18 |
| `listassets` | `ListAssets` | 2026-10-18 |
| `readmcpresource` | `ReadMcpResource` | 2026-10-18 |
| `listsessions` | `ListSessions` | 2026-10-18 |
| `list_sessions` | `ListSessions` | 2026-10-18 |
| `acpexec` | `AcpExec` | 2026-10-18 |
| `acp_exec` | `AcpExec` | 2026-10-18 |

## Consequences

### Positive

- Stable and explicit canonical API for MCP tool consumers.
- Lower migration risk because aliases preserve compatibility.
- Better discoverability and readability of tool names.
- Avoids unnecessary expansion of tool surface area by decomposing compound actions.

### Negative / trade-offs

- Alias maintenance adds temporary compatibility burden.
- Some clients may delay migration while aliases exist.
- Future naming improvements require ADR updates and deprecation process discipline.

### Follow-up expectations

- Keep registry canonical names unchanged unless superseded by a new ADR.
- Prefer canonical PascalCase names in docs, examples, and generated artifacts.
- At 6 months, review alias usage and remove aliases that no longer have dependency demand.
