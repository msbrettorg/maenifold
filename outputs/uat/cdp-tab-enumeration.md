# Maenifold Desktop App: CDP-Based UI Tab Enumeration

**Test Date:** 2026-04-28  
**CDP Connection:** http://127.0.0.1:9222  
**Time-boxed:** 45 minutes  
**Scope:** Live accessibility tree inspection and screenshot capture of all 8 top-level tabs

---

## Tab: Home

**Default view screenshot:** outputs/screenshots/uat-tabs/01-home-default.png

**Visible structure:**
- Heading: "What can I help you with?"
- Four quick-action cards (buttons):
  - 💬 Agents — Start or resume sessions
  - 🔀 Pipelines — Run multi-step workflows
  - 📋 Gate — Track tasks & delivery
  - ⚡ Workflows — Browse & author
- 🧠 Knowledge — Explore the graph (button)
- Large search/input: "Ask me anything..." textbox
- Icon button (likely search/submit)

**Interactive elements:**
1. "Agents" button (card)
2. "Pipelines" button (card)
3. "Gate" button (card)
4. "Workflows" button (card)
5. "Knowledge" button (card)
6. Search textbox "Ask me anything..."
7. Submit/action button

**Capabilities observed:**
- Home screen acts as a discovery/shortcut portal
- Quick access to main features via cards
- Unified search interface

---

## Tab: Agents

**Default view screenshot:** outputs/screenshots/uat-tabs/02-agents-default.png  
**Picker dropdown:** outputs/screenshots/uat-tabs/02-agents-picker-open.png  
**Session loading:** outputs/screenshots/uat-tabs/02-agents-session-loading.png  
**Chat interface:** outputs/screenshots/uat-tabs/02-agents-chat-open.png

**Visible top-level structure:**
- **Left sidebar:**
  - Agent selector dropdown (combobox): "Copilot (1192 active)"
  - "Refresh" button
  - "Select folder" button
  - "New session" button
  - Sessions list heading
  - Sessions list (scrollable) with 18+ prior sessions

- **Main area:**
  - Toggle ACP traffic monitor button (top)
  - Chat region:
    - Session title
    - Mode options (dropdown)
    - Model options (dropdown)
    - Reasoning Effort options (dropdown)
    - Message history (scrollable articles with thinking regions)
    - Input area

**Interactive elements — session list:**
1. Agent picker dropdown
   - Codex (483 active)
   - Copilot (1192 active) — **selected**
   - OpenCode (100 active)
2. "Refresh" button
3. "Select folder" button
4. "New session" button (disabled when session loading)
5. Session list items (each clickable)
6. Delete session buttons (per session)

**Interactive elements — chat area:**
1. "Mode options" dropdown (expanded=false)
   - Likely contains: normal/plan/reasoning modes
2. "Model options" dropdown (expanded=false)
   - Likely contains: model selection (gpt-4, o1, etc.)
3. "Reasoning Effort options" dropdown (expanded=false)
   - Likely contains: low/medium/high effort
4. Message history:
   - Show/hide thinking toggles (💭 icon, clickable)
   - Links within message content (clickable)
5. **Input area:**
   - Image attachment button: "No file chosen"
   - Audio attachment button: "No file chosen"
   - Image button (disabled)
   - Audio button (disabled)
   - @ File button (file reference/mention)
   - Message input textbox
   - Send button (disabled when empty)

**Capability gaps observed:**
- ❌ No visible slash command popup trigger (typing `/` should reveal commands)
- ❌ No visible mode switcher label in input area (though mode options exist in header)
- ❌ No visible voice recording button in input area (only file/audio attachments)
- ⚠️ File and audio buttons appear disabled — unclear if feature is broken or just no media selected

**Expected modern features missing:**
- Slash command discoverability (e.g., `/research`, `/refine`, `/analyze`)
- Voice-to-text record button in input
- Copy/export message button per message
- Message regeneration/retry button
- Stop generation button (when streaming)
- Customizable system prompts or instructions panel

---

## Tab: Pipelines

**Default view screenshot:** outputs/screenshots/uat-tabs/03-pipelines-default.png  
**Flyout list:** outputs/screenshots/uat-tabs/03-pipelines-flyout.png  
**Visualization:** outputs/screenshots/uat-tabs/03-pipelines-visualization.png

**Visible structure:**
- **Navigation bar:**
  - "Open pipelines flyout" button (top-left)
  - "Create new pipeline" button

