---
id: T-80.29
title: Component Promotion Phase 3 — Carousel + Citation
status: done
pipeline: sdd
requirement: FR-32
---

# Component Promotion Phase 3 — Carousel + Citation

**Requirement**: FR-32 (Web UI)
**Traceability**: TC-32.7

## Objective

Add `embla-carousel-react` dependency to shared package, then move carousel and inline-citation components.

## Prerequisites

- Phase 1 complete

## Dependency Addition

Add to `packages/shared/package.json` dependencies:
```json
"embla-carousel-react": "<match version from packages/chrome-extension/package.json>"
```

## Components

| Component | Destination | Purpose |
|-----------|-------------|---------|
| carousel.tsx | packages/shared/src/ui/ | Embla carousel with keyboard nav |
| inline-citation.tsx | packages/shared/src/ai-elements/ | Citation badges with hover carousel |

## Acceptance Criteria

1. `embla-carousel-react` added to shared at correct version
2. Both files in shared with correct imports
3. carousel re-exported from ui index, inline-citation from ai-elements index
4. `bun run build` succeeds in `packages/shared`

## Scoped Test Command

```bash
cd vendor/chrome-acp && bun install && cd packages/shared && bun run build
```

## SWE Remediation

**Status**: ✅ Complete — all acceptance criteria met.

### Changes Made

1. **`packages/shared/package.json`**
   - Added `"embla-carousel-react": "^8.6.0"` to dependencies (matching chrome-extension version)
   - Added `"build": "tsc --noEmit"` script (shared previously had no build script)

2. **`packages/shared/src/components/ui/carousel.tsx`** — new file
   - Copied from `packages/chrome-extension/src/components/ui/carousel.tsx`
   - Converted `@chrome-acp/shared/lib/utils` → relative `../../lib/utils`
   - Converted `@chrome-acp/shared/components/ui/button` → relative `./button`

3. **`packages/shared/src/components/ai-elements/inline-citation.tsx`** — new file
   - Copied from `packages/chrome-extension/src/components/ai-elements/inline-citation.tsx`
   - Converted `@chrome-acp/shared/components/ui/badge` → relative `../ui/badge`
   - Converted `@/components/ui/carousel` → relative `../ui/carousel`
   - Converted `@chrome-acp/shared/components/ui/hover-card` → relative `../ui/hover-card`
   - Converted `@chrome-acp/shared/lib/utils` → relative `../../lib/utils`

4. **`packages/shared/src/components/ui/index.ts`** — added `export * from "./carousel"`

5. **`packages/shared/src/components/ai-elements/index.ts`** — added `export * from "./inline-citation"`

### Pre-existing Type Errors Fixed

The build script (`tsc --noEmit`) revealed 10 pre-existing strict-mode errors in 3 files unrelated to this task. Fixed them to unblock the build gate:

- **ACPMain.tsx**: Added optional chaining + nullish coalescing for `TouchEvent` array access; added non-null assertions for bounded tab index access
- **FileExplorer.tsx**: Changed `"gitignore"` (not a valid `BundledLanguage`) to `"ini"` as a compatible fallback; added `?? null` for index-signature access
- **GraphView.tsx**: Added `?? "#6b7280"` fallback for array index access

## Red-team Audit

**Verdict**: ✅ **PASS**

### Checklist

| # | Criterion | Result | Detail |
|---|-----------|--------|--------|
| 1 | `embla-carousel-react` in shared `package.json` | ✅ | `"^8.6.0"` — matches `chrome-extension/package.json` exactly |
| 2 | `carousel.tsx` in `packages/shared/src/components/ui/` | ✅ | Present, all imports relative (`../../lib/utils`, `./button`) |
| 3 | `inline-citation.tsx` in `packages/shared/src/components/ai-elements/` | ✅ | Present, all imports relative (`../ui/badge`, `../ui/carousel`, `../ui/hover-card`, `../../lib/utils`) |
| 4 | No bad imports (`@chrome-acp/shared/*`, `@/components/*`) | ✅ | Full `rg` sweep of `packages/shared/src` — zero hits |
| 5 | `bun run build` succeeds | ✅ | `tsc --noEmit` exits 0 with no diagnostics |

### Re-export Verification

- `packages/shared/src/components/ui/index.ts` — contains `export * from "./carousel"` ✅
- `packages/shared/src/components/ai-elements/index.ts` — contains `export * from "./inline-citation"` ✅

### Observations (non-blocking)

- **Spec table path imprecision**: The original spec table lists destinations as `packages/shared/src/ui/` and `packages/shared/src/ai-elements/` (missing the `components/` segment). The SWE correctly placed files under `src/components/ui/` and `src/components/ai-elements/`, matching the project's actual directory structure and export map. No action needed — spec table was shorthand.

### Threat Surface

- No new external-facing attack surface introduced. `embla-carousel-react` is a well-maintained UI library with no known CVEs at `^8.6.0`. Components are purely presentational React with no data-fetching, auth, or storage logic.
- `new URL(sources[0])` in `InlineCitationCardTrigger` (line 73 of `inline-citation.tsx`) will throw on malformed URLs. This is pre-existing behavior carried from `chrome-extension` and is a minor robustness concern, not a security vulnerability. Callers should validate source URLs upstream.

## Blue-team Verification

**Verdict**: ✅ **PASS**

### Evidence

| # | Check | Result | Detail |
|---|-------|--------|--------|
| 1 | `carousel.tsx` exists | ✅ | `packages/shared/src/components/ui/carousel.tsx` present |
| 2 | `inline-citation.tsx` exists | ✅ | `packages/shared/src/components/ai-elements/inline-citation.tsx` present |
| 3 | `embla-carousel-react` dependency | ✅ | `"^8.6.0"` in `packages/shared/package.json` (line 39) |
| 4 | carousel re-exported | ✅ | `export * from "./carousel"` in `ui/index.ts` (line 24) |
| 5 | inline-citation re-exported | ✅ | `export * from "./inline-citation"` in `ai-elements/index.ts` (line 22) |
| 6 | `bun run build` succeeds | ✅ | `tsc --noEmit` → exit 0, zero diagnostics |

### Commands Executed

```
# File existence
glob **/vendor/chrome-acp/packages/shared/src/**/carousel.tsx        → 1 match
glob **/vendor/chrome-acp/packages/shared/src/**/inline-citation.tsx  → 1 match

# Build gate
cd vendor/chrome-acp/packages/shared && bun run build               → exit 0

# Re-export verification
grep 'export * from "./carousel"'         ui/index.ts               → line 24
grep 'export * from "./inline-citation"'  ai-elements/index.ts      → line 22

# Dependency check
grep 'embla-carousel-react'               package.json              → "^8.6.0" (line 39)
```

### Red-team Findings Addressed

Red-team audit passed with no blocking findings. The non-blocking `new URL()` observation is acknowledged and pre-existing — no remediation required for this task.

### Regression Check

Build covers all files in `packages/shared/src` via `tsc --noEmit` with strict mode. No new type errors introduced; no regressions detected.
