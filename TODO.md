# TODO

## CLI-JSON-001: Add `--json` Output Flag (P0 - HIGHEST PRIORITY)

**Status**: ✅ Complete
**Priority**: **P0 (Critical - Blocks OpenClaw)**
**Created**: 2026-02-01
**Completed**: 2026-02-01

### Problem

OpenClaw integrations (SessionChannel, Memory Plugin) currently parse CLI output using fragile regex on markdown:

```typescript
// Current fragile parsing in OpenClaw SessionChannel
const SESSION_ID_REGEX = /session-\d+(?:-\d+)?/;
const ADDED_THOUGHT_REGEX = /Added thought (\d+)/i;
```

This blocks reliable programmatic integration. Need structured JSON output.

### Requirements (PRD FR-8.x)

- FR-8.1: CLI SHALL support `--json` flag
- FR-8.2: All MCP tools SHALL return JSON when flag present
- FR-8.3: JSON SHALL include `success`, `data`, `error` fields
- FR-8.4: Error responses SHALL include codes and messages
- FR-8.5: Backward compatible (omit flag = markdown)

### Target Output Format

```json
{
  "success": true,
  "data": {
    "sessionId": "session-1234567890",
    "thoughtNumber": 5,
    // ... tool-specific fields
  },
  "error": null
}
```

Error case:
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "INVALID_SESSION",
    "message": "Session not found: session-999",
    "details": { "sessionId": "session-999" }
  }
}
```

### Tasks

1. [x] T-CLI-JSON-001.1: Add `--json` flag parsing to CLI entry point
2. [x] T-CLI-JSON-001.2: Create `JsonResponse<T>` wrapper class
3. [x] T-CLI-JSON-001.3: Update SequentialThinking tool to return JSON
4. [x] T-CLI-JSON-001.4: Update SearchMemories tool to return JSON
5. [x] T-CLI-JSON-001.5: Update ReadMemory tool to return JSON
6. [x] T-CLI-JSON-001.6: Update BuildContext tool to return JSON
7. [x] T-CLI-JSON-001.7: Update WriteMemory, EditMemory, MoveMemory, DeleteMemory
8. [x] T-CLI-JSON-001.8: Write tests for JSON output format (20 tests)
9. [x] T-CLI-JSON-001.9: Security tests (29 tests, SEC-EDGE-002 remediated)

### Acceptance Criteria

- [x] `maenifold --tool SequentialThinking --payload '...' --json` returns valid JSON
- [x] All tools support `--json` flag
- [x] Error responses include error codes and messages
- [x] Omitting `--json` returns current markdown (backward compat)
- [x] No ANSI escape codes in JSON output
- [x] Tests verify JSON schema (364 total tests, 353 passing)

### Blocks

- **OpenClaw T-0.1**: SessionChannel JSON migration
- **OpenClaw T-0.2**: Memory Plugin implementation

---

## REL-001: Release v1.0.3

**Status**: ✅ Complete (2026-02-01)
**Priority**: High

### Changes for v1.0.3

**Changed:**
- GRAPH-001: BuildContext extracts H2 section containing concept mention
- HOOKS-002: Enhanced graph_rag.sh with depth=2, score filtering, frequency ranking
- EVAL-001e: Refactored SessionStart hook to v2 (JSON output, portable CLI, CWD context)
- SequentialThinking enforces thoughtNumber=0 for new sessions

**Fixed:**
- GRAPH-002: Prevent WikiLink extraction from code blocks (Markdig AST parsing)
- DOCS-001: Synced tool documentation between src/assets/usage/ and docs/usage/
- SequentialThinkingToolsTests: Fixed 14 failing tests

**Security:**
- SEC-001: JSON deserialization depth limits (MaxDepth=32) via SafeJson utility

**Added:**
- TEST-001: 49 new tests (252 total, 242 active)
- DIST-001: Windows MSI installer with PATH integration

**Removed:**
- DEPR-002: 7 unused Config.cs properties
- AddMissingH1: Deprecated migration tool

### Tasks

1. [x] Bump version in `src/Maenifold.csproj` from 1.0.2 to 1.0.3
2. [x] Update CHANGELOG.md: Move [Unreleased] to [1.0.3] with date
3. [x] Run full test suite: `dotnet test`
4. [x] Create WiX MSI configuration (installer/, wix.json, PATH integration)
5. [x] Update release.yml to build MSI on Windows
6. [x] Create PR dev → main
7. [x] After merge, tag v1.0.3 on main and push (triggers release workflow)
8. [x] Verify GitHub Release created with 6 artifacts (5 archives + MSI)
9. [x] Verify Homebrew formula auto-updated

### Acceptance Criteria

- [x] All tests pass
- [x] v1.0.3 tag created on main branch
- [x] GitHub Release created with 6 artifacts (osx-arm64, osx-x64, linux-x64, linux-arm64, win-x64, win-x64.msi)
- [x] MSI installer adds maenifold to system PATH
- [x] MSI uninstall cleanly removes PATH entry
- [x] Homebrew formula updated automatically via repository dispatch
- [x] `brew upgrade maenifold` works after release

---

## EVAL-BC-001: BuildContext + FindSimilarConcepts data quality

**Status**: Active
**Priority**: High
**Created**: 2026-01-30

### Problem

We need to assess the **quality** of data returned by graph operations, especially `buildcontext` (context neighborhood + previews) and `findsimilarconcepts` (embedding neighbors). This is primarily about *user usefulness*: relevance, signal-to-noise, and consistency (naming/normalization), not just functional correctness.

### Scope / Traceability

- Core: `docs/agent/PRD.md` (FR-7.3 `buildcontext`, FR-7.4 `findsimilarconcepts`)
- Related: `memory://audits/graph-operations-test-execution-results-20260129` (functional test run)

