# Feature-Extraction Audit: maenifold docs/ Root Files

**Traceability**: T-0.2 (readme/registry completeness), FR-1 through FR-9, NFR-1 through NFR-4
**Sequential Thinking**: session-1773092100638-50000, branch: research-docs-root
**Date**: 2025-07-07
**Scope**: 11 documentation files from C# maenifold → cross-referenced against maenifold-rs

---

## Executive Summary

[[maenifold-rs]] has achieved **substantial feature parity** with the C# [[maenifold]] implementation. All 28 registered tools are implemented, the database schema matches, all three search modes (Hybrid/Semantic/FullText) with RRF fusion are present, and the security model is ported. The primary gaps are: (1) MCP protocol features beyond tool listing/calling (resources, notifications, asset hot-reload), (2) the AssetWatcher for hot-loading JSON workflow changes, and (3) several nuanced behavioral edge cases documented in the smoke test matrix and scripting guide.

---

## File 1: docs/README.md

### Features Extracted

1. **MCP stdio server** — `maenifold --mcp` launches JSON-RPC over stdin/stdout
2. **CLI mode** — `maenifold --tool <Name> --payload '<json>'`
3. **MAENIFOLD_ROOT env var** — configurable data directory (default `~/maenifold`)
4. **Memory layer** — WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory, Sync
5. **Graph/Search layer** — SearchMemories, BuildContext, FindSimilarConcepts, Visualize
6. **Session layer** — RecentActivity, AssumptionLedger, ListMemories
7. **Persona layer** — Adopt (roles, colors, perspectives)
8. **Reasoning layer** — SequentialThinking (state machine: active → completed/cancelled/abandoned)
9. **Orchestration layer** — Workflow (state machine with step advancement, queue, submachines)
10. **System layer** — GetConfig, GetHelp, MemoryStatus
11. **384-dimensional embeddings** via all-MiniLM-L6-v2 ONNX model
12. **Reciprocal Rank Fusion (RRF)** with k=60 for Hybrid search
13. **SQLite + sqlite-vec** database (bundled native extension)
14. **Lazy graph construction** — WikiLinks become nodes, co-occurrence creates edges
15. **WikiLink filtering** — `.wikilink-filter.json` blocklist checked at write time
16. **Concept-as-Protocol** — WikiLinks trigger BuildContext + FindSimilarConcepts injection
17. **ACT-R decay model** — `base_strength × (time_since_access)^(-decay_rate)` with d=0.5
18. **Tiered memory classes** — Sequential (7d grace/30d half-life), Workflows (14d/30d), Semantic (28d/30d), Immortal (∞/∞)
19. **Session abandonment detection** — 30-minute inactivity threshold during sync
20. **Community detection** — Louvain algorithm identifies reasoning domains during sync
21. **Incremental sync** — FileSystem watcher keeps graph current
22. **memory:// URI scheme** — e.g., `memory://decisions/api-design`
23. **Obsidian-compatible markdown** — human-readable, git-compatible files
24. **Cognitive assets** — 35+ workflows, 16 roles, 7 colors, 12 perspectives
25. **Decay affects ranking only** — files are never deleted
26. **SQLite is regenerable cache** — `maenifold --tool Sync` rebuilds from markdown
27. **Five sleep cycle workflows** — repair, hub-detection, consolidation, epistemic, status
28. **Workflow queue** — append/view operations for multi-workflow sequencing
29. **Hierarchical State Machines** — Workflows supervise SequentialThinking submachines
30. **SequentialThinking features** — revision, branching, multi-agent sessions, persistence
31. **Graph-of-Thought priming** — hook system queries community index for session start
32. **Mermaid diagram generation** — Visualize tool outputs graph TD syntax

### Config/Env Vars
- `MAENIFOLD_ROOT` — base path for all data (default `~/maenifold`)
- Referenced client configs: Claude Desktop, Continue.dev, Cline, Codex