- **Default view (before selection):**
  - Heading: "What would you like to do?"
  - Prompt textbox: "What do you want to do?"
  - Template buttons (4):
    - "Research & Verify" — Investigate a topic using the scientific method...
    - "Think & Critique" — Deeply reason through a problem...
    - "Draft & Refine" — Draft a response, stress-test it...
    - "Explore & Synthesize" — Run agentic research...

- **Pipeline visualization (DAG):**
  - Layout toggles: "↔ Layout LR", "↕ Layout TB"
  - Control buttons: "Rewind", "Delete"
  - Nodes:
    - Tool nodes (Kusto queries, e.g., "verify-connectivity")
    - Sequential thinking nodes (e.g., "analyze-cost-trends")
  - Edges connecting nodes
  - Each node is clickable
  - Edges are clickable
  - Toggle runner button (bottom)

**Interactive elements:**
1. "Open pipelines flyout" button
2. "Create new pipeline" button
3. Template buttons (4)
4. "↔ Layout LR" button
5. "↕ Layout TB" button
6. "Rewind" button
7. "Delete" button
8. Nodes (tool/thinking) — clickable
9. Edges — clickable
10. "Toggle runner" button

**Flyout list (when visible):**
1. "Refresh pipelines" button
2. "Search pipelines…" textbox
3. Pipeline items (buttons with delete options):
   - ftk-analysis-1776362447817 (20/20 nodes, 100% complete)
   - pipeline-1745366400000-contract (0/1 nodes, 0%)
   - ... (more pipelines)

**Capabilities observed:**
- Full DAG visualization with layout control
- Node/edge selection and inspection
- Pipeline templates for guided creation
- Runner control (start/stop/rewind)
- No visible pipeline editing interface (add nodes, edit node params)

**Capability gaps:**
- ❌ No inline node parameter editor visible
- ❌ No edge edit/delete UI visible on hover
- ❌ No export/save pipeline UI
- ❌ No pipeline versioning control

---

## Tab: Gate

**Default view screenshot:** outputs/screenshots/uat-tabs/04-gate-default.png  
**Task detail view:** outputs/screenshots/uat-tabs/04-gate-task-detail.png

**Visible structure:**
- **Task statistics (top):**
  - "0 todo" button
  - "824 done" button
  - "824 total" button

- **Filter/Control bar:**
  - Repository selector combobox: "maenifold-rs"
  - "Browse…" button
  - "Remove" button
  - "Refresh" button
  - "Filter tasks" searchbox
  - Process filter combobox: "All processes"
  - State filter combobox: "All states"
  - Checkbox (likely "hide closed" or similar)

- **Task table:**
  - Column headers: Task ID | Task | Process | Progress
  - Sortable columns (buttons with sort indicators)
  - Rows (each clickable):
    - T-01.0 — Constitution Audit Remediation (epic) | sdd done | Done (5/5)
    - T-01.1 — Gate: fix non-MCP no-unwrap violations | lightweight done | Done (4/4)
    - T-01.2 through T-03.3... (824 total tasks)

**Interactive elements:**
1. Todo/Done/Total count buttons
2. Repository selector combobox
3. "Browse…" button
4. "Remove" button
5. "Refresh" button
6. Filter tasks searchbox
7. Process filter combobox
8. State filter combobox
9. Select-all checkbox
10. Column header buttons (for sorting)
11. Task rows (clickable for detail view)

**Capabilities observed:**
- Full task/issue tracking UI
- Multi-column sortable table
- Filtering by repository, process, state
- Task counts aggregated
- Task linking to on-disk markdown files (breadcrumb shows "File: maenifold-rs/done/T-01.0.md")

**Capability gaps:**
- ❌ No inline task editing visible (edit title, description, assignee)
- ❌ No "create new task" button visible in UI
- ❌ No task dependency visualization
- ❌ No progress bar for individual tasks (only summary progress)
- ⚠️ "Browse…" button purpose unclear

---

## Tab: Knowledge

**Memory default:** outputs/screenshots/uat-tabs/05-knowledge-memory-default.png  
**Memory opened:** outputs/screenshots/uat-tabs/05-knowledge-memory-opened.png  
**Graph default:** outputs/screenshots/uat-tabs/05-knowledge-graph-default.png

**Visible structure:**
- **Sub-tabs (tablist):**
  - Tab "Memory" (includes vault, tree/search, editor)
  - Tab "Graph" (includes co-occurrence graph canvas)

### Memory subtab:

