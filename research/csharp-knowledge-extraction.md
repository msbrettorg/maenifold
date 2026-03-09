# C# Maenifold Institutional Knowledge Extraction

> **Task**: T-0.2 (cross-cutting prerequisite for all sprint work)
> **Date**: 2025-07-17
> **Source**: `/Users/brett/src/ma-collective/maenifold` (C# codebase)
> **Target**: `/Users/brett/src/ma-collective/maenifold-rs` (Rust port)

## Executive Summary

Systematic extraction of all institutional knowledge embedded in the C# maenifold codebase before retirement. The C# repo contains **67 source files** across `src/Tools/`, `src/Utils/`, and `src/Models/`, with **100+ traceability references** (FR-*, NFR-*, T-*) and critical design decisions that must be preserved in the Rust port.

**Critical findings:**
1. **3 active bugs/workarounds** that Rust must address differently
2. **27 environment variable configurations** with specific defaults and rationale
3. **Complex algorithm parameters** (Louvain gamma, decay exponents, ACT-R model) with academic citations
4. **Security hardening** (SafeJson depth limits, path traversal prevention, Unicode bracket-confusable filtering)
5. **Leader election protocol** with fencing epochs for multi-process sync safety

---

## 1. Unresolved TODOs / Active Bugs

### BUG-DUP-H1-001: Duplicate H1 Heading Prevention
- **File**: `src/Tools/MemoryTools.cs:263`
- **Comment**: `// BUG-DUP-H1-001: Prevent duplicate H1 headings`
- **Test**: `tests/Maenifold.Tests/MemoryToolsTests.cs:328` — regression test exists
- **Context**: When writing memory files, the system must prevent creating duplicate `# Title` headings. The C# code has an explicit guard.
- **Rust status**: NEEDS VERIFICATION — check if `WriteMemory` handler prevents duplicate H1s
- **Priority**: HIGH

### NOTE: Workflow Step Not Advanced on Submachine Register (FR-10.4)
- **File**: `src/Tools/WorkflowOperations.Management.cs:215`
- **Comment**: `// NOTE: currentStep intentionally NOT advanced (FR-10.4)`
- **Context**: When a submachine (SequentialThinking session) is registered with a workflow, the current step is **intentionally not advanced**. The step only advances when the submachine completes and the workflow resumes. This is a deliberate design decision, not a bug.
- **Rust status**: NEEDS ATTENTION — Workflow handler must preserve this behavior
- **Priority**: CRITICAL (behavioral correctness)

### NOTE: Workflow Sessions Use Singular Path
- **File**: `src/Tools/ConceptSync.cs:33`
- **Comment**: `// NOTE: workflow sessions are stored under thinking/workflow/ (singular) in current path scheme.`
- **Context**: Despite other references to "workflows" (plural), the actual session storage uses `thinking/workflow/` (singular). This is a path convention that must be preserved exactly.
- **Rust status**: NEEDS VERIFICATION
- **Priority**: HIGH

### NOTE: sqlite-vec Negative Distance Rows
- **File**: `tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs:140`
- **Comment**: `// NOTE: We do not attempt to seed negative distance rows directly because sqlite-vec's distance functions never return negative values under normal operation`
- **Context**: The distance functions in sqlite-vec should never return negative values. The test infrastructure doesn't seed negative distances because they'd be rejected by the extension. However, the runtime code still guards against them (belt-and-suspenders).
- **Rust status**: NEEDS ATTENTION — ensure Rust also guards against NaN/Infinity/negative distances
- **Priority**: MEDIUM

---

## 2. Known Workarounds

### CA1812 / CA1861 / CA2100 Suppressions
- **File**: `src/Tools/ConceptSync.cs:11` — `// CA1812: EdgeData is instantiated via JSON/Dapper deserialization, not direct construction`
- **File**: `src/Tools/ToolRegistry.cs:4` — `// CA1861: Array params in ToolDescriptor constructors are intentionally inline for readability`
- **File**: `src/Utils/SqliteExtensions.cs:74` — `// CA2100: SQL strings are constructed internally, not from user input. Prepared statements used for all user-facing data.`
- **Rust status**: N/A — Rust doesn't have these C# analyzer warnings, but the underlying safety principles apply
- **Priority**: LOW

### CI Journal Mode Workaround
- **File**: `src/Utils/SqliteExtensions.cs`
- **Code**: `private static readonly bool IsCI = Environment.GetEnvironmentVariable("CI") == "true" || Environment.GetEnvironmentVariable("GITHUB_ACTIONS") == "true";`
- **Comment**: `// CI environments (GitHub Actions, etc.) use tmpfs for /tmp which has issues with WAL mode. Use DELETE journal mode in CI for reliability; WAL mode in production for performance.`
- **Context**: In CI environments, WAL mode can fail on tmpfs filesystems. The C# code automatically falls back to DELETE journal mode.
- **Rust status**: NEEDS ATTENTION — Rust should replicate this CI detection
- **Priority**: HIGH (affects CI reliability)

### Connection Pooling Disabled
- **File**: `src/Utils/Config.cs`
- **Code**: `public static string DatabaseConnectionString => $"Data Source={DatabasePath};Pooling=false";`
- **Comment**: `// RTM: CONNECTION-POOLING-FIX-001 - Disable connection pooling to prevent lock contention`
- **Context**: SQLite connection pooling was causing lock contention in multi-tool scenarios. Pooling is explicitly disabled.
- **Rust status**: NEEDS ATTENTION — Rust's rusqlite doesn't pool by default, but verify no connection reuse causes similar issues
- **Priority**: MEDIUM

### WARNING: Nested WikiLinks in EditMemory find_replace
- **File**: `src/Tools/MemoryTools.cs:102-105`
- **Comment**:
  ```
  /// WARNING: This creates nested WikiLinks if replacing text INSIDE existing [[brackets]].
  /// Example: [[machine learning]] with findText="machine learning" and replaceText="[[ML]]"
  /// becomes [[[[ML]]]] (nested brackets).
  ```
- **Context**: The `find_replace` operation in EditMemory is string-based, not AST-aware. Users must use `findText='[[machine learning]]'` to replace the entire WikiLink. This is documented in tool descriptions.
- **Rust status**: NEEDS ATTENTION — same behavior must be preserved (it's documented in the MCP tool description)
- **Priority**: HIGH (user-facing behavior)

---

## 3. Design Decisions (Rationale That Must Be Preserved)

### 3.1 MarkdownIO Boundary Pattern (T-MDIO-001.2)
- **File**: `src/Utils/MarkdownIO.cs`
- **Design**: MarkdownIO is the **boundary between URI-land and path-land**.
  - Above MarkdownIO: everything uses `memory://` URIs
  - Below it (MarkdownReader, MarkdownWriter): only file paths
  - Tools MUST NOT call MarkdownWriter/MarkdownReader directly
- **Comment**: `// T-MDIO-001.2: RTM FR-28.1–28.11. MarkdownIO is the boundary between URI-land (tools) and path-land (filesystem).`
- **URI Validation**: `ValidateUri()` checks for null, empty, missing `memory://` prefix, and path traversal (`..`)
- **Rust status**: ALREADY IMPLEMENTED — verify the boundary is respected
- **Priority**: CRITICAL (architectural invariant)

### 3.2 Decay Calculator Design (T-GRAPH-DECAY-001.*)
- **File**: `src/Utils/DecayCalculator.cs`
- **Design**: Two decay models supported:
  1. **Exponential decay** (opt-in): `weight = 0.5^((age - grace) / halfLife)`
  2. **Power-law decay** (default, ACT-R): `R = a * t^(-b)` where a=1.0, b=0.5 (Wixted & Ebbesen, 1991)
- **Grace periods are tiered by file type**:
  - `thinking/sequential/` or `thinking/compaction/`: 7 days (NFR-7.5.1)
  - `thinking/workflows/`: 14 days (NFR-7.5.1a)
  - All other memory: 28 days (NFR-7.5.2)
- **Assumption-specific decay** (FR-7.8):
  - `validated`: No decay (weight = 1.0 always, exempt)
  - `active` / `refined`: 14d grace, 30d half-life
  - `invalidated`: 7d grace, 14d half-life (aggressive decay)
- **Access boosting**: Uses `lastAccessed` timestamp when available
- **Negative age**: Treated as 0 (very recent content, weight = 1.0)
- **Rust status**: ALREADY IMPLEMENTED — verify parameter values match exactly
- **Priority**: CRITICAL (behavioral parity)

### 3.3 Louvain Community Detection (T-COMMUNITY-001.*)
- **File**: `src/Tools/CommunityDetection.cs`
- **Design**:
  - Phase 1 only (local node moves) — no Phase 2 aggregation. Sufficient for current graph scale.
  - Deterministic via optional seed (Fisher-Yates shuffle, NFR-13.1.2)
  - Edge weight = `sum(proximity × decay)` from `source_files` JSON
  - NMI (Normalized Mutual Information) drift detection at threshold 0.8
  - Atomic DELETE+INSERT in transaction (T-COMMUNITY-001.11, FAIL-001 remediation)
- **Algorithm**: Blondel et al. 2008, with modifications for decay-weighted edges
- **Rust status**: ALREADY IMPLEMENTED — verify NMI computation and atomic persistence
- **Priority**: MEDIUM (algorithm parity)

### 3.4 Leader Election Protocol (T-SYNC-LEADER-001.*)
- **File**: `src/Tools/SyncLeaderElection.cs`
- **Design**: SQLite lease-based cross-process leader election with:
  - `BEGIN IMMEDIATE` transactions for atomic lease operations
  - Fencing epoch prevents split-brain (incremented on ownership change)
  - `SQLITE_BUSY` (error code 5) handled as `Contended` (not `Error`)
  - Release uses UPDATE (not DELETE) to preserve epoch for continuity
  - Three roles via `MAENIFOLD_SYNC_ROLE`: `auto` (default), `leader` (deterministic override), `follower` (skips election)
  - Lease duration: `MAENIFOLD_SYNC_LEASE_MS` default 10000, clamped [1000, 60000]
- **Rust status**: ALREADY IMPLEMENTED — verify fencing epoch logic
- **Priority**: HIGH (concurrency safety)

### 3.5 ConceptRepairTool Two-Mode Design (FR-6)
- **File**: `src/Tools/ConceptRepairTool.cs` (~639 lines)
- **Mode A (WikiLink Replacement)**: Regex `\[\[variant\]\]` → `[[canonical]]`. Simple string replacement across all memory files.
- **Mode B (WikiLink Creation)**: Uses Markdig AST walker to:
  - Find plain-text mentions in paragraphs, headings, and list items
  - Skip code blocks (`CodeBlock`, `FencedCodeBlock`) and inline code (`CodeInline`)
  - Skip existing WikiLinks (`LinkInline`)
  - Check for adjacent `[[` / `]]` brackets to prevent double-wrapping
  - Uses `\b` word boundary regex for variant matching
  - Applies replacements in reverse order (by position) to preserve offsets
- **Semantic Safety**: Cosine similarity validation blocks operation when any variant is below `minSemanticSimilarity` threshold (default 0.7). Graceful degradation if ONNX/vec0 unavailable.
- **Rust status**: NOT YET IMPLEMENTED (T-3.1) — this is the most complex remaining tool
- **Priority**: CRITICAL (must implement for parity)

### 3.6 Reciprocal Rank Fusion (RRF) for Hybrid Search
- **File**: `src/Tools/MemorySearchTools.Fusion.cs`
- **Design**: `k=60` parameter for RRF scoring: `score = 1/(k + rank + 1)`
- Combines vector search results and full-text search results
- Applies decay weighting to fused scores: `finalScore = rrfScore * decayWeight`
- Text scores normalized: `Math.Min(textResults[i].score / 100.0, 1.0)`
- **Rust status**: ALREADY IMPLEMENTED — verify k=60 and normalization
- **Priority**: MEDIUM

### 3.7 Adopt Tool Two-Step Lookup
- **File**: `src/Tools/AdoptTools.cs`
- **Design**: 
  1. Fast path: direct filename match (`{id}.json`)
  2. Fallback: scan all JSON files in folder, parse each, check `id` property
- Uses `SafeJson.DocumentOptions` for parsing
- Returns structured LLM adoption instruction with 5-point behavioral protocol
- **Rust status**: NOT YET IMPLEMENTED (T-1.1) — straightforward to port
- **Priority**: HIGH

### 3.8 SafeJson Depth Limit (SEC-001)
- **File**: `src/Utils/SafeJson.cs`
- **Design**: `MaxDepth = 32` on both `JsonSerializerOptions` and `JsonDocumentOptions`
- **Comment**: `/// SEC-001: Safe JSON deserialization options to prevent DoS via deeply nested JSON`
- Used consistently across the codebase for all JSON operations
- **Rust status**: NEEDS VERIFICATION — ensure serde_json depth limits are in place
- **Priority**: HIGH (security)

---

## 4. Constants / Parameters (Must Match Exactly)

### 4.1 Environment Variables with Defaults

| Variable | Default | RTM | Notes |
|----------|---------|-----|-------|
| `MAENIFOLD_ROOT` | `~/maenifold` | — | Root directory |
| `MAENIFOLD_DATABASE_PATH` | `{root}/memory.db` | — | Database location |
| `MAENIFOLD_DEBOUNCE_MS` | `150` | — | File watcher debounce |
| `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY` | `40` (min 1) | — | Optimize after N syncs |
| `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES` | `720` | — | Vacuum interval (12h) |
| `MAENIFOLD_AUTO_SYNC` | `true` | — | Enable auto-sync |
| `MAENIFOLD_WATCHER_BUFFER` | `65536` | — | FSWatcher buffer size |
| `MAENIFOLD_SNIPPET_LENGTH` | `1000` | — | RecentActivity snippet length |
| `MAENIFOLD_SQLITE_BUSY_TIMEOUT` | `5000` | — | SQLite busy timeout (ms) |
| `MAENIFOLD_SESSION_ABANDON_MINUTES` | `30` | — | Session cleanup threshold |
| `MAENIFOLD_SESSION_CLEANUP` | `true` | — | Enable session cleanup |
| `MAENIFOLD_SYNC_LOGGING` | `true` | — | Sync log output |
| `MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL` | `7` | NFR-7.5.1 | Sequential thinking grace period |
| `MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS` | `14` | NFR-7.5.1a | Workflow grace period |
| `MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT` | `28` | NFR-7.5.2 | Default memory grace period |
| `MAENIFOLD_DECAY_HALF_LIFE_DAYS` | `30` | NFR-7.5.3 | Exponential decay half-life |
| `MAENIFOLD_DECAY_FUNCTION` | `power-law` | NFR-7.5.5 | Decay model (power-law \| exponential) |
| `MAENIFOLD_LOUVAIN_GAMMA` | `1.0` | NFR-13.11.1 | Louvain resolution parameter |
| `MAENIFOLD_COMMUNITY_MIN_SHARED` | `3` | NFR-13.9.1 | Min shared neighbors for siblings |
| `MAENIFOLD_COMMUNITY_MIN_OVERLAP` | `0.4` | NFR-13.9.1 | Min normalized overlap for siblings |
| `MAENIFOLD_COMMUNITY_MAX_SIBLINGS` | `10` | NFR-13.9.1 | Max community siblings returned |
| `MAENIFOLD_COMMUNITY_DEBOUNCE_MS` | `2000` | NFR-13.5.1 | Community recomputation debounce |
| `MAENIFOLD_COMMUNITY_NMI_THRESHOLD` | `0.8` | FR-23.7 | NMI drift alert threshold |
| `MAENIFOLD_SYNC_LEASE_MS` | `10000` (clamped [1000,60000]) | FR-30.5 | Sync leader lease duration |
| `MAENIFOLD_SYNC_ROLE` | `auto` | FR-30.5 | Sync role (auto\|leader\|follower) |
| `MAENIFOLD_EMBEDDING_LOGS` | `false` | — | Embedding debug logs |
| `MAENIFOLD_VECTOR_LOGS` | `false` | — | Vector search debug logs |

### 4.2 Algorithm Constants

| Constant | Value | Source | Notes |
|----------|-------|--------|-------|
| Power-law scale `a` | `1.0` | ACT-R | DecayCalculator |
| Power-law exponent `b` | `0.5` | ACT-R (Wixted & Ebbesen 1991) | "Memory halves when time quadruples" |
| RRF `k` parameter | `60` | Standard RRF | MemorySearchTools.Fusion |
| SafeJson MaxDepth | `32` | SEC-001 | DoS prevention |
| SQLITE_BUSY error code | `5` | SQLite | SyncLeaderElection |
| Embedding dimensions | `384` | all-MiniLM-L6-v2 | VectorTools.EmbeddingLength |
| FindSimilarConcepts max | `50` | T-QUAL-FSC2 | Hard cap on maxResults |
| Concept name max length | `256` | T-QUAL-FSC2 | Input validation |
| Smart preview target | `200` chars | BUILDCONTEXT-PREVIEW-UX-001 | Default preview length |
| Smart preview tolerance | `50` chars | BUILDCONTEXT-PREVIEW-UX-001 | Sentence boundary search zone |
| Assumption grace (active) | `14` days | FR-7.8 | Direct in code, not env var |
| Assumption half-life (active) | `30` days | FR-7.8 | Direct in code, not env var |
| Assumption grace (invalidated) | `7` days | FR-7.8 | Aggressive decay |
| Assumption half-life (invalid) | `14` days | FR-7.8 | Aggressive decay |

### 4.3 SQLite PRAGMA Configuration
```
PRAGMA busy_timeout={MAENIFOLD_SQLITE_BUSY_TIMEOUT}  -- default 5000
PRAGMA journal_mode=WAL (production) / DELETE (CI)
PRAGMA synchronous=NORMAL
PRAGMA query_only=ON (read-only connections)
PRAGMA read_uncommitted=ON (read-only connections)
```

---

## 5. Traceability Map

### FR References Found in C# Source (67 unique)

| FR | Description | Files |
|----|-------------|-------|
| FR-1.1 | Memory tools | MemoryTools.cs |
| FR-7.4 | FindSimilarConcepts validation | VectorSearchTools.cs |
| FR-7.5 | Decay weighting | DecayCalculator.cs, MemorySearchTools.Fusion.cs, VectorSearchTools.cs, GraphTools.cs |
| FR-7.7 | Decay in concept graph | DecayCalculator.cs |
| FR-7.8 | Assumption-specific decay | DecayCalculator.cs |
| FR-8.1–8.5 | Memory operations | MemoryTools.cs |
| FR-10.4 | Workflow step not advanced on submachine register | WorkflowOperations.Management.cs |
| FR-13.1–13.11 | Community detection | CommunityDetection.cs, CommunityDetectionRunner.cs |
| FR-14.1–14.6 | Sequential thinking | SequentialThinkingTools.cs |
| FR-16.10 | NULL guard in SQL queries | SqliteExtensions.cs |
| FR-23.1–23.7 | Proximity/edge weighting | CommunityDetection.cs, GraphTools.cs |
| FR-24.1–24.6 | Asset resources | AssetResources.cs |
| FR-25.2–25.6 | Quality/diagnostics | Various (diagnostic format) |
| FR-27.* | Metadata/config | SystemTools.cs |
| FR-28.1–28.11 | MarkdownIO URI boundary | MarkdownIO.cs |
| FR-30.1–30.10 | Sync leader election | SyncLeaderElection.cs, IncrementalSyncTools.Processing.cs |
| FR-33.1–33.6 | Assumption ledger | AssumptionLedgerTools.cs |

### NFR References Found (25 unique)

| NFR | Description |
|-----|-------------|
| NFR-7.5.1–7.5.5 | Decay grace periods and function selection |
| NFR-7.6.1 | Decay in fusion scoring |
| NFR-7.7.1 | Decay in graph traversal |
| NFR-7.8.x | Assumption status-based decay |
| NFR-13.1.1–13.11.1 | Community detection performance/determinism |
| NFR-14.2.2 | Sequential thinking limits |
| NFR-30.3–30.7 | Leader election atomicity/fencing |

### T-* Task References Found (55+ unique)

Key task families:
- `T-GRAPH-DECAY-001.*` — Decay implementation (7 sub-tasks)
- `T-COMMUNITY-001.*` — Community detection (8 sub-tasks)
- `T-SYNC-LEADER-001.*` — Leader election (5 sub-tasks)
- `T-PROX-001.*`, `T-PROX-003.*` — Proximity weighting (6 sub-tasks)
- `T-QUAL-FSC2.*` — FindSimilarConcepts quality (extensive validation)
- `T-QUAL-SYNC-001.*` — Sync quality (4 sub-tasks)
- `T-MDIO-001.2` — MarkdownIO URI boundary
- `T-PERF-001-SWE` through `T-PERF-004-SWE` — Performance optimizations

### Special Bug/Fix References

| Reference | Description | File |
|-----------|-------------|------|
| BUG-DUP-H1-001 | Duplicate H1 heading prevention | MemoryTools.cs |
| FAIL-001 | Atomic community persistence fix | CommunityDetection.cs |
| MEDIUM-001 | Persist response before registration | WorkflowOperations.Management.cs |
| SEC-001 | JSON depth limit DoS prevention | SafeJson.cs, AssetManager.cs |
| ISSUE-005 | Validate depth parameter | GraphTools.cs |
| ISSUE-006 | Validate iterations parameter | PerformanceBenchmark.cs |
| GRPH-009 | CTE vs N+1 benchmark claim | PerformanceBenchmark.Graph.cs |
| CONNECTION-POOLING-FIX-001 | Disable connection pooling | Config.cs |
| BUILDCONTEXT-PREVIEW-UX-001 | Sentence-aware preview | GraphTools.cs |

---

## 6. Test Infrastructure Notes

### AssetManager Test Path
- **File**: `tests/Maenifold.Tests/AssetManagerTests.cs:343`
- **Comment**: `/// NOTE: The source path is always AppDomain.CurrentDomain.BaseDirectory + "assets".`
- Source path is the build output directory's `assets/` folder, not the repo source

### Workflow Supervisor State
- **File**: `tests/Maenifold.Tests/WorkflowSupervisorTests.cs:358`
- **Comment**: `// NOTE: No 'phase', no 'activeSubmachineType', no 'activeSubmachineSessionId'`
- When workflow has no submachine, these frontmatter keys should be absent (not present with empty values)

### RecentActivity Filter Paths
- **File**: `src/Tools/RecentActivity.Reader.cs`
- **Thinking filter**: `memory://{Path.GetFileName(Config.ThinkingPath)}/%` (e.g., `memory://thinking/%`)
- **Assumptions filter**: `memory://chat/%` (NOT `memory://assumptions/`)
- **Memory filter**: NOT thinking AND NOT chat
- **GOTCHA**: The C# codebase uses `memory://chat/%` for assumptions, which differs from what you might expect. This is a known path convention that must be preserved.

### File Type Classification
```
thinking/sequential/ → "sequential"
thinking/workflow/  → "workflow"  (note: singular!)
thinking/*         → "thinking"
everything else    → "memory"
```

---

## 7. Edge Cases Documented in Comments

### 7.1 Unicode Bracket Confusables (T-QUAL-FSC2)
- **File**: `src/Tools/VectorSearchTools.cs`
- **Context**: FindSimilarConcepts performs NFKC normalization on input to detect Unicode bracket confusables (CJK corners, mathematical brackets, dingbat brackets — 16 pairs blocked)
- **Low-information query rejection**: Blocks emoji-only, punctuation-only, combining-marks-only queries
- **Similarity plateau detection**: If top-10 results have near-identical distances (within 1e-9), reports diagnostic
- **Zero-magnitude embedding detection**: `sumSq > 1e-12` threshold
- **Priority**: HIGH — Rust must replicate these guards

### 7.2 SQLite INTEGER Returns Int64 
- **File**: `src/Utils/SqliteExtensions.cs:152`
- **Comment**: `// SQLite returns INTEGER as Int64; convert to target type for ValueTuple construction.`
- **Also**: `src/Tools/CommunityDetection.cs` — `// Use long for community_id to match SQLite INTEGER return type (avoids Int64→Int32 cast failure).`
- **Rust status**: N/A — rusqlite handles this differently, but be aware of type coercion

### 7.3 Decay Fallback Hierarchy
- **File**: `src/Tools/MemorySearchTools.Fusion.cs`
- When `GetDecayWeightForFile` is called:
  1. First try: Read `created` and `last_accessed` from `file_content` DB table (no filesystem I/O)
  2. Fallback: If no DB row, read frontmatter from disk via `GetDecayWeightFromDisk`
  3. Final fallback: On any error, return `1.0` (neutral weight) and log `[DEGRADED]` diagnostic
- **Pattern**: All degradation follows `[DEGRADED][Module][Method]` format

### 7.4 WikiLinkFilter Static Cache
- **File**: `src/Utils/WikiLinkFilter.cs`
- Uses file modification time (`mtime`) caching with lock for thread safety
- Filter file: `{MemoryPath}/.wikilink-filter.json`
- Malformed JSON treated as empty filter (no error)
- `ResetCache()` is test-only (internal)

### 7.5 ConceptSync Path Convention
- **File**: `src/Tools/ConceptSync.cs:33`
- Workflow sessions stored under `thinking/workflow/` (singular)
- This is distinct from `thinking/workflows/` used for decay grace period matching
- **GOTCHA**: The grace period path matching uses `thinking/workflows` (plural) but actual storage uses `thinking/workflow` (singular). Both patterns must be recognized in decay calculation.

### 7.6 AssetManager Initialization vs Update
- **File**: `src/Utils/AssetManager.cs`
- `InitializeAssets()`: Only copies files that DON'T exist in target (never overwrites)
- `UpdateAssets()`: Compares by file size + SHA256 hash, overwrites if different
- Source path: `AppDomain.CurrentDomain.BaseDirectory + "assets"` (build output)
- Target path: `Config.AssetsPath` (runtime `{root}/assets/`)

### 7.7 Community Detection Edge Weight Computation
- **File**: `src/Tools/CommunityDetection.cs`
- Edge weight = `sum(proximity × decay)` where:
  - `proximity` comes from `source_files` JSON dict values
  - `decay` = `MemorySearchTools.GetDecayWeightForFile(uri, conn)` with connection reuse
  - Conn reuse is critical for performance: `O(edges) → O(1)` connections
- `ComputeEdgeDecayWeight` returns **average** of `(proximity × decay)` across all source files

### 7.8 IsHiddenPath Logic
- **File**: `src/Utils/MarkdownIO.cs`
- Checks if **any segment** of the relative path starts with a dot (e.g., `.git`, `.obsidian`, `.trash`)
- Both forward and backslash normalized before comparison
- Single-char dots (`.`) are NOT treated as hidden (must be `segment.Length > 1`)

### 7.9 Performance Benchmark Insufficient Data Guard
- **File**: `src/Tools/PerformanceBenchmark.cs`
- Returns early message when `< 100 files` indexed: `"Insufficient test data ({fileCount} files). Need at least 100 files for meaningful benchmarks."`
- Sync iterations capped at `Math.Min(iterations, 3)` to prevent excessive runtime

---

## 8. Security Notes

### Path Traversal Prevention
- `MarkdownIO.ValidateUri()` blocks `..` segments in memory:// URIs
- `ResolveFolderUri()` verifies resolved path is within memory directory
- `PathSecurityValidator.cs` exists (separate utility)

### JSON Depth Limit
- `SafeJson.Options` with `MaxDepth=32` used everywhere
- Prevents stack overflow from deeply nested JSON payloads

### SQL Injection Prevention
- All user-facing data uses parameterized queries
- `CA2100` suppression is documented as "SQL strings are constructed internally"

### Unicode Normalization
- `FindSimilarConcepts` applies NFKC normalization before bracket checks
- Blocks 16+ Unicode bracket-confusable character pairs

---

## 9. Files Inventory (For Rust Parity Reference)

### Tools (28 files)
| C# File | Lines (approx) | Rust Status |
|---------|----------------|-------------|
| AdoptTools.cs | 99 | NOT IMPLEMENTED (T-1.1) |
| AssetResources.cs | ~120 | IMPLEMENTED |
| AssetWatcherTools.cs | ~180 | IMPLEMENTED |
| AssumptionLedger.Validation.cs | ~80 | IMPLEMENTED |
| AssumptionLedgerTools.cs | ~300 | IMPLEMENTED |
| CommunityDetection.cs | ~430 | IMPLEMENTED |
| CommunityDetectionRunner.cs | ~120 | IMPLEMENTED |
| ConceptAnalyzer.cs | ~140 | NOT IMPLEMENTED (T-1.2) |
| ConceptRepairTool.cs | ~639 | NOT IMPLEMENTED (T-3.1) |
| ConceptSync.cs | ~400 | IMPLEMENTED |
| ConceptSyncVectorSupport.cs | ~80 | IMPLEMENTED |
| DatabaseMaintenance.cs | ~100 | IMPLEMENTED |
| GraphAnalyzer.cs | ~200 | IMPLEMENTED |
| GraphDatabase.cs | ~150 | IMPLEMENTED |
| GraphTools.cs | ~350 | IMPLEMENTED |
| IncrementalSync*.cs (4 files) | ~400 total | IMPLEMENTED |
| McpResourceTools.cs | ~100 | IMPLEMENTED |
| MemorySearchTools*.cs (4 files) | ~600 total | IMPLEMENTED |
| MemoryTools.cs | ~700 | IMPLEMENTED |
| PerformanceBenchmark*.cs (2 files) | ~365 | NOT IMPLEMENTED (T-2.2) |
| RecentActivity*.cs (3 files) | ~297 | NOT IMPLEMENTED (T-2.1) |
| SequentialThinkingTools.cs | ~400 | IMPLEMENTED |
| SessionCleanup.cs | ~100 | IMPLEMENTED |
| SyncLeaderElection.cs | ~180 | IMPLEMENTED |
| SystemTools.cs | ~200 | IMPLEMENTED |
| ToolRegistry.cs | ~150 | IMPLEMENTED |
| VectorSearchTools.cs | ~400 | IMPLEMENTED |
| Workflow*.cs (4 files) | ~600 total | IMPLEMENTED |

### Utils (16 files)
| C# File | Rust Status |
|---------|-------------|
| AssetManager.cs | NOT IMPLEMENTED (T-1.3: UpdateAssets) |
| Config.cs | IMPLEMENTED (config.rs) |
| CultureInvariantHelpers.cs | IMPLEMENTED |
| DecayCalculator.cs | IMPLEMENTED |
| JsonToolResponse.cs | IMPLEMENTED |
| MarkdownIO.cs | IMPLEMENTED |
| MarkdownReader.cs | IMPLEMENTED |
| MarkdownWriter.cs | IMPLEMENTED |
| OutputContext.cs | IMPLEMENTED |
| PathSecurityValidator.cs | IMPLEMENTED |
| PayloadReader.cs | IMPLEMENTED |
| SafeJson.cs | NEEDS VERIFICATION |
| SqliteExtensions.cs | IMPLEMENTED (different approach) |
| StringBuilderExtensions.cs | N/A |
| ToolHelpers.cs | IMPLEMENTED |
| WikiLinkFilter.cs | IMPLEMENTED |

---

## 10. Recommendations for Rust Port

### CRITICAL (Must Address Before Release)
1. **Implement 6 remaining tools** per TODO.md sprints (T-1.1 through T-3.2)
2. **Verify FR-10.4 behavior**: Workflow step NOT advanced on submachine register
3. **Verify BUG-DUP-H1-001**: WriteMemory prevents duplicate H1 headings
4. **Verify decay parameters**: All 27 env var defaults must match C# exactly
5. **ConceptRepairTool Mode B**: Markdig AST-walking must be replicated with pulldown-cmark

### HIGH (Should Address)
6. **CI journal mode detection**: Check `CI` and `GITHUB_ACTIONS` env vars
7. **SafeJson depth limit**: Ensure serde_json has equivalent protection
8. **Path convention**: `thinking/workflow/` (singular) vs `thinking/workflows/` (plural) for grace periods
9. **RecentActivity assumptions filter**: Uses `memory://chat/%` NOT `memory://assumptions/%`
10. **Unicode bracket filtering**: Replicate the 16-pair confusable block list

### MEDIUM (Should Verify)
11. **Connection pooling**: Verify no equivalent issue in rusqlite
12. **NMI computation**: Test against C# reference output
13. **Smart preview**: Sentence boundary detection algorithm parity
14. **WikiLinkFilter mtime caching**: Thread-safe lazy loading

### LOW (Nice-to-Have)
15. **Diagnostic format**: `[DEGRADED][Module][Method]` pattern for consistency
16. **Performance benchmark claims**: 34% CTE vs N+1, 33ms hybrid search, 27s/2500 files sync

---

## Sources Consulted

1. All 67 `.cs` files in `src/Tools/` and `src/Utils/`
2. Test files in `tests/Maenifold.Tests/` (FR/TC references)
3. `PRD.md`, `RTM.md`, `TODO.md` in maenifold-rs
4. Config.cs environment variable definitions
5. DecayCalculator.cs with ACT-R academic citations
6. CommunityDetection.cs with Blondel et al. 2008 reference
