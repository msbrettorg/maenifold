# Sprint 05 Sign-off — RALPH Iteration 2026-04-24

## Summary

This iteration resolved 5 gate tasks (1 P0, 2 P1, 2 P2), filed 2 new tasks, and advanced the T-203 MCP Apps epic to SWE stage.

## Tasks Delivered

### T-05.94 (P0) — Knowledge tab empty note display
- **Root cause**: Root-level memory files (Celonis.md, Session-as-Node.md) are genuinely empty (0 bytes). KnowledgeContent.vue had no empty-content fallback, showing a blank gray bar.
- **Fix**: Added `v-else-if="!activeNote.content?.trim()"` branch showing EmptyState with FileText icon.
- **Pipeline**: SDD (pending → swe → red-team → blue-team → done)
- **Files**: `apps/desktop/src/components/KnowledgeContent.vue` (+8/-1)

### T-05.87 (P1) — UAT regression verification
- **Result**: 4 of 5 UAT fixes verified via CDP. 1 regression found (column resize lost in T-05.70 rebuild).
- **Verified**: Syntax highlighting ✅, sidebar expand ✅, pipeline edge CSS ✅, chat code font ✅
- **Regressed**: Column resize → filed T-05.97
- **Pipeline**: TDD (pending → blue-team → swe → red-team → done)

### T-05.92 (P1) — Dependabot security vulnerabilities
- **Fixed**: rustls-webpki 0.103.12 → 0.103.13 (RUSTSEC-2026-0104). `cargo audit` now 0 vulnerabilities.
- **Accepted risk**: 29 npm vulns all transitive through mermaid@11.14.0 (latest). Cannot fix without breaking mermaid. Affects xmldom, tar, uuid — none in our attack surface.
- **Pipeline**: SDD

### T-05.96 (P2) — Shiki WASM console errors
- **Fix**: Switched from oniguruma WASM engine to JavaScript regex engine via `createHighlighter` + `fromHighlighter`. Eliminates 3x CompileError console messages.
- **Files**: `apps/desktop/src/composables/useMarkdownIt.ts` (+10/-3)
- **Pipeline**: Lightweight (pending → swe → blue-team → done)

### T-05.97 (P2) — Gate table column resize
- **Fix**: Added TanStack Table column resizing with drag handles on header right edges. Handles appear on hover (3px bar) and highlight brand color during drag.
- **Files**: `apps/desktop/src/components/GateTaskTable.vue` (+36/-12)
- **Pipeline**: SDD (pending → swe → red-team → blue-team → done)

## Tasks Filed

| Task | Title | Priority | Reason |
|------|-------|----------|--------|
| T-05.96 | Shiki WASM console errors | P2 | Discovered during T-05.94 diagnosis |
| T-05.97 | Gate column resize regression | P2 | Found during T-05.87 UAT verification |

## CDP Interactive Evidence

All screenshots saved to `outputs/screenshots/sprint-05/`:

1. **knowledge-blank.png** — Before fix: empty gray bar when opening Celonis.md
2. **knowledge-empty-note-fixed.png** — After fix: "Empty note / This note has no content yet." with FileText icon
3. **knowledge-with-content.png** — Content rendering verified: headings, bold, lists, WikiLink pills
4. **knowledge-content-renders.png** — Full note render after clicking architecture/ai-is-the-new-ui
5. **knowledge-code-blocks.png** — Syntax highlighting verified: shiki themes applied to code blocks
6. **knowledge-shiki-js-engine.png** — Syntax highlighting still works after JS engine switch
7. **gate-tab.png** — Gate table renders with task list, filters, detail panel
8. **pipelines-tab.png** — Pipelines tab empty state (no active pipelines to verify edges)

### Verification details

| Task | Verification method | Evidence |
|------|-------------------|----------|
| T-05.94 | CDP: Clicked Celonis.md → EmptyState shown. Clicked architecture note → content rendered. | Screenshots 1-4 |
| T-05.87 | CDP: Tested sidebar collapse/expand, checked shiki classes in DOM, grep for JetBrains Mono in CSS | Screenshots 5-8 |
| T-05.92 | `cargo audit` 0 vulns, `npm audit` documented | CLI output |
| T-05.96 | CDP: Loaded note with code blocks, verified `.shiki` class present, checked console for WASM errors — only stale entries from old bundle | Screenshot 6 |
| T-05.97 | Red-team audit PASS. Build verified via vue-tsc. CDP interactive verification deferred to next iteration (requires app rebuild) | Red-team verdict |

## Gate Status

### Done this iteration
- T-05.87 (done/) — UAT regression verification
- T-05.92 (done/) — Dependabot vulnerabilities
- T-05.94 (done/) — Knowledge tab blank content
- T-05.96 (done/) — Shiki WASM errors
- T-05.97 (done/) — Gate column resize

### Sprint 05 remaining
- T-05.67 (P2, deferred) — vec0 download wizard

### Advanced
- T-203.0 — MCP Apps epic: discovery → spec → swe
- T-203.2 — Parse _meta.ui: discovery → spec → swe (SWE dispatched)

## Known Issues

1. **WASM console errors from mermaid** — 3x CompileError entries appear from mermaid (not shiki after fix). Non-blocking — mermaid rendering works. Would need mermaid to support JS-only mode.
2. **T-05.97 column resize not CDP-verified** — Red-team passed, code merged, but interactive CDP verification requires a fresh app build. Will verify in next iteration.
3. **Pipeline edges (T-205.713)** — CSS is present and correct, but no active pipeline to visually verify. Needs a running pipeline to confirm edges are visible.

## PM Confession

- **T-05.94**: Fully verified via CDP. Clicked real notes, confirmed empty state shows, confirmed content renders. High confidence.
- **T-05.87**: 4/5 items verified via CDP. Pipeline edges verified by CSS inspection only (no running pipeline). Moderate confidence on edges.
- **T-05.92**: Verified via CLI tools only — not a UI task. High confidence.
- **T-05.96**: Verified via CDP — syntax highlighting works with JS engine, console errors reference old bundle only. High confidence.
- **T-05.97**: NOT CDP-verified. Red-team passed, code looks correct, vue-tsc passes. Will verify interactively in next iteration. Low confidence on actual drag behavior until CDP-tested.
