# Product Requirements Document — maenifold-rs Tool Parity

## Overview
Complete the Rust reimplementation of maenifold by implementing the 6 remaining placeholder tools to full C# oracle parity.

## Functional Requirements

### FR-1: Adopt Tool
The system SHALL provide an `Adopt` tool that reads a role, color, or perspective JSON asset by type and identifier, returning the full asset content formatted as an LLM adoption instruction.
- **Parameters**: `type` (role|color|perspective), `identifier` (asset ID), `learn` (bool)
- **Behavior**: Two-step lookup — fast path by filename, fallback by scanning JSON `id` fields. Returns structured adoption prompt with embedded JSON.
- **Parity**: Output SHALL match C# `AdoptTools.cs` (99 lines)

### FR-2: AnalyzeConceptCorruption Tool
The system SHALL provide an `AnalyzeConceptCorruption` tool that scans all memory files for WikiLink concept variants matching a given concept family, classifies them by pattern type, and suggests repair commands.
- **Parameters**: `conceptFamily` (string), `maxResults` (int, default 50), `learn` (bool)
- **Behavior**: Scans WikiLinks, normalizes concepts, checks containment of family string. Classifies variants into: File Paths, Singular/Plural, Compound (hyphenated), With Suffix, Other. Generates suggested `RepairConcepts` commands for safe merges.
- **Parity**: Output SHALL match C# `ConceptAnalyzer.cs` (~140 lines)

### FR-3: UpdateAssets Tool
The system SHALL provide an `UpdateAssets` tool that compares packaged assets against runtime assets, identifying added/updated files, with optional dry-run mode.
- **Parameters**: `dryRun` (bool, default true), `learn` (bool)
- **Behavior**: Recursive directory comparison using file size + SHA256 hash. In non-dry-run mode, copies changed files. Returns markdown summary with counts and file lists.
- **Parity**: Output SHALL match C# `AssetManager.cs` update logic (~165 lines)

### FR-4: RecentActivity Tool
The system SHALL provide a `RecentActivity` tool that queries the `file_content` database table for recently indexed files, filtered by type and timespan, with optional content inclusion.
- **Parameters**: `limit` (int, default 10), `filter` (thinking|memory|assumptions|all), `timespan` (duration string), `includeContent` (bool), `learn` (bool)
- **Behavior**: Queries `file_content` table ordered by `last_indexed DESC`. Classifies files by path pattern (sequential/workflow/thinking/memory). Extracts H2 sections from content. Formats activity report with thought/step counts and snippets.
- **Parity**: Output SHALL match C# `RecentActivityTools.cs` + `RecentActivity.Reader.cs` + `RecentActivity.Formatter.cs` (~297 lines total)

### FR-5: RunFullBenchmark Tool
The system SHALL provide a `RunFullBenchmark` tool that measures performance across 5 benchmark categories: CTE vs N+1 graph traversal, search (3 modes), sync, complex traversal, and system health.
- **Parameters**: `iterations` (int, default 5), `maxTestFiles` (int, default 1000), `includeDeepTraversal` (bool, default true), `learn` (bool)
- **Behavior**: Runs timed iterations of each benchmark using `std::time::Instant`. Reports min/max/avg/median timings. System health reports DB table counts.
- **Parity**: Output SHALL match C# `PerformanceBenchmark.cs` + `PerformanceBenchmark.Graph.cs` (~365 lines total)

### FR-6: RepairConcepts Tool
The system SHALL provide a `RepairConcepts` tool that replaces concept variants with a canonical form across all memory files, with two modes: WikiLink replacement and WikiLink creation, plus semantic safety validation.
- **Parameters**: `conceptsToReplace` (comma/semicolon/pipe-separated), `canonicalConcept` (string), `folder` (optional), `dryRun` (bool, default true), `createWikiLinks` (bool, default false), `minSemanticSimilarity` (float, default 0.7), `learn` (bool)
- **Behavior Mode A (replacement)**: Regex-find `[[variant]]` WikiLinks, replace with `[[canonical]]`. **Mode B (creation)**: AST-walk markdown paragraphs/headings/lists, find plain-text variant mentions, wrap in `[[canonical]]`, skip code blocks/existing links. **Safety**: If `minSemanticSimilarity > 0`, query vec0 embeddings for cosine similarity; block if any variant below threshold.
- **Parity**: Output SHALL match C# `ConceptRepairTool.cs` (~639 lines)

## Non-Functional Requirements

### NFR-1: C# Oracle Parity
All 6 tools SHALL produce semantically equivalent JSON output to the C# oracle when invoked with identical parameters against identical data, verified by the existing parity test framework.

### NFR-2: Test Coverage
Each tool SHALL have tests at all 3 layers: boundary (in-process), CLI (binary), and parity (oracle comparison).

### NFR-3: No Regressions
Implementation SHALL NOT break any existing passing tests. `cargo test` SHALL remain green.

### NFR-4: Code Quality
All code SHALL pass `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`.