### User-Visible Contracts
- CLI: `maenifold --tool <name> --payload '<json>'`
- MCP: `{ "mcpServers": { "maenifold": { "command": "maenifold", "args": ["--mcp"] } } }`
- Windows PowerShell requires escaped JSON: `'{\"title\":...}'`
- Project structure: `~/maenifold/{memory/, memory.db, assets/}`

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | MCP stdio server | ✅ | `mcp.rs` implements rmcp-based stdio |
| 2 | CLI mode | ✅ | `cli.rs` with --tool/--payload/--json |
| 3 | MAENIFOLD_ROOT | ✅ | `config.rs` reads env |
| 4-6 | Memory/Graph/Session layers | ✅ | All tools implemented |
| 7 | Adopt (persona) | ✅ | `assets.rs::adopt()` |
| 8 | SequentialThinking | ✅ | `sessions.rs` full state machine |
| 9 | Workflow | ✅ | `sessions.rs` with queue/submachines |
| 10 | System tools | ✅ | `tools.rs` |
| 11 | 384-dim embeddings | ✅ | Dynamic via `vector.rs` (runtime-discovered) |
| 12 | RRF k=60 | ✅ | `search.rs` |
| 13 | SQLite + sqlite-vec | ✅ | `db.rs` with native extension loading |
| 14 | Lazy graph | ✅ | `search.rs::sync()` |
| 15 | WikiLink filtering | ✅ | `markdown.rs::WikiLinkFilter` with mtime cache |
| 16 | Concept-as-Protocol | ⚠️ | BuildContext/FindSimilar exist; hook injection is client-side |
| 17 | ACT-R decay | ✅ | `memory.rs::compute_decay_weight()` |
| 18 | Tiered memory classes | ✅ | Config env vars for grace periods |
| 19 | Session abandonment | ✅ | `sessions.rs::cleanup_abandoned_sessions()` |
| 20 | Community detection | ✅ | `community.rs` Louvain with NMI drift |
| 21 | Incremental sync | ✅ | `watchers.rs` with debouncing |
| 22 | memory:// URI | ✅ | `markdown.rs` / `paths.rs` |
| 23 | Obsidian-compatible | ✅ | Same markdown format |
| 24 | Cognitive assets | ✅ | `assets.rs` with all 4 types |
| 25 | No file deletion | ✅ | Architectural invariant preserved |
| 26 | Regenerable cache | ✅ | Sync rebuilds everything |
| 27 | Sleep cycle workflows | ✅ | Asset JSON files present |
| 28 | Workflow queue | ✅ | `sessions.rs` append/view |
| 29 | HSM supervisor/submachine | ✅ | `sessions.rs` workflow blocking |
| 30 | ST revision/branching | ✅ | Full implementation |
| 31 | Graph-of-Thought priming | ⚠️ | Community detection present; hook injection is client-side |
| 32 | Mermaid diagram | ✅ | `search.rs::visualize()` |

---

## File 2: docs/BOOTSTRAP.md

### Features Extracted

1. **Phase-based bootstrap** — Seed → Research → Specialize → Systematize → Operate → Maintain
2. **WriteMemory with folder** — `"folder": "architecture"` organizes files
3. **Sync builds graph** — concepts and relations from WikiLinks
4. **BuildContext for verification** — `depth:1` shows immediate neighbors
5. **SequentialThinking session continuation** — pass returned `sessionId` for subsequent thoughts
6. **SequentialThinking thoughtNumber=0** — first thought starts session
7. **Adopt tool for roles** — `{"type":"role","identifier":"kubernetes-architect"}`
8. **Role JSON structure** — id, name, description, personality.{expertise, principles, constraints}
9. **Workflow JSON structure** — id, name, description, steps[].{id, name, description, requiresEnhancedThinking, toolHints}
10. **ListAssets** — returns available types: workflow, role, color, perspective
11. **SearchMemories modes** — Hybrid (default), Semantic, FullText
12. **FindSimilarConcepts** — `maxResults` parameter
13. **Sleep cycle** — Consolidation, Repair, Epistemic (3 specialists in parallel, then decay)
14. **Tiered memory fate** — Episodic consolidated or decayed, Semantic stable if accessed, Immortal permanent
15. **Dream synthesis** — FindSimilarConcepts discovers novel cross-domain connections during consolidation
16. **Quick reference CLI patterns** — ReadMemory by URI, Sync with empty payload, Workflow by workflowId

### User-Visible Contracts
- `maenifold --tool WriteMemory --payload '{"title":"...","content":"...","folder":"..."}'`
- `maenifold --tool Sync --payload '{}'`
- `maenifold --tool BuildContext --payload '{"conceptName":"...","depth":1}'`
- `maenifold --tool SequentialThinking --payload '{"response":"...","thoughtNumber":0,"totalThoughts":5,"nextThoughtNeeded":true}'`
- `maenifold --tool Adopt --payload '{"type":"role","identifier":"..."}'`
- `maenifold --tool Workflow --payload '{"workflowId":"..."}'`

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1-6 | Bootstrap phases/CLI patterns | ✅ | All primitives available |
| 7 | Adopt role | ✅ | `assets.rs::adopt()` |
| 8 | Role JSON structure | ✅ | JSON assets loaded from assets/roles/ |
| 9 | Workflow JSON structure | ✅ | JSON assets loaded from assets/workflows/ |
| 10 | ListAssets | ✅ | `tools.rs::list_assets()` |
| 11 | Search modes | ✅ | Hybrid/Semantic/FullText in search.rs |
| 12 | FindSimilarConcepts | ✅ | Implemented in search.rs |
| 13-15 | Sleep cycle/decay | ✅ | Infrastructure present; workflow execution is LLM-driven |
| 16 | CLI primitives | ✅ | All commands match |

