# Requirements Traceability Matrix — maenifold-rs Tool Parity

## MUST HAVE (Atomic Functionality)

### FR-1: Adopt Tool (`assets.rs`)
- □ RTM-001: Adopt accepts `type=role` and returns role JSON as adoption instruction | TC-1.1 | Boundary+CLI+Parity
- □ RTM-002: Adopt accepts `type=color` and returns color JSON as adoption instruction | TC-1.2 | Boundary+CLI+Parity
- □ RTM-003: Adopt accepts `type=perspective` and returns perspective JSON as adoption instruction | TC-1.3 | Boundary+CLI+Parity
- □ RTM-004: Adopt rejects invalid type with `InvalidArgument` error | TC-1.4 | Boundary+CLI
- □ RTM-005: Adopt falls back to JSON `id` field scan when filename doesn't match identifier | TC-1.5 | Boundary
- □ RTM-006: Adopt returns help doc when `learn=true` | TC-1.6 | Boundary+CLI
- □ RTM-007: Adopt output contains full JSON configuration embedded in instruction text | TC-1.7 | Boundary

### FR-2: AnalyzeConceptCorruption Tool (`search.rs`)
- □ RTM-008: AnalyzeConceptCorruption scans memory files and finds WikiLink variants matching concept family | TC-2.1 | Boundary+CLI
- □ RTM-009: AnalyzeConceptCorruption classifies variants into 5 categories: file paths, singular/plural, compound, with suffix, other | TC-2.2 | Boundary
- □ RTM-010: AnalyzeConceptCorruption respects maxResults limit | TC-2.3 | Boundary
- □ RTM-011: AnalyzeConceptCorruption returns empty report for unknown family | TC-2.4 | Boundary+CLI
- □ RTM-012: AnalyzeConceptCorruption returns help doc when `learn=true` | TC-2.5 | Boundary+CLI
- □ RTM-013: AnalyzeConceptCorruption generates suggested RepairConcepts commands for safe merges | TC-2.6 | Boundary

### FR-3: UpdateAssets Tool (`assets.rs`)
- □ RTM-014: UpdateAssets dry-run detects files present in source but missing in target | TC-3.1 | Boundary+CLI
- □ RTM-015: UpdateAssets dry-run detects files with different SHA256 hash | TC-3.2 | Boundary
- □ RTM-016: UpdateAssets non-dry-run copies added and updated files to target | TC-3.3 | Boundary
- □ RTM-017: UpdateAssets returns "no changes" summary when source and target are identical | TC-3.4 | Boundary+CLI
- □ RTM-018: UpdateAssets returns help doc when `learn=true` | TC-3.5 | Boundary+CLI
- □ RTM-019: UpdateAssets SHA256 comparison correctly detects content changes with same file size | TC-3.6 | Boundary

### FR-4: RecentActivity Tool (`tools.rs` + `markdown.rs`)
- □ RTM-020: RecentActivity default query returns recently indexed files ordered by last_indexed DESC | TC-4.1 | Boundary+CLI
- □ RTM-021: RecentActivity filter=thinking returns only thinking path files | TC-4.2 | Boundary
- □ RTM-022: RecentActivity filter=memory returns only non-thinking non-assumption files | TC-4.2 | Boundary
- □ RTM-023: RecentActivity filter=assumptions returns only assumption path files | TC-4.2 | Boundary
- □ RTM-024: RecentActivity timespan parameter restricts results to within time window | TC-4.3 | Boundary
- □ RTM-025: RecentActivity includeContent=true includes H2 section content in output | TC-4.4 | Boundary
- □ RTM-026: RecentActivity respects limit parameter | TC-4.5 | Boundary+CLI
- □ RTM-027: RecentActivity returns empty report when no files match | TC-4.6 | Boundary+CLI
- □ RTM-028: RecentActivity returns help doc when `learn=true` | TC-4.7 | Boundary+CLI
- □ RTM-029: RecentActivity classifies files correctly by path (sequential/workflow/memory/assumptions) | TC-4.8 | Boundary
- □ RTM-030: extract_h2_sections helper parses H2 headings and returns (title, content) pairs | TC-4.4 | Boundary

