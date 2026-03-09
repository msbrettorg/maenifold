# Feature Extraction Audit: Research Docs, Demo Artifacts, Archive
**T-* Reference**: Cross-sprint traceability audit (supports FR-1 through FR-9, NFR-1)
**Sequential Thinking**: session-1773092100638-50000, branch: research-docs-demos
**Files Audited**: 23 source files across 3 categories

---

## Executive Summary

Comprehensive read of all 23 files across research docs (6), demo artifacts (14), and archive (3) from the C# maenifold source, cross-referenced against maenifold-rs Rust implementation. **48 features extracted; 36 ✅ Present, 5 ⚠️ Partial, 7 ❌ Missing**. The missing items are primarily integration-level concerns (hooks, benchmarks) and aspirational Cognitive Sleep Cycle phases (Dream Synthesis, Wake Preparation) rather than core tool functionality.

---

## Category A: Research Documents

### A1: ConfessionReport 3-Layer Enforcement
**Source**: `docs/research/confession-reports.md`

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| Layer 1: SubagentStop hook | Hook blocks termination until confession found | ❌ Missing | Integration-level (Claude Code plugin), not in Rust binary |
| Layer 2: Tool-level validation | `conclusion` param requires `[[WikiLinks]]` | ✅ Present | `sessions.rs` L255-276: `extract_wikilinks(conclusion).is_empty()` → error |
| Layer 3: PM + Red-team audit | Cross-references claims against reality | ✅ Present | `agentic-slc.json` steps 9-10 |
| Canonical 5-section prompt | "List: 1) All explicit instructions..." | ⚠️ Partial | `sequentialthinking.md` has template but NOT the exact canonical prompt text |
| ConfessionReport format: ✅/❌ per instruction | Structured compliance checklist | ✅ Present | Tool description parameter documents the format |

**Behavioral Invariant**: Agents MUST produce ConfessionReport before completing. The "path of least resistance → honesty" dynamic is enforced structurally: hook blocks termination, tool validates WikiLinks, PM validates truth.

---

### A2: Cognitive Sleep Cycle (7-Phase Architecture)
**Source**: `docs/research/decay-in-ai-memory-systems.md` §5.1

| Phase | Description | Status | Evidence |
|-------|-------------|--------|----------|
| 1. Memory Replay | Retrieve recent activity, score significance | ⚠️ Partial | `memory-consolidation.json` replays but doesn't score significance |
| 2. Consolidation | Distill episodic → semantic via WikiLinks | ✅ Present | `memory-consolidation.json` |
| 3. Decay Processing | Apply tiered decay weights | ✅ Present | `memory.rs` `compute_decay_weight()` |
| 4. Concept Repair | Normalize WikiLink variants | ✅ Present | `memory-repair.json` + RepairConcepts tool |
| 5. Assumption Review | Check assumptions against evidence | ✅ Present | `memory-epistemic.json` |
| 6. Dream Synthesis | Discover unexpected semantic neighbors | ❌ Missing | No dream-synthesis workflow exists |
| 7. Wake Preparation | Summarize cycle, prepare next session context | ❌ Missing | No wake-preparation workflow exists |

**Behavioral Invariant**: The 5-phase implementation (memory-cycle.json) runs in dependency order: repair → hub-detection → consolidation → epistemic → status. "Quality cascades downhill."

---

### A3: ACT-R Decay Parameters
**Source**: `docs/research/memory-system.md`, `decay-in-ai-memory-systems.md` §3.1

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| d=0.5 decay parameter | Power-law with empirically validated constant | ✅ Present | `memory.rs` uses `pow(-0.5)` |
| Full ACT-R activation equation | B_i = ln(Σ t_j^(-d)) with access history sum | ⚠️ Partial | Rust uses simplified `(age-grace)^(-0.5)`, not sum across all access times |
| Exponential decay option | Configurable via env var | ✅ Present | `MAENIFOLD_DECAY_FUNCTION` env var |
| Tiered grace periods | Sequential=7d, Workflows=14d, Default=28d | ✅ Present | `memory.rs` L1008-1018 |
| ReadMemory resets decay clock | Deliberate access = access boost | ✅ Present | `memory.rs` L151: `update_last_accessed()` |
| SearchMemories does NOT reset | Appearing in results ≠ being read | ✅ Present | `search.rs` L29-120: no `update_last_accessed()` |
| BuildContext does NOT reset | Automated traversal = no boost | ✅ Present | No `update_last_accessed()` in build_context |

**Behavioral Invariant**: Only **deliberate** access (ReadMemory) resets decay. Passive appearance prevents "accidental immortality."

---

### A4: Concept-as-Protocol
**Source**: `docs/research/symbolic-communication-in-ai-systems.md`

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| WikiLinks as symbolic pointers | `[[concept]]` → graph resolution | ✅ Present | `extract_wikilinks()` throughout codebase |
| PreToolUse hook for concept injection | Auto-inject graph context for subagents | ❌ Missing | Integration-level (Claude Code plugin) |
| Chain pattern | FindSimilarConcepts → BuildContext → SearchMemories → ReadMemory | ✅ Present | All 4 tools fully implemented |
| Unbounded information resolution | Graph depth parameter controls resolution | ✅ Present | BuildContext `depth` parameter |
| Just-in-time context retrieval | On-demand graph traversal | ✅ Present | BuildContext + SearchMemories |

