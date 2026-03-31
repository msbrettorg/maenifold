---
id: consolidate-duplicates
status: done
pipeline: sdd
---

# Consolidate Shimmer + Reasoning Duplicates

**Requirement**: FR-32 (Web UI)
**Traceability**: TC-32.7

## Objective

The extension has its own `shimmer.tsx` (identical to shared) and `reasoning.tsx` (diverged — adds Streamdown wrapping). Consolidate so shared is the single source.

## Prerequisites

- Phase 1 complete

## Tasks

### shimmer.tsx
- **Verdict**: REMOVE from extension
- Extension's `shimmer.tsx` is functionally identical to `packages/shared/src/ai-elements/shimmer.tsx`
- Delete extension copy, update extension imports to use `@chrome-acp/shared`

### reasoning.tsx
- **Verdict**: MERGE extension's Streamdown support into shared version
- Extension version wraps children in `<Streamdown>` and types children as `string`
- Shared version is generic (accepts ReactNode)
- Add optional `streamdown?: boolean` prop to shared version. When true, wrap children in Streamdown. Default false for backward compat.

## Acceptance Criteria

1. Extension's shimmer.tsx deleted, extension imports use shared
2. Shared reasoning.tsx supports optional Streamdown wrapping
3. Extension's reasoning.tsx deleted, extension imports use shared
4. `bun run build` succeeds in both `packages/shared` AND `packages/chrome-extension`

## Scoped Test Command

```bash
cd vendor/chrome-acp && bun install && cd packages/shared && bun run build && cd ../chrome-extension && bun run build
```

## SWE Remediation

**Status**: ✅ Complete — all acceptance criteria met.

### Changes Made

#### 1. shimmer.tsx — REMOVED from extension
- **Deleted** `packages/chrome-extension/src/components/ai-elements/shimmer.tsx`
- **Updated** `packages/chrome-extension/src/components/ai-elements/plan.tsx`: changed `import { Shimmer } from "./shimmer"` → `import { Shimmer } from "@chrome-acp/shared/components/ai-elements/shimmer"`
- No other extension files imported the local shimmer (the only other consumer was `reasoning.tsx`, which was also deleted).

#### 2. reasoning.tsx — MERGED Streamdown support into shared, REMOVED from extension
- **Modified** `packages/shared/src/components/ai-elements/reasoning.tsx`:
  - Added `import { Streamdown } from "streamdown"` (already a dependency of shared)
  - Added optional `streamdown?: boolean` prop to `ReasoningContentProps`
  - `ReasoningContent` conditionally wraps children in `<Streamdown>` when `streamdown` is true; renders children directly when false (backward compatible)
- **Deleted** `packages/chrome-extension/src/components/ai-elements/reasoning.tsx`
- No extension files imported the local reasoning module (it was self-contained).

### Verification
- `bun run build` in `packages/shared` — ✅ `tsc --noEmit` passes
- `bun run build` in `packages/chrome-extension` — ✅ build completes successfully

## Red-team Audit

**Verdict**: ✅ PASS

### Checks Performed

| # | Check | Result |
|---|-------|--------|
| 1 | Extension `shimmer.tsx` deleted | ✅ File absent |
| 2 | Extension `reasoning.tsx` deleted | ✅ File absent |
| 3 | Shared `reasoning.tsx` has `streamdown?: boolean` prop | ✅ Line 167: `streamdown?: boolean` in `ReasoningContentProps` |
| 4 | `ReasoningContent` conditionally wraps in `<Streamdown>` when prop true | ✅ Lines 180-185 |
| 5 | `plan.tsx` imports Shimmer from `@chrome-acp/shared` | ✅ `import { Shimmer } from "@chrome-acp/shared/components/ai-elements/shimmer"` |
| 6 | No remaining local imports of shimmer/reasoning in extension `src/` | ✅ grep clean |
| 7 | `bun run build` — `packages/shared` | ✅ `tsc --noEmit` exit 0 |
| 8 | `bun run build` — `packages/chrome-extension` | ✅ Build completed in 1509ms |

### Advisory Findings

**[Low] `children as string` cast when `streamdown=true`**
`ReasoningContentProps.children` is typed `ReactNode`, but line 181 casts to `string` inside the Streamdown branch. If a future consumer passes JSX children with `streamdown=true`, this produces a silent `[object Object]` render, not a compile error. The current extension consumer always passes string reasoning content so this is safe today. Consider narrowing the type to a discriminated union (`streamdown: true` → `children: string`) to catch misuse at compile time.

No blocking issues found.
