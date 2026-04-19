# Full Design System Audit — 2026-04-19

## Summary count by category

| Category | Count |
|---|---|
| `raw-control` (button/select/input/label) | **234** distinct lines |
| `window-api` (window.confirm) | **3** |
| `hand-rolled-modal` (Teleport-based) | **2** components |
| `xss-sink` (v-html) | **10** (1 unsanitized) |
| `color-token` (rgba/hex/non-canonical alias) | **65+** |
| `glass-effect` (backdrop-filter) | **3** |
| `shadow` (non-Fluent / ambient glow) | **24** |
| `radius` (off-scale) | **40+** |
| `typography` (font-family / text-transform) | **20** + token-misuse |

The single highest-impact finding is **#236 `ChatThoughtSection.vue:30`** — `v-html="renderedThought"` consumes raw `marked.parse()` output **without DOMPurify**. Every other markdown sink in the app sanitizes; this one doesn't. Rendered chat content can include arbitrary attacker-supplied text, so this is a live XSS sink. **Critical.**

---

## Findings table

### A. `raw-control` — `<button>` not using shadcn `Button`

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 1 | `components/WorkflowStepInspector.vue` | 115, 133 | Bare `<button>` | Replace with `<Button variant="ghost"/>` |
| 2 | `components/ConfigOptionPicker.vue` | 77 | Bare `<button>` | shadcn `Button` |
| 3 | `components/ChatThoughtSection.vue` | 18 | Bare `<button>` (collapse toggle) | shadcn `Button variant="ghost" size="sm"` |
| 4 | `components/TrafficMonitor.vue` | 80, 88, 91, 102, 121, 135 | Bare `<button class="control-btn">`, `close-btn` | shadcn `Button`/`IconButton` |
| 5 | `components/AuthMethodDialog.vue` | 40, 46, 66 | Bare `<button class="close-btn">`, `cancel-btn` | shadcn `Button` (and entire dialog should use shadcn `Dialog`) |
| 6 | `components/SettingsMcpServersDetail.vue` | 76, 79 | Bare `<button class="save-btn">`, `cancel-btn` | shadcn `Button variant="default"` / `outline` |
| 7 | `components/ToastContainer.vue` | 34 | Bare `<button>` (dismiss) | shadcn `Button` icon variant |
| 8 | `components/SemanticZoomOverlayControls.vue` | 84, 93 | Bare `<button>` | shadcn `Button` |
| 9 | `components/StartupProgress.vue` | 107, 113 | Bare `<button>` | shadcn `Button` |
| 10 | `components/PipelineView.vue` | 242, 267, 332, 347 | Bare `<button>` (`banner-dismiss`, `mobile-panel-toggle`, `mobile-overlay`) | shadcn `Button` |
| 11 | `components/AgentProbePreviewStatus.vue` | 30 | Bare `<button class="refresh-btn">` | shadcn `Button` |
| 12 | `components/ThinkingTraceView.vue` | 155 | Bare `<button class="trace-toggle">` | shadcn `Button` |
| 13 | `components/AddNodeFormFields.vue` | 109 | Bare `<button>` | shadcn `Button` |
| 14 | `components/SessionList.vue` | 186 | Bare `<button>` | shadcn `Button` |
| 15 | `components/PromptAssemblyPreview.vue` | 78 | Bare `<button>` | shadcn `Button` |
| 16 | `components/WorkflowBrowserFilters.vue` | 49, 62, 76, 77, 78, 82 | Bare `<button class="feedback-btn">` (5×) | shadcn `Button variant="outline" size="sm"` |
| 17 | `components/WorkflowStepCard.vue` | 71, 72, 76, 87, 96, 110 | Bare `<button class="card-confirm-btn">`, `card-confirm-btn--danger`, etc | shadcn `Button` w/ `variant="destructive"` for danger |
| 18 | `components/SettingsConnectionKeys.vue` | 49, 51, 69, 102 | Bare `<button class="remove-btn">`, `add-inline-btn` | shadcn `Button` icon/ghost |
| 19 | `components/ToolNodeFieldsPrompt.vue` | 48 | Bare `<button>` | shadcn `Button` |
| 20 | `components/ThinkingSessionCard.vue` | 79 | Bare `<button class="card-header">` (full-card aria-expanded toggle) | shadcn `Button` or shadcn `Collapsible` primitive |
| 21 | `components/ChatAttachmentBar.vue` | 141, 146, 154 | Bare `<button class="ab-btn">`, `ab-remove` | shadcn `Button` |
| 22 | `components/SettingsView.vue` | 31 | Bare `<button>` (settings nav) | shadcn `Button variant="ghost"` |
| 23 | `components/WorkflowBrowser.vue` | 228 | Bare `<button class="btn-import">` | shadcn `Button` |
| 24 | `components/SettingsMaenifold.vue` | 204, 207 | Bare `<button class="save-btn">`, `reset-btn` | shadcn `Button` |
| 25 | `components/PlanView.vue` | 45 | Bare `<button>` | shadcn `Button` |
| 26 | `components/ToolNodeFieldsThinking.vue` | 77 | Bare `<button>` | shadcn `Button` |
| 27 | `components/PermissionEntry.vue` | 152, 160 | Bare `<button>`, `cancel-btn` (permission decision) | shadcn `Button` (likely `default` + `outline`) |
| 28 | `components/GraphViewCanvas.vue` | 17 | Bare `<button class="empty-retry">` | shadcn `Button` |
| 29 | `components/SemanticZoomOverlay.vue` | 280 | Bare `<button>` | shadcn `Button` |
| 30 | `components/WorkflowMetadataForm.vue` | 104, 119 | Bare `<button>` (toggle cards) | shadcn `Switch` or `Toggle` (these are toggle controls, not buttons) |
| 31 | `components/AppSidebar.vue` | 53, 62, 91, 101, 142 | Bare `<button>` (nav rails) | shadcn `Button variant="ghost"` |
| 32 | `components/ChatToolCallItem.vue` | 47 | Bare `<button>` | shadcn `Button` |
| 33 | `components/SemanticZoomOverlayHeader.vue` | 20 | Bare `<button class="zoom-close">` | shadcn `Button` icon |
| 34 | `components/TemplateGalleryCard.vue` | 33 | Bare `<button>` (entire card is a button) | shadcn `Button` w/ `asChild` or `Card` clickable pattern |
| 35 | `components/SettingsMcpServersList.vue` | 26, 40, 41 | Bare `<button class="add-btn">`, `edit-btn`, `delete-btn` | shadcn `Button` (delete = `variant="destructive"`) |
| 36 | `components/ChatMessageList.vue` | 37, 69 | Bare `<button>`, `cancel-btn` | shadcn `Button` |
| 37 | `components/PipelineSidebar.vue` | 90 | Bare `<button>` | shadcn `Button` |
| 38 | `components/TrafficEntry.vue` | 49 | Bare `<button class="copy-btn">` | shadcn `Button` icon |
| 39 | `components/SettingsConnectionAgents.vue` | 72, 100, 103, 123, 129 | Bare `<button class="add-btn">`, `save-btn`, `cancel-btn`, `delete-btn` | shadcn `Button` (delete = destructive) |
| 40 | `components/WorkflowDraftBar.vue` | 46, 49, 65, 68 | Bare `<button class="draft-bar-btn">`, `--ghost`, `--danger` | shadcn `Button variant="ghost"` / `destructive` |
| 41 | `components/HomeDashboard.vue` | 125, 161 | Bare `<button>` | shadcn `Button` |
| 42 | `components/WorkflowStepTimeline.vue` | 241, 242, 243, 247, 300 | Bare `<button class="feedback-btn">`, `add-step-btn` | shadcn `Button` |
| 43 | `components/SettingsPreferences.vue` | 149 | Bare `<button class="secondary-btn">` | shadcn `Button variant="secondary"` |
| 44 | `components/GraphViewControls.vue` | 19, 36, 37 | Bare `<button class="action-btn">` | shadcn `Button` |
| 45 | `components/PipelineIconRail.vue` | 39, 59 | Bare `<button>` | shadcn `Button` |
| 46 | `components/ToolPaletteFields.vue` | 140, 172, 188, 196 | Bare `<button class="mode-badge">`, `tool-palette__option` (listbox options) | shadcn `Toggle`/`ToggleGroup` for mode selection; listbox options should use shadcn `Command` items |
| 47 | `components/WorkflowTreeErrorState.vue` | 19 | Bare `<button class="retry-btn">` | shadcn `Button` |
| 48 | `components/GateTaskTable.vue` | 55, 61, 67, 73 | Bare `<button class="gate-th-button">` (sort headers) | shadcn `Button variant="ghost"` |
| 49 | `components/ChatInputBar.vue` | 98, 112 | Bare `<button class="attachment-btn">`, `send-btn` | shadcn `Button` |
| 50 | `components/SettingsConnectionDaemon.vue` | 67, 74 | Bare `<button class="refresh-btn">`, `copy-btn` | shadcn `Button` |
| 51 | `components/PipelineActionBar.vue` (delete confirm flow) | (whole flow) | Uses `window.confirm` instead of shadcn `AlertDialog` for destructive action — see #226 | shadcn `AlertDialog` |