### FR-5: RunFullBenchmark Tool (`tools.rs`)
- □ RTM-031: RunFullBenchmark produces timing report with all 5 benchmark categories | TC-5.1 | Boundary+CLI
- □ RTM-032: RunFullBenchmark respects iterations parameter | TC-5.2 | Boundary
- □ RTM-033: RunFullBenchmark respects maxTestFiles parameter | TC-5.3 | Boundary
- □ RTM-034: RunFullBenchmark skips deep traversal when includeDeepTraversal=false | TC-5.4 | Boundary
- □ RTM-035: RunFullBenchmark output contains min/max/avg/median timing statistics | TC-5.5 | Boundary
- □ RTM-036: RunFullBenchmark returns help doc when `learn=true` | TC-5.6 | Boundary+CLI
- □ RTM-037: RunFullBenchmark returns early message when insufficient test data (<100 files) | TC-5.1 | Boundary

### FR-6: RepairConcepts Tool (`search.rs`)
- □ RTM-038: RepairConcepts Mode A replaces `[[variant]]` WikiLinks with `[[canonical]]` | TC-6.1 | Boundary+CLI
- □ RTM-039: RepairConcepts Mode B wraps plain-text variant mentions in `[[canonical]]` WikiLinks | TC-6.2 | Boundary
- □ RTM-040: RepairConcepts dry-run returns change summary without modifying files | TC-6.3 | Boundary+CLI
- □ RTM-041: RepairConcepts folder parameter restricts scanning to specified folder | TC-6.4 | Boundary
- □ RTM-042: RepairConcepts semantic validation blocks operation when similarity < threshold | TC-6.5 | Boundary
- □ RTM-043: RepairConcepts semantic validation allows operation when similarity >= threshold (or graceful degradation) | TC-6.6 | Boundary
- □ RTM-044: RepairConcepts Mode B skips code blocks and inline code | TC-6.7 | Boundary
- □ RTM-045: RepairConcepts Mode B prevents double-wrapping of existing WikiLinks | TC-6.8 | Boundary
- □ RTM-046: RepairConcepts returns help doc when `learn=true` | TC-6.9 | Boundary+CLI

### NFR-4: Clippy Baseline (`T-0.3`)
- □ RTM-047: `cargo clippy --workspace --all-targets -- -D warnings` passes clean | NFR-4 | Lint
- □ RTM-048: `normalize_concept` is public in markdown.rs | prerequisite for FR-2, FR-6 | Boundary
- □ RTM-049: `extract_h2_sections` exists in markdown.rs | prerequisite for FR-4 | Boundary

## MUST NOT HAVE
- □ RTM-X01: No ONNX runtime dependency in maenifold-core (graceful degradation for semantic validation)
- □ RTM-X02: No scope creep beyond the 6 placeholder tools
- □ RTM-X03: No breaking changes to existing 22 implemented tools
- □ RTM-X04: No MCP stdio changes in this sprint

## ESCAPE HATCHES
- Non-atomic requirement → STOP and re-decompose
- Git diff exceeds RTM scope → STOP immediately
- Parity test timing values → structure-only comparison (not exact values)
- C# oracle assumptions filter bug → document divergence, use correct path

---

## Sprint 9 — Dynamic Model Loading + CI/CD

### FR-7: Dynamic Model Loading (`vector.rs`)
- □ RTM-050: System discovers *.onnx files in assets/models/ and selects largest | TC-7.1 | Boundary
- □ RTM-051: Embedding dimensions are read from model output tensor, not hardcoded | TC-7.2 | Boundary
- □ RTM-052: Tokenizer loads from tokenizer.json when present (HuggingFace standard) | TC-7.3 | Boundary
- □ RTM-053: Tokenizer falls back to vocab.txt + tokenizer_config.json when tokenizer.json absent | TC-7.4 | Boundary
- □ RTM-054: Max tokens read from tokenizer config, defaults to 512 | TC-7.5 | Boundary
- □ RTM-055: No model present → graceful degradation, generate_embedding returns Err | TC-7.6 | Boundary
- □ RTM-056: Multiple *.onnx files → largest by file size is selected | TC-7.7 | Boundary
- □ RTM-147: Symlink-invoked binary resolves model discovery using canonicalized executable path | TC-20.5.1 | Boundary
- □ RTM-148: Model discovery checks shared runtime root `~/maenifold/assets/models/` for C# cross-compat | TC-20.5.2 | Boundary

