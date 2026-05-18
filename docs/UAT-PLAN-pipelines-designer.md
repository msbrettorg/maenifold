# Pipelines Designer UAT Plan

**Version:** 1.0
**Date:** 2026-05-18
**Status:** Draft
**Total Test Cases:** 252
**TC Range:** TC-700 – TC-951

---

## Overview

This document is an exhaustive User Acceptance Test (UAT) plan for the **Pipelines** tab of the Maenifold desktop application, focused on the pipeline designer (PipelineComposer), its toolbar, sidebar, flyout, context menus, node/edge inspectors, runner, and run-history panel. It complements the umbrella `docs/UAT-PLAN.md`, which already covers the Pipelines tab at a survey level in Sections D and E (TC-159 through ~TC-330). Where the umbrella plan establishes that a control exists, this plan drills into every field, every menu entry, every keyboard chord, every visible UI element, and the canvas/runtime state transitions that the umbrella plan does not enumerate.

Scope: only the Pipelines tab surface area and its dependencies — the composer canvas, layout & view controls, palette, node/edge configuration dialogs, run controls, run-history events, the pipeline flyout, the template launcher, and the cross-cutting save / dirty-state behavior. Out of scope: workflow authoring (separate tab), gate, knowledge, agents chat, settings. Exit criteria: every test case below has been executed against a packaged Electron build via interactive CDP testing (`agent-browser`), every Pass/Fail has been recorded, every Fail has a corresponding gate task filed under the active sprint, and the four known-broken items listed at the bottom have been triaged (fix or WONTFIX-with-evidence). The plan is sized to be runnable end-to-end in a single UAT session.

---

## Pre-flight

1. **Launch the packaged Electron app** in UAT mode with CDP enabled:
   ```bash
   ./scripts/uat-launch.sh T-328 9271
   ```
   Wait for the splash to clear and the main window to render the default tab.

2. **Connect agent-browser to the CDP port**:
   ```bash
   agent-browser connect 9271
   agent-browser snapshot -i | head -40
   ```
   Confirm the snapshot returns the AppTabBar and at least one tab marked active.

3. **Verify the daemon is running on 8080**:
   ```bash
   curl --max-time 5 -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8080/mcp
   ```
   Expected: `405` (the MCP endpoint rejects GET). Any other code (000, 404, 500) → abort and triage daemon health before continuing.

4. **Verify gpt-5.4 inference works end-to-end** via a `GatewayIntent` probe:
   ```bash
   python3 scripts/test-gateway-intent.py "list two pipeline-relevant gateway commands"
   ```
   Expected: a JSON response with `candidates[]` populated and no `error` field. If inference fails, the entire run-pipeline section (P.13–P.15) cannot be validated and must be skipped with a recorded reason.

5. **Navigate to the Pipelines tab** in the app (`agent-browser click @<pipelines-tab-ref>`). Capture a snapshot and confirm the launcher (or last-opened pipeline) renders without console errors.

---

## P.1 — Empty / Launcher State

Pipelines tab on a fresh profile with no pipeline open. Component: `PipelineLauncher` (rendered by `PipelinesView` when no `pipelineId` is selected).

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-700 | P.1 | AppTabBar | Pipelines tab is reachable from any other tab | Click each of Home, Agents, Gate, Knowledge, Workflows, Settings then click Pipelines | Pipelines tab activates, launcher renders within 1s, no console errors | |
| TC-701 | P.1 | PipelineLauncher | Launcher heading present | Snapshot the launcher region | Heading text "Pipelines" or equivalent visible at top of view | |
| TC-702 | P.1 | PipelineLauncher | Subheading / description present | Snapshot launcher | Short descriptive paragraph rendered beneath heading | |
| TC-703 | P.1 | PipelineLauncher | Pipeline-name textbox renders | Snapshot for input role=textbox in launcher | Single empty input field with placeholder visible | |
| TC-704 | P.1 | PipelineLauncher | Pipeline-name textbox accepts text | `agent-browser type @<launcher-input> "uat smoke"` | Field shows "uat smoke", no truncation | |
| TC-705 | P.1 | PipelineLauncher | Pipeline-name textbox accepts paste | Focus field, paste 200 chars via clipboard | Field accepts up to its max length, no crash | |
| TC-706 | P.1 | PipelineLauncher | Template button: Research & Verify | Snapshot launcher | Button labeled "Research & Verify" (or equivalent) visible | |
| TC-707 | P.1 | PipelineLauncher | Template button: Think & Critique | Snapshot launcher | Button labeled "Think & Critique" visible | |
| TC-708 | P.1 | PipelineLauncher | Template button: Draft & Refine | Snapshot launcher | Button labeled "Draft & Refine" visible | |
| TC-709 | P.1 | PipelineLauncher | Template button: Explore & Synthesize | Snapshot launcher | Button labeled "Explore & Synthesize" visible | |
| TC-710 | P.1 | PipelineLauncher | "Create new pipeline" button | Snapshot launcher | Primary action button labeled "Create new pipeline" (or "Create blank pipeline") visible | |
| TC-711 | P.1 | PipelineLauncher | "Open pipelines flyout" button | Snapshot launcher | Secondary button labeled "Open pipelines flyout" or icon-button with that tooltip visible | |
| TC-712 | P.1 | PipelineLauncher | "Browse existing pipelines" button | Snapshot launcher | Button labeled "Browse existing pipelines" visible | |
| TC-713 | P.1 | PipelineLauncher | Tab order through launcher controls | Press Tab repeatedly from launcher | Focus visits name field → template buttons → Create → flyout → browse, in DOM order, with visible focus ring | |
| TC-714 | P.1 | PipelineLauncher | Launcher renders when daemon offline | Stop daemon (`launchctl unload …`), reload tab | Friendly error or fallback launcher renders, no white screen | |

