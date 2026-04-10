# T-73 Workflow Authoring UX — Design Specification

**Kinetic Lumina · Dark-Mode Only · Tauri v2 Desktop**

This document is the single source of truth for all 12 T-73 work items. Every value herein is final. SWEs make no design decisions — they implement exactly what is specified. All CSS values use tokens from `design-tokens.css` unless a net-new token is proposed (marked ⚡NEW).

---

## 0. Foundations

### Design Language Summary
- **Aesthetic:** Precision-tooling, not consumer-app. Every pixel serves a function. Motion communicates state, not decoration.
- **Spacing rhythm:** 4px grid. The dominant vertical gap between grouped elements is `--space-3` (12px). Between groups: `--space-5` (20px). Panel padding: `--space-4` (16px).
- **Typography contract:** Labels → `--font-mono`, `--text-xs`, uppercase, `letter-spacing: 0.05em`. Values → `--font-sans`, `--text-sm`. Titles → `--font-headline`, `--text-lg`/`--text-xl`. Code/IDs → `--font-mono`, `--text-sm`.
- **Hierarchy signal:** Surface tier communicates nesting depth. Browser/Inspector use `--bg-panel`. Timeline uses `--bg-main`. Step cards use `--surface-container`. Active/selected states promote one tier up.
- **Accent protocol:** `--color-node-workflow` (#4d8eff) is the single accent for workflow authoring. No other accent color for interactive states in this page.

### Net-New Token Proposals

These tokens do not exist in `design-tokens.css` but are required by this spec. Add them to `:root` in `design-tokens.css`.

```css
/* ⚡NEW — T-73 additions */
--resize-handle-width: 12px;
--resize-handle-active: color-mix(in srgb, var(--color-node-workflow) 60%, transparent);
--draft-bar-height: 36px;
--step-rail-width: 40px;
--panel-browser-min: 180px;
--panel-browser-max: 360px;
--panel-browser-default: 250px;
--panel-inspector-min: 220px;
--panel-inspector-max: 420px;
--panel-inspector-default: 300px;
```

**Justification:** These 10 tokens isolate the layout constants this page needs. `--resize-handle-*` tokens isolate handle geometry so all three panels share one value. `--draft-bar-height` locks the save-bar height so panels compute their available height correctly. The panel dimension tokens expose layout constants that `WorkflowAuthoringPage.vue` reads for both the flex layout and the resize logic.

---

## 1. Panel Resize Handles (T-73.1)

### Layout Architecture

Replace the fixed `grid-template-columns: 250px 1fr 300px` in `WorkflowAuthoringPage.vue` with a JavaScript-driven flex layout. CSS Grid columns cannot be resized interactively without JS measurement; use `display: flex` with explicit `width` + `flex-shrink: 0` on the flanking panels and `flex: 1` on the center panel.

```
[Browser panel] [handle-left] [Timeline panel] [handle-right] [Inspector panel]
     width=browserW               flex:1                  width=inspectorW
```

### Handle Geometry

| Property | Value |
|---|---|
| Width | `var(--resize-handle-width)` = 12px |
| Height | `100%` |
| Background at rest | `var(--border-default)` (#404040) at 30% opacity — `rgba(64, 64, 64, 0.3)` |
| Background on hover | `var(--border-default)` at 70% opacity — `rgba(64, 64, 64, 0.7)` |
| Background while dragging | `var(--resize-handle-active)` = `color-mix(in srgb, var(--color-node-workflow) 60%, transparent)` |
| Cursor | `col-resize` |
| Hit area | Matches the 12px handle width exactly — the visible handle is the full interactive target |
| Transition | `background var(--transition-fast)` on hover only. No transition during drag (causes lag). |

### Grip Dots

Center three dots vertically in the handle. Each dot: `width: 2px; height: 2px; border-radius: 50%; background: var(--text-muted)`. Gap between dots: 3px. The three dots appear only on hover/drag. Opacity: 0 at rest, 1 on hover — `transition: opacity var(--transition-fast)`.

```html
<!-- Handle markup -->
<div class="panel-handle" data-handle="left|right">
  <div class="handle-track">
    <span class="grip-dot" /><span class="grip-dot" /><span class="grip-dot" />
  </div>
</div>
```

### Constraints

| Panel | Min | Max | Default |
|---|---|---|---|
| Browser (left) | `var(--panel-browser-min)` = 180px | `var(--panel-browser-max)` = 360px | 250px |
| Timeline (center) | 320px (hard floor — must show step cards) | unbounded | `1fr` |
| Inspector (right) | `var(--panel-inspector-min)` = 220px | `var(--panel-inspector-max)` = 420px | 300px |

### Drag Behavior

- **Live resize** — no snap points, continuous tracking.
- On `mousedown` on a handle: record `startX`, `startBrowserW`/`startInspectorW`.
- On `mousemove` (global, while dragging): compute `delta = e.clientX - startX`. Clamp resulting width to [min, max]. Apply directly to panel `width` style.
- On `mouseup` (global): end drag, persist widths to `localStorage` under key `wf-panel-widths-v1` as `{ browser: N, inspector: N }`.
- On mount: read `localStorage`, fall back to defaults if absent or invalid.
- During drag: set `user-select: none` on `<body>` and `cursor: col-resize` on `<html>`. Restore on drag end.
- **No animation during drag** — `transition: none` on panel widths while a drag is active. Re-enable `transition: width var(--transition-normal)` after drag ends.

### Handle States (CSS classes)

```css
.panel-handle { width: 12px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; cursor: col-resize; background: rgba(64,64,64,0.3); transition: background var(--transition-fast); position: relative; }
.panel-handle:hover, .panel-handle:focus-within, .panel-handle--dragging { background: rgba(64,64,64,0.7); }
.panel-handle--dragging { background: var(--resize-handle-active); }
.handle-track { display: flex; flex-direction: column; align-items: center; gap: 3px; }
.grip-dot { width: 2px; height: 2px; border-radius: 50%; background: var(--text-muted); opacity: 0; transition: opacity var(--transition-fast); }
.panel-handle:hover .grip-dot, .panel-handle:focus-within .grip-dot, .panel-handle--dragging .grip-dot { opacity: 1; }
```

---

## 2. Visual Polish & Cohesion (T-73.2)

### Panel Header System

All three panels share a unified header specification. The goal: headers feel like the same component family, not three independent designs.

#### Header Anatomy

```
┌─────────────────────────────────────────────────┐
│  ● SECTION LABEL          [optional chip/badge]  │  height: 44px
└─────────────────────────────────────────────────┘
  bottom: 1px solid var(--border-default)
```

| Property | Value |
|---|---|
| Height | 44px (`min-height: 44px`) |
| Padding | `0 var(--space-4)` |
| Bottom border | `1px solid var(--border-default)` |
| Background | Inherits from panel — no explicit override |
| Label font | `--font-mono`, `--text-xs`, uppercase, `letter-spacing: 0.06em`, `color: var(--text-secondary)` |
| Label leading dot | 6px circle, `background: var(--color-node-workflow)`, `border-radius: 50%`, `margin-right: var(--space-2)` |
| `flex-shrink: 0` | Always — headers never compress |

**Browser panel header:** `● WORKFLOWS` (not "WORKFLOW BROWSER" — too long)
**Inspector panel header:** `● STEP INSPECTOR` when step selected, `● WORKFLOW` when showing workflow metadata.
**Timeline panel:** Has no section header — its `timeline-header` is the workflow title bar, which is functionally different.

#### Dividers Between Panels

The resize handles provide the visual separation between panels — no additional border needed on panel edges. Remove any `border-left` / `border-right` on `panel-inspector` and `panel-browser`. The handle's background color IS the divider.

### Spacing Rhythm

| Zone | Value | Rationale |
|---|---|---|
| Panel outer padding | `var(--space-4)` (16px) | Comfortable but not wasteful |
| Between form groups | `var(--space-5)` (20px) | Clearly grouped |
| Within form group (label → field) | `var(--space-1)` (4px) | Tight coupling signals relationship |
| Step card internal padding | `var(--space-3) var(--space-4)` | 12px top/bottom, 16px sides |
| Timeline container gap | `var(--space-3)` (12px) | Reduce from current `--space-4` — tighter rail |
| Inspector scrollable body | `padding: var(--space-4)` | All sides equal |

### Typography Hierarchy

| Role | Font | Size | Weight | Color | Transform |
|---|---|---|---|---|---|
| Panel section label | `--font-mono` | `--text-xs` | 600 | `--text-secondary` | uppercase |
| Field label | `--font-mono` | `--text-xs` | 500 | `--text-muted` | uppercase, `letter-spacing: 0.05em` |
| Field value (display) | `--font-sans` | `--text-sm` | 400 | `--text-primary` | — |
| Field value (editable) | `--font-sans` | `--text-sm` | 400 | `--text-primary` | — |
| Step name (timeline card) | `--font-sans` | `--text-sm` | 600 | `--text-primary` | — |
| Step description | `--font-sans` | `--text-sm` | 400 | `--text-secondary` | — |
| Timeline workflow title | `--font-headline` | `--text-lg` | 600 | `--text-primary` | — |
| Metadata mono (IDs) | `--font-mono` | `--text-xs` | 400 | `--text-muted` | — |
| Badge / chip text | `--font-mono` | 10px | 600 | (varies) | uppercase |

### Empty States

**Browser — no workflows found:**
```
Icon: magnifying glass SVG, 24×24, color: var(--text-muted), opacity: 0.4
Headline: "Nothing found" — --font-sans, --text-sm, --text-muted
Subtext: "Try a different search" — --font-sans, --text-xs, --text-muted, opacity: 0.6
Padding: var(--space-8) var(--space-4)
```

**Timeline — no workflow selected:**
```
Icon: workflow node SVG (3 circles + lines), 32×32, color: var(--color-node-workflow), opacity: 0.3
Headline: "Select a workflow" — --font-headline, --text-base, --text-muted
Subtext: "Or create one from the browser panel" — --font-sans, --text-xs, --text-muted, opacity: 0.6
Layout: centered column, gap: var(--space-3)
```

**Inspector — no step selected:**
Shows workflow-level metadata panel (see §7). Not an empty state — always has content when a workflow is loaded.

**Inspector — no workflow loaded:**
```
Icon: cursor arrow SVG, 20×20, --text-muted, opacity: 0.4
Text: "Select a workflow to inspect" — --text-sm, --text-muted
```

### Step Card Visual Specification (Refined)

The current cards are functional but flat. Apply these refinements:

```css
.step-card {
  /* existing structure preserved */
  border: 1px solid var(--border-default);          /* ADD: explicit border */
  border-left: 4px solid transparent;               /* existing */
  background: var(--surface-container);             /* existing */
  transition: background var(--transition-fast),
              border-color var(--transition-fast),
              box-shadow var(--transition-fast);    /* ADD box-shadow */
}

.step-card:hover {
  background: var(--bg-surface);
  border-color: var(--border-default);              /* unchanged sides */
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);           /* ADD subtle lift */
}

.step-card--selected {
  background: var(--bg-surface);
  border-left-color: var(--color-node-workflow);
  border-color: color-mix(in srgb, var(--color-node-workflow) 30%, var(--border-default));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--color-node-workflow) 20%, transparent);
}
```

### Chip Filter Refinement (Browser)

Active chip needs stronger differentiation:

```css
.chip--active {
  background: color-mix(in srgb, var(--color-node-workflow) 15%, var(--bg-surface));
  color: var(--color-node-workflow);
  border: 1px solid color-mix(in srgb, var(--color-node-workflow) 40%, transparent);
  /* Remove existing no-border behavior */
}
.chip {
  border: 1px solid transparent;  /* ADD: so active doesn't cause layout shift */
}
```

---

## 3. Draft / Edit Mode (T-73.3)

### Model

The `workflow-library` store gains a `draftDefinition` field — a deep clone of `currentDefinition` that the user edits. `isDirty` compares `draftDefinition` against `currentDefinition`; it does **not** compare against the raw JSON backing store.

Fields that become editable: `step.name`, `step.description`, `step.requiresSequentialThinking`, `step.metadata.reasoning_effort`, `workflow.name`, `workflow.emoji`, `workflow.description`, `workflow.triggers`, `workflow.sequentialThinkingEnabled`, `workflow.multiAgent`, and optional `workflow.category`.

When `isDirty === true` and the user clicks a different workflow in the browser list, intercept the selection instead of switching immediately. Reuse the draft bar for the discard/save confirmation. The store owns this flow via `requestSelectWorkflow(id)` and `confirmDiscardAndContinue()`. Selection only changes after the user discards or saves.

### Always-Editable (No Mode Toggle)

There is **no read-mode / edit-mode toggle**. Fields are always editable in the active inspector context, whether that is a selected step or the workflow metadata form. This is a desktop authoring tool, not a viewer. The draft state signals unsaved changes, not a mode switch.

### Visual Treatment: Editable Fields

Editable fields look subtly different from read-only displays. The difference must be obvious but not distracting.

**Read-only display (workflow ID / derived display-only values):**
```css
.field-display {
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  padding: var(--space-2) 0;          /* No background, no border */
  line-height: 1.5;
}
```

**Editable input:**
```css
.field-input, .field-textarea {
  background: var(--bg-input);         /* #0e0e0e */
  color: var(--text-primary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  width: 100%;
  outline: none;
  transition: border-color var(--transition-fast),
              box-shadow var(--transition-fast);
  resize: vertical;                    /* textarea only */
  min-height: 80px;                    /* textarea only */
}
.field-input:focus, .field-textarea:focus {
  border-color: var(--border-focus);   /* #4da6ff */
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-focus) 20%, transparent);
}
.field-input:hover:not(:focus), .field-textarea:hover:not(:focus) {
  border-color: color-mix(in srgb, var(--border-default) 150%, transparent);
}
```

**Dirty indicator per field:** Do NOT show per-field dirty indicators. It's too noisy. Only the workflow-level dirty bar matters.

### Draft / Dirty State Bar

When `isDirty === true`, a fixed bar appears at the **bottom** of the Timeline panel (not the inspector — the timeline owns the workflow-level actions).

```
┌────────────────────────────────────────────────────────────────────┐
│  ⚡ Unsaved changes          [Discard]              [Save]          │
│  height: var(--draft-bar-height) = 36px                            │
└────────────────────────────────────────────────────────────────────┘
```

```css
.draft-bar {
  height: var(--draft-bar-height);      /* 36px */
  background: var(--surface-container);
  border-top: 1px solid color-mix(in srgb, var(--color-status-active) 30%, var(--border-default));
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-4);
  flex-shrink: 0;
}
.draft-bar-icon {
  color: var(--color-status-active);
  font-size: var(--text-sm);
  flex-shrink: 0;
}
.draft-bar-label {
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  color: var(--color-status-active);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  flex: 1;
}
```

The draft bar animates in:
```css
/* Entry: slides up from bottom */
.draft-bar-enter-active { transition: transform var(--transition-normal), opacity var(--transition-normal); }
.draft-bar-enter-from   { transform: translateY(100%); opacity: 0; }
.draft-bar-leave-active { transition: transform var(--transition-fast), opacity var(--transition-fast); }
.draft-bar-leave-to     { transform: translateY(100%); opacity: 0; }
```

Use `<Transition name="draft-bar">` wrapping the `.draft-bar` div. The timeline layout must be `display: flex; flex-direction: column` — the draft bar sits at the bottom as a sibling to the scrollable step list.

### Discard Action

Clicking **Discard** shows an inline confirmation directly in the bar — no modal:

```
[ ⚡ Unsaved changes — Discard all?  [Cancel]  [Yes, Discard] ]
```

The bar morphs in place (text changes, button changes). Transition: `opacity var(--transition-fast)`. Clicking "Yes, Discard" resets `draftDefinition` to a fresh clone of `currentDefinition`.

### Draft Indicator in Browser List

When the currently selected workflow has `isDirty === true`, show a 4px status-active dot before the emoji in the workflow list item:

```css
.workflow-item--dirty::before {
  content: '';
  display: inline-block;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--color-status-active);
  margin-right: var(--space-1);
  vertical-align: middle;
  animation: pulse-dot 2s ease-in-out infinite;
}
@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50%       { opacity: 0.4; }
}
```

### 3.5 Data Model Architecture

The store keeps two representations in parallel:

- `rawJson: Record<string, unknown>` — the full parsed workflow JSON, including fields the typed editor does not understand
- `currentDefinition` / `draftDefinition` — typed workflow data used by the UI

Round-trip rules:

1. **On load:** parse the full JSON payload into `rawJson`, then extract typed fields into `currentDefinition`.
2. **On edit:** write form changes into `draftDefinition` only.
3. **On save:** clone `rawJson`, deep-merge the editable typed fields from `draftDefinition` into that clone, then serialize the merged clone.
4. **`isDirty`:** compare `draftDefinition` vs `currentDefinition`, never `rawJson`.
5. **Export:** serialize from the merged raw JSON clone, not from `draftDefinition` alone.
6. **Unknown fields:** unknown top-level fields and unknown step-level fields must survive load → add/remove/reorder → save unchanged.

Array synchronization rules:

- On step reorder: reorder `rawJson.steps` in parallel with typed `draftDefinition.steps`.
- On step add: append the new step object to both `rawJson.steps` and `draftDefinition.steps`.
- On step remove: remove by stable `step.id` from both arrays.

### 3.6 Store vs Component Boundaries

| Location | State |
|---|---|
| **Pinia store** | `currentDefinition`, `rawJson`, `draftDefinition`, `isDirty`, `saveState` (`'idle' \| 'saving' \| 'error' \| 'success'`), `origin` (`'packaged' \| 'user' \| 'new'`), `summaries`, `searchQuery`, `categoryFilter` |
| **Component-local** | `selectedStepId`, hover states, inline confirmation visibility, resize drag state, panel widths (persisted to `localStorage` separately) |

---

## 4. Step Management UX (T-73.4 / T-73.5 / T-73.6)

### 4.1 Add Step (T-73.4)

**Trigger:** The "＋ ADD NEW STEP" button at the bottom of the step list.

**New step template:**
```typescript
{
  id: crypto.randomUUID(),
  name: 'New Step',
  description: 'Describe what this step does.',
  requiresSequentialThinking: false,
  metadata: { reasoning_effort: 'medium' }
}
```

**Behavior:** Appends to both `rawJson.steps` and `draftDefinition.steps`. The new step card appears at the bottom of the list with the shared `<TransitionGroup>` entry transition (see §10). The timeline scrolls to bring the new step into view (`scrollIntoView({ behavior: 'smooth', block: 'nearest' })`). The new step is automatically selected (emits `select-step` with new ID). The inspector opens to the new step for immediate editing.

**Add button visual:**
```css
.add-step-btn {
  border: 1px dashed var(--border-default);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: var(--space-3);
  width: 100%;
  cursor: pointer;
  transition: border-color var(--transition-fast), color var(--transition-fast), background var(--transition-fast);
}
.add-step-btn:hover {
  border-color: var(--color-node-workflow);
  color: var(--color-node-workflow);
  background: color-mix(in srgb, var(--color-node-workflow) 6%, transparent);
}
```

The leading dot in `.step-dot` changes to match the hover state:
```css
.add-entry:hover .step-dot {
  background: var(--color-node-workflow);
  transition: background var(--transition-fast);
}
```

### 4.2 Remove Step (T-73.5)

**Trigger location:** Inside the step card (not the inspector). Add a remove button on the **right side of the card header row**, revealed on card hover — not a permanent UI element.

```
┌─────────────────────────────────────────────┬───┐
│  Step Name                          🧠       │ × │   ← × only on hover
├─────────────────────────────────────────────┴───┤
│  Step description text...                        │
└──────────────────────────────────────────────────┘
```

```css
.card-remove-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  opacity: 0;
  transition: opacity var(--transition-fast),
              background var(--transition-fast),
              color var(--transition-fast);
  flex-shrink: 0;
  font-size: var(--text-base);
  line-height: 1;
}
.step-card:hover .card-remove-btn,
.step-card:focus-within .card-remove-btn { opacity: 1; }
.card-remove-btn:hover {
  background: color-mix(in srgb, var(--color-status-abandoned) 15%, transparent);
  color: var(--color-status-abandoned);
}
```

**Confirmation:** Inline, not a modal. Clicking the × morphs the card into confirmation state:

```
┌──────────────────────────────────────────────────────┐
│  Remove "Step Name"?         [Cancel]  [Remove]       │
└──────────────────────────────────────────────────────┘
```

The card background changes to `color-mix(in srgb, var(--color-status-abandoned) 8%, var(--surface-container))` and a 1px left border in `var(--color-status-abandoned)` replaces the normal left border. Transition: `background var(--transition-fast), border-color var(--transition-fast)`.

The card is replaced with a confirmation view — no overlay, no Z-index wars. The confirmation row:
```css
.card-confirm-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  color: var(--text-secondary);
}
.card-confirm-row .confirm-label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

Cancel button: ghost, `--text-sm`. Remove button: `color: var(--color-status-abandoned); border: 1px solid var(--color-status-abandoned); background: transparent; border-radius: var(--radius-sm)`.

**Cannot remove last step:** If `draftDefinition.steps.length === 1`, the × button is hidden entirely (not disabled). The workflow must have at least one step.

**Animation:** On confirm remove, the step card exits through the shared `<TransitionGroup>` leave transition (see §10) before being removed from both `rawJson.steps` and `draftDefinition.steps`. Remove by stable `step.id`, not by stale index. If the removed step was selected, selection moves to the step above it (or the new last step if it was the last).

### 4.3 Reorder Steps (T-73.6)

**Trigger:** Up (↑) and Down (↓) icon buttons on the **left side of the step rail**, revealed on card hover. They appear below/above the step number circle on the rail.

```
┌─────┐   ┌──────────────────────────────────┐
│  ↑  │   │                                  │
│ 01  │   │  Step Name                       │   ← rail extended with reorder buttons
│  ↓  │   │  Description...                  │
└─────┘   └──────────────────────────────────┘
```

The rail becomes `48px` wide when reorder buttons are visible. The number circle stays centered, with the ↑ button above and ↓ below, each `20px × 20px`.

```css
.reorder-btn {
  width: 20px;
  height: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: var(--radius-sm);
  opacity: 0;
  transition: opacity var(--transition-fast),
              background var(--transition-fast),
              color var(--transition-fast);
  font-size: 10px;
  line-height: 1;
}
.step-entry:hover .reorder-btn,
.step-entry:focus-within .reorder-btn { opacity: 1; }
.reorder-btn:hover { background: var(--bg-surface); color: var(--text-primary); }
.reorder-btn:disabled { opacity: 0 !important; cursor: default; pointer-events: none; }
```

**Disabled states:**
- First step: ↑ button is `disabled` (and opacity 0, so invisible even on hover).
- Last step: ↓ button is `disabled`.

**Behavior on click:** Swap the step with its neighbor in both `draftDefinition.steps` and `rawJson.steps`. Animate the swap through `<TransitionGroup>` move transitions (see §10). The selected step follows the moved `step.id`, not its previous index.

**SVG icons for reorder buttons:**
```html
<!-- Up -->
<svg width="10" height="10" viewBox="0 0 10 10" fill="none">
  <path d="M5 2 L9 7 L1 7 Z" fill="currentColor"/>
</svg>
<!-- Down -->
<svg width="10" height="10" viewBox="0 0 10 10" fill="none">
  <path d="M5 8 L1 3 L9 3 Z" fill="currentColor"/>
</svg>
```

---

## 5. Action Bar / Toolbar (T-73.7 / T-73.11 / T-73.12)

### Button Hierarchy

The timeline header hosts four actions. Left side: workflow title. Right side: action group.

```
[  🔣 Workflow Name        ]   [Import ↑]  [Export ↓]  [Save]  [▶ Run]
                                  ghost       ghost     outline  primary
```

**Grouping:** Export and Import are visually grouped (adjacent, no gap between, share a border treatment). Save and Run are separated by `var(--space-2)` from the export/import group, and by `var(--space-2)` between themselves.

### Button Specifications

#### Import (ghost, icon+label)
```css
.btn-import-wf {
  /* inherits .btn base */
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm) 0 0 var(--radius-sm);  /* fused left */
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.btn-import-wf:hover { background: var(--bg-surface); color: var(--text-primary); }
```

#### Export (ghost, icon+label)
```css
.btn-export-wf {
  /* mirror of import */
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;  /* fused right */
  border-left: none;                                      /* shared internal border removed */
}
```

#### Save
```css
/* State: clean (no unsaved changes) */
.btn-save { background: transparent; color: var(--text-muted); border: 1px solid var(--border-default); border-radius: var(--radius-sm); pointer-events: none; opacity: 0.4; }

/* State: dirty */
.btn-save--dirty { color: var(--color-status-active); border-color: var(--color-status-active); background: color-mix(in srgb, var(--color-status-active) 10%, transparent); pointer-events: auto; opacity: 1; }
.btn-save--dirty:hover { background: color-mix(in srgb, var(--color-status-active) 18%, transparent); }

/* State: saving (async) */
.btn-save--saving { color: var(--text-muted); border-color: var(--border-default); cursor: wait; opacity: 0.7; }
/* Show spinner: add a CSS border-spin ::after pseudo */
.btn-save--saving::after {
  content: '';
  display: inline-block;
  width: 10px; height: 10px;
  border-radius: 50%;
  border: 1.5px solid var(--text-muted);
  border-top-color: var(--text-primary);
  animation: spin 0.6s linear infinite;
  margin-left: var(--space-1);
}

/* State: save error */
.btn-save--error { color: var(--color-status-abandoned); border-color: var(--color-status-abandoned); background: color-mix(in srgb, var(--color-status-abandoned) 10%, transparent); }

@keyframes spin { to { transform: rotate(360deg); } }
```

**Save button label logic:**
- `isDirty === false`: "Saved" (greyed, non-interactive)
- `isDirty === true, !saving`: "Save"
- `saving === true`: "Saving…"
- `saveError !== null`: "Retry"

#### Save Flow

`WorkflowStepTimeline` emits `save` → the store action `saveDraft()` runs → Tauri `invoke('save_workflow_asset', { json, path })` persists the merged raw JSON payload → on success the store updates `currentDefinition`, `rawJson`, clears `draftDefinition` drift, and resets `isDirty` to `false`.

The draft bar owns save feedback:

- `saveState === 'saving'`: show inline "Saving…"
- `saveState === 'success'`: show inline `Saved ✓`, then fade it after 2s
- `saveState === 'error'`: show inline `Save failed` with a retry affordance

When saving a packaged workflow (`origin: 'packaged'`), the draft bar text changes to `Packaged workflow — will save as copy`. The Save button label becomes `Save Copy`. Clicking it auto-generates a new workflow ID of `{original-id}-custom`, writes the file to the user workflows directory, and then makes the forked workflow the active selection.

#### Run
```css
.btn-run {
  background: var(--color-node-workflow);   /* #4d8eff */
  color: var(--text-on-accent);
  border: none;
  border-radius: var(--radius-sm);
  padding: var(--space-1) var(--space-4);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  font-weight: 600;
  cursor: pointer;
  transition: background var(--transition-fast), opacity var(--transition-fast), transform var(--transition-fast);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.btn-run:hover { background: #6ab5ff; }   /* one tonal step up from #4d8eff */
.btn-run:active { transform: scale(0.97); }
.btn-run:disabled { opacity: 0.4; cursor: not-allowed; transform: none; }
```

**Run button disabled when:**
1. No workflow loaded (`!currentId`)
2. `isDirty === true`

**Run button label logic:**
- Normal: `▶ Run`
- `isDirty === true`: `▶ Run` but disabled; tooltip via `title="Save changes before running"`
- `running === true`: `◼ Running…` (square stop icon, same color, no action)

---

## 6. Inspector Edit Mode (T-73.3 continued)

### Inspector Layout When Step Selected

```
┌─────────────────────────────────────────────┐
│ ● STEP INSPECTOR          [step ID mono]    │  44px header
├─────────────────────────────────────────────┤
│ (scrollable body, padding: var(--space-4))  │
│                                             │
│  STEP NAME                                  │
│  ┌─────────────────────────────────────┐    │
│  │ New Step                            │    │  field-input
│  └─────────────────────────────────────┘    │
│                                             │
│  DESCRIPTION                                │
│  ┌─────────────────────────────────────┐    │
│  │ Describe what this step does.       │    │  field-textarea
│  │                                     │    │
│  │                                     │    │  min-height: 80px
│  └─────────────────────────────────────┘    │
│                                             │
│  REASONING EFFORT                           │
│  ┌──────────┬──────────┬──────────┐        │  segmented control (editable)
│  │   LOW    │  MEDIUM  │   HIGH   │        │
│  └──────────┴──────────┴──────────┘        │
│                                             │
│  ┌─────────────────────────────────────┐    │
│  │ Sequential Thinking         [toggle]│    │
│  └─────────────────────────────────────┘    │
│                                             │
│  TOOL HINTS (if present)                    │
│  STOP CONDITIONS (if present)               │
│  ESCAPE HATCHES (if present)                │
│                                             │
└─────────────────────────────────────────────┘
```

### Step ID in Header

Move the step ID from the body into the header, right-aligned:

```css
.inspector-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  padding: 0 var(--space-4);
  border-bottom: 1px solid var(--border-default);
  flex-shrink: 0;
}
.inspector-header-label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.inspector-header-label::before {
  content: '';
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--color-node-workflow);
  flex-shrink: 0;
}
.inspector-step-id {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
  opacity: 0.7;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

### Reasoning Effort — Editable Segmented Control

When editable, the segments are `<button>` elements (not `<span>`). Active segment shows `--color-node-workflow` with `color: var(--bg-main)` (dark text on blue). Inactive segments show as ghost.

```css
.effort-segments { display: grid; grid-template-columns: repeat(3, 1fr); border-radius: var(--radius-md); overflow: hidden; border: 1px solid var(--border-default); }
.effort-segment {
  text-align: center;
  padding: var(--space-2) var(--space-1);
  font-size: var(--text-xs);
  font-family: var(--font-mono);
  font-weight: 600;
  letter-spacing: 0.04em;
  color: var(--text-muted);
  background: var(--bg-input);
  border: none;
  cursor: pointer;
  transition: background var(--transition-fast), color var(--transition-fast);
}
.effort-segment + .effort-segment { border-left: 1px solid var(--border-default); }
.effort-segment:hover:not(.active) { background: var(--bg-surface); color: var(--text-secondary); }
.effort-segment.active {
  background: var(--color-node-workflow);
  color: var(--bg-main);            /* dark text on blue */
}
```

Click on a segment → updates `draftDefinition.steps.find(step => step.id === selectedStepId)?.metadata.reasoning_effort`.

### Sequential Thinking Toggle — Editable

The toggle is always clickable. Clicking it flips `draftDefinition.steps.find(step => step.id === selectedStepId)?.requiresSequentialThinking`.

```css
.thinking-indicator {
  width: 36px; height: 20px;
  border-radius: 10px;
  background: var(--bg-surface);
  border: 1px solid var(--border-default);
  display: flex;
  align-items: center;
  padding: 2px;
  cursor: pointer;
  transition: background var(--transition-normal), border-color var(--transition-normal);
  flex-shrink: 0;
}
.thinking-indicator.on {
  background: var(--color-node-thinking);   /* #d0bcff */
  border-color: var(--color-node-thinking);
  justify-content: flex-end;
}
.indicator-dot {
  width: 14px; height: 14px;
  border-radius: 50%;
  background: var(--text-primary);
  transition: transform var(--transition-normal);
  box-shadow: 0 1px 2px rgba(0,0,0,0.4);
}
```

When `on`: the dot slides right (handled by `justify-content: flex-end`). The transition on `justify-content` does not animate; use `transform: translateX` instead:

```css
/* Alternative implementation using transform for smooth slide */
.thinking-indicator { position: relative; }
.indicator-dot { position: absolute; left: 2px; transition: left var(--transition-normal); }
.thinking-indicator.on .indicator-dot { left: calc(100% - 16px); }
```

### Validation Error Display

Field-level validation errors appear below the field, not as alerts:

```css
.field-error {
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  color: var(--text-error);               /* #ff6b6b */
  margin-top: var(--space-1);
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.field-error::before {
  content: '!';
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px; height: 14px;
  border-radius: 50%;
  border: 1px solid var(--text-error);
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
}
```

When a field has an error, its `field-input` / `field-textarea` gains:
```css
.field-input--error, .field-textarea--error {
  border-color: var(--border-error);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--border-error) 30%, transparent);
}
```

**Validation rules:**
- Step name: required, max 80 chars. Error: "Name is required" / "Max 80 characters"
- Description: optional, max 500 chars. No required error.
- Reasoning effort: always has a value (default: 'medium'). No validation needed.
- `requiresSequentialThinking`: boolean, always valid.

---

## 7. Workflow Metadata Editor (T-73.8)

### Placement

**When no step is selected** — the inspector shows workflow-level metadata. This is the only place workflow metadata is edited. No modal, no secondary panel. The inspector adapts to show workflow fields when `selectedStepId === null`.

### Inspector: Workflow Mode

Header changes to `● WORKFLOW` with the workflow emoji as a secondary element.

```
┌─────────────────────────────────────────────┐
│ ● WORKFLOW                  🔣              │  44px header — emoji right-aligned
├─────────────────────────────────────────────┤
│                                             │
│  ID                                         │
│  think-tank-game-theory  (monospace, muted) │  read-only for existing workflows
│                                             │
│  EMOJI                                      │
│  ┌─────────────────────────────────────┐    │
│  │ 🔣                                  │    │  editable text input
│  └─────────────────────────────────────┘    │
│                                             │
│  NAME                                       │
│  ┌─────────────────────────────────────┐    │
│  │ Think Tank Game Theory              │    │  editable
│  └─────────────────────────────────────┘    │
│                                             │
│  DESCRIPTION                                │
│  ┌─────────────────────────────────────┐    │
│  │ Multi-agent consensus workflow...   │    │  editable, min-height: 80px
│  └─────────────────────────────────────┘    │
│                                             │
│  TRIGGERS                                   │
│  ┌─────────────────────────────────────┐    │
│  │ think-tank, game-theory             │    │  editable comma-separated input
│  └─────────────────────────────────────┘    │
│                                             │
│  CATEGORY                                   │
│  ┌─────────────────────────────────────┐    │
│  │  reasoning              ▾           │    │  optional select dropdown
│  └─────────────────────────────────────┘    │
│                                             │
│  SEQUENTIAL THINKING          [toggle]      │
│                                             │
│  MULTI-AGENT                    [toggle]    │
│                                             │
└─────────────────────────────────────────────┘
```

### Category Select Dropdown

```css
.field-select {
  appearance: none;
  -webkit-appearance: none;
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-7) var(--space-2) var(--space-3);
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  width: 100%;
  cursor: pointer;
  outline: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' fill='none'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%23707070' stroke-width='1.5' stroke-linecap='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right var(--space-3) center;
  transition: border-color var(--transition-fast), box-shadow var(--transition-fast);
}
.field-select:focus {
  border-color: var(--border-focus);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--border-focus) 20%, transparent);
}
```

Options: "All Categories", "reasoning", "scientific", "multi-agent", "devops", "memory", "custom".

The `WorkflowAssetDefinition` type needs `category?: WorkflowCategory` added. This field maps to the `categorizeWorkflow()` utility — the editor sets an explicit category override stored in `draftDefinition`. Precedence rule: explicit `category` wins over heuristic derivation. When exported to JSON, the category appears as a top-level `category` field.

### Workflow Name Field

`workflow.name` is editable. Change is reflected immediately in:
1. The inspector header emoji area
2. The timeline header `<h1>` (reads from `draftDefinition`)
3. The browser list item name

All three read from `draftDefinition.name`, not `currentDefinition.name`. This is the correct live preview behavior.

### Workflow Metadata Fields

When no step is selected, the metadata form edits `draftDefinition` directly for:

- `name`
- `emoji`
- `description`
- `triggers` (comma-separated text input mapped to `string[]`)
- `sequentialThinkingEnabled`
- `multiAgent`
- optional `category`

Workflow ID remains read-only for existing workflows. All metadata edits update `draftDefinition` and therefore participate in `isDirty`.

---

## 8. Create New Workflow (T-73.9)

### Trigger

"+ New Workflow" button in the browser toolbar. The button already exists but is unwired.

### New Workflow Template

```typescript
const newWorkflow: WorkflowAssetDefinition = {
  id: crypto.randomUUID(),
  name: 'Untitled Workflow',
  emoji: '⚙️',
  description: '',
  triggers: [],
  sequentialThinkingEnabled: false,
  multiAgent: false,
  steps: [
    {
      id: crypto.randomUUID(),
      name: 'First Step',
      description: 'Describe what this step does.',
      requiresSequentialThinking: false,
      metadata: { reasoning_effort: 'medium' }
    }
  ]
}
```

### Behavior

1. If `isDirty === true` on the current workflow: show the inline Discard confirmation in the draft bar first. Only proceed when user confirms or there are no dirty changes.
2. Create `newWorkflow` object.
3. Add it to the `summaries` list in the workflow library store (prepended — appears first).
4. Set `draftDefinition = newWorkflow`, `isDirty = true` immediately (it's new — must be saved to persist).
5. Set `currentId = newWorkflow.id`.
6. Scroll browser list to top.
7. Auto-select first step (index 0).
8. Focus the name field in the inspector.

The new workflow appears in the browser list with a pulsing status-active dot (dirty state). The timeline shows the template step. The inspector is open for editing.

**New workflow `isDirty` indicator is status-active dot + "(unsaved)" suffix in the browser label:**
```html
<span class="workflow-name">{{ wf.name }}<span v-if="isDirtyAndNew" class="unsaved-suffix"> (unsaved)</span></span>
```

```css
.unsaved-suffix { font-style: italic; color: var(--text-muted); font-size: var(--text-xs); margin-left: var(--space-1); }
```

### Browser "New" Entry Visual Distinction

New (never-saved) workflows have a subtle visual treatment in the list:
```css
.workflow-item--new {
  opacity: 0.85;
  /* italic name is the only signal — no border, no badge spam */
}
.workflow-item--new .workflow-name { font-style: italic; }
```

---

## 9. Run Button — Session Lifecycle (T-73.12)

Run is straightforward: the action stays disabled while `isDirty || !currentId`. Keep the tooltip explaining why the disabled state exists. Do not add any intermediate confirmation bar or auto-save-on-run path.

### Run Button Loading State

While running (awaiting MCP response):
- `▶ Run` → `◌ Starting…` with spinner (same CSS spin animation)
- Button `cursor: wait`
- Button becomes non-interactive (`pointer-events: none`)

### Navigation on Success

On successful `maenifold-Workflow` call: emit a `workflow-started` event from `WorkflowStepTimeline` to `WorkflowAuthoringPage`, which propagates up to the main app to navigate to the execution view (`WorkflowTreeView`). The workflow session ID from the MCP response is passed as a payload.

---

## 10. Micro-Interactions & Motion (T-73.2 / all)

### Step Enter / Leave / Move

Use one `<TransitionGroup>` for the step list and let Vue handle add, remove, and reorder transitions. Do **not** add JS-timed class toggles or custom keyframe animations for these cases.

```html
<TransitionGroup name="step-reorder" tag="ol" class="step-list">
```

### Step Reorder Animation

```css
.step-reorder-move {
  transition: transform var(--transition-normal) var(--easing-standard);
}
.step-reorder-enter-active { transition: opacity var(--transition-fast), transform var(--transition-fast); }
.step-reorder-enter-from   { opacity: 0; transform: translateX(-4px); }
.step-reorder-leave-active { transition: opacity var(--transition-fast), transform var(--transition-fast); }
.step-reorder-leave-to     { opacity: 0; transform: translateX(-4px); }
```

**Critical:** `<TransitionGroup>` requires `:key` on each `<li>`. Use `step.id` (not array index) as the key. This is what enables correct diffing during reorder.

### Step Number Circle Update

Keep this to simple property transitions only:
```css
.step-number { transition: border-color var(--transition-fast), color var(--transition-fast), transform var(--transition-fast); }
```

### Save Success Feedback

After a successful save:
1. The draft bar shows inline `Saved ✓` feedback for 2s, then fades away.
2. `isDirty` is set to `false`.
3. The status-active dot on the browser list item disappears.

### Panel Resize Feedback

- Handle color change on hover: `var(--transition-fast)` (already specified in §1).
- No snap or bounce — resize is mechanical and precise.
- On drag end: the panel edges transition smoothly if the handle was released: `transition: width var(--transition-normal)` re-enables for the post-drag snap (there is no snap, but re-enabling transition allows smooth release feel).

### Selection Transition

When clicking a different step card, the selection state (left border + background) transitions:
- Outgoing card: border left color `→ transparent`, background `→ var(--surface-container)` — `var(--transition-fast)`
- Incoming card: border left color `→ var(--color-node-workflow)`, background `→ var(--bg-surface)` — `var(--transition-fast)`

These already use `transition: background var(--transition-fast), border-color var(--transition-fast)` — confirm both properties are present.

Inspector content change: The inspector **does not animate** panel content when switching between steps. The content swaps immediately. Animation here would cause disorientation — the panel is spatially fixed, only the content changes.

### Loading State

When `definitionLoading === true` in the timeline, show a skeleton instead of the empty-text:

```css
.skeleton-line {
  height: 12px;
  background: var(--surface-container);
  border-radius: var(--radius-sm);
  animation: skeleton-pulse 1.4s ease-in-out infinite;
}
@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.5; }
  50%       { opacity: 1; }
}
```

Show 3 skeleton step cards (fixed, not real data), each with a skeleton for the title (60% width) and description (90% width). This prevents layout shift when real data loads.

---

## 11. File: WorkflowAuthoringPage.vue — Layout Specification

The page must shift from CSS Grid to a Flex-based layout to enable drag-resize. Final structure:

```html
<div class="workflow-authoring-page">
  <div class="panel-browser-shell" :style="{ width: browserW + 'px' }">
    <WorkflowBrowser />
  </div>
  <div class="panel-handle" data-handle="left" @mousedown="startResize('left', $event)" />
  <div class="panel-timeline">
    <WorkflowStepTimeline ... />
    <Transition name="draft-bar">
      <WorkflowDraftBar v-if="isDirty || saveState !== 'idle'" @save="saveDraft" @discard="discardDraft" />
    </Transition>
  </div>
  <div class="panel-handle" data-handle="right" @mousedown="startResize('right', $event)" />
  <div class="panel-inspector-shell" :style="{ width: inspectorW + 'px' }">
    <WorkflowStepInspector />
  </div>
</div>
```

```css
.workflow-authoring-page {
  display: flex;
  height: 100%;
  overflow: hidden;
}
.panel-browser-shell,
.panel-inspector-shell {
  height: 100%;
  flex-shrink: 0;
}
.panel-browser-shell > *,
.panel-inspector-shell > * {
  width: 100%;
  height: 100%;
}
.panel-timeline {
  flex: 1;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  background: var(--bg-main);
  overflow: hidden;
}
```

The draft bar is a child of `.panel-timeline` (flex child at bottom), not inside `WorkflowStepTimeline`. The timeline's scrollable content area must be `flex: 1; overflow-y: auto`.

Panel widths are owned exclusively by `WorkflowAuthoringPage.vue`. Browser and inspector children must not declare fixed widths; they render at `width: 100%; height: 100%` inside the parent-owned shells.

### Required Subcomponent Extraction

To keep files under 300 lines, extract these pieces instead of leaving them embedded in the page-level components:

- `PanelResizeHandle.vue` — resize drag logic + visual handle
- `WorkflowDraftBar.vue` — dirty indicator bar with save/discard
- `WorkflowStepCard.vue` — individual step card (extracted from timeline)
- `WorkflowTimelineHeader.vue` — title + action buttons
- `WorkflowStepForm.vue` — editable step fields (extracted from inspector)
- `WorkflowMetadataForm.vue` — workflow-level metadata fields
- `WorkflowInspectorHeader.vue` — inspector header with step ID

---

## 12. Import / Export (T-73.10 / T-73.11)

### Export

**Trigger:** Export button in the timeline header.

**Behavior:**
1. If `isDirty === true`: show inline prompt in the action area: "Export draft or saved version?" — `[Draft]` `[Saved]` `[Cancel]`. Default to `[Draft]`.
2. Open Tauri save dialog via `@tauri-apps/plugin-dialog` `save({ defaultPath: '${workflowId}.json', filters: [{ name: 'JSON', extensions: ['json'] }] })`.
3. Serialize the chosen definition from the merged raw JSON clone: `JSON.stringify(mergedRawJson, null, 2)`.
4. Write to the chosen path via `writeTextFile()`.
5. On success: show inline draft-bar status `Exported ✓` for 2s.
6. On error: show inline draft-bar error state with retry.

### Import

**Trigger:** Import button in the timeline header.

**Behavior:**
1. If `isDirty === true`: run discard confirmation sequence first.
2. Open Tauri open dialog via `@tauri-apps/plugin-dialog` `open({ filters: [{ name: 'JSON', extensions: ['json'] }] })`.
3. Read the file via `readTextFile()`. Parse as JSON.
4. Validate against `WorkflowAssetDefinition` shape. Required: `id`, `name`, `steps[]`. Missing required fields → show validation error inline.
5. If workflow ID already exists in `summaries`: show inline warning bar: `"A workflow with this ID already exists. Overwrite?"` — `[Overwrite]` `[Import as Copy]` `[Cancel]`. "Import as Copy" appends `-copy` to the ID.
6. On success: add to `summaries`, set as `currentId`, set `isDirty = true` (it's not saved to the maenifold asset store yet).

---

## 13. Implementation Order for SWEs

This order minimizes blocking dependencies across T-73 work items:

| Order | Task | Depends On |
|---|---|---|
| 1 | T-73.3 — Draft model infrastructure | None (store changes only) |
| 2 | T-73.2 — Visual polish | None (CSS only) |
| 3 | T-73.8 — Workflow metadata editor | T-73.3 |
| 4 | T-73.1 — Panel resize handles | None (layout only) |
| 5 | T-73.4 — Add step | T-73.3 |
| 6 | T-73.5 — Remove step | T-73.3, T-73.4 |
| 7 | T-73.6 — Reorder steps | T-73.3, T-73.4 |
| 8 | T-73.7 — Save to disk | T-73.3 |
| 9 | T-73.9 — Create new workflow | T-73.3, T-73.7 |
| 10 | T-73.11 — Export | T-73.7 |
| 11 | T-73.10 — Import | T-73.3, T-73.7 |
| 12 | T-73.12 — Run button | T-73.7 |

---

## 14. Accessibility Notes

These are implementation requirements, not aspirational:

- All `<button>` elements have `type="button"` (prevent form submit).
- All interactive controls have accessible names: icon-only buttons use `aria-label`.
- Resize handles: `role="separator"`, `aria-orientation="vertical"`, `aria-valuenow` (current panel width), `aria-valuemin`, `aria-valuemax`. Keyboard: Arrow keys move the handle by 10px.
- Draft bar has `role="status"` and `aria-live="polite"` so screen readers announce "Unsaved changes".
- Toggle (sequential thinking): `role="switch"`, `aria-checked="true|false"`.
- Step list: `<ol>` with `aria-label="Workflow steps"`. Each step `<li>` has `aria-selected` reflecting selection state.
- Remove confirmation: `aria-live="assertive"` on the confirmation row so it's announced immediately.
- Inline save/export/import status uses `role="status"` and `aria-live="polite"`.

---

*End of T-73 Design Specification. All values are final. Ambiguities in this document take precedence over existing component implementations. Contact the design lead (via the T-73 gate task comments) if a specification is physically impossible to implement — do not make ad-hoc substitutions.*