### B. `raw-control` — `<select>` not using shadcn `Select`

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 52 | `components/GateViewToolbar.vue` | 62 | Bare `<select class="gate-root-select">` | shadcn `Select` |
| 53 | `components/GraftNodesDialog.vue` | 76, 94, 107, 120 | Bare `<select>` (4×) | shadcn `Select` |
| 54 | `components/SettingsMcpServersDetail.vue` | 49 | Bare `<select>` | shadcn `Select` |
| 55 | `components/AddNodeFormFields.vue` | 75, 124, 137, 158 | Bare `<select>` (4×) | shadcn `Select` |
| 56 | `components/DesignNodeConfigPanel.vue` | 645 | Bare `<select>` | shadcn `Select` |
| 57 | `components/SettingsConnectionKeys.vue` | 77 | Bare `<select v-model="server.type">` | shadcn `Select` |
| 58 | `components/ToolNodeFieldsPrompt.vue` | 36, 63, 69, 80 | Bare `<select>` (4×) | shadcn `Select` |
| 59 | `components/SettingsMaenifoldGraph.vue` | 34 | Bare `<select v-model="form.decay.function">` | shadcn `Select` |
| 60 | `components/SettingsMaenifoldMemory.vue` | 21, 108 | Bare `<select>` | shadcn `Select` |
| 61 | `components/GateViewFilters.vue` | 59, 64, 69 | Bare `<select>` (3 filter dropdowns) | shadcn `Select` |
| 62 | `components/AgentProbePreviewCapabilities.vue` | 33 | Bare `<select>` | shadcn `Select` |
| 63 | `components/AgentSelector.vue` | 67 | Bare `<select>` | shadcn `Select` |
| 64 | `components/JoinPipelineDialog.vue` | 114 | Bare `<select>` | shadcn `Select` |
| 65 | `components/AddNodeFormTypeSelector.vue` | 27 | Bare `<select>` | shadcn `Select` |
| 66 | `components/WorkflowMetadataForm.vue` | 74 | Bare `<select id="workflow-category">` | shadcn `Select` |
| 67 | `components/ToolPaletteFields.vue` | 128, 143, 144, 145 | Bare `<select>` (4×) | shadcn `Select` |