- **Left panel: Vault tree/search**
  - Heading "Vault"
  - Refresh button
  - Search textbox: "Search notes..."
  - Tablist: "Tree" | "Search"
  - Folder/file tree:
    - AGENTS.md (button)
    - Celonis.md
    - Session-as-Node.md
    - kb-index.md
    - DailyLogs (3 files, expandable)
    - Dashboards (0, expandable)
    - Knowledgebase (1, expandable)
    - ... (30+ folders)
  - Separator (resizable, shown as drag handle)

- **Center panel: Note editor**
  - Breadcrumb: "Knowledge AGENTS"
  - Tab for open note: "AGENTS" with close button
  - Note content (markdown):
    - Heading: "AGENTS"
    - Markdown content (MSX/MCP issues encountered)
  - Separator (resizable)

- **Right panel: Related concepts**
  - "Collapse related concepts" button
  - List of related concepts with similarity scores:
    - agents-tab Similarity 0.865
    - agent-name Similarity 0.861
    - agent Similarity 0.960
    - ... (more)

**Interactive elements — Memory:**
1. Refresh button (vault)
2. Search textbox
3. Tree/Search tab toggle
4. Folder/file tree buttons (expandable/clickable)
5. Note editor (markdown, probably read-only in this view)
6. Related concepts buttons (clickable to navigate)
7. "Collapse related concepts" button
8. Resizable separators (left/center and center/right)

### Graph subtab:

- **Top toolbar:**
  - Filter communities textbox
  - Separator/handle (resizable)
  - Filter graph textbox
  - Communities combobox: "All communities" (disabled)
  - "Reset" button
  - "Reload" button

- **Canvas:**
  - Interactive knowledge graph visualization (nodes + edges)
  - Clickable nodes (concepts)
  - Draggable nodes (likely)

- **Left panel:** Communities list (if visible, noted as "18" in separator)

- **Separators:** Resizable dividers (left/canvas and canvas/properties)

**Interactive elements — Graph:**
1. Filter communities textbox
2. Filter graph textbox
3. Communities combobox
4. "Reset" button (clear filters/state)
5. "Reload" button (refresh graph from disk)
6. Graph nodes (clickable, likely draggable)
7. Graph edges (may be hoverable for info)
8. Resizable separators

**Capabilities observed:**
- Dual view: file-based Memory vault + co-occurrence Graph
- Tree navigation with search fallback
- Related concept discovery via similarity ranking
- Graph visualization with filtering

**Capability gaps:**
- ❌ No "new note" button visible in Memory view
- ❌ No note edit/delete/rename buttons visible
- ❌ No graph node detail panel (click node → see info)
- ❌ No graph export (PNG, SVG, JSON)
- ❌ No community highlighting/selection in left panel
- ⚠️ Graph manipulation (add edge, merge nodes) not visible

---

## Tab: Workflows

**Default view:** outputs/screenshots/uat-tabs/06-workflows-default.png

**Visible structure:**
- **Left panel: Workflow library**
  - Textbox: "Find workflow"
  - Category buttons: "All" | "Reasoning" | "Scientific" | "Multi-agent" | "DevOps" | "Memory"
  - Workflow list (scrollable, 80+ workflows):
    - Abductive Reasoning (Reasoning, 5 steps) — **selected**
    - Constitutional Agent Architecture Workflow (Scientific, 22 steps)
    - Agentic SLC (Simple, Lovable, Complete) (DevOps, 11 steps)
    - ... (many more)
  - "New workflow" button (bottom)
  - "Import workflow" button

- **Right panel: Workflow detail**
  - Workflow step timeline (editor):
    - Heading: "Abductive Reasoning"
    - "Expand all" button
    - "Export" button
    - "Saved" button (disabled)
    - Steps (5):
      1. Observe Surprising Facts (expandable, with move/remove buttons)
      2. Generate Explanatory Hypotheses (expandable, with move/remove buttons)
      3. Evaluate by Selection Criteria (expandable, with move/remove buttons)
      4. Abductive Validation (expandable, with move/remove buttons)
      5. Economy of Research (expandable, with move/remove buttons)
    - "＋ Add new step" button

  - Workflow metadata (edit form):
    - NAME textbox: "Abductive Reasoning"
    - EMOJI textbox: "💭"
    - CATEGORY combobox: "Auto"
    - DESCRIPTION textbox: long description
    - TRIGGERS textbox: comma-separated trigger phrases
    - Switch: "Enable workflow-level sequential thinking" (ON)
    - Switch: "Mark this workflow as multi-agent" (OFF)