---

## P.2 — Pipeline Flyout

Component: `PipelineFlyout`. Opened via the toolbar flyout button or launcher "Open pipelines flyout" button.

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-715 | P.2 | PipelineFlyout | Open via launcher button | Click "Open pipelines flyout" from launcher | Flyout slides in from right, list of pipelines populates | |
| TC-716 | P.2 | PipelineFlyout | Open via toolbar | With a pipeline open, click the flyout-toggle button in PipelineComposerToolbar | Flyout opens overlaying canvas at z-index 11 | |
| TC-717 | P.2 | PipelineFlyout | Search box present | Snapshot flyout | Search input visible at top, focused or focusable | |
| TC-718 | P.2 | PipelineFlyout | Search filters list | Type a substring of a known pipeline name | List narrows to matching rows in real time | |
| TC-719 | P.2 | PipelineFlyout | Search clears | Clear search input | Full list restored | |
| TC-720 | P.2 | PipelineFlyout | Refresh button | Click refresh icon | List re-fetches via MCP `pipeline_List`, spinner appears briefly | |
| TC-721 | P.2 | PipelineFlyout | Sort control (if present) | Click sort dropdown | Options (Name, Updated, Created) appear; selecting one reorders list | |
| TC-722 | P.2 | PipelineFlyout | Per-row metadata | Inspect a row | Row shows name, last-modified timestamp, optional description | |
| TC-723 | P.2 | PipelineFlyout | Per-row delete button | Hover row | Delete (trash) icon appears on right edge of row | |
| TC-724 | P.2 | PipelineFlyout | Delete button triggers confirm | Click delete icon | Confirmation dialog or inline confirm appears | |
| TC-725 | P.2 | PipelineFlyout | Click row to open pipeline (desired) | Click anywhere on a pipeline row body | Pipeline opens in composer, flyout closes — **KNOWN BROKEN: currently a no-op** | |
| TC-726 | P.2 | PipelineFlyout | Keyboard navigate rows | Tab into list, press Down arrow | Focus moves to next row, visible focus indicator | |
| TC-727 | P.2 | PipelineFlyout | Enter on focused row opens pipeline | Press Enter on focused row | Same as TC-725; same known-broken behavior | |
| TC-728 | P.2 | PipelineFlyout | Close on outside click | Click on canvas outside flyout panel | Flyout closes | |
| TC-729 | P.2 | PipelineFlyout | Close on Escape | Press Escape with flyout focus | Flyout closes, focus returns to toggle button | |
| TC-730 | P.2 | PipelineFlyout | Empty state | With zero pipelines, open flyout | Empty-state message rendered, no error | |
| TC-731 | P.2 | PipelineFlyout | Error state | Stop daemon, click refresh | Inline error banner, retry button visible | |

---

## P.3 — Create New Pipeline (Blank)

Component: launcher "Create new pipeline" path → composer initial state.

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-732 | P.3 | PipelineLauncher | Click Create with empty name | Click "Create new pipeline" with name field blank | Button disabled, OR validation message "Name required" shown | |
| TC-733 | P.3 | PipelineLauncher | Click Create with name "uat-blank" | Type "uat-blank", click Create | Composer mounts, empty canvas with no nodes, pipeline title shows "uat-blank" | |
| TC-734 | P.3 | PipelineLauncher | Description field present | Inspect create dialog | Optional description textarea visible beneath name | |
| TC-735 | P.3 | PipelineLauncher | Description accepted | Enter 100-char description, create | Composer opens, description retrievable via inspector / metadata | |
| TC-736 | P.3 | PipelineComposer | Initial canvas state | Snapshot canvas | Empty grid background, no nodes, toolbar visible, no console errors | |
| TC-737 | P.3 | PipelineComposer | New pipeline persisted | Refresh app, open flyout | "uat-blank" appears in flyout list | |
| TC-738 | P.3 | PipelineLauncher | Duplicate-name handling | Try to create another "uat-blank" | Backend rejects (or suffixes); UI surfaces the result clearly | |

---

## P.4 — Create New Pipeline (Each Template)

Templates injected by the launcher: Research & Verify, Think & Critique, Draft & Refine, Explore & Synthesize. Each sub-section creates a new pipeline from that template with topic "Climate adaptation in coastal cities".

### P.4.1 Research & Verify

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-739 | P.4.1 | PipelineLauncher | Click Research & Verify | Click template button | Template detail panel appears with Topic input and "Create pipeline" CTA | |
| TC-740 | P.4.1 | PipelineLauncher | Topic required | Click Create with topic blank | Create button disabled or validation error shown | |
| TC-741 | P.4.1 | PipelineLauncher | Create with topic | Topic = "Climate adaptation in coastal cities", click Create | Composer mounts with template nodes pre-populated | |
| TC-742 | P.4.1 | PipelineComposer | Node count | Inspect canvas | At least 3 nodes rendered (research → verify → synthesize, exact count per template definition) | |
| TC-743 | P.4.1 | PipelineComposer | Edge count | Inspect canvas | Nodes connected in expected DAG, no orphans | |
| TC-744 | P.4.1 | PipelineComposer | Topic substituted into prompts | Double-click any prompt node, open inspector | Topic string appears in prompt body where template references `{{topic}}` | |