### Tasks (grouped by tool)

#### BuildContext (`buildcontext`)

1. [ ] T-QUAL-BC1: Empirically evaluate BuildContext output quality across ~8 concepts and parameter sweeps
   - Concepts: pick a balanced set across domains (e.g., security, graph, finops) and include at least 2 “hub” concepts to quantify hubness.
   - Sweep: depth={1,2,3}, maxEntities={10,20,50}, includeContent={false,true}.
   - Capture: direct relation relevance, expanded relation usefulness, file evidence diversity, and preview usefulness.
   - Evidence: write representative sample outputs + observations into the sequential thinking session.
2. [ ] T-QUAL-BC2: Blue-team quality review: identify systemic BuildContext quality failure modes + metric proposal
   - Focus: user usefulness (relevance, signal-to-noise), not functional correctness.
   - Deliverable: propose measurable quality metrics + thresholds suitable for release gating.
3. [ ] T-QUAL-BC3: Red-team adversarial review of BuildContext: concept pollution/hub dominance/self-reference/preview misuse scenarios
   - Focus: how low-quality or poisoned corpus content can mislead downstream agents via retrieval context.
   - Deliverable: 3–5 reproducible scenarios with tool outputs + mitigations.

#### FindSimilarConcepts (`findsimilarconcepts`)

4. [ ] T-QUAL-FSC1: Evaluate FindSimilarConcepts quality across query variants (casing, hyphens, compounds) and domains
   - Include: casing (MCP/mcp), hyphenation (graph rag/graph-rag/graphrag), alphanumerics (oauth2), pluralization (tool/tools), and at least 3 FinOps terms.
   - Capture: score distributions, top-K overlap across variants, and malformed concepts surfaced.

### Quality Gates (proposed)

These are **release-blocking** quality checks for `buildcontext` and `findsimilarconcepts` (traceable to `docs/agent/PRD.md` FR-7.3 / FR-7.4):