**Interactive elements:**
1. "Find workflow" search textbox
2. Category filter buttons (5)
3. Workflow list items (clickable)
4. "New workflow" button
5. "Import workflow" button
6. "Expand all" button
7. "Export" button
8. Step expand toggles (per step)
9. Move step up/down buttons (per step)
10. Remove step buttons (per step)
11. "＋ Add new step" button
12. NAME, EMOJI, CATEGORY, DESCRIPTION, TRIGGERS textboxes
13. Sequential thinking switch
14. Multi-agent switch

**Capabilities observed:**
- Comprehensive workflow library with 80+ pre-built workflows
- Workflow filtering by category (Reasoning, Scientific, Multi-agent, DevOps, Memory)
- Step-by-step workflow editor with reordering
- Metadata editing (name, emoji, category, description, triggers)
- Workflow-level sequential thinking toggle
- Export functionality

**Capability gaps:**
- ❌ No workflow run/execute button visible
- ❌ No workflow versioning control
- ❌ No workflow duplication/clone button
- ❌ No step parameter inline editor
- ❌ No workflow sharing/collaboration UI
- ⚠️ "Auto" category appears to be a fallback; unclear how to set custom categories

---

## Tab: Schedules

**Default view:** outputs/screenshots/uat-tabs/07-schedules-default.png

**Visible structure:**
- Heading: "Schedules"
- "Refresh" button
- "New schedule" button
- Empty table with columns:
  - Name
  - Schedule
  - Agent
  - Permissions
  - Status
  - Last run
  - Actions

**Visible message:**
"No schedules configured yet."

**Interactive elements:**
1. "Refresh" button
2. "New schedule" button
3. Column headers (likely sortable, but no data to test)

**Capabilities observed:**
- Schedule management UI structure is present but no schedules exist
- Can create new schedules via "New schedule" button
- Table columns suggest: name, cron/frequency, agent selection, permissions, status, last run timestamp, and action buttons

**Capability gaps:**
- ❌ No example/template schedules visible
- ❌ No inline schedule editor visible (would need to create one first)
- ⚠️ **Tab marked as "untested per user" — no schedules have been created yet**

---

## Tab: Settings

**Default view:** outputs/screenshots/uat-tabs/08-settings-default.png

**Visible structure:**

### Configuration section:
- Heading: "Configuration"
- Toolbar buttons:
  - "text" | "tree" | "table" (view mode toggles)
  - "Expand all" button
  - "Collapse all" button
  - "Format JSON" button (Ctrl+I)
  - "Compact JSON" button (Ctrl+Shift+I)
  - "Sort" button
  - "Transform contents" button (filter, sort, project)
  - "Search" button (Ctrl+F)
  - "Undo" button (disabled)
  - "Redo" button (disabled)

- JSON editor textbox (large, editable):
  - MCP servers configuration
  - Array of servers with type, name, url/command, args, gateway flag
  - Current servers: maenifold, microsoft-learn, workiq, msx-mcp, fetch, ...

- Buttons:
  - "Save" button (disabled when no changes)
  - "Revert" button (disabled when no changes)

### Embeddings section:
- Heading: "Embeddings"
- "Regenerate embeddings" button

- Embeddings table:
  - Column headers: Model | Specs | Quality | Status | Actions
  - Embedding models:
    1. all-MiniLM-L6-v2 (Apache-2.0)
       - 384d, 256 tokens, 90 MB
       - MTEB: 41.90
       - Status: Installed
       - Actions: Activate | Delete

    2. BGE Small English v1.5 (MIT)
       - 384d, 512 tokens, 67 MB
       - MTEB: 51.70
       - Status: **Active**
       - Actions: Active — switch first to manage

    3. Snowflake Arctic Embed M v2.0 (Apache-2.0)
       - 768d, 8192 tokens, 610 MB
       - MRL: 256/512/768; MTEB: 55.40
       - Status: Not installed
       - Actions: Download

    4. mxbai Embed Large v1 (Apache-2.0)
       - 1024d, 512 tokens, 670 MB
       - MRL: 256/512/768/1024; MTEB: 54.40
       - Status: Not installed
       - Actions: Download

    5. Nomic Embed Text v1.5 (Apache-2.0)
       - 768d, 8192 tokens, 200 MB
       - MRL: 64/128/256/512/768; MTEB: 53.00
       - Status: Not installed
       - Actions: Download

    6. Jina Embeddings v3 (CC-BY-NC-4.0)
       - 1024d, 8192 tokens, 1.1 GB
       - MRL: 32/64/128/256/512/768/1024; MTEB: 55.80
       - Status: Not installed
       - Actions: Download

    7. Qwen3 Embedding 0.6B (Apache-2.0)
       - 1024d, 32768 tokens, 1.2 GB
       - MRL: 32/64/128/256/512/768/1024; MTEB: 64.30
       - Status: Installed
       - Actions: Activate | Delete

    8. BGE Large English v1.5 (MIT)
       - 1024d, 512 tokens, 400 MB
       - MTEB: 54.30
       - Status: Not installed
       - Actions: Download

