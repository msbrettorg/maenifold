# CODE QUALITY REPORT

## 1. Summary

**Overall health score: C**

### Key metrics
- **TypeScript typecheck:** `npx vue-tsc --noEmit` ✅ passed (no output)
- **Frontend build:** `npm run build` ✅ passed
- **Vitest:** `npm test` ❌ 3 failed, 36 passed, 14 skipped across 14 files
- **Rust tests:** `cargo test` ✅ passed, but **0 tests** ran
- **Rust clippy:** ✅ completed with **1 warning**
- **Duplication:** `jscpd` found **21 clones**, **271 duplicated lines (2.57%)** in `src/`
- **Dead code:** `knip` found **11 unused files**, **2 unused dependencies**, **3 unused devDependencies**, **24 unused exports**, **27 unused exported types**, **1 duplicate export group**
- **File size violations (>300 lines):** **3 files**
- **Unfinished work scan:** **0 TODO/FIXME/HACK/XXX** markers, but the broad scan surfaced **49 `placeholder`** and **135 `mock`** matches (mostly tests/UI attributes)
- **Debug leftovers:** no `console.log/debug/warn/error` matches in `src/`
- **Raw import statements:** **529**
- **tsconfig caveat:** `strict: true`, but `skipLibCheck: true` limits third-party type checking

### Tool status snapshot
| Tool | Status | Notes |
|---|---|---|
| `jscpd` | Worked | Useful output |
| `knip` | Worked | Reported findings; exited non-zero because issues were found |
| `eslint` | Not configured | No eslint config present |
| `npx cr` | Failed | `npm error could not determine executable to run` |
| `npx complexity-report` | Partial / unusable | Installed, but did not emit usable TS complexity data |
| `vue-tsc` | Worked | Clean |
| unfinished-work grep | Worked | High noise from tests/placeholders |
| debug grep | Worked | No matches |
| duplicate-function-name grep | Worked | Heuristic only |
| `cargo clippy` | Worked | 1 warning |

## 2. File Size Violations

Project rule is **300 lines max per file**.

### Violations
- `src/composables/__tests__/mcpIntegration.test.ts` — **526** lines
- `src/components/GateView.vue` — **322** lines
- `src/composables/usePipelineMcp.ts` — **302** lines

### Near-threshold files
- `src/lib/acp-client-core.ts` — 300 lines
- `src/composables/workflowTreeBuilder.ts` — 300 lines
- `src/components/SessionList.vue` — 300 lines
- `src/composables/useSessionConnection.ts` — 298 lines
- `src/components/TrafficMonitor.vue` — 297 lines
- `src/components/WorkflowAuthoringPage.vue` — 296 lines

### Rust note
No Rust files in `src-tauri/src/` exceeded 300 lines. Largest were:
- `src-tauri/src/gate.rs` — 242 lines
- `src-tauri/src/config.rs` — 232 lines

## 3. Code Duplication

`jscpd` scanned 77 TypeScript/Vue files in `src/` and found **21 clone groups**.

### Highest-value duplication clusters
1. **Session update/cancellation logic**
   - `src/stores/session.ts` lines 98–166, 110–190
   - `src/composables/useMessageHandler.ts` lines 203–212 and related session update code
   - Risk: behavior drift in session lifecycle handling

2. **Pipeline node form logic duplicated across add/graft flows**
   - `src/composables/useAddNodeForm.ts`
   - `src/composables/useGraftNodes.ts`
   - Four clone groups across validation/defaulting/submit handling

3. **Startup/lifecycle overlap**
   - `src/composables/useSessionLifecycle.ts`
   - `src/composables/useSessionStartup.ts`
   - Shared orchestration logic suggests these responsibilities are not clearly separated

4. **Pipeline transform logic repeated**
   - `src/lib/pipeline-composer-flow.ts`
   - `src/stores/pipeline-view-transforms.ts`
   - Repeated node/position mapping logic

5. **Test duplication is substantial**
   - `src/composables/__tests__/composableIntegration.test.ts`
   - `src/composables/__tests__/memoryApiIntegration.test.ts`
   - `src/composables/__tests__/mcpIntegration.test.ts`
   - `src/lib/__tests__/acp-client-core.test.ts`
   - Several clone groups are test scaffolding rather than production logic, but they still raise maintenance cost