### P.4.2 Think & Critique

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-745 | P.4.2 | PipelineLauncher | Click Think & Critique | Click template button | Template detail panel appears | |
| TC-746 | P.4.2 | PipelineLauncher | Create with topic | Topic filled, click Create | Composer mounts | |
| TC-747 | P.4.2 | PipelineComposer | Sequential-thinking node present | Inspect nodes | At least one node of type `SequentialThinking` rendered | |
| TC-748 | P.4.2 | PipelineComposer | Critique node present | Inspect nodes | At least one prompt-type "critique" node rendered, wired downstream of thinking | |

### P.4.3 Draft & Refine

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-749 | P.4.3 | PipelineLauncher | Click Draft & Refine | Click template button | Template detail panel appears | |
| TC-750 | P.4.3 | PipelineLauncher | Create with topic | Topic filled, click Create | Composer mounts | |
| TC-751 | P.4.3 | PipelineComposer | Draft + refine pair | Inspect nodes | Two prompt nodes labeled "Draft" and "Refine" connected via Success edge | |
| TC-752 | P.4.3 | PipelineComposer | Field mapping on edge | Double-click Draft→Refine edge | Edge dialog shows mapping passing draft output into refine input | |

### P.4.4 Explore & Synthesize

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-753 | P.4.4 | PipelineLauncher | Click Explore & Synthesize | Click template button | Template detail panel appears | |
| TC-754 | P.4.4 | PipelineLauncher | Create with topic | Topic filled, click Create | Composer mounts | |
| TC-755 | P.4.4 | PipelineComposer | Fan-out pattern | Inspect canvas | One explore node fans out to multiple synthesis nodes, then converges | |
| TC-756 | P.4.4 | PipelineComposer | Layout sensible | Snapshot canvas | Nodes are laid out without overlap; default direction LR | |

---

## P.5 — Toolbar (PipelineComposerToolbar)

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-757 | P.5 | Toolbar | Layout LR button | Click "LR" or left-right icon | All nodes re-laid out left-to-right, viewport refits | |
| TC-758 | P.5 | Toolbar | Layout TB button | Click "TB" or top-bottom icon | All nodes re-laid out top-to-bottom, viewport refits | |
| TC-759 | P.5 | Toolbar | Layout preserves edges | Switch LR → TB → LR | All edges remain connected, no duplicates | |
| TC-760 | P.5 | Toolbar | Fit View button | Pan/zoom away, click Fit View | Viewport zooms/pans so all nodes are visible with padding | |
| TC-761 | P.5 | Toolbar | Palette toggle: open | Click palette button when closed | Sidebar with AddNodeForm opens, button shows active state | |
| TC-762 | P.5 | Toolbar | Palette toggle: close | Click palette button when open | Sidebar closes, canvas reclaims width | |
| TC-763 | P.5 | Toolbar | Save button: disabled when clean | No edits made | Save button disabled OR shows "Saved" non-interactive state | |
| TC-764 | P.5 | Toolbar | Save button: enabled when dirty | Add a node | Save button becomes enabled, badge changes to "Unsaved" | |
| TC-765 | P.5 | Toolbar | Save button: click saves | Click Save | MCP `pipeline_Save` invoked, badge becomes "Saved", button returns to disabled | |
| TC-766 | P.5 | Toolbar | Toggle pipeline design mode | Click design-mode toggle | Mode switches between design and runtime; canvas chrome updates (drag enabled/disabled, context menu items change) | |
| TC-767 | P.5 | Toolbar | Toggle runner | Click runner-toggle | Runner panel opens beneath/beside canvas | |
| TC-768 | P.5 | Toolbar | Toolbar buttons reachable by keyboard | Tab through toolbar | Focus visits every button in left-to-right order with visible focus ring | |
| TC-769 | P.5 | Toolbar | Toolbar tooltips | Hover each button | Tooltip appears within 500ms with button purpose | |

---