**Interactive elements — Configuration:**
1. View mode buttons (text/tree/table)
2. "Expand all" button
3. "Collapse all" button
4. "Format JSON" button
5. "Compact JSON" button
6. "Sort" button
7. "Transform contents" button
8. "Search" button
9. Undo/Redo buttons
10. JSON editor (editable textbox)
11. "Save" button
12. "Revert" button

**Interactive elements — Embeddings:**
1. "Regenerate embeddings" button
2. Column header buttons (sortable)
3. "What is MRL?" info buttons (per model, 6 instances)
4. "Activate" buttons (per installed model)
5. "Delete" buttons (per installed model)
6. "Download" buttons (per not-installed model)

**Capabilities observed:**
- Full configuration file editor (MCP servers, potentially other settings)
- JSON formatting/compacting utilities
- Search and transform tools for JSON
- Embedding model management:
  - View installed/available models
  - Switch between models (only one active at a time)
  - Download new models
  - Delete installed models
  - Quality metrics (MTEB score, MRL dimensions)
  - License display

**Capability gaps:**
- ❌ No UI schema validation or error display during edit
- ❌ No configuration export/backup button
- ❌ No settings reset to defaults option
- ❌ No per-section settings panels (all in one JSON editor)
- ⚠️ Configuration editor is raw JSON; no structured form UI

---

## Capability Gap Summary

### **Major Missing Features** (Expected in modern AI-first desktop clients like Claude Desktop, Cursor, Zed)

#### **Agents Tab:**
1. ❌ **Slash commands** — No `/research`, `/refine`, `/analyze` command discovery
2. ❌ **Voice input** — No voice-to-text recording button
3. ❌ **Message regeneration** — No retry/edit + resend button per message
4. ❌ **Stream control** — No visible stop/cancel button during generation
5. ❌ **Copy/export** — No copy message to clipboard, export conversation
6. ❌ **System prompt customization** — No visible instructions panel or system context editor
7. ❌ **Multi-file context** — No visible drag-drop file attachment UI
8. ⚠️ **Mode switcher** — Mode options exist but not labeled/discoverable in chat input

#### **Pipelines Tab:**
1. ❌ **Inline node editing** — Cannot edit node parameters directly
2. ❌ **Node creation UI** — Cannot add new nodes via UI
3. ❌ **Edge editing** — Cannot add/remove edges
4. ❌ **Pipeline save/publish** — No explicit save button
5. ❌ **Version control** — No pipeline versioning, branching, or diff view
6. ❌ **Node search/autocomplete** — Cannot search for node types by name

#### **Gate Tab:**
1. ❌ **Task creation** — No "new task" button visible
2. ❌ **Inline editing** — Cannot edit task title, description, status inline
3. ❌ **Bulk actions** — Cannot batch-edit selected tasks
4. ❌ **Dependency visualization** — No task dependency graph
5. ❌ **Assignee/owner field** — No visible assignee column
6. ❌ **Priority/severity** — No visible priority or severity indicators
7. ❌ **Custom fields** — No extensible metadata per task

#### **Knowledge Tab:**
1. ❌ **Note creation UI** — No "new note" button visible
2. ❌ **Note editing** — Note appears read-only; no edit/delete/rename buttons
3. ❌ **Inline graph editing** — Cannot add/delete nodes or edges
4. ❌ **Community selection** — Cannot click community in left panel to highlight
5. ❌ **Node detail sidebar** — Cannot click node to open properties panel
6. ❌ **Graph export** — No export to PNG, SVG, JSON
7. ❌ **Search relevance scoring** — No visible scores in search results