### Duplication takeaway
The most important duplication is **not** the repeated tests; it is the repeated **session orchestration** and **pipeline form** behavior. Those are likely to diverge as more agents touch the codebase.

## 4. Dead Code / Unused Exports

`knip` produced the strongest dead-code signal in this audit.

### Unused files (11)
- `src/components/EnvVarEditor.vue`
- `src/components/ThinkingMarkdownContent.vue`
- `src/components/ThinkingSessionCard.vue`
- `src/components/ThinkingThoughtItem.vue`
- `src/components/ToolCallCard.vue`
- `src/components/WorkflowNodeCard.vue`
- `src/components/WorkflowStepDetail.vue`
- `src/components/WorkflowTreeErrorState.vue`
- `src/components/WorkflowTreeLevel.ts`
- `src/components/WorkflowTreeLoadingState.vue`
- `src/components/WorkflowTreeView.vue`

### Unused dependencies (2)
- `@tauri-apps/plugin-opener`
- `vue-router`

### Unused devDependencies (3)
- `@pinia/testing`
- `@types/dompurify`
- `@vitest/coverage-v8`

### Selected unused exports (24 total)
- `getModelIcon`, `getModeIcon` — `src/components/configOptionIcons.ts`
- `normalizeConfigSelectOption`, `normalizeConfigSelectOptions`, `applyConfigOptionsState`, `applyLegacySessionState`, `synthesizeClientCommands` — `src/composables/session-utils.ts`
- `normalizeToolCallLocations`, `normalizeToolCallContent`, `normalizeToolCallPayload`, `createToolCallInfo`, `warnNonTextContentBlock`, `appendMessageBlock`, `appendThoughtBlock` — `src/composables/useStreamHandler.ts`
- `positionCacheKey`, `defaultPosition`, `toFlowNodeData`, `toPipelineNode` — `src/lib/pipeline-composer-flow.ts`
- `trackMetric`, `isTelemetryEnabled` — `src/lib/telemetry.ts`

### Unused exported types (27 total)
Includes multiple ACP / pipeline / workflow helper types that appear to be defined for anticipated usage but not consumed.

### Duplicate exports
- `normalizeAuthMethods|normalizeSupportedAuthMethods|normalizeAppAuthMethods` in `src/lib/auth-methods.ts`

## 5. Complexity Hotspots

### Automated complexity status
- **ESLint-based complexity scan:** unavailable because the repo has **no ESLint configuration**.
- **Requested fallback (`npx cr src/**/*.ts --format json`):** failed with `npm error could not determine executable to run`.
- **Secondary fallback (`npx complexity-report`):** installed successfully but did **not** produce usable TypeScript complexity output.

Because automated cyclomatic metrics were unavailable, the following are **proxy hotspots** based on file size, symbol density, duplicated logic, and function nesting observed through LSP on TypeScript files.

### Proxy hotspots
1. **`src/composables/usePipelineMcp.ts` (302 lines)**
   - Large surface area of MCP operations in one composable
   - Notable dense functions:
     - `startPolling` (line 208)
     - `diffViews` (line 249)
     - `unwrapEnvelope` / `unwrapToolData` utility chain
   - Smell: one file owns transport, polling, diffing, and command dispatch

2. **`src/composables/useSessionConnection.ts` (298 lines)**
   - High orchestration load around session lifecycle
   - Notable dense functions:
     - `bindPendingPermission` (line 78)
     - `disconnect` (line 142)
     - `handleAgentClosed` (line 219)
   - Smell: state mutation, watcher setup, cleanup, and UX messaging are tightly coupled

3. **`src/composables/useMessageHandler.ts` (288 lines)**
   - Notable dense functions:
     - `handleSessionUpdate` (line 69)
     - `sendPrompt` (line 200)
     - `cancelOperation` (line 263)
   - Smell: message assembly, session mutation, tool-call cancellation, and persistence logic mixed together

4. **`src/components/GateView.vue` (322 lines)**
   - Oversized view component and current test failure hotspot
   - Smell: filters, watcher lifecycle, markdown detail, and table state are likely too concentrated in one SFC

5. **`src/stores/session.ts` / `src/composables/session-utils.ts`**
   - Repeated clone hits indicate branching logic that likely deserves consolidation