## P.6 — Add Node Form (AddNodeForm in Palette)

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-770 | P.6 | AddNodeForm | Type selector renders 4 options | Open palette, click type dropdown | Options: Workflow, Prompt, SequentialThinking, Tool | |
| TC-771 | P.6 | AddNodeForm | Switch to Workflow | Select Workflow | Fields: Label, Workflow ID (combobox), ACP client (combobox), Model, Thought Level | |
| TC-772 | P.6 | AddNodeForm | Workflow ID lists workflows | Open Workflow ID dropdown | Options populated from MCP `workflow_List` | |
| TC-773 | P.6 | AddNodeForm | ACP client lists agents | Open ACP client dropdown | Options populated from `maenifold.json` agents block | |
| TC-774 | P.6 | AddNodeForm | Switch to Prompt | Select Prompt | Fields: Label, Prompt (textarea), Role (dropdown), Color, Perspective, Requires role (checkbox), Working directory, Model, Thought Level | |
| TC-775 | P.6 | AddNodeForm | Prompt textarea accepts multiline | Type 5 lines with Shift+Enter | Newlines preserved, scroll appears if needed | |
| TC-776 | P.6 | AddNodeForm | Role dropdown populated | Open Role dropdown | Lists roles from `assets/roles/`, plus "(none)" | |
| TC-777 | P.6 | AddNodeForm | Color dropdown populated | Open Color dropdown | Lists colors from `assets/colors/` with swatch previews | |
| TC-778 | P.6 | AddNodeForm | Perspective dropdown populated | Open Perspective dropdown | Lists perspectives from `assets/perspectives/` | |
| TC-779 | P.6 | AddNodeForm | Working-directory accepts absolute path | Type "/tmp/uat" | Field accepts string, no validation error | |
| TC-780 | P.6 | AddNodeForm | Switch to SequentialThinking | Select SequentialThinking | Fields: Label, Prompt, Thought Level (low/medium/high), Model, ACP client | |
| TC-781 | P.6 | AddNodeForm | Thought Level options | Open Thought Level dropdown | Exactly: low, medium, high | |
| TC-782 | P.6 | AddNodeForm | Switch to Tool | Select Tool | Fields: Label, Tool ID, Allowed Tools (multi), Max Turns (number), Model | |
| TC-783 | P.6 | AddNodeForm | Max Turns accepts integer | Type 12 | Field accepts integer; rejects negative / non-numeric | |
| TC-784 | P.6 | AddNodeForm | Add Node disabled until required filled | Leave Label blank | Add Node button disabled | |
| TC-785 | P.6 | AddNodeForm | Add Node enabled when valid | Fill required fields | Add Node button enabled | |
| TC-786 | P.6 | AddNodeForm | Submit creates node on canvas | Click Add Node | New node appears on canvas, becomes selected, dirty state set | |
| TC-787 | P.6 | AddNodeForm | Form resets after add | After successful add | Form clears (or stays for batch add per UX intent — record actual behavior) | |
| TC-788 | P.6 | AddNodeForm | Cancel/close palette discards form | Edit form, close palette | No node added, no dirty state change | |
| TC-789 | P.6 | AddNodeForm | Adding a node when daemon offline | Stop daemon, click Add Node | Inline error or toast; no silent failure | |

---

## P.7 — Canvas: Panning, Zooming, Fit View

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-790 | P.7 | PipelineComposer | Pan with drag on background | Click empty canvas and drag | Viewport pans following cursor; nodes do not move | |
| TC-791 | P.7 | PipelineComposer | Pan with two-finger trackpad | Two-finger drag on background | Viewport pans | |
| TC-792 | P.7 | PipelineComposer | Zoom in with scroll | Scroll up on canvas | Zoom level increases, centered on cursor | |
| TC-793 | P.7 | PipelineComposer | Zoom out with scroll | Scroll down on canvas | Zoom level decreases | |
| TC-794 | P.7 | PipelineComposer | Zoom min bound | Scroll down repeatedly | Zoom clamps at a minimum (e.g. 0.2), does not invert | |
| TC-795 | P.7 | PipelineComposer | Zoom max bound | Scroll up repeatedly | Zoom clamps at a maximum (e.g. 2.0) | |
| TC-796 | P.7 | PipelineComposer | Fit View recenters | Pan far off-screen, click Fit View | Viewport returns to encompass all nodes | |
| TC-797 | P.7 | PipelineComposer | Double-click background | Double-click empty canvas | No-op OR opens add-node menu (record actual); should NOT crash | |

---

## P.8 — Canvas: Nodes

Use a template pipeline with at least 3 nodes including one Prompt, one Workflow, one SequentialThinking.

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-798 | P.8 | PipelineNodeCard | Single-click selects node | Click on node body | Node gains selection ring, selection store updates | |
| TC-799 | P.8 | PipelineNodeCard | Single-click shows inspector (desired) | Click on node body | Inspector panel renders node details — **KNOWN BROKEN: currently only "Delete Selected" shown** | |
| TC-800 | P.8 | PipelineNodeCard | Shift+click multi-select | Click node A, Shift+click node B | Both selected, selection ring on both | |
| TC-801 | P.8 | PipelineNodeCard | Box select | Drag-select a rectangle covering 2 nodes | Both inside rectangle become selected on release | |
| TC-802 | P.8 | PipelineNodeCard | Drag node in design mode | Enter design mode, drag a node | Node follows cursor, position persists in state | |
| TC-803 | P.8 | PipelineNodeCard | Drag node in runtime mode | Switch to runtime mode, try to drag | Node does not move; cursor indicates not-draggable | |
| TC-804 | P.8 | PipelineNodeInspector | Double-click opens config dialog | Double-click any node | Modal dialog "Node configuration" opens | |
| TC-805 | P.8 | PipelineNodeInspector | Dialog: Label field | Inspect dialog | Label input present, pre-filled, editable | |
| TC-806 | P.8 | PipelineNodeInspector | Dialog: ID field | Inspect dialog | ID field present, read-only or with explicit edit affordance | |
| TC-807 | P.8 | PipelineNodeInspector | Dialog (Workflow): Workflow ID | Open Workflow node | Workflow ID combobox present, lists workflows | |
| TC-808 | P.8 | PipelineNodeInspector | Dialog (Workflow): ACP client | Open Workflow node | ACP client combobox present, lists configured agents | |
| TC-809 | P.8 | PipelineNodeInspector | Dialog (Prompt): Prompt body | Open Prompt node | Prompt textarea present, pre-filled, multi-line | |
| TC-810 | P.8 | PipelineNodeInspector | Dialog (Prompt): Role | Open Prompt node | Role dropdown, current value pre-selected | |
| TC-811 | P.8 | PipelineNodeInspector | Dialog (Prompt): Color | Open Prompt node | Color picker / dropdown present | |
| TC-812 | P.8 | PipelineNodeInspector | Dialog (Prompt): Perspective | Open Prompt node | Perspective dropdown present | |
| TC-813 | P.8 | PipelineNodeInspector | Dialog (Prompt): Requires role checkbox | Open Prompt node | Checkbox renders, toggling persists | |
| TC-814 | P.8 | PipelineNodeInspector | Dialog (Prompt): Working directory | Open Prompt node | Text input present, accepts path | |
| TC-815 | P.8 | PipelineNodeInspector | Dialog: Model | Open any inferencing node | Model combobox lists configured models from `assets/models/` | |
| TC-816 | P.8 | PipelineNodeInspector | Dialog: Thought Level | Open inferencing node | Thought Level dropdown shows low/medium/high | |
| TC-817 | P.8 | PipelineNodeInspector | Dialog (Tool): Allowed Tools | Open Tool node | Multi-select of allowed tool IDs visible | |
| TC-818 | P.8 | PipelineNodeInspector | Dialog (Tool): Max Turns | Open Tool node | Numeric input, integer only | |
| TC-819 | P.8 | PipelineNodeInspector | Edit field and Save | Change Label to "renamed", click Save | Dialog closes, node card shows new label, dirty state set | |
| TC-820 | P.8 | PipelineNodeInspector | Edit and Cancel | Change a field, click Cancel | Dialog closes, node card unchanged, dirty state unchanged | |
| TC-821 | P.8 | PipelineNodeInspector | Escape closes dialog without save | Press Escape | Dialog closes; behaves as Cancel | |
| TC-822 | P.8 | PipelineNodeInspector | Required-field validation | Clear Label, click Save | Inline error "Label required", dialog stays open | |
| TC-823 | P.8 | PipelineNodeCardPorts | Source handle visible in LR | Inspect node in LR layout | Right-edge handle visible, hover-targetable | |
| TC-824 | P.8 | PipelineNodeCardPorts | Target handle visible in LR | Inspect node in LR layout | Left-edge handle visible | |
| TC-825 | P.8 | PipelineNodeCard | Selected node + Delete key | Select node, press Backspace/Delete | Node removed from canvas (design mode), dirty state set | |
| TC-826 | P.8 | PipelineNodeCard | Multi-select + Delete | Select 2 nodes, press Delete | Both removed | |
| TC-827 | P.8 | PipelineNodeCard | Click on canvas background | Click empty area | Selection cleared | |