### FR-8: Dynamic Vec0 Schema (`db.rs`)
- □ RTM-057: model_metadata table stores model filename + embedding dimensions | TC-8.1 | Boundary
- □ RTM-058: Vec0 tables created with dimensions matching loaded model | TC-8.2 | Boundary
- □ RTM-059: Model change (different dimensions) triggers vec0 table recreation | TC-8.3 | Boundary
- □ RTM-060: No model loaded → vec0 tables not created, no crash | TC-8.4 | Boundary

### FR-9: CI/CD Pipeline (`.github/workflows/`)
- □ RTM-061: build.yml runs fmt+clippy+test on push/PR | TC-9.1 | Infrastructure
- □ RTM-062: release.yml produces osx-arm64 binary on v* tag | TC-9.2 | Infrastructure
- □ RTM-063: release.yml produces linux-x64 binary on v* tag | TC-9.3 | Infrastructure
- □ RTM-064: release.yml produces win-x64 binary on v* tag | TC-9.4 | Infrastructure
- □ RTM-065: Release ZIPs include binary + assets/ but exclude *.onnx | TC-9.5 | Infrastructure

---

## Sprint 10 — Port Parity Completion

### FR-10: Workflow Asset Parity (`assets/workflows/`)
- □ RTM-066: All 69 C# workflow JSONs present in Rust assets/workflows/ | TC-10.1 | Asset
- □ RTM-067: Workflow tool resolves all 69 workflow IDs without error | TC-10.2 | Boundary+CLI

### FR-11: MCP Resource Parity (`mcp.rs`)
- □ RTM-068: MCP server capabilities include `resources` | TC-11.1 | MCP
- □ RTM-069: resources/list returns asset catalogs for all 4 types | TC-11.2 | MCP
- □ RTM-070: resources/read returns individual asset by URI | TC-11.3 | MCP
- □ RTM-071: Asset file changes emit notifications/resources/list_changed | TC-11.4 | MCP

### FR-12: MCP Auto-Start Watchers (`cli.rs`)
- □ RTM-072: --mcp mode auto-starts incremental sync watcher | TC-12.1 | CLI
- □ RTM-073: --mcp mode auto-starts asset watcher | TC-12.2 | CLI

### FR-13: Oracle Parity Tests (`tests/compat/`)
- □ RTM-074: Adopt oracle parity test exists and passes | TC-13.1 | Parity
- □ RTM-075: AnalyzeConceptCorruption oracle parity test exists and passes | TC-13.2 | Parity
- □ RTM-076: UpdateAssets oracle parity test exists and passes | TC-13.3 | Parity
- □ RTM-077: RecentActivity oracle parity test exists and passes | TC-13.4 | Parity
- □ RTM-078: RunFullBenchmark oracle parity test (structure only) exists and passes | TC-13.5 | Parity
- □ RTM-079: RepairConcepts oracle parity test exists and passes | TC-13.6 | Parity

### FR-14: CI Parity Gate (`.github/workflows/build.yml`)
- □ RTM-080: build.yml runs `cargo test` at workspace root (not crate-scoped) | TC-14.1 | Infrastructure