### C. `raw-control` — `<input>` not using shadcn `Input` (or wrong primitive)

The shadcn `Input` primitive in `components/ui/input/Input.vue:44` is the only legitimate raw `<input>`. Everything else should consume it (or `Switch`/`Checkbox`/`RadioGroup` for non-text types).

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 68 | `components/TrafficMonitor.vue` | 114 | Bare `<input>` (search) | shadcn `Input` |
| 69 | `components/SettingsMcpServersDetail.vue` | 44, 58, 70 | Bare `<input type="text"/url>` (3×) | shadcn `Input` |
| 70 | `components/CommandPalette.vue` | 122 | Bare `<input>` (palette query) | shadcn `Input` (or shadcn `Command` `CommandInput`) |
| 71 | `components/DesignNodeConfigPanel.vue` | 602, 611, 620 | Bare `<input type="radio">` (3 radios) | shadcn `RadioGroup` |
| 72 | `components/WorkflowBrowserFilters.vue` | 39 | Bare `<input class="search-input">` | shadcn `Input` |
| 73 | `components/SettingsConnectionKeys.vue` | 47, 48, 84, 88, 96 | Bare `<input type="text">` (5×) | shadcn `Input` |
| 74 | `components/ChatAttachmentBar.vue` | 123, 131 | Bare `<input>` (file pickers) | shadcn `Input type="file"` (or hidden file input pattern wrapped by shadcn `Button`) |
| 75 | `components/SettingsMaenifoldGraph.vue` | 18, 22, 26, 30, 48, 52, 56, 60, 64, 68, 73, 83 | Bare `<input type="number">` (12 numeric fields) | shadcn `Input type="number"` for each |
| 76 | `components/SettingsMaenifoldGraph.vue` | 97, 98, 99 | Bare `<input type="checkbox">` (3 inline checkboxes) | shadcn `Checkbox` + `Label` |
| 77 | `components/SettingsMaenifoldMemory.vue` | 29, 42, 46, 50, 54, 58, 69, 79, 91 | Bare `<input type="number">` (9 numeric fields) | shadcn `Input` |
| 78 | `components/SettingsMaenifoldMemory.vue` | 31, 32, 33, 101 | Bare `<input type="checkbox">` (4 checkboxes) | shadcn `Checkbox` |
| 79 | `components/WorkflowBrowser.vue` | 194 | Bare `<input>` | shadcn `Input` |
| 80 | `components/GateViewFilters.vue` | 75 | Bare `<input type="checkbox">` | shadcn `Checkbox` |
| 81 | `components/SettingsMaenifoldWorkflows.vue` | 17, 24, 33 | Bare `<input type="checkbox">` (3×) | shadcn `Checkbox` |
| 82 | `components/SettingsConnectionAgents.vue` | 81, 85 | Bare `<input type="text">` | shadcn `Input` |
| 83 | `components/HomeDashboard.vue` | 155 | Bare `<input>` | shadcn `Input` |
| 84 | `components/SettingsPreferences.vue` | 122, 140 | Bare `<input>`, `<input type="checkbox">` | shadcn `Input` / `Switch` |
| 85 | `components/TemplateGalleryIntentForm.vue` | 26 | Bare `<input>` | shadcn `Input` |

### D. `raw-control` — `<label>` not using shadcn `Label`

Only the most frequent offenders listed; **every** `<label>` outside `components/ui/` is a violation per DESIGN.md §10 ("Prefer shadcn-vue components and tokens"). Total bare `<label>` count: ~95.