1. [x] T-QUAL-GATE-001: Define a fixed evaluation query suite (10–15 concepts + 3–5 controls) and document how to run it
   - Why: create a stable, repeatable "health check" that detects regressions in retrieval quality.
   - Include controls: random/garbage tokens + short tokens to ensure we don't return confident nonsense.
   - Output: documented query list + expected "shape" of results (not exact matches).
   - **Done**: See `docs/QUALITY-GATES.md` (12 test concepts + 5 controls)
2. [x] T-QUAL-GATE-002: Define acceptance thresholds and document pass/fail rubric (grouped by tool):
   - BuildContext:
     - Precision@10 (manual spot-check) >= 0.70 for non-hub anchors
     - Hub Pollution Rate@10 <= 0.20 for non-hub anchors
     - Evidence Concentration: no single memory:// file accounts for > 0.50 of evidence across top relations
     - Preview Grounding Rate (includeContent): >= 0.90 of previews contain the anchor concept early
   - FindSimilarConcepts:
     - Similarity Sanity: fail if top-K similarity mass at 1.000 or near-zero score variance on controls
   - Notes:
     - "Hub concepts" (e.g., tool-like generics) are evaluated separately; they are expected to have lower precision.
     - Precision scoring is human-judged initially; later we can automate proxies (e.g., overlap with curated allowlists).
   - **Done**: See `docs/QUALITY-GATES.md` (4 BuildContext + 4 FindSimilarConcepts metrics with thresholds)

### Improvements (prioritized backlog)

#### FindSimilarConcepts improvements

1. [ ] T-QUAL-FSC2: Root-cause and fix FindSimilarConcepts “similarity=1.000 plateau” behavior for common short/compound queries
   - Context: empirical eval showed many unrelated queries collapse into an identical top-K list with similarity=1.000, destroying trust.
   - Hypotheses to validate:
     - query embedding is invalid (zero/NaN/constant) for certain tokens
     - distance function returns 0/NULL for many rows
     - normalization tokenization mismatch between query and stored concepts
   - Acceptance:
     - Control suite contains 0 cases where all top-10 results are similarity=1.000.
     - Score distribution shows non-trivial variance for non-identical queries.

#### BuildContext improvements

2. [ ] T-QUAL-BC4: Add hub-dampening and evidence-diversity re-ranking to BuildContext results (reduce generic hub dominance and single-file domination)
   - Context: co-occurrence ranking over-emphasizes high-degree concepts and can be dominated by a single high-link note/session.
   - Candidate approach:
     - Penalize relations whose evidence comes disproportionately from a single file (MMR-style diversity)
     - Optional: downweight evidence from `thinking/` folder unless explicitly requested
   - Acceptance:
     - Evidence concentration metric passes in the quality gate suite.
     - Non-hub anchors show increased unique-file diversity in top-10 without large relevance loss.

#### Cross-cutting (corpus hygiene / ingestion)

3. [ ] T-QUAL-HYGIENE-001: Locate and remediate malformed concept tokens in the corpus (prevent bracket-fragment concepts from entering embeddings)
   - Context: FindSimilarConcepts surfaced malformed concepts like `[[focus` / `[azure-data-explorer` implying ingestion issues.
   - Plan:
     - Use FullText search to locate source files containing malformed bracket fragments.
     - Decide remediation: edit offending notes OR add sync-time validation filter to drop malformed tokens.
   - Acceptance:
     - Quality suite returns 0 malformed concept tokens in top-10 for any query.

#### BuildContext (preview UX/safety)

4. [ ] T-QUAL-BC5: Make BuildContext previews explicitly untrusted (safe formatting + lightweight prompt-shaped-text detection)
   - Context: `includeContent=true` previews can carry prompt-shaped text; downstream LLMs may treat it as instruction.
   - Approach:
     - Wrap previews as quoted or fenced “UNTRUSTED RETRIEVAL” blocks.
     - Detect and annotate common directive patterns (SYSTEM:, ignore previous, tool-call JSON shapes) as flags.
   - Acceptance:
     - Previews remain readable and grounded (Preview Grounding Rate still passes).
     - Any flagged directive patterns are clearly marked as untrusted.

### Acceptance Criteria

