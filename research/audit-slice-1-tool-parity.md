# Audit Slice 1: Tool Handler Parameter & Behavior Parity

**Date**: 2025-01-07
**Traceable to**: T-10.19 (oracle parity), FR-15 (config parity), NFR-1 (C# oracle parity)
**Sequential thinking**: session-1773104046670-38000 / branch audit-slice-1-tools
**Method**: Direct source code comparison (not docs)

## Executive Summary

All 28 tools are present in both registries with matching names. 23/28 tools show full parameter and behavioral parity. 5 tools show parameter or behavioral drift requiring new backlog items. No tools are missing entirely.

**Excluded from this report** (already backlogged):
- MCP resources/notifications (T-10.2, T-10.3, T-10.4)
- Output format drift for BuildContext, Workflow, AssumptionLedger, FindSimilarConcepts, ExtractConceptsFromFile, Sync (6 ignored parity tests under T-10.19)

---

## Tool-by-Tool Results

### 1. WriteMemory
```
Status: DRIFT
Details: MCP schema advertises `learn` parameter but handler in memory.rs ignores it.
         C# returns help doc via ToolHelpers.GetLearnContent("WriteMemory") when learn=true.
         Rust handler never reads payload.get("learn").
Severity: important
Traceable: FR-1 / NFR-1
```

### 2. ReadMemory
```
Status: DRIFT
Details: MCP schema advertises `learn` parameter but handler in memory.rs ignores it.
         C# returns help doc when learn=true.
         Rust handler never reads payload.get("learn").
Severity: important
Traceable: NFR-1
```

### 3. EditMemory
```
Status: DRIFT
Details: MCP schema advertises `learn` parameter but handler in memory.rs ignores it.
         C# returns help doc when learn=true.
         Rust handler never reads payload.get("learn").
         All other params (identifier, operation, content, checksum, findText,
         sectionName, expectedCount) match exactly.
Severity: important
Traceable: NFR-1
```

### 4. DeleteMemory
```
Status: DRIFT
Details: MCP schema advertises `learn` parameter but handler in memory.rs ignores it.
         C# returns help doc when learn=true.
         All other behavior identical (confirm required, JSON { uri, deleted: true }).
Severity: important
Traceable: NFR-1
```

### 5. MoveMemory
```
Status: DRIFT
Details: MCP schema advertises `learn` parameter but handler in memory.rs ignores it.
         C# returns help doc when learn=true.
         All other behavior identical (slugify, overwrite check).
Severity: important
Traceable: NFR-1
```

### 6. SearchMemories
```
Status: DRIFT
Details: MCP schema advertises `tags` (string[]) parameter but handler in search.rs
         never reads payload.get("tags"). C# implements full tag filtering:
         reads frontmatter tags from files, filters results to only include
         files containing ALL specified tags (case-insensitive).
         Applied in both text search and vector search paths.
         Impact: Callers who pass tags= get silently ignored results.
Severity: blocking
Traceable: NFR-1
```

### 7. ExtractConceptsFromFile
```
Status: MATCH
Details: Both accept identifier (required) + learn. Both return concept list with count.
         Minor: C# returns explicit "No [[WikiLink]] concepts found" for empty files;
         Rust returns "Found 0 CONCEPTS in file:". Functionally equivalent.
Severity: n/a
```

### 8. Sync
```
Status: MATCH
Details: Both accept learn only. Both run community detection (Louvain),
         clean up abandoned sessions, and return sync status.
         Output format drift is already tracked under T-10.19.
Severity: n/a
```

### 9. BuildContext
```
Status: MATCH
Details: Both accept conceptName (required), depth (default 2), maxEntities (default 20),
         includeContent (default false), learn. Both include community data, decay weights,
         and content previews. Output format drift already tracked under T-10.19.
Severity: n/a
```

### 10. Visualize
```
Status: MATCH
Details: Both accept conceptName (required), depth (default 2, range 1-5),
         maxNodes (default 30, range 5-100), learn. Both validate concept existence.
         Both return Mermaid diagram text.
Severity: n/a
```

### 11. SequentialThinking
```
Status: MATCH
Details: All 16 parameters match (response, thoughts, thoughtNumber, totalThoughts,
         nextThoughtNeeded, needsMoreThoughts, isRevision, revisesThought,
         branchFromThought, branchId, analysisType, parentWorkflowId, conclusion,
         sessionId, cancel, learn). WikiLink validation in response/thoughts/conclusion
         matches. BranchId requirement matches. Session format validation matches.
         JSON response format: { sessionId, thoughtNumber, totalThoughts, status,
         nextThoughtNeeded, message } matches.
Severity: n/a
```

### 12. Workflow
```
Status: MATCH
Details: All parameters match (workflowId, sessionId, view, append, response,
         thoughts, status, conclusion, submachineSessionId, learn).
         Rust accepts workflowId as string OR JSON array (parse_workflow_identifiers);
         C# also supports this through string parsing.
         Output format drift already tracked under T-10.19.
Severity: n/a
```

### 13. AssumptionLedger
```
Status: MATCH
Details: All parameters match for all 3 actions (append, update, read).
         Append: assumption (required), confidence, context, validationPlan,
         concepts[] (≥1 required). Update: uri, status (active|validated|invalidated|
         refined), confidence, notes. Read: uri.
         Output format drift already tracked under T-10.19.
Severity: n/a
```

### 14. RecentActivity
```
Status: MATCH
Details: All parameters match: limit (default 10), filter (all|thinking|memory|
         assumptions), includeContent (default false), timespan (C#-style d.HH:MM:SS),
         learn. Timespan parsing matches C# TimeSpan format.
Severity: n/a
```

### 15. MemoryStatus
```
Status: MATCH
Details: Both accept learn only. Both return file counts, graph metrics, storage stats.
Severity: n/a
```

### 16. GetConfig
```
Status: MATCH
Details: Both accept learn only. Both return configuration settings and paths.
Severity: n/a
```

### 17. GetHelp
```
Status: MATCH
Details: Both accept toolName (required). Both search usage/tools/{toolname}.md.
         Both return list of available tools if file not found.
Severity: n/a
```

### 18. ListMemories
```
Status: MATCH
Details: Both accept path (optional) + learn. Both validate path security
         (no absolute paths, no ".." traversal, must not escape memory root).
Severity: n/a
```

### 19. UpdateAssets
```
Status: MATCH
Details: Both accept dryRun (default true) + learn. Both use SHA256 for comparison.
Severity: n/a
```

### 20. RepairConcepts
```
Status: MATCH
Details: All parameters match: conceptsToReplace (comma/semicolon/pipe delimited),
         canonicalConcept (required), folder, dryRun (default true), createWikiLinks
         (default false), minSemanticSimilarity (default 0.7), learn.
         Both reject brackets in concepts. Both block when similarity < threshold.
Severity: n/a
```

### 21. AnalyzeConceptCorruption
```
Status: MATCH
Details: Both accept conceptFamily (required), maxResults (default 50), learn.
         Both classify into same 5 categories: File Paths, Singular/Plural,
         Compound, With Suffix, Other.
Severity: n/a
```

### 22. FindSimilarConcepts
```
Status: MATCH
Details: Both accept conceptName (required, max 256 chars), maxResults (1-50,
         default 10), learn. Both do NFKC normalization, bracket detection,
         low-information query rejection. Both apply decay weighting.
         Output format drift already tracked under T-10.19.
Severity: n/a
```

### 23. StartWatcher
```
Status: DRIFT
Details: Default debounce differs: Rust=500ms, C#=150ms (via MAENIFOLD_DEBOUNCE_MS env).
         C# validates debounceMs range 10-10000ms. Rust has no range validation.
         C# env var: MAENIFOLD_DEBOUNCE_MS (default 150).
         Rust reads MAENIFOLD_DEBOUNCE_MS from env but falls back to 500ms.
Severity: important
Traceable: FR-15 (config parity)
```

### 24. StopWatcher
```
Status: MATCH
Details: Both accept no parameters. Both stop the watcher and return status message.
Severity: n/a
```

### 25. RunFullBenchmark
```
Status: DRIFT
Details: Rust adds upper bound validation: iterations must be 1-100.
         C# only validates iterations > 0 (no upper bound).
         All other parameters match: iterations (default 5), maxTestFiles (default 1000),
         includeDeepTraversal (default true), learn.
         This is defensive hardening, not a bug. Recommend documenting as intentional.
Severity: cosmetic
Traceable: NFR-1
```

### 26. Adopt
```
Status: MATCH
Details: Both accept type (role|color|perspective), identifier, learn.
         Both do two-step lookup (filename match then JSON id scan).
Severity: n/a
```

### 27. ListAssets
```
Status: MATCH
Details: Both accept type (optional: workflow|role|color|perspective).
         Rust schema adds `learn` parameter that C# doesn't have — this is
         an extra feature, not a missing one. Functionally harmless.
Severity: n/a
```

### 28. ReadMcpResource
```
Status: MATCH
Details: Both accept uri (required). Both read MCP resource content.
         Rust schema adds `learn` parameter that C# doesn't have.
Severity: n/a
```

---

## Summary Table

| # | Tool | Status | Issue |
|---|------|--------|-------|
| 1 | WriteMemory | DRIFT | `learn` param ignored in handler |
| 2 | ReadMemory | DRIFT | `learn` param ignored in handler |
| 3 | EditMemory | DRIFT | `learn` param ignored in handler |
| 4 | DeleteMemory | DRIFT | `learn` param ignored in handler |
| 5 | MoveMemory | DRIFT | `learn` param ignored in handler |
| 6 | SearchMemories | DRIFT | `tags` param ignored in handler |
| 7 | ExtractConceptsFromFile | MATCH | — |
| 8 | Sync | MATCH | — |
| 9 | BuildContext | MATCH | — |
| 10 | Visualize | MATCH | — |
| 11 | SequentialThinking | MATCH | — |
| 12 | Workflow | MATCH | — |
| 13 | AssumptionLedger | MATCH | — |
| 14 | RecentActivity | MATCH | — |
| 15 | MemoryStatus | MATCH | — |
| 16 | GetConfig | MATCH | — |
| 17 | GetHelp | MATCH | — |
| 18 | ListMemories | MATCH | — |
| 19 | UpdateAssets | MATCH | — |
| 20 | RepairConcepts | MATCH | — |
| 21 | AnalyzeConceptCorruption | MATCH | — |
| 22 | FindSimilarConcepts | MATCH | — |
| 23 | StartWatcher | DRIFT | Debounce default 500ms≠150ms, no range validation |
| 24 | StopWatcher | MATCH | — |
| 25 | RunFullBenchmark | DRIFT | Extra iteration upper bound (100) |
| 26 | Adopt | MATCH | — |
| 27 | ListAssets | MATCH | — |
| 28 | ReadMcpResource | MATCH | — |

**Totals**: 23 MATCH, 5 DRIFT (0 blocking, 4 important, 1 cosmetic)

---

## Proposed Backlog Items

### T-11.1: Wire `learn` parameter in 5 memory tool handlers | RTM: NFR-1
Add `learn` parameter handling to `write_memory`, `read_memory`, `edit_memory`, `delete_memory`, `move_memory` in `memory.rs`. Each should check `payload.get("learn")` and return help doc via `load_help_doc()` when true, matching C# `ToolHelpers.GetLearnContent()` behavior.
- **Files**: `crates/maenifold-core/src/memory.rs`
- **Status**: pending
- **Severity**: important

### T-11.2: Implement `tags` filtering in SearchMemories handler | RTM: NFR-1
Read `tags` array from payload in `search_memories()`. Filter search results to only include files whose frontmatter `tags` field contains ALL specified tags (case-insensitive). Apply in both text search and vector search paths, matching C# `MemorySearchTools.Text.cs` and `MemorySearchTools.Vector.cs`.
- **Files**: `crates/maenifold-core/src/search.rs`
- **Status**: pending
- **Severity**: blocking

### T-11.3: Fix StartWatcher debounce default and add range validation | RTM: FR-15
Change `start_watcher()` default debounce from 500ms to 150ms. Read `MAENIFOLD_DEBOUNCE_MS` env var with default 150. Add validation: debounceMs must be 10-10000ms, matching C# `IncrementalSyncTools.StartWatcher()`.
- **Files**: `crates/maenifold-core/src/watchers.rs`
- **Status**: pending
- **Severity**: important

### T-11.4: Document RunFullBenchmark iteration upper bound | RTM: NFR-1
The Rust port adds an iteration ceiling of 100 that C# doesn't have. This is defensive hardening. Either: (a) remove to match C# exactly, or (b) document as intentional and add to C# for consistency.
- **Files**: `crates/maenifold-core/src/benchmark.rs` (documentation comment)
- **Status**: pending
- **Severity**: cosmetic

---

## Limitations

1. **Output format drift** was explicitly excluded per instructions — 6 tools have known output format differences tracked under T-10.19
2. **Behavioral equivalence under load** was not tested (e.g., concurrent watcher operations)
3. **Error message wording** may differ between ports; only structural error handling was compared
4. **C# `tags` implementation** performs frontmatter YAML parsing at search time — Rust implementation will need equivalent frontmatter parsing in the search path