| # | File | Lines | Violation | Recommended Fix |
|---|---|---|---|---|
| 86 | `components/WorkflowStepInspector.vue` | 83, 87, 131, 149 | Bare `<label class="field-label">` (4×) | shadcn `Label` |
| 87 | `components/GraftNodesDialog.vue` | 74, 87, 92, 105, 118, 132, 139, 143, 150, 154, 161, 165, 169, 173 | Bare `<label>` (14×) | shadcn `Label` |
| 88 | `components/SettingsMcpServersDetail.vue` | 42, 47, 56, 61, 68 | Bare `<label class="form-group">` (5×) | shadcn `Label` |
| 89 | `components/AddNodeFormFields.vue` | 47, 62, 73, 122, 135, 156, 182, 192, 203, 214 | Bare `<label>` (10×) | shadcn `Label` |
| 90 | `components/PromotionDialogPreview.vue` | 30 | Bare `<label class="field-block search-field">` | shadcn `Label` |
| 91 | `components/DesignNodeConfigPanel.vue` | 413, 440, 449, 486, 507, 528, 549, 573, 601, 610, 619, 630, 643, 655 | Bare `<label class="pipeline-inspector__field">` (14×) | shadcn `Label`; the radio-option ones (601, 610, 619) belong inside shadcn `RadioGroupItem` |
| 92 | `components/SettingsConnectionKeys.vue` | 75, 82, 86, 94 | Bare `<label class="form-group">` (4×) | shadcn `Label` |
| 93 | `components/ToolNodeFieldsPrompt.vue` | 34, 61, 67, 78 | Bare `<label>` (4×) | shadcn `Label` |
| 94 | `components/EdgeMappingEditor.vue` | 184 | Bare `<label class="mapping-row">` | shadcn `Label` |
| 95 | `components/SettingsMaenifoldGraph.vue` | 16, 20, 24, 28, 32, 46, 50, 54, 58, 62, 66, 71, 82, 97, 98, 99 | Bare `<label>` (16×) | shadcn `Label` |
| 96 | `components/SettingsMaenifoldMemory.vue` | 19, 27, 31, 32, 33, 40, 44, 48, 52, 56, 67, 78, 89, 100, 106 | Bare `<label>` (15×) | shadcn `Label` |
| 97 | `components/PromotionDialogEditor.vue` | 63, 67, 71, 77 | Bare `<label class="field-block">` (4×) | shadcn `Label` |
| 98 | `components/GateViewFilters.vue` | 74 | Bare `<label class="gate-toggle">` | shadcn `Label` + `Switch` |
| 99 | `components/AgentProbePreviewCapabilities.vue` | 32 | Bare `<label class="field-label">` | shadcn `Label` |
| 100 | `components/AgentSelector.vue` | 66 | Bare `<label for="agent-select">` | shadcn `Label` |
| 101 | `components/JoinPipelineDialog.vue` | 92, 104, 108, 112 | Bare `<label>` (4×) | shadcn `Label` |
| 102 | `components/ToolNodeFieldsThinking.vue` | 111 | Bare `<label>` | shadcn `Label` |
| 103 | `components/AddNodeFormTypeSelector.vue` | 25 | Bare `<label class="toolbar-type-field">` | shadcn `Label` |
| 104 | `components/WorkflowMetadataForm.vue` | 57, 62, 68, 73, 84, 93, 103, 118 | Bare `<label class="field-label">` (8×) | shadcn `Label` |
| 105 | `components/AppSidebar.vue` | 81 | Bare `<label>` | shadcn `Label` |
| 106 | `components/SettingsMaenifoldWorkflows.vue` | 16, 22, 32 | Bare `<label>` (3×) | shadcn `Label` |
| 107 | `components/SettingsConnectionAgents.vue` | 79, 83 | Bare `<label class="form-group">` | shadcn `Label` |
| 108 | `components/SettingsPreferences.vue` | 116, 135 | Bare `<label class="card toggle-card">` | shadcn `Label` (toggle should be `Switch` inside `Label`) |
| 109 | `components/ToolPaletteFields.vue` | 126, 143, 144, 145, 150, 204 | Bare `<label>` (6×) | shadcn `Label` |

### E. `window-api` — `window.confirm` for destructive actions

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 110 | `components/PipelineActionBar.vue` | 201 | `window.confirm('Delete this pipeline?...')` | shadcn `AlertDialog` (destructive) — native confirm cannot be themed and breaks Fluent design |
| 111 | `components/PipelineSidebar.vue` | 52 | `window.confirm('Discard unsaved pipeline draft?')` | shadcn `AlertDialog` |
| 112 | `components/PipelineSidebar.vue` | 58 | `window.confirm('Discard unsaved pipeline draft?')` | shadcn `AlertDialog` |

(`window.alert`, `window.prompt`: 0 instances. `document.createElement` non-test: useMermaid.ts:29/64 builds error/render block holders — acceptable for d3-style rendering shims; `workflow-library-save-actions.ts:47` builds an `<a>` for download — acceptable browser-download idiom. Test-file `createElement` calls are not flagged.)

### F. `hand-rolled-modal` — Teleport-based overlays / manual backdrop

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 113 | `components/ToastContainer.vue` | 17–45 | `<Teleport to="body">` builds a custom toast container instead of shadcn `Sonner` (which is wired up at `components/ui/sonner/Sonner.vue`) | Migrate to shadcn `Sonner` toast primitive; remove the bespoke container |
| 114 | `components/SemanticZoomOverlay.vue` | 256–337 | `<Teleport to="body">` constructs a full-screen modal with manual backdrop (`background: rgba(0,0,0,0.6)`, line 347), manual close button (line 280), manual `box-shadow: 0 20px 48px rgba(0,0,0,0.35)` (line 359) — i.e. a hand-rolled Dialog | shadcn `Dialog` with `DialogContent`/`DialogOverlay`; this also fixes findings #209, #233 |
| 115 | `components/PipelineActionBar.vue` | 245 | `backdrop-filter: blur(14px)` — frosted-glass action bar overlay | DESIGN.md §4 explicitly bans the glass aesthetic. Use solid `bgElevated` + `shadow16`. |

### G. `xss-sink` — `v-html` bindings (sorted by risk)

| # | File | Line | Source | Sanitized? | Risk | Recommended Fix |
|---|---|---|---|---|---|---|
| 116 | **`components/ChatThoughtSection.vue`** | **30** | `marked.parse(props.thought)` (line 12) | **NO — direct marked output** | **CRITICAL XSS** — agent thought text rendered as raw HTML | Wrap with `DOMPurify.sanitize(...)` like the other 9 sinks. Until fixed, attacker-controlled markdown in a thought block can inject `<script>` / `<img onerror>`. |
| 117 | `components/ThinkingTraceView.vue` | 177 | `renderMarkdown(thought.text)` | ✓ DOMPurify (line 85) | Low | OK; flagged for inventory only. |
| 118 | `components/PromptAssemblyPreview.vue` | 95 | `renderMarkdown(section.content)` | ✓ DOMPurify (line 20) | Low | OK; inventory only. |
| 119 | `components/KnowledgeNoteRenderer.vue` | 87 | `rendered` (computed) | ✓ DOMPurify (line 58) | Low | OK; inventory only. |
| 120 | `components/ChatMessageItem.vue` | 94 | `legacyContentWithDOMPurify` | ✓ DOMPurify | Low | Verify the binding name's promise still holds in store/composable. |
| 121 | `components/ThinkingMarkdownContent.vue` | 42 | `renderedContentWithDOMPurify` | ✓ DOMPurify | Low | Verify binding name's promise still holds. |
| 122 | `components/GateTaskDetail.vue` | 58 | `renderMarkdown(body)` | ✓ DOMPurify (line 34) | Low | OK; inventory only. |
| 123 | `components/GateTaskDetailHistory.vue` | 64 | `renderMarkdown(entry.notes)` | ✓ DOMPurify (line 19) | Low | OK; inventory only. |
| 124 | `components/ChatContentBlocks.vue` | 111 | `renderMarkdown(block.text)` | ✓ DOMPurify (line 10) | Low | OK; inventory only. |
| 125 | `components/WorkflowStepDetail.vue` | 93 | `renderMarkdownWithDOMPurify(...)` | ✓ DOMPurify | Low | OK; inventory only. |