- [ ] Clear set of quality dimensions + metrics (relevance, diversity, duplication, hub pollution, preview usefulness)
- [ ] At least 5 concrete, prioritized improvement recommendations tied to observed failure modes
- [ ] Evidence includes tool outputs (representative samples) and links to memory:// notes used
- [ ] Quality gates defined (T-QUAL-GATE-001/002) and approved as release criteria for FR-7.3/FR-7.4

### Evidence / Notes

- Primary working evidence: `memory://thinking/sequential/1970/01/01/session-1769801329103-37116`
- Baseline functional audit: `memory://audits/graph-operations-test-execution-results-20260129`

---

## GRAPH-DECAY-001: Recency decay weighting for search

**Status**: ✅ Complete
**Priority**: High
**Created**: 2026-01-31
**Completed**: 2026-02-02

### Problem

Search result rankings should reflect recency: knowledge we revisit frequently should stay fresh, and stale content should naturally decay. Apply configurable exponential decay with tiered grace periods across all search ranking paths.

### Scope / Traceability

- PRD: FR-7.5 (tiered decay), FR-7.6 (access boosting), FR-7.9 (consolidation), NFR-7.5.1–7.5.5, NFR-7.6.1
- RTM: T-GRAPH-DECAY-001.*, T-GRAPH-DECAY-002.*, T-GRAPH-DECAY-005.*
- Research: Moltbook discussion "TIL: Memory decay makes retrieval BETTER" (2026-01-30, 491 comments)
- Research: docs/research/decay-in-ai-memory-systems.md (Ebbinghaus, ACT-R, Richards & Frankland)

### Design

**Tiered Grace Periods:**
- `memory/thinking/sequential/` → 7d grace (episodic reasoning tied to specific tasks)
- `memory/thinking/workflows/` → 14d grace (structured multi-step processes)
- All other `memory/` → 14d grace (evolving knowledge notes)

**Half-Life:** 30 days (exponential decay after grace period)

**Access Boosting:** Retrieval resets `last_accessed` timestamp → frequently-accessed content stays fresh

**Formula:**
```
decay_weight = 
  if (age < grace_period): 1.0
  else: 0.5 ^ ((age - grace_period) / half_life)

final_score = semantic_score × decay_weight
```

**Permanent Knowledge:** Stored outside `memory://` (AGENTS.md, skills, system prompts, code) — not subject to decay.

### Tasks