---

## P.9 — Canvas: Edges

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-828 | P.9 | PipelineNodeCardPorts | Source/target handles visible LR | LR layout active | Each node shows handles on left (target) and right (source), at least 4px hit area | |
| TC-829 | P.9 | PipelineNodeCardPorts | Source/target handles visible TB | Switch to TB layout | Handles re-render on top (target) and bottom (source) | |
| TC-830 | P.9 | PipelineComposer | Drag handle-to-handle creates edge (desired) | Drag from source handle of A onto target handle of B | New edge appears, dirty state set — **KNOWN BROKEN: handles not interactive** | |
| TC-831 | P.9 | PipelineComposer | Edge click selects | Click on an existing edge path | Edge gains highlight stroke | |
| TC-832 | P.9 | PipelineComposer | Edge selection ring | After click | Visual selected-state distinct from hover | |
| TC-833 | P.9 | PipelineComposer | Double-click edge opens dialog | Double-click an edge | "Edge configuration" dialog opens | |
| TC-834 | P.9 | EdgeDialog | Edge type selector | Inspect dialog | Dropdown with options: Success, Always, Conditional | |
| TC-835 | P.9 | EdgeDialog | Switching to Always | Select Always | Conditional fields hidden, no condition required | |
| TC-836 | P.9 | EdgeDialog | Switching to Conditional | Select Conditional | Path and Equals input fields appear | |
| TC-837 | P.9 | EdgeDialog | Conditional path validation | Type "result.status", Equals "ok" | Fields accept, save persists | |
| TC-838 | P.9 | EdgeDialog | Field mapping: Add mapping | Click "Add mapping" | New empty mapping row appears | |
| TC-839 | P.9 | EdgeDialog | Field mapping: edit source path | Type "output.summary" in source path | Field accepts dotted path | |
| TC-840 | P.9 | EdgeDialog | Field mapping: edit target path | Type "input.context" in target path | Field accepts | |
| TC-841 | P.9 | EdgeDialog | Field mapping: delete | Click trash icon on mapping row | Row removed | |
| TC-842 | P.9 | EdgeDialog | Save persists changes | Click Save | Dialog closes, edge label / style reflects type | |
| TC-843 | P.9 | EdgeDialog | Cancel discards changes | Edit, click Cancel | No change | |
| TC-844 | P.9 | PipelineComposer | Edge delete via keyboard | Select edge, press Backspace | Edge removed, dirty state set | |

---

## P.10 — Context Menu (Design Mode)

