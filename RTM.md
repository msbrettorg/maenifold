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
