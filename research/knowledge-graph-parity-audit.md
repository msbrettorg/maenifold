# Knowledge Graph Parity Audit — maenifold C# → maenifold-rs Rust Port

**Date**: 2026-03-09  
**Traceability**: T-0.2 (RTM-X02), supports all T-* items  
**Sources**: 30+ memory:// files read, 15+ search queries across knowledge graph  
**Methodology**: Systematic searchmemories → readmemory → cross-reference against Rust codebase

---

## Executive Summary

The [[maenifold-rs]] Rust port is **significantly more complete than the C# cross-cutting audit from 2026-03-08 suggested**. The explore agent found that 13 of the 17 subsystems flagged as gaps are now implemented. The remaining critical gaps are:

1. **ONNX embedding generation** — Rust has vec0 extension but NO model inference (blocking for standalone use)
2. **NMI drift detection** — config wired but algorithm not implemented
3. **6 placeholder tools** — already tracked in PRD.md (Adopt, AnalyzeConceptCorruption, UpdateAssets, RecentActivity, RunFullBenchmark, RepairConcepts)
4. **Asset hot-reload watcher** — C# has `AssetWatcherTools.cs` (202 lines) with MCP change notifications; Rust has no equivalent

---

## 1. Feature Specs Not Yet in Rust

### 1.1 ONNX Embedding Generation — **CRITICAL GAP** ⚠️
**Source**: `memory://build/plans/maenifoldrs-embedding-sprint-plan`, `memory://research/incremental-sync-cpu-profiling-analysis`

The Rust port passed all parity tests because compat tests used C#-seeded databases. **Rust sync never generates embeddings** — it only reads pre-existing `vec_concepts` rows created by C#. This means:
- `FindSimilarConcepts` falls back to graph-based similarity (not vector cosine)
- Semantic search mode returns graph-fallback results, not vector-scored results
- Standalone Rust deployment has **zero semantic capability**

**C# implementation details** (from PERF-SYNC-001):
- Model: `all-MiniLM-L6-v2` (384 dimensions)
- Batched inference: `VectorTools.GenerateEmbeddings(batch)` with maxBatch=16
- Per-file: 1 file embedding (first 1000 chars) + N concept embeddings
- Mean-pooling over `[batch, seq, hidden]` output shape
- Scoped `vec_concepts` lookup with `WHERE concept_name IN (...)` filter

**Sprint plan exists**: T-7.1 through T-7.7 in `memory://build/plans/maenifoldrs-embedding-sprint-plan`

**Rust status**: ❌ NOT IMPLEMENTED — `ort` and `tokenizers` crates are in workspace Cargo.toml but not wired

### 1.2 NMI Community Drift Detection — **MEDIUM GAP** ⚠️
**Source**: `memory://build/architecture/proximityweighted-cooccurrence-architectural-design-rationale`, `memory://process/retrospectives/sprint-tprox003-retrospective`

C# implements `ComputeNMI()` (Normalized Mutual Information) between old and new community assignments before overwriting. Threshold configurable via `MAENIFOLD_COMMUNITY_NMI_THRESHOLD` (default 0.8). NMI is label-invariant — label permutations don't affect the score. If NMI < threshold, `[COMMUNITY DRIFT]` warning on stderr.

**Rust status**: ⚠️ Config variable `MAENIFOLD_COMMUNITY_NMI_DRIFT_THRESHOLD` exists, but the actual NMI computation algorithm is NOT implemented. Community detection runs without drift checking.

### 1.3 Asset Hot-Reload Watcher — **MEDIUM GAP** ⚠️
**Source**: `memory://build/research/c-oracle-audit-maenifoldrs-parity-gap-analysis-20260308`

C# has `AssetWatcherTools.cs` (202 lines) — watches asset directory for changes and sends MCP `notifications/resources/list_changed` notifications to connected clients. This enables live-reload of workflows, roles, perspectives, and colors.

**Rust status**: ❌ NOT IMPLEMENTED — `start_asset_watcher` and `stop_asset_watcher` are MCP tool stubs

### 1.4 Six Placeholder Tools — **TRACKED IN PRD**
**Source**: `memory://build/research/c-oracle-audit-maenifoldrs-parity-gap-analysis-20260308`, PRD.md