Component: `PipelineComposerContextMenu*`. Enter design mode first.

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-845 | P.10 | ContextMenu | Right-click on canvas opens menu | Right-click empty canvas | Menu opens at cursor with z-index 60 | |
| TC-846 | P.10 | ContextMenu | Menu: Add prompt node | Click "Add prompt node" | New Prompt node added at right-click coordinates | |
| TC-847 | P.10 | ContextMenu | Menu: Add thinking node | Click "Add thinking node" | New SequentialThinking node added | |
| TC-848 | P.10 | ContextMenu | Menu: Add workflow node | Click "Add workflow node" | New Workflow node added | |
| TC-849 | P.10 | ContextMenu | Menu: Add tool node | Click "Add tool node" | New Tool node added | |
| TC-850 | P.10 | ContextMenu | Right-click on node opens node menu | Right-click any node | Node-scoped menu opens | |
| TC-851 | P.10 | ContextMenu | Node menu: Edit | Click Edit | Node config dialog opens (same as double-click) | |
| TC-852 | P.10 | ContextMenu | Node menu: Duplicate | Click Duplicate | Cloned node appears nearby with new ID | |
| TC-853 | P.10 | ContextMenu | Node menu: Delete | Click Delete | Node removed; if has edges, they are removed too | |
| TC-854 | P.10 | ContextMenu | Node menu: Disconnect edges | Click Disconnect edges | All incoming and outgoing edges removed; node remains | |
| TC-855 | P.10 | ContextMenu | Node menu: Convert to template (if present) | Click option | Template-creation flow starts (or option absent — record) | |
| TC-856 | P.10 | ContextMenu | Right-click on edge opens edge menu | Right-click an edge | Edge-scoped menu opens | |
| TC-857 | P.10 | ContextMenu | Edge menu: Edit | Click Edit | Edge dialog opens | |
| TC-858 | P.10 | ContextMenu | Edge menu: Delete | Click Delete | Edge removed | |
| TC-859 | P.10 | ContextMenu | Edge menu: Reverse direction (if present) | Click Reverse | Source/target swap; if absent, record as missing | |
| TC-860 | P.10 | ContextMenu | Escape closes menu | Right-click, press Escape | Menu closes, no action taken | |
| TC-861 | P.10 | ContextMenu | Click outside closes menu | Right-click, click on canvas | Menu closes, no action taken | |
| TC-862 | P.10 | ContextMenu | Keyboard navigation | Right-click, Arrow Down | Focus moves through items; Enter activates | |

---

## P.11 — Context Menu (Server / Runtime Mode)

Run a pipeline first (P.13). For each TC, also assert that the corresponding MCP call appears in the Logs tab or daemon log.

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-863 | P.11 | ContextMenu | Right-click node during run | Right-click an active node | Runtime menu opens with run-scoped actions | |
| TC-864 | P.11 | ContextMenu | Restart node | Click Restart | MCP `pipeline_RestartNode` invoked; node returns to "active" state | |
| TC-865 | P.11 | ContextMenu | Rewind node | Click Rewind | MCP `pipeline_RewindNode` invoked; node returns to "loaded" state | |
| TC-866 | P.11 | ContextMenu | Fork from here | Click Fork | MCP `pipeline_Fork` invoked; new run row appears in history | |
| TC-867 | P.11 | ContextMenu | Abandon | Click Abandon | MCP `pipeline_AbandonNode` invoked; node transitions to "abandoned" | |
| TC-868 | P.11 | ContextMenu | Run pipeline (from canvas) | Right-click canvas, click Run | MCP `pipeline_Run` invoked; run begins | |
| TC-869 | P.11 | ContextMenu | Rewind all | Right-click canvas, click Rewind all | MCP `pipeline_RewindAll` invoked; all nodes return to "loaded" | |
| TC-870 | P.11 | ContextMenu | Graft | Click Graft | Graft dialog or inline action; MCP `pipeline_Graft` invoked | |
| TC-871 | P.11 | ContextMenu | Promote to memory | Click Promote to memory | MCP `memory_Write` invoked with node output; toast confirms | |
| TC-872 | P.11 | ContextMenu | Restart on completed node | Right-click completed node | Restart available; on click, node re-runs and downstream re-runs | |
| TC-873 | P.11 | ContextMenu | Rewind on failed node | Right-click failed node | Rewind available, restores to loaded state, error cleared | |
| TC-874 | P.11 | ContextMenu | Fork preserves upstream state | Fork from mid-pipeline | New run shares snapshot of upstream node outputs | |
| TC-875 | P.11 | ContextMenu | Abandon ends pipeline if terminal | Abandon last live node | Pipeline status becomes "abandoned" | |
| TC-876 | P.11 | ContextMenu | Run while already running | Right-click canvas, click Run during active run | Either disabled, or queues / rejects with message — no double-run | |
| TC-877 | P.11 | ContextMenu | Rewind all confirmation | Click Rewind all | Confirmation dialog (destructive) appears, OK / Cancel | |
| TC-878 | P.11 | ContextMenu | Promote captures node metadata | Promote a node | Memory entry contains node ID, pipeline ID, timestamp, output content | |

---

## P.12 — Save Flow + Dirty State

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-879 | P.12 | Toolbar | Save indicator on fresh open | Open pipeline | Badge shows "Saved" or empty (no dirty marker) | |
| TC-880 | P.12 | Toolbar | Dirty after node add | Add a node | Badge changes to "Unsaved" or "•", Save button enables | |
| TC-881 | P.12 | Toolbar | Dirty after node edit | Edit node label | Badge becomes Unsaved | |
| TC-882 | P.12 | Toolbar | Dirty after edge add | Add an edge | Badge becomes Unsaved | |
| TC-883 | P.12 | Toolbar | Save via button | Click Save | Badge transitions Saving → Saved within 2s | |
| TC-884 | P.12 | Toolbar | Save error surfaces | Stop daemon, click Save | Badge becomes "Error", inline error message shown | |
| TC-885 | P.12 | Toolbar | Save while running | Run pipeline, edit, Save | Backend response respected: either Save succeeds (if allowed) or returns error displayed to user — no silent drop | |
| TC-886 | P.12 | PipelineComposer | Navigate away with unsaved changes | Edit, click Knowledge tab | Either prompt user OR auto-save; no silent data loss | |
| TC-887 | P.12 | PipelineComposer | Reload after crash | Edit, kill app, relaunch | Either resumes draft OR opens last-saved state; document actual behavior | |