### FR-15: Config Env Var Parity (`config.rs`, `watchers.rs`, `search.rs`)
- □ RTM-081: MAENIFOLD_SESSION_ABANDON_MINUTES (not ABANDONMENT) read correctly | TC-15.1 | Boundary
- □ RTM-082: MAENIFOLD_AUTO_SYNC (not ENABLE_INCREMENTAL_SYNC) read correctly | TC-15.2 | Boundary
- □ RTM-083: MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY read with default 40 | TC-15.3 | Boundary
- □ RTM-084: MAENIFOLD_INCREMENTAL_VACUUM_MINUTES read with default 720 | TC-15.4 | Boundary
- □ RTM-085: MAENIFOLD_SESSION_CLEANUP read with default true | TC-15.5 | Boundary
- □ RTM-086: MAENIFOLD_SYNC_LOGGING read with default true | TC-15.6 | Boundary
- □ RTM-087: MAENIFOLD_WATCHER_BUFFER read with default 65536 | TC-15.7 | Boundary
- □ RTM-088: MAENIFOLD_SQLITE_BUSY_TIMEOUT read as env var with default 5000 | TC-15.8 | Boundary
- □ RTM-089: MAENIFOLD_EMBEDDING_LOGS read with default false | TC-15.9 | Boundary
- □ RTM-090: MAENIFOLD_VECTOR_LOGS read with default false | TC-15.10 | Boundary

### FR-16: Documentation Remediation
- □ RTM-091: readme.md reflects 28/28 implemented tools | TC-16.1 | Docs
- □ RTM-092: TODO.md status fields updated for completed items | TC-16.2 | Docs
- □ RTM-093: docs/DEVELOPMENT.md exists with Rust build/test guide | TC-16.3 | Docs

### FR-17: RepairConcepts Mode B Verification (`repair_concepts_boundary.rs`)
- □ RTM-094: Mode B plain-text WikiLink creation verified with real assertions | TC-17.1 | Boundary
- □ RTM-095: Semantic validation blocking verified with real assertions | TC-17.2 | Boundary
- □ RTM-096: Code-block exclusion verified with real assertions | TC-17.3 | Boundary
- □ RTM-097: Double-wrap prevention verified with real assertions | TC-17.4 | Boundary

### FR-18: Sprint 9 RTM Coverage Completion
- □ RTM-098: RTM-052 tokenizer.json preferred over vocab.txt tested | TC-18.1 | Boundary
- □ RTM-099: RTM-054 max tokens read from config, default 512 tested | TC-18.2 | Boundary
- □ RTM-100: RTM-058 vec0 tables use runtime dimensions tested | TC-18.3 | Boundary
- □ RTM-101: RTM-059 vec0 recreation on dimension change tested | TC-18.4 | Boundary

### FR-19: WikiLink Filter Format Parity (`markdown.rs`)
- □ RTM-102: Rust reads `.wikilink-filter.json` (JSON format) matching C# WikiLinkFilter.cs | TC-19.1 | Boundary
- □ RTM-103: JSON filter with concept→reason mapping blocks matching WikiLinks | TC-19.2 | Boundary
- □ RTM-104: Shared root with C#-written .wikilink-filter.json works identically | TC-19.3 | Boundary+Parity

### FR-20: Asset Catalog Isolation (`assets.rs`)
- □ RTM-105: preferred_catalog_assets_root() does not return sibling C# repo path | TC-20.1 | Boundary
- □ RTM-106: Missing Rust assets surface as errors, not masked by C# fallback | TC-20.2 | Boundary

---

## Hosted Deployment — Dual-Transport, Dual-Backend

### FR-22: Storage Abstraction Layer (`db/backend.rs`)
- □ RTM-115: `trait DbBackend` exists with operation-oriented domain methods (not connection handles) | TC-22.1 | Boundary
- □ RTM-116: `SqliteBackend` wraps existing rusqlite code internally with zero behavioral change | TC-22.2 | Boundary
- □ RTM-117: `PgBackend` connects via `PgPool` with configurable pool size | TC-22.3 | Boundary
- □ RTM-118: `ToolContext` carries `Arc<dyn DbBackend>` instead of `database_path` | TC-22.4 | Boundary
- □ RTM-119: All 27 `open_connection()` call sites migrated to trait domain methods | TC-22.5 | Boundary
- □ RTM-120: PG schema creates equivalent tables: file_content, concepts, concept_mentions, concept_graph, concept_communities, model_metadata, sync_leader_lease | TC-22.6 | Boundary
- □ RTM-121: PG schema uses `pgvector` VECTOR(N) columns replacing vec0 virtual tables | TC-22.7 | Boundary
- □ RTM-122: PG schema uses HNSW index on vector columns | TC-22.8 | Boundary
- □ RTM-123: `vec_distance_cosine()` queries rewritten to pgvector `<=>` operator | TC-22.9 | Boundary
- □ RTM-124: `cargo build` (no features) produces binary identical in behavior to current | TC-22.10 | Boundary+CLI
- □ RTM-125: `cargo build --features postgres` compiles and links sqlx + pgvector | TC-22.11 | Build