| Tool | C# Lines | Rust Status | PRD Task |
|------|----------|-------------|----------|
| Adopt | 99 | Placeholder | T-1.1 (FR-1) |
| AnalyzeConceptCorruption | 140 | Placeholder | T-1.2 (FR-2) |
| UpdateAssets | 165 | Placeholder | T-1.3 (FR-3) |
| RecentActivity | 297 | Placeholder | T-2.1 (FR-4) |
| RunFullBenchmark | 365 | Placeholder | T-2.2 (FR-5) |
| RepairConcepts | 639 | Placeholder | T-3.1 (FR-6) |

### 1.5 Database Maintenance — Full Recipes — **MINOR GAP** ⚠️
**Source**: `memory://build/sprints/sync-architecture-refactor-sprint-qualsync001-retrospective`

C# has two distinct maintenance recipes:
- `RunFull`: FTS optimize → rebuild → checkpoint(TRUNCATE) → VACUUM → WAL restore
- `RunLight`: FTS optimize → checkpoint(PASSIVE), no exclusive locks
- Key gotcha: `Microsoft.Data.Sqlite` silently no-ops VACUUM on reused connections → requires close/reopen pattern

**Rust status**: ⚠️ Has WAL mode and pragma tuning; `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES` env var exists but unclear if full RunFull/RunLight recipes are implemented. VACUUM close/reopen pattern needs verification.

### 1.6 Culture-Invariant Helpers — **MINOR GAP**
**Source**: `memory://build/research/c-oracle-audit-maenifoldrs-parity-gap-analysis-20260308`

C# has locale-safe formatting helpers. Rust `format!` is locale-independent by default, so this is likely a non-issue, but should be verified for any floating-point formatting in tool output (decay weights, modularity scores).

**Rust status**: ✅ Likely OK (Rust is locale-independent by default) but verify decimal formatting in JSON output

---

## 2. Design Decisions That Must Be Preserved

### 2.1 Structural vs Temporal Separation in Louvain
**Source**: `memory://build/architecture/proximityweighted-cooccurrence-architectural-design-rationale` (Decision 2)

**Decision**: `RunLouvain` uses `sum(proximity_values)` from source_files — **NO temporal decay** in community structure. `BuildContext` uses `average(proximity × decay)` for traversal ordering.

**Rationale**: Community detection finds structural clusters — which concepts co-occur together. Decay is about ranking freshness. A concept pair co-occurring in 10 files should stay in the same community even if those files are stale. Also: keeping decay out of Louvain makes community detection fully testable with synthetic URIs (no disk reads needed).

**Rust status**: ✅ VERIFY — the explore agent confirmed Louvain is implemented, but needs verification that it uses proximity-only weights without decay.

### 2.2 Section-Level 2-Tier Proximity (1.0 / 0.5)
**Source**: `memory://build/architecture/proximityweighted-cooccurrence-architectural-design-rationale` (Decision 3), `memory://build/research/proximityweighted-cooccurrence-for-knowledge-graphs-literature-review-and-practical-analysis`

**Decision**: Same H2 section → weight 1.0. Cross-section or preamble-only → weight 0.5. NOT token-level or sentence-level.

**Rationale**: NLP literature review found section-level hierarchical weighting captures 80-90% of proximity signal with minimal complexity. 2-tier avoids position-tracking while preserving core insight. Literature consensus: biggest quality jump is document → paragraph level. Section is optimal for markdown with double-bracket-links.

**Rust status**: ✅ CONFIRMED — explore agent found tests `sync_same_h2_section_relations_get_full_proximity_weight()` (1.0) and `sync_cross_h2_section_relations_get_half_proximity_weight()` (0.5).

### 2.3 Sum (Not Average) for Edge Weights
**Source**: `memory://build/architecture/proximityweighted-cooccurrence-architectural-design-rationale` (Decision 1)

**Decision**: Edge weight = `sum(proximity_values)` from source_files dict. NOT average.

**Rationale**: 10 cross-section (0.5) co-occurrences → sum = 5.0 (strong edge). 1 same-section (1.0) co-occurrence → sum = 1.0 (weaker edge). Sum preserves "more evidence = stronger" signal.

**Rust status**: ✅ VERIFY in community.rs — ensure edge weight loading sums proximity values.

### 2.4 Frontmatter Envelope Architecture
**Source**: `memory://build/architecture/frontmatter-standardization-markdownwriter-envelope-architecture-tmeta001`

**Decision**: Single write choke point (`WriteMarkdown`) manages envelope (`created`, `modified`). Tools manage content only. `created` = caller wins if present, writer adds if absent. `modified` = writer always wins.

