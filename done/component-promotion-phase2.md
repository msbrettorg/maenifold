---
id: component-promotion-phase2
status: done
pipeline: sdd
---

# Component Promotion Phase 2 — @xyflow/react Components

**Requirement**: FR-32 (Web UI)
**Traceability**: TC-32.7

## Objective

Add `@xyflow/react` dependency to shared package, then move 7 ReactFlow-based components from the Chrome extension to shared.

## Prerequisites

- Phase 1 complete (shared ai-elements barrel exists)

## Dependency Addition

Add to `packages/shared/package.json` dependencies:
```json
"@xyflow/react": "<match version from packages/chrome-extension/package.json>"
```

## Components (→ `packages/shared/src/ai-elements/`)

| Component | Purpose |
|-----------|---------|
| canvas.tsx | ReactFlow diagram wrapper |
| controls.tsx | Styled zoom/fit controls |
| panel.tsx | Styled card panel for diagrams |
| toolbar.tsx | Bottom-positioned node toolbar |
| node.tsx | Flow diagram node with handles |
| edge.tsx | Animated/temporary edges |
| connection.tsx | Bézier connection lines |

## Acceptance Criteria

1. `@xyflow/react` added to shared package.json at correct version
2. All 7 files exist in shared with correct import paths
3. All re-exported from ai-elements index
4. `bun run build` succeeds in `packages/shared`

## Scoped Test Command

```bash
cd vendor/chrome-acp && bun install && cd packages/shared && bun run build
```

## SWE Remediation

**Status**: ✅ Complete — all 7 ReactFlow components promoted to shared.

### Changes Made

| File | Action | Notes |
|------|--------|-------|
| `packages/shared/package.json` | Modified | Added `"@xyflow/react": "^12.9.3"` to dependencies |
| `packages/shared/src/components/ai-elements/canvas.tsx` | Created | Moved from extension; no import changes needed |
| `packages/shared/src/components/ai-elements/controls.tsx` | Created | Rewritten import: `@chrome-acp/shared/lib/utils` → `../../lib/utils` |
| `packages/shared/src/components/ai-elements/panel.tsx` | Created | Rewritten import: `@chrome-acp/shared/lib/utils` → `../../lib/utils` |
| `packages/shared/src/components/ai-elements/toolbar.tsx` | Created | Rewritten import: `@chrome-acp/shared/lib/utils` → `../../lib/utils` |
| `packages/shared/src/components/ai-elements/node.tsx` | Created | Rewritten imports: `@chrome-acp/shared/components/ui/card` → `../ui/card`, `@chrome-acp/shared/lib/utils` → `../../lib/utils` |
| `packages/shared/src/components/ai-elements/edge.tsx` | Created | Moved from extension; no import changes needed (only `@xyflow/react` imports) |
| `packages/shared/src/components/ai-elements/connection.tsx` | Created | Moved from extension; no import changes needed (only `@xyflow/react` imports) |
| `packages/shared/src/components/ai-elements/index.ts` | Modified | Added 7 barrel re-exports: `canvas`, `connection`, `controls`, `edge`, `node`, `panel`, `toolbar` |

### Import Path Convention

All new files follow the existing relative-import pattern used by other ai-elements in shared:
- UI components: `from "../ui/<component>"`
- Utilities: `from "../../lib/utils"`

### Build Verification

- `bun install` — ✅ resolves cleanly, `@xyflow/react` installed
- `tsc --noEmit` — **0 errors in new files**; 10 pre-existing errors in `ACPMain.tsx` (6), `FileExplorer.tsx` (3), `GraphView.tsx` (1) unrelated to this change

## Red-team Audit

**Verdict**: ✅ **PASS**

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | `@xyflow/react` version match | ✅ PASS | Shared has `"^12.9.3"`, chrome-extension has `"^12.9.3"` — exact match |
| 2 | All 7 files exist in `ai-elements/` | ✅ PASS | `canvas.tsx`, `controls.tsx`, `panel.tsx`, `toolbar.tsx`, `node.tsx`, `edge.tsx`, `connection.tsx` — all present |
| 3 | No stale `@chrome-acp/shared/*` or `@/components/*` imports | ✅ PASS | `rg` across all 7 files returned zero matches; imports use relative paths (`../../lib/utils`, `../ui/card`) |
| 4 | All 7 re-exported from barrel index | ✅ PASS | `index.ts` lines 27–33 export `canvas`, `connection`, `controls`, `edge`, `node`, `panel`, `toolbar` |
| 5 | `tsc --noEmit` (packages/shared) | ✅ PASS | Exit code 0, zero errors; 10 pre-existing errors live in chrome-extension, not shared |

### Adversarial Notes

- **Export collision check**: All exported symbol names (`Canvas`, `Controls`, `Panel`, `Toolbar`, `Node*`, `Edge`, `Connection` + type aliases) are unique across the barrel — no namespace collisions.
- **Spec path typo**: Spec § Components header says `packages/shared/src/ai-elements/` but actual path is `packages/shared/src/components/ai-elements/`. Implementation is correct; spec header has a cosmetic typo (non-blocking).
- **Peer dependency surface**: `@xyflow/react ^12.9.3` pulls React 18/19 peer — already satisfied by shared's `react: ^19` peer dep. No version conflict.
- **No new attack surface**: Components are pure presentational React wrappers with no network calls, no dynamic `eval`, no `dangerouslySetInnerHTML`, no user-controlled code execution paths.

## Blue-team Verification

**Verdict**: ✅ **PASS**
**Date**: 2025-07-17
**Scope**: Acceptance criteria 1–4

### Test Commands & Results

| # | Command | Exit Code | Notes |
|---|---------|-----------|-------|
| 1 | `bun install` (workspace root) | 0 | `@xyflow/react` resolved, 579 installs, no changes needed |
| 2 | `bun run build` (`packages/shared`) | 0 | `tsc --noEmit` — zero errors, zero warnings |

### Acceptance Criteria Checklist

| # | Criterion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | `@xyflow/react` added at correct version | ✅ PASS | Shared: `"^12.9.3"`, Extension: `"^12.9.3"` — exact match |
| 2 | All 7 files exist in `ai-elements/` | ✅ PASS | `canvas.tsx`, `controls.tsx`, `panel.tsx`, `toolbar.tsx`, `node.tsx`, `edge.tsx`, `connection.tsx` — all present |
| 3 | All 7 re-exported from barrel index | ✅ PASS | `index.ts` lines 27–33: `canvas`, `connection`, `controls`, `edge`, `node`, `panel`, `toolbar` |
| 4 | `bun run build` succeeds | ✅ PASS | `tsc --noEmit` exit 0, clean output |

### Red-team Findings Addressed

Red-team audit passed with no findings requiring remediation. Verified:
- No stale `@chrome-acp/shared/*` imports in promoted files
- No namespace collisions in barrel exports
- No new attack surface (pure presentational components)

### Regression Check

No regressions — build is clean with zero new errors. The 10 pre-existing type errors noted by SWE live in `chrome-extension` package files (`ACPMain.tsx`, `FileExplorer.tsx`, `GraphView.tsx`), not in `shared`.