### H. `glass-effect` — banned `backdrop-filter`

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 126 | `components/PipelineActionBar.vue` | 245 | `backdrop-filter: blur(14px);` | Remove. Use `bgElevated` + Fluent `shadow16`. |
| 127 | `components/PipelineActionBar.vue` | 246 | `-webkit-backdrop-filter: blur(14px);` | Remove. |
| 128 | `components/ui/alert-dialog/AlertDialogContent.vue` | 40 | `supports-backdrop-filter:backdrop-blur-sm` on the overlay | Drop the supports-query class; per DESIGN.md the scrim is solid `bgOverlay` (`rgba(0,0,0,0.5)`). Currently uses `bg-background/80` + opportunistic blur — neither matches Fluent. |
| 129 | `components/ui/dialog/DialogScrollContent.vue` | 34 | Same `supports-backdrop-filter:backdrop-blur-sm` overlay | Same fix. |

### I. `shadow` — non-Fluent shadows / ambient brand-tinted glow

`shadow8/16/64` are the only legal shadows. Nothing in shadcn/ui currently uses them. Every shadow below is a token violation.

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 130 | `components/ui/tooltip/TooltipContent.vue` | 27 | `shadow-[0_0_32px_rgba(173,198,255,0.08)]` — ambient blue glow | Use `shadow-[var(--shadow-16)]` |
| 131 | `components/ui/alert-dialog/AlertDialogContent.vue` | 48 | Same `shadow-[0_0_32px_rgba(173,198,255,0.08)]` | `shadow-[var(--shadow-64)]` (modal-level) |
| 132 | `components/ui/dropdown-menu/DropdownMenuContent.vue` | 35 | Same | `shadow-[var(--shadow-16)]` |
| 133 | `components/ui/context-menu/ContextMenuContent.vue` | 30 | Same | `shadow-[var(--shadow-16)]` |
| 134 | `components/ui/dialog/DialogContent.vue` | 39 | Same | `shadow-[var(--shadow-64)]` |
| 135 | `components/ui/select/SelectContent.vue` | 39 | Same | `shadow-[var(--shadow-16)]` |
| 136 | `components/ui/sheet/SheetContent.vue` | 10 | Same | `shadow-[var(--shadow-16)]` |
| 137 | `components/PipelineFlyout.vue` | 178 | `box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);` | `var(--shadow-16)` |
| 138 | `components/DesignNodeConfigModal.vue` | 81 | Same | `var(--shadow-64)` |
| 139 | `components/CommandPalette.vue` | 174, 240 | Same (2×) | `var(--shadow-16)` / `var(--shadow-64)` |
| 140 | `components/ToastContainer.vue` | 73 | `box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);` | `var(--shadow-8)` |
| 141 | `components/PipelineActionBar.vue` | 247 | `box-shadow: 0 0.5rem 1.5rem color-mix(...)` | Replace with `var(--shadow-16)` |
| 142 | `components/ThinkingThoughtItem.vue` | 82 | `box-shadow: 0 0 0 4px var(--thinking-thought-shadow)` (custom glow ring) | If retained, drive ring via Fluent focus pattern; otherwise remove. |
| 143 | `components/WorkflowStepCard.vue` | 145 | `box-shadow: 0 1px 3px rgba(0,0,0,0.3);` | `var(--shadow-8)` |
| 144 | `components/WorkflowStepCard.vue` | 158 | Custom node-color glow ring | Replace with Fluent border + `var(--shadow-8)` |
| 145 | `components/ThinkingSessionCard.vue` | 123 | `box-shadow: 0 6px 18px color-mix(...)` | `var(--shadow-8)` |
| 146 | `components/PipelineView.css` | 158, 183 | `box-shadow: 0 0 32px color-mix(in srgb, var(--blue) 8%, transparent);` (2× ambient brand glow) | Remove glow; use `var(--shadow-16)` if elevation needed. **DESIGN.md §4 explicitly bans this.** |
| 147 | `components/WorkflowNodeCard.vue` | 179 | `box-shadow: 0 2px 8px color-mix(...)` | `var(--shadow-8)` |
| 148 | `components/GateViewFilters.vue` | 100 | `box-shadow: 0 0 0 2px rgb(77 142 255 / 0.15);` (focus ring) | Use Fluent focus token / `--ring` |
| 149 | `components/SemanticZoomOverlay.vue` | 359 | `box-shadow: 0 20px 48px rgba(0, 0, 0, 0.35);` | `var(--shadow-64)` |
| 150 | `components/PipelineComposer.vue` | 288 | `box-shadow: 0 0 32px color-mix(in srgb, var(--blue) 8%, transparent);` ambient brand glow | Remove. Banned by §4. |
| 151 | `components/PipelineComposer.vue` | 316 | Custom edge-indicator shadow | `var(--shadow-8)` |
| 152 | `components/ToolNodeFields.vue` | 59 / `ToolPaletteFields.vue:240` | `box-shadow: 0 0 0 1px var(--color-brand);` (selected ring) | Use Fluent border emphasis (1.5px brand border) per DESIGN.md §5 |
| 153 | `assets/tailwind.css` | 368 | `box-shadow: 0 1px 4px rgba(0,0,0,0.4);` | `var(--shadow-8)` |