---

## P.13 — Run Pipeline (PipelineRunner)

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-888 | P.13 | PipelineRunner | Run button location | Inspect runner panel | Prominent "Run" button at top of runner | |
| TC-889 | P.13 | PipelineRunner | Optional prompt input | Inspect runner | Textarea labeled "Prompt" (optional) renders | |
| TC-890 | P.13 | PipelineRunner | Optional inputs key/value | Inspect runner | "Add input" control adds key/value rows | |
| TC-891 | P.13 | PipelineRunner | Run with no inputs | Click Run with empty fields | Pipeline starts, MCP `pipeline_Run` invoked | |
| TC-892 | P.13 | PipelineRunner | Run with prompt | Type "Summarize", click Run | Prompt passed to first node; visible in run-history | |
| TC-893 | P.13 | PipelineRunner | Run with key/value inputs | Add input topic=climate, click Run | Inputs reach the pipeline via run params | |
| TC-894 | P.13 | PipelineRunner | Node state: loaded | At run start | All non-active nodes display "loaded" visual state | |
| TC-895 | P.13 | PipelineRunner | Node state: active | During run | Currently-running node shows "active" (spinner / pulse) | |
| TC-896 | P.13 | PipelineRunner | Node state: completed | After successful node | Completed nodes show "completed" (checkmark / green ring) | |
| TC-897 | P.13 | PipelineRunner | Final status: completed | Successful run | Pipeline status header shows "Completed" with elapsed time | |

---

## P.14 — Run History Panel (PipelineRunHistoryPanel)

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-898 | P.14 | RunHistory | Event rendered: loaded | Inspect history during run | "loaded" event row appears for each node | |
| TC-899 | P.14 | RunHistory | Event rendered: active | Inspect | "active" event row appears | |
| TC-900 | P.14 | RunHistory | Event rendered: completed | Inspect | "completed" event row appears | |
| TC-901 | P.14 | RunHistory | Event rendered: failed | Force a failure | "failed" event row with error message | |
| TC-902 | P.14 | RunHistory | Event rendered: abandoned | Abandon a node | "abandoned" event row appears | |
| TC-903 | P.14 | RunHistory | Event rendered: stuck | Cause stuck (deadlock test pipeline) | "stuck" event row appears | |
| TC-904 | P.14 | RunHistory | Event rendered: cancelled | Cancel run mid-flight | "cancelled" event row appears | |
| TC-905 | P.14 | RunHistory | Hover event highlights node (desired) | Hover event row | Corresponding canvas node gains highlight | |
| TC-906 | P.14 | RunHistory | Sort: Newest first | Select sort=Newest | Events ordered descending by timestamp | |
| TC-907 | P.14 | RunHistory | Sort: Oldest first | Select sort=Oldest | Events ordered ascending | |
| TC-908 | P.14 | RunHistory | Sort: By node | Select sort=By node | Events grouped by node ID | |
| TC-909 | P.14 | RunHistory | Refresh button | Click refresh | History re-fetched | |
| TC-910 | P.14 | RunHistory | Export transcript (if present) | Click Export | Transcript downloaded as JSON/Markdown (or option missing — record) | |
| TC-911 | P.14 | RunHistory | Copy event details | Click copy on a row | Event JSON copied to clipboard 📋 | |

---

## P.15 — Cancel / Rewind / Restart / Fork / Graft / Promote

### P.15.1 Cancel

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-912 | P.15.1 | Runner | Cancel button visible during run | Start run | Cancel button replaces or appears next to Run | |
| TC-913 | P.15.1 | Runner | Cancel triggers MCP cancel | Click Cancel | MCP `pipeline_Cancel` invoked; pipeline transitions to cancelled within 5s | |
| TC-914 | P.15.1 | Runner | UI feedback post-cancel | After cancel | Status badge shows "Cancelled"; node states freeze | |

### P.15.2 Rewind

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-915 | P.15.2 | ContextMenu | Rewind node via right-click | Right-click completed node → Rewind | MCP `pipeline_RewindNode` invoked | |
| TC-916 | P.15.2 | RunHistory | Rewind reflected in history | After rewind | New "loaded" event appears for that node | |

### P.15.3 Restart

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-917 | P.15.3 | ContextMenu | Restart node via right-click | Right-click node → Restart | MCP `pipeline_RestartNode` invoked; node re-executes | |
| TC-918 | P.15.3 | Canvas | Downstream nodes invalidated | After restart | Downstream completed nodes return to loaded | |

### P.15.4 Fork

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-919 | P.15.4 | ContextMenu | Fork from node via right-click | Right-click → Fork from here | MCP `pipeline_Fork` invoked; new run ID returned | |
| TC-920 | P.15.4 | RunHistory | Fork creates new run row | After fork | New run appears in history with link to parent | |

### P.15.5 Graft

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-921 | P.15.5 | ContextMenu | Graft via right-click | Right-click → Graft | Graft dialog opens, target run selectable | |
| TC-922 | P.15.5 | Canvas | Graft applies state | After graft | Selected node state replaced from target run | |

### P.15.6 Promote to Memory

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-923 | P.15.6 | ContextMenu | Promote node output | Right-click completed node → Promote to memory | MCP `memory_Write` invoked; toast confirms write | |

---