## 6. Unfinished Work

### Strong markers
- `TODO` / `FIXME` / `HACK` / `XXX`: **none found** in `src/`

### Broad-scan results (noisy)
The requested broad grep surfaced:
- **49** `placeholder` matches
- **135** `mock` matches
- **3** `stub` matches

Most of these are **not actual unfinished code comments**. They are primarily:
- test mocking in `src/test/setup.ts` and test files
- HTML input `placeholder` attributes in Vue components
- strings like “terminal output will appear here once terminal rendering lands” in `src/components/ChatToolCallDetails.vue`

### Highest-signal unfinished-work items
- `src/components/ChatToolCallDetails.vue:120` — “Live terminal output will appear here once terminal rendering lands.” This looks like a real unfinished UX path.
- `src/test/setup.ts` — extensive global mocks/stubs; this is significant because the project rules explicitly reject mocks/fakes where possible.
- Skipped/aborted live integration tests:
  - `src/composables/__tests__/memoryApiIntegration.test.ts`
  - `src/composables/__tests__/composableIntegration.test.ts`
  - `src/composables/__tests__/mcpIntegration.test.ts` (14 skipped)

## 7. Debug Leftovers

The scan for `console.log`, `console.debug`, `console.warn`, and `console.error` in `src/` returned **no matches**.

This is one of the cleaner parts of the codebase: there are **no obvious debug print leftovers** in application source.

## 8. Rust Backend

### Clippy
`cargo clippy --all-targets --all-features` completed with **1 warning**:

- `src-tauri/src/config.rs:176`
  - `path.parent().map_or(false, |p| p.exists())`
  - Clippy recommends `is_some_and(...)`

### Cargo tests
- `cargo test` passed, but **0 tests** ran in both `src/lib.rs` and `src/main.rs`
- This means the Rust backend is currently **linted but effectively untested**

## 9. Recommendations

### P0 — Fix immediately
1. **Fix the failing `GateView` tests**
   - `src/components/__tests__/GateView.test.ts` has 3 failures
   - The failures suggest UI text and watcher behavior drifted from test expectations
   - This is the clearest signal of recent regression

2. **Split oversized production files below 300 lines**
   - `src/components/GateView.vue`
   - `src/composables/usePipelineMcp.ts`
   - These are already audit hotspots and should not keep growing

3. **Remove or rewire dead files/components before more code lands on top of them**
   - Especially the `WorkflowTree*` and `Thinking*` component clusters
   - Keeping unused UI files around invites accidental divergence and duplicate implementations

### P1 — High-value cleanup
4. **Consolidate duplicated session and pipeline-form logic**
   - Merge or extract shared logic between:
     - `useAddNodeForm.ts` and `useGraftNodes.ts`
     - `useSessionLifecycle.ts` and `useSessionStartup.ts`
     - `pipeline-composer-flow.ts` and `pipeline-view-transforms.ts`

5. **Introduce a reliable TS/Vue complexity/lint setup**
   - The repo currently has no trustworthy automated cyclomatic complexity reporting
   - Add a real ESLint flat config or a TS-capable complexity tool so future audits are not forced into heuristics

6. **Delete unused dependencies and exports**
   - Start with `vue-router` and `@tauri-apps/plugin-opener`
   - Then prune unused exports in `session-utils.ts`, `useStreamHandler.ts`, and `pipeline-composer-flow.ts`

### P2 — Structural quality improvements
7. **Reduce mock-heavy tests and skipped integration coverage**
   - Current test infrastructure leans heavily on mocks in `src/test/setup.ts`
   - That conflicts with the repo’s “no mocks/fakes” guidance and weakens confidence in Tauri/runtime behavior

8. **Add real Rust backend tests**
   - Clippy is clean enough, but zero tests means regressions in `src-tauri/src/` will rely on manual detection

9. **Address small but real Rust lint debt**
   - Fix the `config.rs` clippy warning now before more small paper-cuts accumulate

### Suggested next audit pass
If you want a follow-up cleanup sprint, start in this order:
1. `GateView.vue` + `GateView.test.ts`
2. `usePipelineMcp.ts`
3. Dead `WorkflowTree*` / `Thinking*` component removal
4. Session orchestration deduplication (`useSessionConnection.ts`, `useMessageHandler.ts`, `session.ts`)
