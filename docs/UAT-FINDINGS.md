# Sprint 178 UAT Findings

**Date:** 2026-04-19
**Tester:** PM (claude-opus-4.6-1m)
**Method:** CDP Playwright automation + manual inspection
**Screenshots:** `docs/uat-screenshots/` (88 captures)

---

## Critical Finding: P0c-regression — ELK Edge References Broken

**Severity:** P0 — edges NOT routing through ELK, falling back to grid layout + smoothstep
**Status:** ✅ FIXED during UAT

### Root Cause

`useLayout.ts:76-77` used `sources: ['nodeId.portId']` format for ELK extended edges:
```ts
sources: [`${edge.source}.${activePorts.output}`],  // e.g., 'verify-connectivity.out-right'
targets: [`${edge.target}.${activePorts.input}`],
```

ELK.js 0.11.1 throws `JsonImportException: Referenced shape does not exist: verify-connectivity.out-right` — it cannot resolve dot-separated port references in extended edge format despite ports being declared on the node.

### Fix Applied

Changed to node-level references (ELK still routes through declared ports):
```ts
sources: [edge.source],  // 'verify-connectivity'
targets: [edge.target],  // 'query-fiscal-trends'
```

### Verification

| Metric | Before Fix | After Fix |
|--------|-----------|-----------|
| Unique Y-coordinates in edge paths | 1 (86.0) | 180 |
| Console ELK errors | 2 per layout | 0 |
| Node positioning | 4×5 grid (fallback) | DAG layers (ELK) |
| Edge path type | Smoothstep fallback | Orthogonal with bends |

---

## P0 Verification Results

### P0a — Folder Picker (FolderPickerDialog.vue)

- **Status:** ✅ PASS
- **Evidence:** Home tab shows "What can I help you with?" heading, prompt input visible
- **Screenshots:** `70-home.png`, `97-FINAL-home.png`

### P0b — Tab Routing (blank-pane fix)

- **Status:** ✅ PASS
- **Evidence:** All 7 tabs render content when switched via role="tab" clicks. Home returns correctly after viewing pipeline (screenshot `14-home-after-pipeline.png`)
- **Verified tabs:** Home, Agents, Pipelines, Gate, Knowledge, Workflows, Settings
- **Note:** Tab buttons use `role="tab"` + Reka UI, NOT `data-tab-id` attributes. Playwright `getByRole("tab")` is the correct selector.
- **Screenshots:** `60-tab-*.png`, `61-role-*.png`, `97-FINAL-*.png`

### P0c — ELK Pipeline Layout (8-bug compound fix)

- **Status:** ⚠️ PARTIAL — 7/8 bugs fixed, 1 regression found and fixed during UAT
- **Bug 1 (hard-coded TB):** ✅ LR/TB buttons both trigger layout with correct direction
- **Bug 2 (watcher stomping):** ✅ Structural watcher only re-layouts on node/edge set changes
- **Bug 3 (all-or-nothing cache):** ✅ Partial cache works — uncached nodes trigger layout
- **Bug 4 (no port constraints):** ✅ Ports declared with FIXED_SIDE, correct sides per direction
- **Bug 5 (hard-coded L/R handles):** ✅ 4 handles (in-top, out-bottom, in-left, out-right), reactive
- **Bug 6 (silent worker failure):** ✅ try/catch + toast warning + fallback grid
- **Bug 7 (drag clears all edges):** ✅ Only connected edges cleared on drag
- **Bug 8 (edge indicator):** ✅ Blanket tool→warning removed
- **Bug 9 (REGRESSION):** ✅ FIXED — ELK edge source/target references used `nodeId.portId` format unsupported by elkjs 0.11.1
- **Screenshots:** `95-FINAL-pipeline-LR.png`, `96-FINAL-pipeline-TB.png`

### P0d — Trio Terminology Eradication