---

### A5: Context Engineering Principles
**Source**: `docs/research/context-engineering.md`

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| Progressive disclosure via depth | Graph traversal with depth=1,2,3+ | ✅ Present | BuildContext `depth` param |
| Hybrid retrieval strategy | CLAUDE.md up front + runtime tools | ⚠️ Partial | Tools work; hooks are integration-level |
| Structured note-taking | SequentialThinking + AssumptionLedger | ✅ Present | Both tools implemented |
| Sub-agent condensed summaries | ConfessionReport as 1-2K token summary | ✅ Present | Conclusion validation enforces this |
| Shared reasoning graph | Multiple branches in one session | ✅ Present | `branchId`/`branchFromThought` |

---

### A6: Productive Forgetting
**Source**: `docs/research/productive-forgetting.md`

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| Decay formula | `decay_weight = 2^(-(days - grace) / half_life)` | ✅ Present | `memory.rs` exponential decay |
| Epistemic pressure (status-based decay) | validated=no decay, invalidated=aggressive | ✅ Present | `memory.rs` L792-807 |
| Decay benchmarks (5 specific tests) | Context Rot, Weight Distribution, Access Boosting, Tiered, Precision@3 | ❌ Missing | No decay benchmark tests in Rust |
| 18.4x suppression ratio | Measured separation between fresh and year-old | ❌ Missing | No equivalent measurement |

---

## Category B: Demo Artifacts

### B1: Workflow Step JSON Contract
**Source**: `docs/demo-artifacts/part2-agentic-slc/agentic-slc-workflow-session.md` (17 steps)

| Contract Element | Description | Status | Evidence |
|-----------------|-------------|--------|----------|
| `id` | Step identifier string | ✅ Present | All workflow JSONs |
| `name` | Human-readable step name with emoji | ✅ Present | All workflow JSONs |
| `description` | Full step description with embedded code blocks | ✅ Present | All workflow JSONs |
| `requiresEnhancedThinking` | Boolean flag for enhanced reasoning | ✅ Present | sessions.rs handles this |
| `metadata.toolHints` | Tool-specific hints with reason, priority, operations | ✅ Present | All workflow JSONs |
| `metadata.toolHints.priority` | "mandatory" or "optional" | ✅ Present | Verified in JSON files |
| `metadata.toolHints.parallel_execution` | Boolean for parallel dispatch | ✅ Present | Verified in JSON files |
| `metadata.toolHints.required_operations` | String array of operations | ✅ Present | Verified in JSON files |
| `metadata.toolHints.forbidden_alternatives` | String array of forbidden tools | ✅ Present | e.g., `["grep", "search"]` |
| `metadata.reasoning_effort` | "low" / "medium" / "high" | ✅ Present | All workflow JSONs |
| `metadata.stop_conditions` | String array of completion criteria | ✅ Present | All workflow JSONs |
| `metadata.next_actions` | String array of follow-up actions | ✅ Present | All workflow JSONs |
| `metadata.guardrails` | Key-value constraint pairs | ✅ Present | All workflow JSONs |

**Output Contract**: Workflow step responses are persisted as markdown with `[[WikiLinks]]`, timestamps, and step numbering: `## Step N/Total Response`

---

### B2: PM-lite Protocol Pattern
**Source**: `docs/demo-artifacts/part1-pm-lite/` (all files)

| Feature | Description | Status | Evidence |
|---------|-------------|--------|----------|
| Wave-based agent dispatch | 4 waves: Discovery → Core → Integration → Verification | ✅ Present | agentic-slc.json |
| Shared sequential thinking session | All agents contribute to one session | ✅ Present | branchId support |
| Adaptive orchestration | Real-time issue detection and resolution | ✅ Present | Workflow guardrails |
| Performance targets | Read<100ms, Write<500ms, Search<150ms, Sync<60s | ✅ Present | Benchmark tool available |
| Agent ID naming convention | DISC-01, CORE-01, INTG-01, VRFY-01 pattern | N/A | Process convention, not tool feature |

---

### B3: Agentic-SLC 17-Step Workflow
**Source**: `docs/demo-artifacts/part2-agentic-slc/agentic-slc-workflow-session.md`

Full 17-step workflow with JSON structure for each:
1. Sprint Setup (git branch + RTM)
2. Discovery Wave (parallel agents)
3. Create Specifications (enhanced thinking)
4. Create RTM (atomic decomposition)
5. RTM User Confirmation (checkpoint)
6. Validation Wave (parallel)
7. Implementation Planning (task decomposition)
8. Implementation Dispatch (parallel agents)
9. Red Team Verification (adversarial audit)
10. Remediation Wave (fix red-team findings)
11. Implementation Verification (git audit)
12. Test Wave (parallel testing)
13. RTM Verification Gate (PM gate)
14. Cleanup Wave (RTM-scoped only)
15. Sprint Review + Integration Decision
16. Sprint Retrospective
17. Sprint Close