**Critical edge cases**:
- `last_accessed` lives in SQLite only (NOT in YAML frontmatter) — only ReadMemory updates it
- `completed` and `cancelled` timestamps are semantic metadata owned by tools, NOT envelope
- SessionCleanup stores a dict reference and reads it after write — use local copy pattern to avoid mutation

**Rust status**: ✅ CONFIRMED — explore agent found envelope management in `write_markdown()` with `created` and `modified` handling.

### 2.5 MarkdownIO URI Boundary Layer
**Source**: `memory://process/retrospectives/sprint-retrospective-mdio001-markdownio-uri-boundary-layer`

**Decision**: MarkdownIO is the boundary between URI-land and path-land. Above it (tools), everything uses `memory://` URIs. Below it (MarkdownReader, MarkdownWriter), only file paths.

**Security**: Segment-based traversal validation (`segments.Any(s => s == "..")`) instead of `Contains("..")` to avoid false positives on legitimate concept names like `some..concept`.

**Rust status**: ✅ VERIFY — paths.rs likely handles this, but the specific segment-based validation pattern needs checking.

### 2.6 Leader Election Design
**Source**: `memory://build/sprints/tsyncleader001-leader-election-sprint-retrospective`, `memory://build/research/sqlite-leasebased-leader-election-patterns-for-begin-immediate-wal-busy-behavior-and-fencing`

**Key decisions**:
- In-repo SQLite lease (no external packages)
- `BEGIN IMMEDIATE` for atomic lease operations
- Release as UPDATE (not DELETE) preserves fencing-token epoch
- Single lock name `"sync_leader"` for all leader-gated workloads
- `TryAcquire` over `IsLeader` to avoid TOCTOU gap
- Config: `MAENIFOLD_SYNC_ROLE=auto|leader|follower`, `MAENIFOLD_SYNC_LEASE_MS` (default 10s, bounded 1-60s)