## P.16 — Delete Pipeline

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-924 | P.16 | PipelineFlyout | Delete button per row | Hover row | Trash icon appears | |
| TC-925 | P.16 | PipelineFlyout | Delete confirmation | Click trash | Confirm dialog with pipeline name and irreversible-warning | |
| TC-926 | P.16 | PipelineFlyout | Confirm removes from list | Click Delete in confirm | Row disappears, list refreshes | |
| TC-927 | P.16 | PipelineFlyout | 30-day quarantine | Inspect filesystem `assets/pipelines/.trash/` | Deleted pipeline file moved here with timestamp | |

---

## P.17 — Cross-Feature

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-928 | P.17 | PipelineComposer | Open A → open B | Open pipeline A; from flyout open pipeline B | A unloads cleanly, B mounts; no stale nodes from A | |
| TC-929 | P.17 | PipelineComposer | Tab switch round-trip | Open pipeline, switch to Knowledge, return | Same pipeline still open, same selection, same zoom/pan | |
| TC-930 | P.17 | PipelineComposer | Reload app while running | Start run, quit app, relaunch | Pipeline reconnects to in-flight run OR shows clear "run lost" message | |
| TC-931 | P.17 | PipelineComposer | Daemon down on open | Stop daemon, open pipeline | Graceful error banner; no white screen; retry available | |
| TC-932 | P.17 | PipelineComposer | Concurrent edits in two windows | Open same pipeline in 2 windows, edit in each, save in both | Last-write-wins OR conflict surfaced; no silent corruption | |

---

## P.18 — Accessibility

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-933 | P.18 | Toolbar | Tab order through toolbar | Tab from address area | Focus visits each toolbar button in left-to-right order | |
| TC-934 | P.18 | Canvas | ARIA roles on nodes | Inspect DOM | Each node has role="button" or role="group" with aria-label | |
| TC-935 | P.18 | Canvas | ARIA roles on edges | Inspect DOM | Each edge SVG has aria-label or aria-describedby | |
| TC-936 | P.18 | Canvas | Screen reader announces selection | Select node with VoiceOver on | Node label and type announced | |
| TC-937 | P.18 | Canvas | Backspace deletes selected | Select, press Backspace | Selection deleted (design mode) | |
| TC-938 | P.18 | NodeDialog | Escape closes dialog | Open node dialog, press Escape | Dialog closes; focus returns to node | |
| TC-939 | P.18 | NodeDialog | Focus trap | Open dialog, Tab through | Focus stays within dialog, wraps from last to first | |
| TC-940 | P.18 | Toolbar | High-contrast pass | Enable OS high-contrast | All toolbar controls remain legible, no missing borders | |

---

## P.19 — Error States

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-941 | P.19 | PipelineFlyout | Daemon down: list fails | Stop daemon, open flyout | Graceful "Could not load pipelines" message + Retry | |
| TC-942 | P.19 | PipelineComposer | Invalid pipeline validation | Build pipeline with disconnected required input, click Run | Validation error displayed inline before MCP call | |
| TC-943 | P.19 | PipelineRunner | Missing inference config | Remove model entry from `maenifold.json`, run | Error message linking to settings/setup; no silent retry loop | |
| TC-944 | P.19 | PipelineRunner | MCP timeout | Force a 30s-stalled tool call | Error toast appears with Retry button; pipeline remains in known state | |
| TC-945 | P.19 | NodeDialog | Save with malformed JSON in mapping | Enter invalid JSON in field-mapping value | Inline validation error; Save remains disabled | |
| TC-946 | P.19 | PipelineComposer | Schema mismatch on load | Open a pipeline file with deprecated schema field | Migration runs OR clear error with the bad field cited | |

---

## P.20 — Logs Tab Integration

| TC ID | Section | Component | Step | How | Expected | Pass/Fail |
|-------|---------|-----------|------|-----|----------|-----------|
| TC-947 | P.20 | LogsView | Inferencing event present | Run a pipeline with at least one Prompt node, switch to Logs tab | Inferencing event rows appear, one per LLM call | |
| TC-948 | P.20 | LogsView | Event: decision='inferencing' | Inspect event details | `decision` field equals `"inferencing"` | |
| TC-949 | P.20 | LogsView | Event: model field | Inspect event | `model` field present and matches the node's configured model | |
| TC-950 | P.20 | LogsView | Event: request payload visible | Expand event | Request body / messages visible | |
| TC-951 | P.20 | LogsView | Event: response payload visible | Expand event | Response body visible, including token counts if provided | |

---

## Known Broken (Live UAT Findings)

The following items were observed during the UAT pre-flight against sprint T-328 and **must be filed as gate tasks** before this plan is run end-to-end. Reference these here when recording Pass/Fail for the corresponding desired-behavior TCs.

1. **PipelineFlyout row click is a no-op.** Clicking a pipeline row in the flyout does not open the pipeline. Only the explicit "Open" action (if present) works. Affects TC-725, TC-727.
2. **"Browse existing pipelines" launcher button is a no-op.** Clicking the button does nothing — neither flyout nor any list opens. Affects TC-712.
3. **Node connection handles are invisible / non-interactive.** Source and target handles render with insufficient hit area (and, in some layouts, no visible affordance), so the designer cannot wire connections by drag. Affects TC-823, TC-824, TC-828, TC-829, TC-830.
4. **Single-click on node shows only "Delete Selected" — no inspector panel.** Only double-click opens the node configuration dialog; there is no single-click inspector sidebar. Affects TC-799.

Each known-broken item should be filed under the active sprint with severity P1 and linked back to the relevant TC IDs above.