1. [x] T-GRAPH-DECAY-001.1: Apply decay weighting to search rankings (BuildContext, FindSimilarConcepts, SearchMemories)
2. [x] T-GRAPH-DECAY-001.2: Add sequential grace config — `MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL` (default 7)
3. [x] T-GRAPH-DECAY-001.2a: Add workflows grace config — `MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS` (default 14)
4. [x] T-GRAPH-DECAY-001.3: Add default grace config — `MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT` (default 14)
5. [x] T-GRAPH-DECAY-001.4: Add half-life config — `MAENIFOLD_DECAY_HALF_LIFE_DAYS` (default 30)
6. [x] T-GRAPH-DECAY-001.5: Ensure decay affects ranking only (direct ReadMemory bypasses decay)
7. [x] T-GRAPH-DECAY-002.1: Update `last_accessed` on ReadMemory (explicit read = intentional access)
8. [x] T-GRAPH-DECAY-002.2: Verify SearchMemories does NOT update `last_accessed` (appearing in results ≠ being read)
9. [x] T-GRAPH-DECAY-002.3: Verify BuildContext does NOT update `last_accessed` (automated context ≠ intentional access)
10. [x] T-GRAPH-DECAY-003.1: Add `created`, `last_accessed`, `decay_weight` to ListMemories output
11. [x] T-GRAPH-DECAY-003.2: Compute decay_weight using file's tier (sequential=7d, workflows=14d, other=14d grace)
12. [x] T-GRAPH-DECAY-004: Add tests for tiered decay config defaults
13. [x] T-GRAPH-DECAY-005: Add tests for access-boosting behavior (ReadMemory updates, Search/BuildContext don't)
14. [x] T-GRAPH-DECAY-006: Update GetConfig summary output to include decay settings
15. [x] T-GRAPH-DECAY-007: Document decay behavior in docs/usage and tool docs
16. [x] T-GRAPH-DECAY-004.1: Exempt `validated` assumptions from decay
17. [x] T-GRAPH-DECAY-004.2: Apply 14d grace / 30d half-life to `active` and `refined` assumptions
18. [x] T-GRAPH-DECAY-004.3: Apply 7d grace / 14d half-life to `invalidated` assumptions (aggressive decay)
19. [x] T-GRAPH-DECAY-001.6: Implement optional power-law decay (`R = a × t^(-b)`) via `MAENIFOLD_DECAY_FUNCTION` env var
20. [x] T-GRAPH-DECAY-005.1: Implement Cognitive Sleep Cycle workflow for periodic consolidation
21. [x] T-GRAPH-DECAY-005.2: Consolidation distills episodic (thinking/) → semantic (memory://) with WikiLinks

### Acceptance Criteria

- [x] `memory/thinking/sequential/` grace = 7d, `memory/thinking/workflows/` grace = 14d, other = 14d, half-life = 30d
- [x] Env overrides work for all four parameters (SEQUENTIAL, WORKFLOWS, DEFAULT, HALF_LIFE)
- [x] All search ranking paths apply decay weighting
- [x] Only ReadMemory updates `last_accessed` (SearchMemories/BuildContext do not)
- [x] Direct ReadMemory returns full content regardless of decay (ranking not deletion)
- [x] Tests cover tiered defaults and access-boosting behavior
- [x] Assumption decay by status: validated=exempt, active/refined=14d/30d, invalidated=7d/14d
- [x] Power-law decay (`R = a × t^(-b)`) available via `MAENIFOLD_DECAY_FUNCTION=power_law` (default: exponential)
- [x] Cognitive Sleep Cycle workflow consolidates high-value episodic content into semantic memory

---

## DIST-001: Windows MSI installer

**Status**: Merged into REL-001
**Priority**: High
**Created**: 2026-01-27

### Problem

Windows users must manually download and extract release archives. An MSI installer would provide a better experience.

### Resolution

Merged into REL-001 (v1.0.3 release). See REL-001 tasks 4-5 for implementation.

---

## CLEANUP-001: Close Active Thinking Sessions

**Status**: Pending
**Priority**: Low
**Created**: 2026-01-28

### Problem

Several SequentialThinking sessions from recent activity are still marked as "active" but appear abandoned.

### Sessions to Review

1. `session-1769578881329` (Jan 28, 10:33) - Compaction log, 4 thoughts, active
2. `session-1769561775163` (Jan 28, 07:53) - Security review, 1 thought, active
3. `session-1769558002091` (Jan 28, 07:00) - Persist test, 1 thought, active

### Tasks

1. [ ] Review each session for any valuable insights to persist
2. [ ] Close or cancel abandoned sessions
3. [ ] Document any learnings in appropriate memory:// notes

### Acceptance Criteria

- [ ] No orphaned active sessions older than 24 hours

---

## GOV-RETRO-001: Add RETROSPECTIVES.md

**Status**: ✅ Complete (2026-02-02)
**Priority**: Low

### Problem

We need a canonical place to capture sprint retrospectives.

### Tasks

1. [x] T-GOV-RETRO-001.1: Create RETROSPECTIVES.md in repo root
2. [x] T-GOV-RETRO-001.2: Add retrospective template and initial sprint entries

### Acceptance Criteria

- [x] RETROSPECTIVES.md exists at repo root
- [x] Contains retrospective template for future sprints
- [x] Documents CLI JSON Output sprint (T-CLI-JSON-001)
- [x] Documents Embeddings Quality sprint (T-QUAL-FSC2)
- [x] Includes backlog notes for decay weighting sprints