### FR-23: Streamable HTTP Transport (`mcp/http.rs`)
- □ RTM-126: `maenifold --http --port 8080` starts axum HTTP server | TC-23.1 | CLI
- □ RTM-127: HTTP server exposes MCP Streamable HTTP endpoints (POST /mcp, GET /mcp, DELETE /mcp) | TC-23.2 | Integration
- □ RTM-128: `maenifold --mcp` continues to start stdio server (unchanged) | TC-23.3 | CLI
- □ RTM-129: HTTP sessions tracked via `Mcp-Session-Id` header | TC-23.4 | Integration
- □ RTM-130: All 28 tools invokable via HTTP transport | TC-23.5 | Integration

### FR-24: Entra ID Authentication (`auth/`)
- □ RTM-131: `GET /.well-known/oauth-protected-resource` returns RFC 9728 metadata | TC-24.1 | Integration
- □ RTM-132: Bearer token in Authorization header validated against Entra ID JWKS | TC-24.2 | Integration
- □ RTM-133: Requests without valid token receive 401 with WWW-Authenticate header | TC-24.3 | Integration
- □ RTM-134: Token audience validated against configured server URL | TC-24.4 | Integration
- □ RTM-135: `--no-auth` flag bypasses authentication for local development | TC-24.5 | CLI
- □ RTM-136: PG connection uses managed identity token (no stored password) | TC-24.6 | Integration

### FR-25: Azure Deployment Manifests (`deploy/`)
- □ RTM-137: Dockerfile produces runnable container image | TC-25.1 | Infrastructure
- □ RTM-138: Bicep/azd templates deploy ACA + PG Flex + Azure Files + managed identity | TC-25.2 | Infrastructure
- □ RTM-139: SRE Agent connector YAML provided with correct endpoint and auth config | TC-25.3 | Infrastructure
- □ RTM-140: CI workflow builds and pushes container image on v* tags | TC-25.4 | Infrastructure

### FR-26: Polling-Based Watcher Fallback (`sync/watchers.rs`)
- □ RTM-141: `MAENIFOLD_WATCHER_POLL_SECONDS` env var configures polling interval (default 30) | TC-26.1 | Boundary
- □ RTM-142: Polling watcher detects new/changed/deleted .md files | TC-26.2 | Boundary
- □ RTM-143: Polling watcher triggers same incremental sync logic as native watcher | TC-26.3 | Boundary

### NFR-5: Local Mode Preservation
- □ RTM-144: `cargo build` (no postgres/http features) produces identical binary behavior | TC-NFR5.1 | Boundary+CLI

### NFR-6: Concurrent Access
- □ RTM-145: 5+ simultaneous PG connections succeed without contention timeout | TC-NFR6.1 | Load
- □ RTM-146: Concurrent WriteMemory + SearchMemories do not corrupt data | TC-NFR6.2 | Load

### FR-21: SearchMemories Scoring Parity (`search.rs`)
- □ RTM-107: Text scoring uses occurrence counting matching C# CalculateSearchScore | TC-21.1 | Boundary
- □ RTM-108: Text scoring adds +5 title bonus per term found | TC-21.2 | Boundary
- □ RTM-109: Vector search queries vec_memory_files with vec_distance_cosine | TC-21.3 | Boundary
- □ RTM-110: Vector search applies min-max score normalization with epsilon 1e-12 | TC-21.4 | Boundary
- □ RTM-111: RRF fusion merges text and vector pipelines with k=60 | TC-21.5 | Boundary
- □ RTM-112: SearchMemories degrades gracefully when vec0 unavailable | TC-21.6 | Boundary
- □ RTM-113: Snippet extraction uses 20-char leading context and 200-char total | TC-21.7 | Boundary
- □ RTM-114: Snippet whitespace normalized (\\r\\n and \\n to space) | TC-21.8 | Boundary
