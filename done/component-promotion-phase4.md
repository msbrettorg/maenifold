# Component Promotion Phase 4 — Progress + Confirmation + Context

**Requirement**: FR-32 (Web UI)
**Traceability**: TC-32.7

## Objective

Add `@radix-ui/react-progress` and `tokenlens` deps to shared, then move progress, confirmation, and context components.

## Prerequisites

- Phase 1 complete

## Dependency Additions

Add to `packages/shared/package.json` dependencies:
```json
"@radix-ui/react-progress": "<match version from chrome-extension>",
"tokenlens": "<match version from chrome-extension>"
```

## Components

| Component | Destination | Purpose |
|-----------|-------------|---------|
| progress.tsx | packages/shared/src/ui/ | Radix progress bar |
| confirmation.tsx | packages/shared/src/ai-elements/ | Approval/rejection dialog for tool calls |
| context.tsx | packages/shared/src/ai-elements/ | Token usage stats + cost hover card |

## Acceptance Criteria

1. Both deps added to shared at correct versions
2. All 3 files in shared with correct imports
3. progress re-exported from ui index, others from ai-elements index
4. `bun run build` succeeds in `packages/shared`

## Scoped Test Command

```bash
cd vendor/chrome-acp && bun install && cd packages/shared && bun run build
```

## SWE Remediation

**Status**: ✅ Complete

### Changes Made

1. **Dependency additions** (`packages/shared/package.json`):
   - Added `"@radix-ui/react-progress": "^1.1.8"` (matched from chrome-extension)
   - Added `"tokenlens": "^1.3.1"` (matched from chrome-extension)