#### **Workflows Tab:**
1. ❌ **Workflow execution UI** — No visible run/execute button
2. ❌ **Workflow duplication** — No clone/copy button
3. ❌ **Step parameter editor** — Cannot edit step inputs/outputs inline
4. ❌ **Step validation** — No error indicators for invalid steps
5. ❌ **Workflow versioning** — No version history or draft/published state
6. ❌ **Conditional logic UI** — Cannot visually define conditions between steps

#### **Schedules Tab:**
1. ❌ **Cron editor** — No visual cron expression builder
2. ❌ **Template schedules** — No example schedules to clone from
3. ❌ **Run history** — No visible past run logs
4. ❌ **Dry-run button** — Cannot test schedule without executing
5. ⚠️ **Feature incomplete** — Tab exists but no data; untested

#### **Settings Tab:**
1. ❌ **Settings organization** — No grouped sections, all raw JSON
2. ❌ **Form UI** — No structured forms for MCP servers, embeddings preferences
3. ❌ **Validation feedback** — No real-time validation or error highlighting
4. ❌ **Configuration export** — No backup/export feature
5. ❌ **Reset to defaults** — No reset button
6. ❌ **Settings search** — No searchable settings index

### **Cross-Tab Gaps:**

1. ❌ **Keyboard shortcuts cheat sheet** — No visible help or shortcuts reference
2. ❌ **Context menu support** — No right-click context menus observed
3. ❌ **Drag-and-drop** — No visible drag-drop support for reordering or file attachment
4. ❌ **Undo/redo** — Limited undo/redo (disabled in most tabs except Settings)
5. ❌ **Find-and-replace** — No visible find/replace feature (only search in Knowledge and Workflows)
6. ❌ **Accessibility features** — No visible high-contrast mode, text scaling, or screen reader hints
7. ❌ **Collaboration/sharing** — No share, export, or invite UI visible across tabs
8. ❌ **Notification center** — Alt+T toggles notifications region, but no notifications visible in this session

### **UI/UX Gaps:**

1. ⚠️ **Loading states** — "Connecting to Copilot" message appears during session load, but no spinner or progress bar
2. ⚠️ **Empty states** — Schedules and other empty lists have minimal guidance
3. ⚠️ **Inline help** — No visible tooltips or help icons (e.g., "What is MRL?" button in Embeddings exists but is minimal)
4. ❌ **Breadcrumb navigation** — Breadcrumbs exist in some tabs but are disabled (not clickable)
5. ❌ **Tab state persistence** — Unknown if scroll position, filters, and selections persist across tab switches

---

## Summary Statistics

| Tab | Status | Notable Gaps |
|-----|--------|--------------|
| Home | ✅ Functional | Minimal (portal only) |
| Agents | ⚠️ Partial | Slash commands, voice, message controls |
| Pipelines | ⚠️ Partial | Node/edge editing, version control |
| Gate | ⚠️ Partial | Task creation, inline editing, dependencies |
| Knowledge | ⚠️ Partial | Note creation, graph editing, communities |
| Workflows | ✅ Functional | Execution UI, versioning |
| Schedules | ❌ Incomplete | Feature skeleton only, untested |
| Settings | ✅ Functional | Form UI, validation, organization |

**Total interactive elements across all tabs:** 200+  
**Missing core features:** 35+  
**Feature completeness:** ~60% (core surfaces present; advanced editing/UX features sparse)

---

## Session Notes

- CDP connection stable; all snapshots captured successfully
- App responds smoothly to navigation and interaction
- Accessibility tree well-formed with proper ARIA labels
- No UI crashes or errors observed
- Chat session connected and functional (messages/thinking visible)
- Pipeline visualization renders large DAGs without apparent performance issues
- Knowledge graph loaded and interactive (though no nodes visible in screenshot)
- Settings JSON editor fully functional with syntax highlighting

---

## Recommendations for Future Enhancement

1. **Prioritize Agents tab enhancements:** Slash commands, voice input, and message controls are table-stakes for modern AI clients
2. **Graph editing in Knowledge tab:** Add node/edge creation and deletion; this is a core workflow gap
3. **Task creation in Gate:** Add UI for creating new tasks; currently required external markdown editing
4. **Schedule templates:** Provide pre-built schedules to reduce friction
5. **Settings refactor:** Move from raw JSON to structured forms for MCP servers, embeddings, and core config
6. **Workflow execution:** Add visible run button and real-time execution monitoring
7. **Search across tabs:** Unified search that spans agents, tasks, workflows, and knowledge
8. **Keyboard shortcuts:** Implement and expose common shortcuts (Cmd+K for search, Cmd+/ for help, etc.)