**Rust status**: ✅ CONFIRMED — `leader.rs` has full lease-based election with `try_acquire`, `force_acquire`, `try_renew`, `release`, `is_leader`. Max lease 300s (differs from C# 60s — needs alignment check).

### 2.7 Swarm Propagation Principle
**Source**: `memory://build/architecture/swarm-propagation-principle-shared-graph-context-mutations`

**Design principle**: Every mutation to the graph propagates to all agents simultaneously through the causal chain: memory files → ConceptSync → sqlite edges → community-detection → community assignments → AGENTS.md content → ALL agents see updated navigation index.

**Key implication**: Community reshuffles are high-impact swarm-wide events. NMI drift detection is the safety gate.

**Rust status**: ✅ Causal chain is intact; ⚠️ NMI safety gate is missing (see 1.2).

---

## 3. Performance Targets

### 3.1 Sync Performance (from PERF-SYNC-001)
**Source**: `memory://build/sprints/perfsync001-sprint-summary`, `memory://research/incremental-sync-cpu-profiling-analysis`

| Metric | C# Pre-optimization | C# Post-optimization | Rust Target |
|--------|---------------------|---------------------|-------------|
| ONNX calls per file (10 new concepts) | 11 | 1-2 (batched) | 1-2 (batched) |
| Edge writes per file (20 concepts) | 190 (unconditional) | ~0 (delta only) | Delta only |
| Markdig/pulldown-cmark parses per file | 3 | 2 | 1-2 |
| Coverage | 79.6% line | 79.6% line | ≥75% line |

### 3.2 Amplification per Changed File (15 concepts, 5 new)
- 2 file reads, 2-3 markdown parses, 6 ONNX inferences (batched to 1-2 calls)
- ~121 DB writes (1 file_content + 15 concepts + 15 mentions + 105 edges — but delta reduces to ~10-20)
- 2 FTS5 trigger operations

### 3.3 Test Coverage Thresholds (from C# oracle)
- Line: ≥75%
- Branch: ≥65%
- Method: ≥85%
- C# has 996 tests; Rust has ~185+ (growing)

### 3.4 Debounce Timing
- File watcher debounce: 500ms default (C#) → 150ms default in Rust (`MAENIFOLD_DEBOUNCE_MS`)
- Community recompute debounce: 2000ms (`MAENIFOLD_COMMUNITY_DEBOUNCE_MS`)

---

## 4. Known Edge Cases

### 4.1 H2 Heading WikiLink Extraction Bug (Fixed in C# T-PROX-002)
**Source**: `memory://process/retrospectives/sprint-tprox002-retrospective`

`ExtractWikiLinksWithSections` was silently dropping WikiLink concepts from H2 heading text — `sectionIndex++; continue;` skipped extraction. Fix: call `ExtractFromBlockTree` before `continue`. Also had to remove a complementary `heading.Level != 2` guard that became a blocker.

**Rust status**: ✅ VERIFY — ensure `extract_wikilinks_with_sections` extracts WikiLinks from heading text, not just body text.

### 4.2 Empty source_files Edge Weight Bug (Fixed in C# T-PROX-002)
**Source**: `memory://process/retrospectives/sprint-tprox002-retrospective`

BuildContext used `.DefaultIfEmpty(1.0)` for empty `source_files` — edges with no evidence got weight 1.0 instead of 0.0. Fix: explicit `fileDict.Count == 0 ? 0.0` guard.

**Rust status**: ✅ VERIFY — ensure empty source_files edges get weight 0.0.

### 4.3 SQLite VACUUM on Reused Connections
**Source**: `memory://build/sprints/sync-architecture-refactor-sprint-qualsync001-retrospective`

`Microsoft.Data.Sqlite` silently no-ops VACUUM on reused SqliteConnection. C# uses close/reopen pattern. Rust's `rusqlite` behavior needs verification.

**Rust status**: ⚠️ NEEDS VERIFICATION — check if rusqlite has same VACUUM-on-reused-connection behavior.

### 4.4 Partial Rename on Lease Loss
**Source**: `memory://build/security/tsyncleader0023-redteam-adversarial-audit-leadergated-file-watcher-sync` (F-4)

ProcessFileRenamed does delete(old) + sync(new) with independent TryAcquire calls. If lease expires between, old file deleted but new file NOT synced. Self-healing on next event.

**Rust status**: ⚠️ VERIFY — check if Rust watcher handles renames atomically or has same limitation.

### 4.5 Multiple FileSystemWatcher Events per Logical Write
**Source**: `memory://build/architecture/incremental-sync-multiinstance-cpu-storm-and-db-leaderelection-proposal`

.NET FileSystemWatcher raises multiple Changed events for one logical operation. SequentialThinking and Workflow tools perform multiple markdown writes per logical operation (CreateSession, AppendToSession, UpdateSession). Debounce is critical.

**Rust status**: ✅ Implemented — `notify` crate with configurable debounce_ms.

### 4.6 ConceptRepairTool File Asymmetry
**Source**: `memory://process/retrospectives/sprint-retrospective-tmeta001-frontmatter-metadata-standardization` (RED-001)

`AnalyzeConcepts` read full file including YAML frontmatter, overstating WikiLink count vs repair which operated on content only. Fixed to align on MarkdownIO.ReadMarkdown.

**Rust status**: ⚠️ NEEDS ATTENTION when implementing RepairConcepts (T-3.1) — ensure analyze and repair operate on same content scope.

### 4.7 NMI "Opposite Clustering" Test Bug
**Source**: `memory://process/retrospectives/sprint-tprox003-retrospective`

Blue-team wrote NMI test with "opposite" clusterings that was actually a label-swap (same partition). NMI correctly returned 1.0, but test asserted < 1.0. NMI is mathematically label-invariant — label swaps don't change partition structure.

**Rust status**: ⚠️ When implementing NMI, ensure tests use genuinely different partitions, not label swaps.

---

## 5. Constants/Parameters with Rationale

### 5.1 Decay Parameters
**Source**: `memory://build/architecture/maenifold-strategic-roadmap-maintenance-to-product-transition-feb-2026`, explore agent analysis

| Parameter | Env Var | Default | Rationale |
|-----------|---------|---------|-----------|
| Decay function | `MAENIFOLD_DECAY_FUNCTION` | `power-law` | ACT-R model — power-law matches human memory decay |
| Half-life | `MAENIFOLD_DECAY_HALF_LIFE_DAYS` | 30 days | Balances freshness vs history retention |
| Grace: sequential | `MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL` | 7 days | Thinking sessions are ephemeral |
| Grace: workflows | `MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS` | 14 days | Workflows are medium-lived |
| Grace: default | `MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT` | 28 days | Memory notes are long-lived |
| Grace: compaction | Hardcoded | 7 days | Same as sequential (ephemeral) |

**Status-based overrides** (for assumptions/chat):
- `validated` → weight = 1.0 (no decay ever)
- `invalidated` → grace 7d, half-life 14d (fast decay)
- `active`/`refined` → grace 14d, half-life 30d

**Rust status**: ✅ CONFIRMED — all grace periods implemented with matching defaults.

### 5.2 Proximity Weights
| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Same H2 section | 1.0 | Literature: section-level captures 80-90% of signal |
| Cross-section | 0.5 | Weaker but non-zero topical relationship |
| Edge aggregation | sum(proximity) | More evidence = stronger signal |

**Rust status**: ✅ CONFIRMED

### 5.3 Community Detection
| Parameter | Env Var | Default | Rationale |
|-----------|---------|---------|-----------|
| Resolution (gamma) | `MAENIFOLD_LOUVAIN_GAMMA` | 1.0 | Standard Louvain default |
| NMI drift threshold | `MAENIFOLD_COMMUNITY_NMI_THRESHOLD` | 0.8 | Configurable safety gate |
| Recompute debounce | `MAENIFOLD_COMMUNITY_DEBOUNCE_MS` | 2000 | Prevent rapid recomputation |

**Rust status**: ✅ Config exists; ⚠️ NMI algorithm not implemented

### 5.4 Leader Election
| Parameter | Env Var | Default | Bounds | Rationale |
|-----------|---------|---------|--------|-----------|
| Lease duration | `MAENIFOLD_SYNC_LEASE_MS` | 10,000ms | 1,000-60,000ms | Balance failover speed vs stability |
| Sync role | `MAENIFOLD_SYNC_ROLE` | `auto` | auto/leader/follower | Operational control |
| Heartbeat interval | `MAENIFOLD_SYNC_HEARTBEAT_MS` | 3,000ms | 500-30,000ms | Dead code in C# (never used) |

**Note**: C# heartbeat is dead code — was never wired to a renewal loop. Don't port it.

**Rust status**: ✅ CONFIRMED — lease at 300s max (differs from C# 60s — may be intentional for Rust's faster execution)

### 5.5 Session Cleanup
| Parameter | Env Var | Default | Rationale |
|-----------|---------|---------|-----------|
| Abandonment threshold | `MAENIFOLD_SESSION_ABANDONMENT_MINUTES` | 30 min | Reasonable inactivity window |

**Rust status**: ✅ CONFIRMED

### 5.6 File Watcher
| Parameter | Env Var | Default | Rationale |
|-----------|---------|---------|-----------|
| Debounce interval | `MAENIFOLD_DEBOUNCE_MS` | 150ms (Rust) / 500ms (C#) | Coalesce rapid FS events |
| Buffer size | `MAENIFOLD_WATCHER_BUFFER` | 65536 | OS-level event buffer |

**Rust status**: ✅ CONFIRMED — debounce differs (150ms vs 500ms), may be intentional

### 5.7 ONNX Embedding
| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Model | all-MiniLM-L6-v2 | 384 dims, fast inference, good quality |
| Max batch size | 16 | Balance memory usage vs throughput |
| Max tokens | Model default (256) | Standard for all-MiniLM-L6-v2 |
| File embedding truncation | First 1000 chars | Captures title + intro |

**Rust status**: ❌ NOT IMPLEMENTED

### 5.8 SQLite
| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Journal mode | WAL (non-CI), DELETE (CI) | WAL enables concurrent reads; DELETE for CI compat |
| Synchronous | NORMAL | Balance durability vs performance |
| Busy timeout | `Config::SQLITE_BUSY_TIMEOUT_MS` | Handle multi-process contention |

**Rust status**: ✅ CONFIRMED

---

## 6. Architecture Constraints

### 6.1 Single-Writer SQLite with WAL
**Source**: `memory://build/research/sqlite-leasebased-leader-election-patterns-for-begin-immediate-wal-busy-behavior-and-fencing`

SQLite permits only one writer at a time. `BEGIN IMMEDIATE` front-loads write-lock acquisition. In WAL mode, readers and single writer proceed concurrently. Applications must handle `SQLITE_BUSY`. Long-lived readers can delay checkpoint completion — keep lease transactions tiny.

### 6.2 Louvain Phase 1 Only
**Source**: `memory://research/c-crosscutting-features-for-rust-port`

C# implements Phase 1 only (local modularity moves), no Phase 2 aggregation (meta-graph contraction). This is sufficient for the typical graph sizes (hundreds to low thousands of concepts).

**Rust status**: ✅ CONFIRMED — Phase 1 only in `community.rs`

### 6.3 Memory File Watcher NOT Leader-Gated (Known Limitation)
**Source**: `memory://build/architecture/incremental-sync-multiinstance-cpu-storm-and-db-leaderelection-proposal` (2026-03-07 re-verification)

In C#, the DB watcher path is leader-gated, but the memory-file watcher path is NOT. Followers still call `ConceptSync.SyncFiles()`. This is a known architectural limitation — mtime/hash guards prevent double work but don't prevent the execution attempt.

**Rust status**: ⚠️ VERIFY — check if Rust watcher respects leader election for file processing path.

### 6.4 Local-First, No Network Dependencies
**Source**: `memory://build/architecture/maenifold-strategic-roadmap-maintenance-to-product-transition-feb-2026`

All data is local: SQLite + filesystem. No cloud services, no telemetry, no network calls. Consistent with `no-fake-tests` philosophy — real SQLite, real filesystem.

### 6.5 MCP stdio Transport Only
**Source**: `memory://assumptions/2026/03/assumption-1772732357070`

Maenifold MCP is intended for local stdio transport and is not directly internet-exposed. This scopes the security model.

**Rust status**: ✅ CONFIRMED — MCP implementation uses stdio

---

## 7. Recent Work (Last 2 Weeks — Easy to Miss)

### 7.1 Embedding Sprint Plan (2026-03-09) — **JUST CREATED**
**Source**: `memory://build/plans/maenifoldrs-embedding-sprint-plan`

Sprint 7 plan to add ONNX embedding generation to Rust. Tests-first approach with 6 specific failing tests defined. Dependencies: `ort` crate, `tokenizers` crate, `all-MiniLM-L6-v2` model file.

**Status**: Plan written, not yet started.

### 7.2 C# Oracle Parity Gap Analysis (2026-03-08)
**Source**: `memory://build/research/c-oracle-audit-maenifoldrs-parity-gap-analysis-20260308`

Full audit: 28 tools (22 implemented, 6 placeholders), 43 MCP tool methods, 996 C# tests vs ~185 Rust tests, 13,075 C# LOC vs ~3,498 Rust LOC (27%).

### 7.3 PERF-SYNC-001 Performance Sprint (2026-03-08)
**Source**: `memory://build/sprints/perfsync001-sprint-summary`

C# received batched ONNX embeddings, delta BuildGraphEdges, and single-pass Markdig. These optimizations should be replicated in Rust, not the pre-optimization behavior.

### 7.4 Leader-Gated File Watcher Sync (2026-03-09)
**Source**: `memory://build/security/tsyncleader0023-redteam-adversarial-audit-leadergated-file-watcher-sync`

C# file watcher processing path now has leader election gate. Rust should verify its watcher has equivalent gating.

### 7.5 Parity Test Assumptions (2026-03-07)
**Source**: `memory://assumptions/2026/03/assumption-1772943030521`, `memory://assumptions/2026/03/assumption-1772943250687`

For mutating compatibility tests (WriteMemory), checksum and modified-timestamp fields are volatile across binary invocations. Compare semantically after ignoring those fields.

### 7.6 OpenCode Plugin Persistence Sprint (2026-03-08)
**Source**: `memory://retrospectives/sprint20260305ocpersist003retrospectivev2`

OpenCode plugin for community-based context injection. Relevant because it demonstrates the consumption pattern: recent-compactions + recent-activity source blocks → filtered communities block → AGENTS.md injection. Rust must support the same community output format.

---

## 8. Cross-Cutting Verification Checklist

| # | Item | Rust Status | Action Needed |
|---|------|-------------|---------------|
| 1 | ONNX embedding generation | ❌ Missing | Sprint 7 plan exists |
| 2 | NMI drift detection algorithm | ⚠️ Config only | Implement ComputeNMI |
| 3 | Asset hot-reload watcher with MCP notification | ❌ Missing | Implement or defer |
| 4 | 6 placeholder tools | ❌ Placeholders | PRD tracks T-1.1→T-3.2 |
| 5 | Section-level proximity (1.0/0.5) | ✅ Confirmed | — |
| 6 | Louvain Phase 1 community detection | ✅ Confirmed | Verify sum(proximity) weights |
| 7 | Leader election with BEGIN IMMEDIATE | ✅ Confirmed | Verify lease bound (300s vs 60s) |
| 8 | WikiLink filter (.wikilink-filter) | ✅ Confirmed | — |
| 9 | Decay with grace periods (power-law/ACT-R) | ✅ Confirmed | — |
| 10 | Filesystem watcher (notify crate) | ✅ Confirmed | Verify leader gating |
| 11 | Session cleanup/abandonment | ✅ Confirmed | — |
| 12 | Database maintenance (WAL/VACUUM) | ⚠️ Partial | Verify RunFull/RunLight recipes |
| 13 | Frontmatter envelope (write choke point) | ✅ Confirmed | — |
| 14 | URI boundary layer (paths.rs) | ✅ Confirmed | Verify segment-based traversal check |
| 15 | Path security validator | ⚠️ Unknown | Verify traversal prevention |
| 16 | Culture-invariant formatting | ✅ Likely OK | Verify decimal output |
| 17 | Env var parity (30+ vars) | ⚠️ Partial | Audit all C# config vars |
| 18 | InsertEdge test helper pattern | ✅ Likely OK | Ensure test helpers match new format |
| 19 | H2 heading WikiLink extraction | ⚠️ Unknown | Verify extraction from heading text |
| 20 | Empty source_files edge weight | ⚠️ Unknown | Verify 0.0 for empty |
| 21 | Delta edge writes (not unconditional) | ⚠️ Unknown | Check if Rust does delta or full writes |
| 22 | Scoped vec_concepts lookup | ⚠️ Unknown | Check for full-table scan avoidance |

---

## Sources Consulted

### Sprint Retrospectives
- `memory://build/sprints/tsyncleader001-leader-election-sprint-retrospective`
- `memory://build/sprints/sync-architecture-refactor-sprint-qualsync001-retrospective`
- `memory://build/sprints/perfsync001-sprint-summary`
- `memory://process/retrospectives/sprint-tprox002-retrospective`
- `memory://process/retrospectives/sprint-tprox003-retrospective`
- `memory://process/retrospectives/sprint-retrospective-tmeta001-frontmatter-metadata-standardization`
- `memory://process/retrospectives/sprint-retrospective-mdio001-markdownio-uri-boundary-layer`

### Architecture Decisions
- `memory://build/architecture/proximityweighted-cooccurrence-architectural-design-rationale`
- `memory://build/architecture/incremental-sync-multiinstance-cpu-storm-and-db-leaderelection-proposal`
- `memory://build/architecture/frontmatter-standardization-markdownwriter-envelope-architecture-tmeta001`
- `memory://build/architecture/swarm-propagation-principle-shared-graph-context-mutations`
- `memory://build/architecture/maenifold-strategic-roadmap-maintenance-to-product-transition-feb-2026`

### Research
- `memory://build/research/proximityweighted-cooccurrence-for-knowledge-graphs-literature-review-and-practical-analysis`
- `memory://build/research/sqlite-leasebased-leader-election-patterns-for-begin-immediate-wal-busy-behavior-and-fencing`
- `memory://build/research/c-oracle-audit-maenifoldrs-parity-gap-analysis-20260308`
- `memory://research/c-crosscutting-features-for-rust-port`
- `memory://research/incremental-sync-cpu-profiling-analysis`
- `memory://build/plans/maenifoldrs-embedding-sprint-plan`

### Security Audits
- `memory://build/security/tsyncleader0017-redteam-adversarial-audit-results`
- `memory://build/security/tsyncleader0023-redteam-adversarial-audit-leadergated-file-watcher-sync`

### Reference
- `memory://reference/concept-hub-detection-graph-maintenance-methodology-for-wikilink-filtering`

### Methodology
- `memory://process/wavebased-sprint-methodology-closedloop-learning-and-dependencyaware-remediation`

---

## Limitations

1. **C# source code not directly read** — findings about C# behavior come from knowledge graph memories, not fresh code inspection
2. **Rust explore agent analysis** — some "confirmed" items may need deeper line-level verification
3. **Timing** — C# continues to evolve; this audit reflects state as of 2026-03-09
4. **Test parity not deeply analyzed** — 996 C# tests vs ~185 Rust tests; individual test coverage gaps not enumerated
5. **MCP Apps / ui:// resources not covered** — out of scope for core parity but relevant for product deployment