2. **Component files created**:
   - `packages/shared/src/components/ui/progress.tsx` — Radix progress bar, imports rewritten from `@chrome-acp/shared/lib/utils` → relative `../../lib/utils`
   - `packages/shared/src/components/ai-elements/confirmation.tsx` — Approval/rejection dialog; imports rewritten from `@/components/ui/alert` → `../ui/alert`, `@chrome-acp/shared/components/ui/button` → `../ui/button`, `@chrome-acp/shared/lib/utils` → `../../lib/utils`; removed 6 `@ts-expect-error` directives that were unused (AI SDK v6 states already defined in shared's `ai` devDep)
   - `packages/shared/src/components/ai-elements/context.tsx` — Token usage stats + cost hover card; imports rewritten from `@/components/ui/progress` → `../ui/progress`, `@chrome-acp/shared/components/ui/*` → `../ui/*`, `@chrome-acp/shared/lib/utils` → `../../lib/utils`

3. **Barrel exports updated**:
   - `packages/shared/src/components/ui/index.ts` — added `export * from "./progress"`
   - `packages/shared/src/components/ai-elements/index.ts` — added `export * from "./confirmation"` and `export * from "./context"`

### Decisions

- **Relative imports**: All intra-shared imports use relative paths (e.g., `../ui/button`, `../../lib/utils`) matching the established pattern in existing shared components (tool.tsx, permission-request.tsx, etc.).
- **Removed `@ts-expect-error` directives**: The chrome-extension source had 6 `@ts-expect-error` annotations for AI SDK v6 states (`approval-requested`, `approval-responded`, `output-denied`). The shared package's `ai` devDep (v6) already defines these types, making the directives unused and causing TS2578 errors. Removed them.
- **Pre-existing build errors**: `bun run build` (`tsc --noEmit`) reports 10 pre-existing errors in ACPMain.tsx, FileExplorer.tsx, and GraphView.tsx. Zero errors in the 3 new files.

### Verification

```
$ cd vendor/chrome-acp && bun install && cd packages/shared && bun run build
# 10 pre-existing errors (ACPMain, FileExplorer, GraphView) — 0 errors in new files
$ tsc --noEmit 2>&1 | grep -c "confirmation\|context\|progress"
# 0
```

## Red-team Audit

**Verdict**: ✅ PASS

**Auditor**: red-team agent
**Date**: 2025-07-15

### Checklist

| # | Check | Result |
|---|-------|--------|
| 1 | `@radix-ui/react-progress` in `packages/shared/package.json` at `^1.1.8` (matches chrome-extension) | ✅ |
| 2 | `tokenlens` in `packages/shared/package.json` at `^1.3.1` (matches chrome-extension) | ✅ |
| 3 | `progress.tsx` exists in `packages/shared/src/components/ui/` | ✅ |
| 4 | `progress` re-exported from `packages/shared/src/components/ui/index.ts` | ✅ |
| 5 | `confirmation.tsx` exists in `packages/shared/src/components/ai-elements/` | ✅ |
| 6 | `context.tsx` exists in `packages/shared/src/components/ai-elements/` | ✅ |
| 7 | `confirmation` + `context` re-exported from `packages/shared/src/components/ai-elements/index.ts` | ✅ |
| 8 | Zero `@chrome-acp/shared/*` imports in new files | ✅ |
| 9 | Zero `@/components/*` imports in new files | ✅ |
| 10 | Zero bad imports (`@chrome-acp/shared`, `@/components`) anywhere in `packages/shared/src/` | ✅ |
| 11 | `npx tsc --noEmit` exits 0 — zero type errors (pre-existing errors now resolved) | ✅ |

### Import Audit Detail

- **progress.tsx**: `@radix-ui/react-progress`, `../../lib/utils` — correct relative paths
- **confirmation.tsx**: `../ui/alert`, `../ui/button`, `../../lib/utils`, `ai`, `react` — correct relative paths, no `@ts-expect-error` directives
- **context.tsx**: `../ui/button`, `../ui/hover-card`, `../ui/progress`, `../../lib/utils`, `ai`, `react`, `tokenlens` — correct relative paths

### Adversarial Notes

- **Version pinning**: Both deps use caret ranges (`^1.x.x`) matching chrome-extension exactly. No version drift risk.
- **`@ts-expect-error` removal**: SWE removed 6 `@ts-expect-error` directives from confirmation.tsx that were unused because shared's `ai` devDep (v6) already defines the states. This is correct — leaving unused `@ts-expect-error` causes TS2578. Verified no runtime type gap.
- **Build health improvement**: Spec mentioned 10 pre-existing errors; current `tsc --noEmit` exits 0. The promoted files introduced zero regressions.
- **No security surface**: These are pure UI presentation components with no network calls, auth logic, or user-input processing beyond React props. Attack surface is nil.

## Blue-team Verification

**Verdict**: ✅ PASS

**Verifier**: blue-team agent
**Date**: 2025-07-15

### Test Executed

```bash
cd vendor/chrome-acp/packages/shared && bun run build
# tsc --noEmit → exit code 0
```

### Checks

| # | Criterion | Evidence | Result |
|---|-----------|----------|--------|
| 1 | `bun run build` succeeds (exit 0) | `tsc --noEmit` exited 0, zero errors | ✅ |
| 2 | `progress.tsx` exists at `src/components/ui/progress.tsx` | `find` confirmed | ✅ |
| 3 | `confirmation.tsx` exists at `src/components/ai-elements/confirmation.tsx` | `find` confirmed | ✅ |
| 4 | `context.tsx` exists at `src/components/ai-elements/context.tsx` | `find` confirmed | ✅ |
| 5 | `progress` re-exported from `ui/index.ts` | Line 25: `export * from "./progress"` | ✅ |
| 6 | `confirmation` re-exported from `ai-elements/index.ts` | Line 24: `export * from "./confirmation"` | ✅ |
| 7 | `context` re-exported from `ai-elements/index.ts` | Line 25: `export * from "./context"` | ✅ |
| 8 | `@radix-ui/react-progress` dep at `^1.1.8` | `package.json` line 30 | ✅ |
| 9 | `tokenlens` dep at `^1.3.1` | `package.json` line 58 | ✅ |
| 10 | Zero forbidden imports (`@chrome-acp/shared`, `@/components`) in new files | `grep` returned no matches | ✅ |

### Summary

All 4 acceptance criteria satisfied:
1. ✅ Both deps added at correct versions
2. ✅ All 3 files in shared with correct relative imports
3. ✅ `progress` re-exported from ui index; `confirmation` + `context` from ai-elements index
4. ✅ `bun run build` exits 0 with zero type errors

No regressions. No unaddressed findings. Implementation matches spec intent.