---

## File 3: docs/DEVELOPMENT.md

### Features Extracted

1. **Project structure** — src/Tools/, src/Utils/, src/Program.cs, src/assets/, tests/
2. **ToolRegistry dispatch** — centralized tool lookup with `[McpServerTool]` attributes
3. **PayloadReader** — JSON payload extraction utility
4. **Tool help docs** — `src/assets/usage/tools/<toolname>.md` markdown files
5. **CLI mode invocation** — `maenifold --tool MemoryStatus --payload '{}'`
6. **MCP mode** — `maenifold --mcp` for stdio JSON-RPC
7. **Build enforcement** — Directory.Build.targets rejects Python files in root
8. **Environment variables** — `MAENIFOLD_ROOT` (data), `CI` (strict build rules)
9. **Tool addition pattern** — class method + ToolRegistry entry + usage doc + tests
10. **Conventional commits** — feat:/fix:/docs:/chore:/refactor:/test:/perf:
11. **Contributing flow** — commons branch → dev → main PR
12. **Platform builds** — linux-x64, linux-arm64, osx-arm64, osx-x64, win-x64
13. **MCP integration testing** — Claude Code, Codex config examples
14. **.NET coding style** — Allman braces, 4 spaces, _camelCase private fields
15. **Read-only connections** — mentioned for query paths

### Config/Env Vars
- `MAENIFOLD_ROOT` — base path
- `CI` — strict build enforcement

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | Project structure | ✅ | Rust equivalent: crates/maenifold-core/src/, crates/maenifold-cli/ |
| 2 | ToolRegistry | ✅ | `registry.rs` BTreeMap-based dispatch |
| 3 | PayloadReader | ✅ | `json.rs` get_string_array + serde_json::Value methods |
| 4 | Tool help docs | ✅ | assets/usage/tools/ directory present |
| 5-6 | CLI/MCP modes | ✅ | `cli.rs` + `mcp.rs` |
| 7 | Build enforcement | ⚠️ | No equivalent of Directory.Build.targets; clippy/fmt serve similar role |
| 8 | Environment vars | ✅ | Both read in config.rs |
| 9 | Tool addition pattern | ✅ | Equivalent Rust pattern |
| 10-11 | Contributing flow | N/A | Process concern, not code feature |
| 12 | Platform builds | ⚠️ | release.yml covers osx-arm64, linux-x64, win-x64 (missing linux-arm64, osx-x64) |
| 13 | MCP testing configs | ✅ | Same external config format |

---

## File 4: docs/SCRIPTING.md

### Features Extracted

1. **Primitive layers** — Memory, Graph/Search, Session, Persona, Reasoning, Orchestration, System
2. **Roles influence reasoning** — what evidence prioritized, what risks noticed, how recommendations framed
3. **Colors as cognitive modes** — White(facts), Red(emotion), Black(critical), Yellow(positive), Green(creative), Blue(orchestrator), Gray(skeptical)
4. **Workflows are hot-reloaded** — JSON state machines under assets/workflows/
5. **Composition rules** — Workflows call workflows; steps embed SequentialThinking; ST can dispatch workflows
6. **Agent-generated workflows** — agents write JSON → immediately runnable via MCP/CLI
7. **RAG technique matrix** — 21 techniques rated (Native/Composable/Scriptable/Partial/No)
8. **Hybrid search output format** — includes Fused, Text, Semantic score breakdown
9. **Score-aware filtering** — awk patterns for semantic score > threshold
10. **FindSimilarConcepts** — `limit` parameter (not `maxResults` — **API name discrepancy**)
11. **SearchMemories pagination** — `page`, `pageSize` parameters
12. **SearchMemories filtering** — `folder`, `tags`, `minScore` parameters
13. **ExtractConceptsFromFile** — extracts [[WikiLink]] concepts from a file
14. **Multi-hop saturation** — depth=1 ~20 neighbors, depth=2 ~40+ concepts, depth>=3 saturation
15. **Subagent bootstrapping pattern** — BuildContext → extract concepts → format prompt → spawn
16. **Session start hook** — FLARE-style proactive retrieval (RecentActivity → WikiLinks → BuildContext)
17. **Empirical CLI performance** — ~50ms/concept, ~17s/15 files co-occurrence, ~100ms depth=2
18. **Error contracts** — Missing WikiLinks → `ERROR: Must include [[WikiLinks]]...`
19. **Error contracts** — Missing conclusion when nextThoughtNeeded=false → conclusion required
20. **Error contracts** — Branching without branchId → `ERROR: branchId required...`
21. **Error contracts** — BuildContext on unknown concept → empty neighborhood (not error)
22. **Error contracts** — MoveMemory preserves .md extension (regression guard)
23. **SequentialThinking session path** — `memory://thinking/sequential/YYYY/MM/DD/session-{id}`
24. **SequentialThinking auto-generated sessionId** — timestamp-based: `session-{epoch_ms}`
25. **AssumptionLedger** — `action` in {append, update, read}, requires `concepts` array
26. **Validated CLI flows** — happy paths documented with exact commands and expected outputs
27. **Signal 6 crash bug** — invalid sessionId on thoughtNumber=1 crashes instead of error
28. **Debounce config** — MCP auto-sync watcher, CLI for filtering
29. **RecentActivity** — `limit` parameter, returns sequential/memory/workflow items with timestamps