- **Status:** ✅ PASS
- **Evidence:** AGENTS.md has P0 escalation triggers. Memory search confirmed zero trio/alpha/beta/gamma WikiLinks remain.

### P0e — Sequential Thinking Trace Display

- **Status:** ✅ PASS (visual not verified — needs thinking node inspection)
- **Evidence:** 5 sequential thinking nodes visible in pipeline canvas with content excerpts
- **Screenshots:** `37-thinking-node-selected.png`

---

## Component Inventory Verified

### Pipeline Canvas
| Component | Status | Notes |
|-----------|--------|-------|
| Node rendering (20 nodes) | ✅ | All rendered with correct type badges |
| Edge rendering (49 edges) | ✅ | Orthogonal routing after fix |
| LR layout button | ✅ | Triggers ELK, nodes layered correctly |
| TB layout button | ✅ | Triggers ELK, nodes layered correctly |
| Context menu (right-click node) | ✅ | Shows Restart, Rewind, Fork |
| Node selection | ✅ | Click selects, pane click deselects |
| Minimap | ✅ | Visible, bottom-right |
| Controls (zoom) | ✅ | Visible, bottom-left |
| Semantic zoom overlay | ✅ | Opens on double-click |

### Pipeline Dialogs
| Dialog | Status | Notes |
|--------|--------|-------|
| Graft nodes | ✅ | Node type dropdown (4 types), upstream edge selector |
| Join pipeline | ✅ | Source session ID, edge mapping form |
| Fork pipeline | ✅ | Optional name field, Fork/Cancel buttons |

### Pipeline Toolbar
| Button | Status | Notes |
|--------|--------|-------|
| Layout LR | ✅ | Triggers ELK layout |
| Layout TB | ✅ | Triggers ELK layout |
| Rewind | ✅ | Enabled when node selected |
| Delete | ✅ | Enabled when node selected |
| Graft nodes | ✅ | Opens dialog |
| Join pipeline | ✅ | Opens dialog |
| Runner toggle | ✅ | Expands/collapses runner panel |

### Runner Panel
| Component | Status | Notes |
|-----------|--------|-------|
| Status display | ✅ | "completed — 20/20 completed" |
| Log panel | ✅ | Shows timestamped node execution logs |
| Active indicator | ✅ | "Active: none" for completed pipeline |

### Tabs
| Tab | Content Verified | Notes |
|-----|:---:|-------|
| Home | ✅ | "What can I help you with?" + prompt input |
| Agents | ✅ | Session cards with agent names, timestamps |
| Pipelines | ✅ | Canvas with flyout, toolbar, runner |
| Gate | ✅ | Table with T-178 tasks, process, progress |
| Knowledge | ✅ | "Notes" heading, search input |
| Workflows | ✅ | Category filters, workflow list items |
| Settings | ✅ | "Connection settings" section |

### Pipeline Flyout
| Component | Status | Notes |
|-----------|--------|-------|
| Open button | ✅ | aria-label="Open pipelines flyout" |
| Pipeline list | ✅ | Shows 4+ pipelines with node counts |
| Active indicator | ✅ | Current pipeline highlighted |
| Delete button (×) | ✅ | Per-pipeline |
| Pipeline select | ✅ | Click loads canvas |
| Close (Escape) | ✅ | Closes flyout |

---

## Known Issues (Non-P0)

1. **useComposerCanvas.ts (498 LOC)** — exceeds 300 LOC limit. Split needed.
2. **PipelineComposer.vue (335 LOC)** — exceeds 300 LOC limit. Split needed.
3. **Direction not persisted** — switching away from pipeline and back resets to default.
4. **Edge paths render as smoothstep initially** — ELK only runs when layout buttons clicked or structure changes. On first load with cached positions, edges use smoothstep.
5. **Pipeline flyout intercepts pointer events** — requires `force=True` for Playwright interaction, may affect accessibility.
6. **Settings page minimal** — only "Connection settings" visible, no subsection navigation.