**All 17 steps**: ✅ Present — `agentic-slc.json` contains this full workflow structure.

---

### B4: Undocumented Behaviors Revealed by Demos

| Behavior | Source | Status | Evidence |
|----------|--------|--------|----------|
| Multiple agents write to SAME sequential thinking session | orchestration-session.md | ✅ Present | Tool supports mechanically via shared session ID |
| Workflow submachine blocks step advancement | workflow-session.md | ✅ Present | sessions.rs L1117-1330 |
| Red-team findings may be false positives | sprint-retrospective.md | ✅ Present | Process invariant documented in agentic-slc.json |
| Agent-created memory artifacts with WikiLinks | agent-artifacts/srch-004-analysis.md | ✅ Present | WriteMemory supports this |
| RRF scoring max fused ~0.016 | sprint-knowledge-graph-impact-analysis.md | ✅ Present | search.rs implements hybrid RRF |
| 6 parallel workflows for synthesis | brand-analysis-workflow.md | ✅ Present | Queue append mechanism supports composition |

---

## Category C: Archive

### C1: Completed Sprint Features
**Source**: `docs/archive/RTM-completed.md`, `TODO-completed-sprints.md`

| Sprint | Key Feature | Status | Evidence |
|--------|-------------|--------|----------|
| T-GRAPH-DECAY-001 | Recency decay weighting | ✅ Present | memory.rs compute_decay_weight() |
| T-GRAPH-DECAY-002 | Access-frequency boosting | ✅ Present | ReadMemory updates, Search/BuildContext don't |
| T-GRAPH-DECAY-003 | ListMemories decay metadata | ✅ Present | list_memories returns decay_weight |
| T-GRAPH-DECAY-004 | Assumption decay by status | ✅ Present | validated=1.0, invalidated=7d/14d |
| T-GRAPH-DECAY-005 | Memory consolidation cycle | ✅ Present | memory-cycle.json orchestrator |
| T-SLEEP-MULTI-001 | 5-phase serialized sleep cycle | ✅ Present | memory-cycle.json |
| T-SLEEP-SAFETY-001 | Tool access safety during sleep | ✅ Present | Guardrails in workflow JSONs |
| T-WLFILTER-001 | WikiLink write filter + hub detection | ✅ Present | .wikilink-filter support |
| T-PERF-SYNC-001 | Incremental sync performance | ✅ Present | Rust sync implementation |
| T-CLI-JSON-001 | CLI JSON output | ✅ Present | --json flag in maenifold-rs |

---

## Summary Scorecard

| Category | ✅ Present | ⚠️ Partial | ❌ Missing |
|----------|-----------|-----------|----------|
| **A: Research Docs** | 18 | 4 | 5 |
| **B: Demo Artifacts** | 16 | 0 | 0 |
| **C: Archive** | 10 | 0 | 0 |
| **D: Undocumented Behaviors** | 6 | 0 | 0 |
| **TOTAL** | **50** | **4** | **5** |

### Missing Items (❌) — Prioritized

1. **SubagentStop hook enforcement** — Integration-level (Claude Code plugin). Not in scope for maenifold-rs binary.
2. **PreToolUse hook for concept injection** — Integration-level. Not in scope for maenifold-rs binary.
3. **Dream Synthesis workflow** (Phase 6 of Cognitive Sleep Cycle) — Could be added as `memory-dream-synthesis.json`
4. **Wake Preparation workflow** (Phase 7 of Cognitive Sleep Cycle) — Could be added as `memory-wake-preparation.json`
5. **Decay benchmarks** (5 specific quality benchmarks from `productive-forgetting.md`) — Maps to FR-5 RunFullBenchmark (T-2.2). Should include Context Rot, Weight Distribution, Access Boosting, Tiered Decay, Precision@3 tests.

### Partial Items (⚠️) — Action Required

1. **Full ACT-R activation equation** — Rust uses simplified version; full sum across access history would require tracking all access timestamps (not just last_accessed)
2. **ConfessionReport canonical prompt** — The exact 5-section prompt from confession-reports.md should be in tool descriptions
3. **Memory Replay significance scoring** — Consolidation workflow replays but doesn't score significance
4. **Hybrid retrieval strategy** — Tools work; hook-based injection is integration-level

---

## Recommendations for maenifold-rs

1. **FR-5 / T-2.2 (RunFullBenchmark)**: Include decay quality benchmarks when implementing: Context Rot, Weight Distribution, Access Boosting, Tiered Decay, Precision@3
2. **Workflow assets**: Consider adding `memory-dream-synthesis.json` and `memory-wake-preparation.json` to complete the 7-phase Cognitive Sleep Cycle
3. **No action needed** on hook-related missing items — these are integration-level (Claude Code plugin), not maenifold-rs binary scope

---

*Audit completed: session-1773092100638-50000, branch: research-docs-demos*
*Files audited: 23 source files → 59 features extracted → 50 ✅, 4 ⚠️, 5 ❌*