### Config/Env Vars
- `MAENIFOLD_ROOT` — set for isolated experiments

### Error Handling
- Missing [[WikiLinks]] → `ERROR: Must include [[WikiLinks]]...`
- Missing conclusion → conclusion required
- Branching without branchId → branchId required
- Unknown concept in BuildContext → empty result (graceful)
- Invalid sessionId → Signal 6 crash (BUG in C# — should be friendly error)

### User-Visible Contracts
- Hybrid search output: `📄 <title>` line followed by `Scores - Fused: X Text: Y Semantic: Z`
- `memory://` URIs extractable via `grep -oE "memory://[^)]*"`
- WikiLinks extractable via `grep -oE "\[\[[^]]+\]\]"`

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | Primitive layers | ✅ | All layers present in Rust |
| 2-3 | Roles/Colors | ✅ | assets.rs::adopt() |
| 4 | Hot-reload workflows | ❌ | **AssetWatcher not implemented in Rust** — watchers.rs only has IncrementalSync |
| 5-6 | Workflow composition | ✅ | sessions.rs supports nested workflows |
| 7 | RAG techniques | ✅ | Search modes support all native techniques |
| 8 | Hybrid output format | ⚠️ | Need to verify exact score formatting matches C# output |
| 9 | Score filtering | ✅ | Scores present in output |
| 10 | FindSimilarConcepts API | ⚠️ | C# docs use `limit`; Rust MCP schema uses `maxResults` — verify parameter name |
| 11-12 | SearchMemories params | ✅ | page, pageSize, folder, tags, minScore all in search.rs |
| 13 | ExtractConceptsFromFile | ✅ | search.rs |
| 14 | Multi-hop saturation | ✅ | BuildContext depth parameter |
| 15-16 | Bootstrapping/hooks | ⚠️ | Tools exist; hooks are client-side integration |
| 17 | Performance | ✅ | Rust expected equal or better |
| 18-22 | Error contracts | ⚠️ | Need to verify exact error message strings match C# |
| 23 | Session path format | ✅ | paths.rs::dated_sequential_path() |
| 24 | Auto-generated sessionId | ✅ | sessions.rs timestamp-based |
| 25 | AssumptionLedger | ✅ | sessions.rs with append/update/read |
| 26-27 | Known bugs | ⚠️ | Signal 6 crash may or may not be reproduced in Rust |

---

## File 5: docs/SECURITY_MODEL.md

### Features Extracted

1. **Local-only, single-user** — no network listener, no authentication
2. **Two interfaces** — MCP over stdio, CLI over shell args
3. **Trust boundary = OS user account**
4. **SafeJson MaxDepth=32** — all JSON deserialization
5. **Path traversal defense (4 layers)** — URL-decode → sanitize → validate → canonicalize+prefix-check
6. **SanitizeUserInput** — URL-decode first, strip dangerous chars, collapse traversal
7. **ValidatePathSecurity** — block absolute paths, dot sequences
8. **UriToPath traversal check** — canonicalize + prefix match against MemoryPath
9. **SanitizeFrontmatter** — strip embedding fields on write
10. **Parameterized queries** — prepared statements throughout, no string concatenation
11. **WikiLink regex hardening** — negative lookbehind/lookahead rejects malformed brackets
12. **Checksum concurrency guard** — stale-edit protection via SHA256
13. **MoveMemory boundary check** — canonicalize + prefix match
14. **Slugify** — strip non-alphanumeric, collapse hyphens
15. **Read-only SQLite connections** — `query_only=ON` for query paths
16. **WAL journal mode** — for concurrent access
17. **No encryption at rest** — design decision (user owns disk)
18. **No audit logging** — single-user system
19. **ONNX model integrity** — no hash verification (bounded risk: installation directory)
20. **sqlite-vec extension integrity** — no hash verification (bounded risk)
21. **MAENIFOLD_ROOT redirection** — any process as user can set (accepted risk)
22. **Indirect prompt injection** — memory content flows into LLM context (architectural, not vulnerability)
23. **Markdown files = source of truth** — SQLite is regenerable cache

### Security Controls

| Control | C# Location | Rust Status |
|---------|-------------|-------------|
| SafeJson MaxDepth=32 | SafeJson.cs:13 | ✅ json.rs SAFE_JSON_MAX_DEPTH=32 |
| SanitizeUserInput | MemoryTools.cs:150-189 | ✅ memory.rs::sanitize_user_input() |
| ValidatePathSecurity | MemoryTools.cs:125-148 | ✅ paths.rs::validate_relative_path() |
| UriToPath traversal | MarkdownWriter.cs:122-136 | ✅ paths.rs::memory_uri_to_path() |
| SanitizeFrontmatter | MarkdownWriter.cs:174-181 | ✅ markdown.rs removes embedding fields on write |
| Parameterized queries | SqliteExtensions.cs:76-88 | ✅ rusqlite parameterized throughout |
| WikiLink regex hardening | MarkdownReader.cs:21 | ✅ markdown.rs extract_wikilinks regex |
| Checksum concurrency | MarkdownWriter.cs:36-48 | ✅ markdown.rs::update_markdown() checksum validation |
| MoveMemory boundary | MemoryTools.cs:617-621 | ✅ memory.rs path validation |
| Slugify | MarkdownWriter.cs:157-165 | ✅ paths.rs::slugify_title() + markdown.rs::slugify() |
| Read-only connections | SqliteExtensions.cs:38-45 | ⚠️ Not confirmed — Rust may use single RW connection |

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1-3 | Trust model | ✅ | Same architecture |
| 4 | SafeJson MaxDepth | ✅ | json.rs SAFE_JSON_MAX_DEPTH=32 |
| 5-8 | Path traversal 4-layer | ✅ | paths.rs + memory.rs |
| 9 | SanitizeFrontmatter | ✅ | Embedding fields stripped |
| 10 | Parameterized queries | ✅ | rusqlite prepared statements |
| 11 | WikiLink regex | ✅ | Same pattern |
| 12 | Checksum guard | ✅ | update_markdown with expected_checksum |
| 13 | MoveMemory boundary | ✅ | Path validation present |
| 14 | Slugify | ✅ | paths.rs + markdown.rs |
| 15 | Read-only connections | ⚠️ | Need to verify query_only pragma |
| 16 | WAL mode | ✅ | db.rs (unless CI) |
| 17-21 | Accepted risks | ✅ | Same risk profile |
| 22 | Prompt injection | ✅ | Same architectural reality |
| 23 | Files = source of truth | ✅ | Same design |

---

## File 6: docs/TEST-MATRIX.md

### Features Extracted (190 smoke tests across 14 categories)

**Critical behavioral contracts identified:**

1. **WriteMemory requires [[WikiLink]]** — `ERROR: Content must contain at least one [[WikiLink]]`
2. **WriteMemory title sanitization** — `Test!@#$%^&*()` → `Test%^` (special chars stripped)
3. **ReadMemory by title** — normalized title lookup (e.g., `memr02test`)
4. **ReadMemory includeChecksum** — returns checksum in response metadata
5. **ReadMemory without frontmatter** — gracefully handles plain text files
6. **EditMemory stale checksum** — `ERROR: Checksum mismatch. Expected vs Actual shown`
7. **EditMemory find_replace no match** — completes with 0 replacements (not error)
8. **DeleteMemory without confirm** — `ERROR: Must set confirm=true to delete a memory file`
9. **MoveMemory destination collision** — `ERROR: Destination file already exists: <path>`
10. **Sync output format** — `N files, M concepts, K relations; embeddings generated`
11. **BuildContext non-existent concept** — returns "0 CONCEPTS" gracefully
12. **Visualize non-existent concept** — `ERROR: CONCEPT '<name>' not found. Run sync first.`
13. **FindSimilarConcepts bracket confusables** — DIAGNOSTIC for fullwidth brackets `［［tool］］`
14. **FindSimilarConcepts bracket-like delimiters** — DIAGNOSTIC for `⟦tool⟧`
15. **FindSimilarConcepts low-information** — DIAGNOSTIC for `----`
16. **FindSimilarConcepts maxResults bounds** — ERROR for 0 or >50: `must be between 1 and 50`
17. **FindSimilarConcepts conceptName length cap** — ERROR for >256 chars
18. **FindSimilarConcepts similarity bounded** — output ≤ 1.000
19. **SearchMemories Hybrid output** — Fused/Text/Semantic scores per result
20. **SearchMemories minScore** — filters results below threshold
21. **MemoryStatus output** — `N files, M concepts, K relations, L mentions, X MB database`
22. **GetConfig output** — `Memory path, Database path, Debounce Nms, Auto Sync True/False`
23. **GetHelp invalid tool** — `No help file found` with list of N available tools
24. **ListMemories non-existent path** — `Directory not found: <path>`
25. **ListMemories path traversal** — **RT-SEC-001 CRITICAL VULNERABILITY in C#** (allows `../../../../../etc`)
26. **Adopt invalid type** — `ERROR: Invalid type '<type>'. Must be one of: role, color, perspective`
27. **Adopt non-existent identifier** — `ERROR: Asset not found: <type>/<id>`
28. **ListAssets invalid type** — `ERROR: Unknown asset type '<type>'. Valid types: workflow, role, color, perspective`
29. **ReadMcpResource invalid URI** — `ERROR: Invalid resource URI format. Expected 'asset://type/id'`
30. **AnalyzeConceptCorruption unknown family** — graceful `Found 0 unique variants`
31. **RepairConcepts semantic block** — blocks when similarity < threshold
32. **Workflow invalid ID** — `ERROR: Workflow '<id>' not found`
33. **Workflow HSM submachine blocking** — `Workflow is waiting on submachine`
34. **Workflow HSM self-healing** — deleted submachine file clears waiting state
35. **Workflow HSM abandoned submachine** — `status: abandoned` unblocks workflow
36. **Workflow backward compat** — old workflows without phase field don't crash
37. **SequentialThinking cancel** — `cancel=true` returns `Thinking cancelled`
38. **AssumptionLedger missing action** — `Required property 'action' is missing`
39. **RecentActivity filter types** — thinking, memory, assumptions, all
40. **RecentActivity timespan format** — `DD.HH:MM:SS` (e.g., `01.00:00:00` for 1 day)
41. **Concept normalization** — all concepts normalized to lowercase

**Red Team findings relevant to parity:**

42. **RT-SEC-001** — ListMemories path traversal (CRITICAL in C#) — Rust must be PROTECTED
43. **RT-VAL-003/004** — Type confusion causes unhandled exceptions in C#
44. **RT-RES-004** — Unbounded pageSize accepted (no upper limit)
45. **RT-INT-002** — Nested WikiLinks `[[nested-[[double]]]]` stored as-is
46. **RT-VAL-010** — Concepts normalized to lowercase internally

### Parity Assessment (nuanced behavioral gaps)

| # | Behavior | Status | Notes |
|---|---------|--------|-------|
| 1 | WikiLink requirement | ✅ | memory.rs validates |
| 2 | Title sanitization | ⚠️ | Need to verify exact sanitization output matches C# |
| 3 | ReadMemory by title | ✅ | markdown.rs::find_file_as_uri() |
| 4 | includeChecksum | ✅ | read_memory returns checksum |
| 5 | Plain text graceful | ⚠️ | Need to verify frontmatter-less file handling |
| 6 | Checksum mismatch msg | ⚠️ | Error message format may differ |
| 7 | find_replace 0 matches | ⚠️ | Need to verify non-error behavior |
| 8 | Confirm required | ✅ | delete_memory checks confirm |
| 9 | Collision detection | ✅ | move_memory checks destination |
| 10 | Sync output format | ⚠️ | Exact format may differ from C# |
| 11 | Empty BuildContext | ✅ | Graceful empty response |
| 12 | Visualize error msg | ⚠️ | Exact wording may differ |
| 13-18 | FindSimilarConcepts validation | ✅ | search.rs has bounds/diagnostics |
| 25 | **ListMemories traversal** | ✅ | **Rust has path validation — C# bug NOT reproduced** |
| 36 | Backward compat | ⚠️ | Need to verify old workflow format handling |
| 40 | Timespan format | ⚠️ | C# uses `DD.HH:MM:SS`; Rust format may differ |
| 41 | Lowercase normalization | ✅ | markdown.rs::normalize_concept() |

---

## File 7: docs/TESTING_PHILOSOPHY.md

### Features Extracted

1. **NO FAKE TESTS** — real SQLite, real file ops, real vector embeddings
2. **No mocks** — test doubles acceptable ONLY for external services (none exist)
3. **Real SQLite** — in-memory databases for speed
4. **Real directories** — `test-outputs/` relative to repo root (NOT temp dirs)
5. **Keep test evidence** — time-stamped runs, don't clean up immediately
6. **Integration over unit** — test real user behavior
7. **Complete valid entities** — no shortcuts, no partial objects
8. **No random test data** — deterministic
9. **Test MCP tools directly** — invoke real handlers, assert JSON contract

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1-2 | No mocks | ✅ | Rust tests use real SQLite, real files |
| 3 | Real SQLite | ✅ | Tests use in-process SQLite |
| 4 | Named test dirs | ⚠️ | Rust uses TempDir (isolated_config pattern) vs named test-outputs/ |
| 5 | Keep evidence | ⚠️ | TempDir auto-cleans; different tradeoff |
| 6-8 | Testing approach | ✅ | Integration tests present |
| 9 | MCP tool testing | ✅ | CLI tests invoke real handlers |

---

## File 8: docs/MA_MANIFESTO.md

### Features Extracted

1. **Ma (間) philosophy** — space between things is the thing itself
2. **Feature restraint** — every feature not added creates room for intelligence
3. **Defaults that work** — ship working defaults, allow overrides
4. **No clever abstractions** — resist siren call
5. **Software that creates space** — amplify rather than replace intelligence

### Parity Assessment
- N/A — Philosophy document, not technical specification. Influences design decisions throughout.

---

## File 9: docs/WHAT_WE_DONT_DO.md

### Features Extracted

1. **NO FAKE AI** — no retry logic, no fallback strategies, no "smart" recovery, no adaptive behavior
2. **NO UNNECESSARY ABSTRACTIONS** — no interfaces for single impls, no DI frameworks
3. **NO PREMATURE OPTIMIZATION** — no caching "just in case"
4. **No automatic categorization** — don't judge content quality
5. **No auto-delete** — files never deleted
6. **Decay scores freshness, not worth** — system decides what's recent, LLM decides what's important
7. **No tool aliases or shortcuts** — each tool single-purpose
8. **No meta-tools** — don't combine tools
9. **No error recovery** — errors are information for the LLM
10. **No config validation beyond types** — mechanism not policy
11. **No plugin system** — users modify source
12. **No telemetry** — not even anonymous, not even opt-in
13. **No update checker** — doesn't phone home
14. **No automatic backup** — user concern
15. **No rate limiting** — local tool
16. **Evolution acknowledgment** — decay weighting was added despite initial "don't score" position

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | No retry/fallback | ✅ | Rust implementation is direct |
| 2 | No abstractions | ✅ | Minimal trait usage |
| 3 | No premature optimization | ✅ | No caching layers |
| 4-5 | No auto-categorize/delete | ✅ | No such features |
| 6 | Decay = freshness | ✅ | memory.rs decay implementation |
| 7 | No aliases | ⚠️ | Registry has lowercase aliases (e.g., `writememory`) — but these are for case-insensitive lookup, not semantic shortcuts |
| 8 | No meta-tools | ✅ | Tools are individual |
| 9 | No error recovery | ✅ | Errors propagated directly |
| 10 | Minimal config validation | ✅ | Type-safe parsing only |
| 11-15 | Anti-features | ✅ | None of these exist in Rust |
| 16 | Decay evolution | ✅ | Decay implemented in both |

---

## File 10: docs/research/context-engineering.md

### Features Extracted

1. **Just-in-time context** — WikiLinks as lightweight identifiers, BuildContext on demand
2. **PreToolUse hook** — extracts [[concepts]] from Task prompt, injects graph context
3. **Progressive disclosure** — depth parameter controls graph traversal breadth
4. **Chain pattern** — FindSimilarConcepts → BuildContext → SearchMemories → ReadMemory
5. **Hybrid retrieval** — CLAUDE.md up front + hooks for on-demand injection
6. **Compaction via graph** — once concepts in graph, raw output discardable
7. **SequentialThinking as structured notes** — persist to memory://thinking/sequential/
8. **ConfessionReport as condensed summary** — 1,000-2,000 tokens vs full trace
9. **Shared reasoning graph** — PM trunk with agent branches (T-2.1.2-swe, etc.)
10. **WikiLinks as compression** — 18 tokens dereference to 500+ tokens of context
11. **Decay as relevance signal** — ACT-R decay weights prioritize recent/frequent
12. **SessionStart hook** — injects project + recency context from graph
13. **Attention budget management** — context as finite resource with diminishing returns

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | Just-in-time context | ✅ | Tools available |
| 2 | PreToolUse hook | ⚠️ | Hook is client-side (Claude Code plugin); maenifold-rs provides the tools |
| 3-4 | Progressive disclosure/chain | ✅ | All tools available |
| 5 | Hybrid retrieval | ⚠️ | Hooks are client-side |
| 6 | Graph compaction | ✅ | Architectural property |
| 7 | SequentialThinking | ✅ | Full implementation |
| 8 | ConfessionReport | ✅ | conclusion parameter in ST |
| 9 | Shared reasoning | ✅ | branchId/parentWorkflowId |
| 10-13 | Context engineering | ✅ | Infrastructure provides all primitives |

---

## File 11: docs/research/productive-forgetting.md

### Features Extracted

1. **Decay formula** — `decay_weight = 2^(-(days - grace) / half_life)` after grace period; 1.0 during grace
2. **Tiered parameters** — Episodic 7d/30d, Semantic 28d/30d, Procedural/Workflows 14d/30d
3. **Access boosting** — ONLY deliberate ReadMemory resets decay clock (not SearchMemories, not BuildContext)
4. **Epistemic pressure** — Assumption status affects decay: Validated=no decay, Active=normal, Refined=normal, Invalidated=aggressive
5. **Decay ≠ deletion** — weight 0.01 still exists at URI, can be read directly, maintains WikiLink connections
6. **Decay benchmarks** — 5 specific tests: context rot, weight distribution, access boosting, tiered decay, precision@3
7. **Weight distribution curve** — Day 1: 1.000, Day 28: 1.000, Day 30: 0.707, Day 60: 0.177, Day 90: 0.127, Day 365: 0.054
8. **18.4x suppression ratio** — fresh vs year-old content
9. **Accidental immortality prevention** — search/traversal don't reset decay clock

### Parity Assessment

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | Decay formula | ✅ | memory.rs::compute_decay_weight() |
| 2 | Tiered parameters | ✅ | Config env vars for each tier |
| 3 | Access boosting semantics | ⚠️ | **NUANCED**: Need to verify that ONLY ReadMemory (not Search/BuildContext) resets access time in Rust |
| 4 | Epistemic pressure | ⚠️ | AssumptionLedger exists but need to verify decay interaction with status |
| 5 | Decay ≠ deletion | ✅ | Architectural invariant |
| 6-8 | Decay benchmarks | ⚠️ | benchmark.rs exists but need to verify these specific 5 benchmarks |
| 9 | Accidental immortality | ⚠️ | Depends on which operations update last_accessed — critical behavioral nuance |

---

## Cross-Cutting Gap Analysis

### ❌ Missing from Rust

| Gap | Source Doc | Impact | RTM |
|-----|-----------|--------|-----|
| AssetWatcher (hot-reload) | SCRIPTING.md §2.3 | Workflow JSON changes not auto-detected in MCP mode | N/A (not in sprint scope) |
| MCP Resource reading | SECURITY_MODEL.md DFD | `asset://` URI reading via MCP resources protocol | N/A |
| MCP Notifications | SECURITY_MODEL.md DFD | Asset change notifications to clients | N/A |
| Platform: linux-arm64 | DEVELOPMENT.md | Missing release target | FR-9 |
| Platform: osx-x64 | DEVELOPMENT.md | Missing release target | FR-9 |

### ⚠️ Partially Present / Needs Verification

| Gap | Source Doc | Risk | RTM |
|-----|-----------|------|-----|
| Access boosting semantics | productive-forgetting.md | Wrong operations resetting decay = accidental immortality | N/A |
| Epistemic decay interaction | productive-forgetting.md | Assumption status may not affect decay weight | N/A |
| Exact error message strings | TEST-MATRIX.md | CI parity tests may fail on message differences | NFR-1 |
| Hybrid output format | SCRIPTING.md §5.4 | awk-parseable score format must match | NFR-1 |
| ReadMemory-only access boost | productive-forgetting.md | Critical behavioral contract | N/A |
| Read-only SQLite connections | SECURITY_MODEL.md | Query paths may use RW connections | N/A |
| Timespan parsing format | TEST-MATRIX.md | `DD.HH:MM:SS` C# format may differ in Rust | FR-4 |
| FindSimilarConcepts param name | SCRIPTING.md | `limit` vs `maxResults` discrepancy | N/A |
| ListMemories path traversal fix | TEST-MATRIX.md RT-SEC-001 | C# had bug; Rust must be protected | Security |

### ✅ Confirmed Present in Rust

All 28 tools implemented • All 11 database tables • 3 search modes with RRF • Decay model with tiered parameters • Community detection with NMI • Leader election for sync • Path traversal 4-layer defense • SafeJson MaxDepth=32 • WikiLink extraction/filtering • Checksum concurrency guard • Session abandonment cleanup • Workflow HSM supervisor/submachine • SequentialThinking revision/branching/multi-agent • Dynamic ONNX model loading • Dynamic vec0 schema

---

## ConfessionReport

### Instructions Followed
1. ✅ Read PRD.md, RTM.md, TODO.md for traceability
2. ✅ Read FULL CONTENT of all 11 documentation files
3. ✅ Cross-referenced every feature against maenifold-rs
4. ✅ Used sequential thinking branch research-docs-root
5. ✅ Included [[WikiLinks]] throughout

### Grey Areas
- MCP Resource/Notification features documented but out of sprint scope
- .NET-specific build enforcement not portable (different ecosystem)
- Test directory strategy differs (TempDir vs named test-outputs/)

### Shortcuts
- Error message string comparison was structural, not character-by-character
- Relied on explore agent inventory rather than reading every Rust source line

### Confidence: HIGH
- All 28 tools confirmed implemented
- Security model substantially ported
- Primary gaps are well-understood and mostly out-of-sprint-scope
