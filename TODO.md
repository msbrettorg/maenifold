# TODO

## REL-001: Release v1.0.3

**Status**: Active
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

1. [ ] Bump version in `src/Maenifold.csproj` from 1.0.2 to 1.0.3
2. [ ] Update CHANGELOG.md: Move [Unreleased] to [1.0.3] with date
3. [ ] Run full test suite: `dotnet test`
4. [ ] Create WiX MSI configuration (installer/, wix.json, PATH integration)
5. [ ] Update release.yml to build MSI on Windows
6. [ ] Create PR dev → main
7. [ ] After merge, tag v1.0.3 on main and push (triggers release workflow)
8. [ ] Verify GitHub Release created with 6 artifacts (5 archives + MSI)
9. [ ] Verify Homebrew formula auto-updated

### Acceptance Criteria

- [ ] All tests pass
- [ ] v1.0.3 tag created on main branch
- [ ] GitHub Release created with 6 artifacts (osx-arm64, osx-x64, linux-x64, linux-arm64, win-x64, win-x64.msi)
- [ ] MSI installer adds maenifold to system PATH
- [ ] MSI uninstall cleanly removes PATH entry
- [ ] Homebrew formula updated automatically via repository dispatch
- [ ] `brew upgrade maenifold` works after release

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

1. [ ] T-QUAL-GATE-001: Define a fixed evaluation query suite (10–15 concepts + 3–5 controls) and document how to run it
   - Why: create a stable, repeatable “health check” that detects regressions in retrieval quality.
   - Include controls: random/garbage tokens + short tokens to ensure we don’t return confident nonsense.
   - Output: documented query list + expected “shape” of results (not exact matches).
2. [ ] T-QUAL-GATE-002: Define acceptance thresholds and document pass/fail rubric (grouped by tool):
   - BuildContext:
     - Precision@10 (manual spot-check) >= 0.70 for non-hub anchors
     - Hub Pollution Rate@10 <= 0.20 for non-hub anchors
     - Evidence Concentration: no single memory:// file accounts for > 0.50 of evidence across top relations
     - Preview Grounding Rate (includeContent): >= 0.90 of previews contain the anchor concept early
   - FindSimilarConcepts:
     - Similarity Sanity: fail if top-K similarity mass at 1.000 or near-zero score variance on controls
   - Notes:
     - “Hub concepts” (e.g., tool-like generics) are evaluated separately; they are expected to have lower precision.
     - Precision scoring is human-judged initially; later we can automate proxies (e.g., overlap with curated allowlists).

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

## OPENCODE-001: OpenCode Plugin for Maenifold

**Status**: Active
**Priority**: High
**Created**: 2026-01-27

### Problem

OpenCode lacks native maenifold integration. Need a TypeScript plugin that provides:
- Session start context injection (graph context from SearchMemories/RecentActivity)
- Compaction preservation (save session knowledge via WriteMemory)
- Concept augmentation (enrich tool inputs containing `[[WikiLinks]]` with BuildContext)

### Completed Tasks

- [x] T-OC-1: Research OpenCode plugin API and hook surface
- [x] T-OC-2: Research session.compacted event capture pattern
- [x] T-OC-3: Design plugin architecture (SPEC.md)
- [x] T-OC-4: Implement plugin (`~/.config/opencode/plugins/maenifold.ts`)
- [x] T-OC-5: Blue-team defensive review (1 HIGH, 7 MEDIUM, 11 LOW findings)
- [x] T-OC-6: Red-team security assessment (1 CRITICAL, 3 HIGH, 3 MEDIUM)
- [x] T-OC-6.1: Fix Unicode homoglyph bypass (NFKC normalization)
- [x] T-OC-6.2: Fix symlink path traversal (realpathSync validation)
- [x] T-OC-6.3: Fix type guard (extractMessageText implementation)
- [x] T-OC-7: Verify security remediations (CONDITIONAL PASS)

### Outstanding Tasks

1. [ ] T-OC-8: Implement compaction summary persistence with security mitigations
   - Treat summary as untrusted data (schema separation, fenced reinjection)
   - Aggressive sanitization (NFKC, control char stripping)
   - Strict `[[wikilinks]]` allowlisting to prevent graph corruption
   - See: memory://thinking/sequential/2026/01/28/session-1769625876165
2. [ ] T-OC-9: Add secret/PII scrubbing before persistence
3. [ ] T-OC-10: Add project isolation (namespace by project.id)
4. [ ] T-OC-11: Add retention controls (TTL, opt-out toggle)
5. [ ] T-OC-12: Integration testing with live OpenCode session
6. [ ] T-OC-13: Documentation and README

### Acceptance Criteria

- [ ] Plugin loads successfully in OpenCode
- [ ] Session start injects graph context
- [ ] Compaction summaries persist to SequentialThinking (with sanitization)
- [ ] `[[WikiLinks]]` in prompts trigger BuildContext enrichment
- [ ] All security mitigations verified by red-team

### Related Memory

- memory://tech/integrations/opencode-plugin-for-maenifold
- memory://tech/integrations/opencode-plugin-api-reference
- memory://tech/integrations/opencode-sessioncompacted-event-summary-capture-pattern
- memory://thinking/sequential/2026/01/28/session-1769625876165 (security analysis)

---

## MAENIFOLDPY-001: Python Port Quality Baseline

**Status**: Active
**Priority**: Medium
**Created**: 2026-01-28

### Problem

The maenifold-py repository has type checking issues that need resolution before further development.

### Findings (2026-01-28)

- All pytest tests pass
- mypy reports 51 errors across 16 files
- Key issues:
  - MCP `repair_concepts` kwarg mismatch (`create_wiki_links` vs `create_wikilinks`)
  - ConfigMeta metaclass typing pattern needs adjustment
  - SafeJson generic signature issues
  - Watcher tools bytes/str mismatches
  - Missing PyYAML type stubs

### Tasks

1. [ ] T-PY-006: Fix MCP `repair_concepts` kwarg mismatch
2. [ ] T-PY-007: Establish clean mypy baseline (fix errors or configure ignores)
3. [ ] T-PY-008: Add types-PyYAML to dev dependencies
4. [ ] T-PY-009: Update TODO.md to remove stale C# references

### Acceptance Criteria

- [ ] `mypy src` reports 0 errors (or documented ignores)
- [ ] All pytest tests continue to pass
- [ ] MCP server repair_concepts works correctly

### Related Memory

- memory://tech/maenifoldpy-repo-evaluation-notes-20260128

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