### J. `radius` — off-scale `border-radius` (allowed: 4 / 8 / 12 / 999px)

The lone canonical token vars `--radius` / `--radius-sm` / `--radius-md` / `--radius-lg` / `--radius-xl` need verification — DESIGN.md §6 says only **4, 8, 12, 999**. `--radius-lg` and `--radius-xl` (used in `GateViewStatus.vue:25`, `SchemaFormRawEditor.vue:46`, `SemanticZoomOverlay.vue:356`) are off-scale unless they map to one of those four. Only listing literals here.

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 154 | `components/WorkflowStepInspector.vue` | 195 | `border-radius: 3px;` | 4px |
| 155 | `components/ThinkingThoughtItem.vue` | 75, 105 | `border-radius: 4px;` direct literal — should be tokenized | Use `var(--radius-sm)` or design token |
| 156 | `components/ChatThoughtSection.vue` | 72 | `border-radius: 3px;` | 4px |
| 157 | `components/GateViewToolbar.vue` | 92 | `border-radius: 0.25rem;` (=4px but rem) | Use `4px` literal or `var(--radius-sm)` for consistency |
| 158 | `components/ToastContainer.vue` | 70 | `border-radius: 0.75rem;` (=12px ok scale, but rem) | `12px` |
| 159 | `components/DesignNodeConfigModal.vue` | 78 | `border-radius: 0;` (no radius on a modal) | Modal must be 8px |
| 160 | `components/SettingsView.vue` | 109 | `border-radius: 0;` | At minimum 4px |
| 161 | `components/KnowledgeNoteRenderer.vue` | 194 | `border-radius: 0;` | Use 4 / 8 |
| 162 | `components/ToolNodeFields.vue` | 72 | `border-radius: 0.75rem;` (=12px, rem) | `12px` |
| 163 | `components/ToolPaletteFields.vue` | 245 | `border-radius: 0.75rem;` | `12px` |
| 164 | `components/PipelineView.css` | 93, 119, 177, 201 | `border-radius: 0.25rem;` (4×) | tokenize |
| 165 | `components/PipelineActionBar.vue` | 243 | `border-radius: var(--radius);` (verify value is 8) | confirm |
| 166 | `components/GateViewFilters.vue` | 88, 114 | `border-radius: 0.25rem;` | tokenize |
| 167 | `components/GateTaskDetail.vue` | 153, 170 | `border-radius: 0.25rem;` | tokenize |
| 168 | `components/GateTaskDetailStages.vue` | 89 | `border-radius: 0.25rem;` | tokenize |
| 169 | `components/GateTaskDetailHistory.vue` | 85, 120, 138 | `border-radius: 0.25rem;` | tokenize |
| 170 | `components/WorkflowBrowserList.vue` | 83, 86 | `border-radius: 6px;`, `50%` (4px badge dot is fine but 6px is off-scale) | Use 4 or 8 / 999px |
| 171 | `components/TemplateGalleryCard.vue` | 55, 69 | `border-radius: 0.25rem; / 0.25rem 0 0 0.25rem;` | tokenize |
| 172 | `components/GraphViewCanvas.vue` | 94 | `border-radius: 0.25rem;` | tokenize |
| 173 | `components/GateTaskTable.vue` | 177, 182 | `border-radius: 0.25rem 0 0 0.25rem;`, mirror | tokenize |
| 174 | `components/GraphViewControls.vue` | 68, 108 | `border-radius: 0.25rem;` | tokenize |
| 175 | `components/TemplateGallerySelection.vue` | 60 | `border-radius: 0.25rem;` | tokenize |
| 176 | `components/PipelineSidebar.vue` | 260, 267 | `border-radius: 2px;` | 4px |
| 177 | `components/WorkflowNodeCard.vue` | 236, 243 | `border-radius: 2px;` | 4px |
| 178 | `components/WorkflowTreeLoadingState.vue` | 100, 178 | `border-radius: 2px;`, `3px;` | 4px |
| 179 | `components/WorkflowStepDetail.vue` | 227, 283 | `border-radius: 3px;` | 4px |
| 180 | `components/SettingsMcpServersList.vue` | 79 | `border-radius: 4px;` (literal in inline `.type-badge`) | tokenize |
| 181 | `components/__tests__/evidence/T-125.3/NodePaletteEvidenceApp.vue` | 269, 298 | `border-radius: 1rem;` (=16px) | 12px (or 8) |

(All `4px`, `8px`, `12px`, `999px`, `var(--radius)` literals matching scale are inventory-only — too numerous to enumerate per DESIGN.md §6 they are valid; only off-scale and untokenized literals listed above.)

### K. `color-token` — non-canonical hex / rgba / aliases

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 182 | `assets/tailwind.css` | 31 | `--color-brand: var(--blue);` Obsidian alias `--blue` survived migration | Map `--color-brand` directly to `#0078D4` per `App.jsx:27` |
| 183 | `assets/tailwind.css` | 159, 318, 323, 327 | `var(--blue)` aliased to brand/accent/primary/ring | Same — `var(--blue)` is non-canonical and DESIGN.md §2 explicitly names `brand`/`brandHover`/`brandLight` as the only color tokens |
| 184 | `components/PipelineView.css` | 158, 183 | `color-mix(in srgb, var(--blue) 8%, transparent)` | Use `var(--brand-light)` |
| 185 | `components/PipelineComposer.vue` | 288 | Same `var(--blue)` mix | Same fix |
| 186 | `components/ui/tooltip/TooltipContent.vue` | 27 | `bg-[#353534]` hard-coded hex (not in Fluent palette: should be `bgSurfaceHover #3A3A3A` or `bgElevated #2D2D2D`) | Replace with `bg-[var(--color-bg-elevated)]` |
| 187 | `components/ui/dropdown-menu/DropdownMenuContent.vue` | 35 | `bg-[#353534]` | Same fix |
| 188 | `components/ui/context-menu/ContextMenuContent.vue` | 30 | `bg-[#353534]` | Same fix |
| 189 | `components/ui/alert-dialog/AlertDialogContent.vue` | 48 | `bg-[#201f1f]` (not a Fluent token; nearest is `bgSidebar #282828`) | Replace with token |
| 190 | `components/ui/dialog/DialogContent.vue` | 39 | `bg-[#201f1f]` | Replace with token |
| 191 | `components/ui/input/Input.vue` | 48 | `bg-[#1c1b1b]`, `border-[rgba(66,71,83,0.15)]` (#1c1b1b is close to `bgApp #1C1C1C`, but rgba border is non-canonical: should be `borderSubtle #383838` or `borderCard rgba(255,255,255,0.08)`) | Use `bg-[var(--color-bg-app)]` and `border-[var(--color-border-subtle)]` |
| 192 | `components/ui/textarea/Textarea.vue` | 47 | Same `bg-[#1c1b1b]`, `border-[rgba(66,71,83,0.15)]` | Same fix |
| 193 | `components/ChatThoughtSection.vue` | 39 | `border: 1px solid rgba(66, 71, 83, 0.15);` | `var(--color-border-subtle)` |
| 194 | `components/GraftNodesDialog.vue` | 78, 96, 109, 122 | `border-[rgba(66,71,83,0.15)]` (4×) | tokenize |
| 195 | `components/AuthMethodDialog.vue` | 127, 170 | `border: 1px solid rgba(66, 71, 83, 0.15);` (2×) | tokenize |
| 196 | `components/CommandPalette.vue` | 172, 238 | Same | tokenize |
| 197 | `components/AddNodeFormFields.vue` | 13 | Same in inline class string | tokenize |
| 198 | `components/AddNodeFormTypeSelector.vue` | 6 | Same | tokenize |
| 199 | `components/JoinPipelineDialog.vue` | 116 | Same | tokenize |
| 200 | `components/ChatMessageItem.vue` | 161 | Same | tokenize |
| 201 | `components/AddNodeForm.vue` | 162, 171 | Same (2×) | tokenize |
| 202 | `components/GraphViewCanvas.vue` | 93, 111 | Same | tokenize |
| 203 | `components/GateTaskDetail.vue` | 152, 169 | Same | tokenize |
| 204 | `components/GateTaskDetailHistory.vue` | 84, 119 | Same | tokenize |
| 205 | `components/ChatMessageList.vue` | 99, 115, 136 | Same | tokenize |
| 206 | `components/GraphViewControls.vue` | 67, 107 | Same | tokenize |
| 207 | `components/TemplateGallerySelection.vue` | 59 | Same | tokenize |
| 208 | `components/ChatInputBar.vue` | 129, 144 | Same | tokenize |
| 209 | `components/ChatInputBar.vue` | 156 | `border-color: rgba(77, 142, 255, 0.4);` (focus = arbitrary blue, not brand) | `var(--color-brand)` or `--ring` |
| 210 | `components/WorkflowBrowserFilters.vue` | 94 | `border-color: rgba(77, 142, 255, 0.4);` | `var(--color-brand)` |
| 211 | `components/GateViewFilters.vue` | 92 | `border: 1px solid rgba(255, 255, 255, 0.1);` (close to but not `borderCard rgba(255,255,255,0.08)`) | tokenize |
| 212 | `components/GateViewFilters.vue` | 113 | `rgba(66, 71, 83, 0.15)` | tokenize |
| 213 | `components/GateTaskTable.vue` | 120 | `scrollbar-color: rgba(255, 255, 255, 0.1) transparent;` | tokenize |
| 214 | `components/GraphViewCanvas.vue` | 40, 41, 64 | `radial-gradient(...)` with arbitrary blue/green tints | Either remove decorative gradients or align tints with brand/success tokens |
| 215 | `components/SemanticZoomOverlay.vue` | 347 | `background: rgba(0,0,0,0.6);` (overlay scrim) | `var(--bg-overlay)` (which is `rgba(0,0,0,0.5)`) |
| 216 | `components/KnowledgeNoteRenderer.vue` | 16 | Mermaid `primaryBorderColor: 'rgba(255,255,255,0.12)'` | Should be `var(--color-border-card)` value `rgba(255,255,255,0.08)` |
| 217 | `components/KnowledgeNoteRenderer.vue` | 241 | `background: rgba(255, 255, 255, 0.02);` | tokenize or remove |
| 218 | `components/KnowledgeNoteRenderer.vue` | 270 | `background: rgba(0, 120, 212, 0.12);` (raw brand-light hex but wrong opacity 0.12 vs canonical 0.15) | `var(--brand-light)` |
| 219 | `components/PipelineComposerContextMenuDesign.vue` | 98 | `background: rgba(255,255,255,0.1); color: rgba(255,255,255,0.56);` | tokenize |

### L. `typography` — `font-family` and `text-transform`

#### L.1 `font-family` — token-level violation

| # | File | Lines | Violation | Recommended Fix |
|---|---|---|---|---|
| 220 | `assets/tailwind.css` | 43, 141, 142, 166 | `--font-code`/`--font-mono` resolve to `"JetBrains Mono", "Cascadia Code", Consolas, monospace` — DESIGN.md §11 explicitly killed JetBrains-heavy identity. Code is allowed but the canonical font is one stack only. | Either (a) drop JetBrains and use system mono `ui-monospace, "SF Mono", Consolas, monospace`, or (b) restrict `--font-code` use to genuine code blocks only and audit every consumer. |
| 221 | All files in `font-family scan` (#106–181) using `var(--font-code)` / `var(--font-mono)` for non-code chrome (e.g. `WorkflowStepInspector:180` `.meta-mono`, `GateViewToolbar:114, 143`, `TrafficMonitor:272`, `SettingsView:77` `--font-display` for buttons, `WorkflowStepTimeline:341` meta strip, `WorkflowMetadataForm:139`) | various | UI chrome (meta strips, buttons, captions, badges) styled with monospace stack — violates DESIGN.md §3 "Use textPrimary/textSecondary/textTertiary for hierarchy; do not introduce alternate headline font families" | Replace `var(--font-code)`/`var(--font-mono)` with `var(--font-body)` (Segoe UI Variable) for chrome; reserve mono for genuine code/payload contexts (`TrafficJsonPayload`, `ChatToolCallDetails` payload, `KnowledgeNoteRenderer pre/code` block). |
| 222 | `components/KnowledgeNoteRenderer.vue` | 92 | Hard-coded `'Segoe UI Variable',...` instead of token | Use `var(--font-body)` |
| 223 | `components/KnowledgeNoteRenderer.vue` | 177 | `'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace` hardcoded | Tokenize; consider switching to system mono |
| 224 | `components/WorkflowBrowserList.vue` | 91 | `'JetBrains Mono', monospace` hardcoded for `.meta-steps` (UI chrome, not code) | Use `var(--font-body)` |
| 225 | `components/SettingsPreferences.vue` | 258 | `'JetBrains Mono', 'SF Mono', 'Fira Code', monospace` hardcoded | Tokenize |
| 226 | `components/SettingsConnectionDaemon.vue` | 142 | Same hardcoded mono | Tokenize |

#### L.2 `text-transform`

DESIGN.md §3 type ramp does not reserve any role for forced upper/lower case; the `What Changed` table does not mention it. These uppercase/lowercase abuses violate Fluent identity (Fluent eyebrow text is sentence case).

| # | File | Line | Violation | Recommended Fix |
|---|---|---|---|---|
| 227 | `components/CommandPalette.vue` | 264 | `text-transform: none;` (override that confirms surrounding stylesheet uses transforms) | Remove the override AND find/fix upstream |
| 228 | `components/StartupProgress.vue` | 161 | `text-transform: lowercase;` | Remove |
| 229 | `components/KnowledgeNoteRenderer.vue` | 237 | `text-transform: uppercase;` | Remove |
| 230 | `components/SettingsMaenifold.vue` | 227 | `text-transform: uppercase;` | Remove |
| 231 | `components/PlanView.vue` | 205 | `text-transform: capitalize;` | Remove |
| 232 | `components/SettingsConnection.vue` | 252 | `text-transform: uppercase;` | Remove |
| 233 | `components/PermissionEntry.vue` | 170 | `text-transform: capitalize;` | Remove |
| 234 | `components/PipelineComposer.vue` | 308 | `text-transform: lowercase;` | Remove |
| 235 | `components/SettingsMcpServersList.vue` | 62, 79 | `text-transform: uppercase;`, `lowercase;` | Remove |
| 236 | `components/SettingsConnectionAgents.vue` | 150 | `text-transform: uppercase;` | Remove |
| 237 | `components/SettingsPreferences.vue` | 171 | `text-transform: uppercase;` | Remove |
| 238 | `components/GateTaskTable.vue` | 236 | `text-transform: lowercase;` | Remove |
| 239 | `components/__tests__/evidence/T-125.3/NodePaletteEvidenceApp.vue` | 248, 276 | `text-transform: uppercase;` (2×) | Remove |

---

## Recommended remediation prioritization

1. **#116 ChatThoughtSection v-html unsanitized** — fix today; one-line DOMPurify wrap.
2. **#110–112 `window.confirm` for destructive pipeline actions** — destructive UX inside a Fluent app rendering native browser modals is jarring and untestable; replace with `AlertDialog`.
3. **#130–136 + #126–129** — every shadcn/ui primitive (Tooltip, Dialog, AlertDialog, Sheet, Select, Dropdown, Context, Input, Textarea) is shipping with Obsidian-era ambient blue glow shadows and `bg-[#353534]/#201f1f/#1c1b1b` hex literals that do not match the Fluent palette. **Fixing these primitives propagates to every consumer for free** — highest leverage in the audit.
4. **#114 SemanticZoomOverlay** — ~80 LOC of hand-rolled modal that duplicates `Dialog` capability and ships its own shadow + scrim violations.
5. **#85–109** — `<label>` migration is mechanical and unblocks the form components from full shadcn conformance.
6. **#220–226** — drop or restrict `--font-code`/`--font-mono`/JetBrains across UI chrome; this is a token decision that, like #3, propagates everywhere.
7. **#227–239** — `text-transform` removal is a pure delete; do as a single sweep.
8. Long tail of bare `<button>`/`<input>`/`<select>`/radius/rgba — partition by file owner and burn down.