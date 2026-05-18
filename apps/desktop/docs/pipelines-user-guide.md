# Pipeline Designer User Guide

## What pipelines are

A pipeline is a small directed graph of work — each box (a **node**) runs one thing (an agent prompt, a workflow, a tool call, or a sequential-thinking session) and the arrows (**edges**) decide what runs next. Use a pipeline when one prompt isn't enough: when you want to draft, then critique, then revise; or research, then peer-review; or fan a problem out across several agents and collect the results.

Pipelines persist as sessions, so you can stop, inspect, rewind, fork, and resume them — they are not one-shot scripts.

## Opening the Pipelines tab

The Pipelines tab lives in the main app shell alongside Memory, Workflows, and Gate. Click it to open the pipeline workspace.

On first launch (no pipelines saved, design mode off) the canvas is replaced by a **template gallery** plus a "Browse existing pipelines" button. See [components/PipelineView.vue](apps/desktop/src/components/PipelineView.vue) and [components/TemplateGallery.vue](apps/desktop/src/components/TemplateGallery.vue).

## Anatomy of the editor

```
┌──────────────────────────────────────────────────────────────────────┐
│ Breadcrumbs                                                          │
├──┬───────────────────────────────────────────────────────────┬───────┤
│  │ ┌──────────────── Toolbar (top-right) ───────────────────┐│       │
│  │ │ Save status · Design toggle · Action bar · Layout · Fit ││       │
│I │ │ View · Palette                                          ││ Node  │
│c │ └─────────────────────────────────────────────────────────┘│ Insp- │
│o │                                                            │ ector │
│n │                       Canvas                               │ (when │
│  │                  (Vue Flow graph)                          │  a    │
│R │                                                            │ node  │
│a │                                                            │  is   │
│i │                                                            │ sel-  │
│l │                                                            │ ected)│
│  │                                                            │       │
│  ├────────────────────────────────────────────────────────────┤       │
│  │ Runner · status line · run history · event log             │       │
│  └────────────────────────────────────────────────────────────┴───────┤
└──────────────────────────────────────────────────────────────────────┘
```

- **Icon rail** (far left): switch between draft/loaded pipelines, open the pipeline flyout, start a new pipeline. See [components/PipelineIconRail.vue](apps/desktop/src/components/PipelineIconRail.vue).
- **Toolbar** (top-right of canvas): save status, design/done toggle, run/pause/cancel actions, layout controls. See [components/PipelineComposerToolbar.vue](apps/desktop/src/components/PipelineComposerToolbar.vue).
- **Canvas**: the graph itself. Nodes and edges. See [components/PipelineComposer.vue](apps/desktop/src/components/PipelineComposer.vue).
- **Inspector** (right): appears when you double-click a node. See [components/PipelineNodeInspector.vue](apps/desktop/src/components/PipelineNodeInspector.vue).
- **Runner** (bottom, collapsible): status line, run history list, scrolling event log. See [components/PipelineRunner.vue](apps/desktop/src/components/PipelineRunner.vue).

## Creating a new pipeline

You have two paths.

### From a template (recommended)

The template gallery is shown when there is no loaded pipeline and the canvas is empty.

1. In the template gallery pick a template.
2. Fill in the required input(s) (most templates ask for a topic).
3. Click the create button — the pipeline is instantiated, nodes are stamped with display labels, and you land in the editor.

The four universal templates (no extra capabilities required) are defined in [lib/pipeline-templates.ts](apps/desktop/src/lib/pipeline-templates.ts):

| Template | What it loads |
|---|---|
| **Research & Verify** | Two workflow nodes: `scientific-method` → `peer-review` (success edge). Asks for a topic. |
| **Think & Critique** | A `sequentialThinking` node (8 thoughts) → `critical-thinking` workflow. Asks for a topic. |
| **Draft & Refine** | Three nodes: `prompt` draft → `falsification` workflow → `prompt` revise. Asks for a topic. |
| **Explore & Synthesize** | Two workflow nodes: `agentic-research` → `crta` synthesis. Asks for a topic. |

Additional enterprise templates appear only when their required MCP tools are discovered.

### From scratch

1. Click the **+ New** action in the icon rail (or in the sidebar header when shown).
2. Design mode turns on automatically.
3. Add nodes via the palette form on the left (or right-click the canvas → Add).
4. Draw edges between nodes (see [Connecting nodes](#connecting-nodes-edges)).
5. Click **Save pipeline** in the toolbar to persist it.

## Opening an existing pipeline

1. Click the **Pipelines** icon in the icon rail to open the pipeline flyout. See [components/PipelineFlyout.vue](apps/desktop/src/components/PipelineFlyout.vue).
2. Type in the search box to filter by name or id.
3. Click a pipeline row to load it.

> ⚠ Currently inoperative: in production builds the "Browse existing pipelines" launcher button on the empty-state screen ([components/PipelineView.vue](apps/desktop/src/components/PipelineView.vue)) does not open the flyout when clicked. Workaround: open the flyout from the icon rail.

> ⚠ Currently inoperative: clicking a pipeline row in the flyout does not load the pipeline. Workaround: load via the sidebar list (when visible), or via the MCP `pipeline` tool with `sessionId` + `view=true`.

## Node types

Pipelines support four node types. Add any of them via the palette form ([components/AddNodeForm.vue](apps/desktop/src/components/AddNodeForm.vue)) or the canvas right-click menu.

### Workflow node

- **What it does**: runs a saved Maenifold workflow against a specified ACP agent.
- **Required**: `workflowId`, `acpClient` (the agent that will run it).
- **Optional**: `prompt` (seed text), `agentConfig`, `adopt` (role/color/perspective), `requiresRole`, `model`, `cwd`.
- **When to use**: you want a structured, multi-step procedure (e.g. `scientific-method`, `peer-review`) executed end-to-end by one agent.
- **Example**: `workflowId: scientific-method`, `acpClient: claude-opus`, `prompt: "Investigate cache eviction strategies"`.

### Prompt node

- **What it does**: sends a single freeform prompt to an agent.
- **Required**: `prompt`, `agent`.
- **Optional**: `toolMode` (`disabled` to forbid tool calls), `sessionRef` (attach to an existing ACP session), `model`, `adopt`, `requiresRole`.
- **When to use**: you want one focused turn — a draft, a summary, a classification — without workflow scaffolding.
- **Example**: `prompt: "Summarize {{input}} in 5 bullets"`, `agent: claude-haiku`, `toolMode: disabled`.

### SequentialThinking node

- **What it does**: runs the maenifold sequential-thinking tool — the agent emits a numbered chain of thoughts that build on each other.
- **Required**: `topic`, `agent`, `totalThoughts`.
- **Optional**: `analysisType`, `model`, `adopt`.
- **When to use**: open-ended reasoning where you want the model to deliberate before answering.
- **Example**: `topic: "Evaluate tradeoffs of X vs Y"`, `agent: claude-opus`, `totalThoughts: 8`.

### Tool node

- **What it does**: invokes a single MCP tool with a JSON argument blob.
- **Required**: `toolName`, `toolArgs` (valid JSON object).
- **Optional**: none.
- **When to use**: deterministic side-effects — read a resource, write a memory note, kick off a task.
- **Example**: `toolName: memory`, `toolArgs: { "operation": "write", "path": "/memories/notes.md", "content": "…" }`.

Canonical capability shape: [crates/maenifold-core/src/mcp/schema/pipeline_params.rs](crates/maenifold-core/src/mcp/schema/pipeline_params.rs) and [assets/usage/tools/pipeline.md](assets/usage/tools/pipeline.md).

## Connecting nodes (edges)

In design mode, each node has connection handles on its left/right (LR layout) or top/bottom (TB layout) edges. Drag from a source handle to a target handle to draw a new edge.

> ⚠ Currently inoperative: in production rendering, node connection handles (the small dots from [components/PipelineNodeCardPorts.vue](apps/desktop/src/components/PipelineNodeCardPorts.vue)) are not visible or interactive on canvas nodes. You cannot draw a new edge by dragging. Workaround: build the graph from a template, or use the MCP `pipeline` tool with `definition.edges` to set the topology.

### Edge types

Set via the edge config dialog ([components/DesignNodeConfigEdges.vue](apps/desktop/src/components/DesignNodeConfigEdges.vue)). Defined in [lib/pipeline-types-edge.ts](apps/desktop/src/lib/pipeline-types-edge.ts).

| Type | Fires when | Use for |
|---|---|---|
| **success** | The source node completes successfully (default for new edges). | The common case — proceed to the next step. |
| **always** | The source node finishes in any non-pending state (completed, abandoned, stuck). | Cleanup / finalization steps that must run regardless of upstream outcome. |
| **conditional** | The source node's output JSON has a field matching the rule. Requires `path` (e.g. `result.status`) and `equals` (a JSON literal — string, number, or boolean, with type selector). | Branching based on the source node's structured output. |

### Field mapping

A mapping copies one field from the upstream node's output into a named input on the downstream node, so the downstream prompt or tool args can reference it. Configured per-edge in the edge config dialog via [components/EdgeMappingEditor.vue](apps/desktop/src/components/EdgeMappingEditor.vue).

To add a mapping row: open the edge config dialog (double-click the edge), find the **Mapping** section, click the add-row control, and provide a source path and target key.

Example: a mapping of `result.summary` → `input` makes the upstream summary available as `{{input}}` in the downstream prompt.

## Layout controls

In the canvas toolbar ([components/PipelineComposerToolbar.vue](apps/desktop/src/components/PipelineComposerToolbar.vue)):

- **↔ Layout LR** — auto-arrange the graph left-to-right.
- **↕ Layout TB** — auto-arrange top-to-bottom.
- **⊞ Fit View** — zoom and pan to fit all nodes (design mode only).
- **+ Palette / ✕ Palette** — show or hide the left-hand add-node form (design mode only).
- **Delete Selected** — destructive button, appears only when something is selected in design mode.

## Designing vs running (design mode toggle)

The **Design / Done** button (top-right toolbar, see [components/PipelineDesignToggle.vue](apps/desktop/src/components/PipelineDesignToggle.vue)) switches between two modes:

- **Design mode** — you can add, move, configure, and delete nodes and edges; the palette and Fit View buttons appear; the canvas right-click menu offers "add node" actions.
- **Done (run) mode** — nodes become read-only; the action bar shows Run / Pause / Cancel / Rewind / Delete depending on pipeline status; the right-click menu offers Restart / Rewind / Fork / Graft / Promote actions.

If you toggle from Design to Done with unsaved changes, you'll be asked to confirm discarding them.

## Saving

A pipeline is **not** auto-saved while you design it. You must click **Save pipeline** in the toolbar ([components/PipelineSaveStatusBadge.vue](apps/desktop/src/components/PipelineSaveStatusBadge.vue)).

The save badge shows:

- **Unsaved changes** (warning) — draft has been edited but not persisted.
- **Saving…** (brand) — write in progress.
- **Saved** (success) — last write succeeded.
- **Error** (danger) — last write failed; the runner panel will expand with the error.

If you switch tabs or load another pipeline with unsaved changes, a confirmation dialog asks whether to discard them. Closing the window with unsaved changes triggers the browser/Electron beforeunload prompt.

## Running a pipeline

1. Make sure you're in **Done** (run) mode. Save first if you have draft edits.
2. Locate the pipeline action bar in the toolbar ([components/PipelineActionBar.vue](apps/desktop/src/components/PipelineActionBar.vue)).
3. Optionally set ambient inputs/prompt via the runner area before running. The Run action passes them through to agent-dispatched nodes.
4. Click **▶ Run**. The action is only present when the pipeline is in `active` state (created but not yet started, or restarted after rewind) and not already running.
5. The runner panel at the bottom expands automatically when a node becomes active or the pipeline gets stuck — watch the status line, the run history list, and the streaming event log.

## Run history panel

In the runner ([components/PipelineRunHistoryPanel.vue](apps/desktop/src/components/PipelineRunHistoryPanel.vue)) each row is a node attempt with a status chip and an outcome chip.

Sort selector controls order:

- **Newest activity** (default) — most recently started first.
- **Oldest activity** — chronological.
- **Node name** — alphabetical by node id.

Event types in the event log:

| Event | Meaning |
|---|---|
| **loaded** | The pipeline was opened and a node's stored state was rehydrated (not a fresh transition). |
| **active** | A node has been claimed by the scheduler and is currently working. |
| **completed** | A node finished successfully; its output is available downstream. |
| **failed** | A node ended with an error. |
| **abandoned** | A node gave up — usually after exhausting retries or hitting an unrecoverable condition. |
| **stuck** | A node cannot make progress (e.g. agent disconnected, dependency missing). It is not making attempts and needs user intervention. |
| **cancelled** | The pipeline (or this node within it) was cancelled by the user. |

## Cancelling / pausing / rewinding

All of the following are reached either through the **action bar** in the toolbar or the **right-click context menu** on the canvas ([components/PipelineComposerContextMenu.vue](apps/desktop/src/components/PipelineComposerContextMenu.vue), server-mode variant in [components/PipelineComposerContextMenuServer.vue](apps/desktop/src/components/PipelineComposerContextMenuServer.vue)).

### Cancel

Stops the pipeline. Active nodes are signalled to stop and no new nodes are dispatched.
**Path:** Toolbar action bar → **Cancel** (visible when status is `active` or `paused` or `stuck`).

### Rewind a single node

Resets one node back to ready and resets every node downstream of it. Useful when you want to re-run a step after editing its config or inputs.
**Path:** Right-click the node → **Rewind**. Available when the node is `completed`, `abandoned`, or `stuck`.

### Rewind all

Resets every rewindable node (any `completed`/`abandoned`/`stuck` node) so the pipeline can be re-run from the start.
**Path:** Toolbar action bar → **Rewind** (visible when at least one node is rewindable), or right-click the canvas → **↩ Rewind**.

### Restart abandoned node

Recreates the underlying session for a single node so it can be retried, without resetting downstream state.
**Path:** Right-click the node → **Restart**. Available when the node is `completed`, `abandoned`, or `stuck`.

There is also an inline **Resume from this node** button on the node card itself ([components/PipelineNodeCard.vue](apps/desktop/src/components/PipelineNodeCard.vue)) for `abandoned`/`stuck` nodes when the pipeline isn't running.

### Fork at a node

Clones the chosen node and its downstream subgraph into a brand-new pipeline session, leaving the original untouched.
**Path:** Right-click the node → **Fork**. Available when the node is `completed` and has output.

### Graft new nodes into a running pipeline

Adds new nodes and edges to the current pipeline while it is non-terminal (active/paused/stuck), so you can extend the graph without starting over.
**Path:** Toolbar **Graft nodes** button when the runner is expanded, or right-click the canvas → **⑂ Graft**. Opens [components/GraftNodesDialog.vue](apps/desktop/src/components/GraftNodesDialog.vue).

### Promote node output to memory

Saves a completed node's output as a Maenifold memory note so it can be linked and reused later.
**Path:** Right-click the canvas → **✦ Promote to Memory** (visible when at least one completed node has output). Opens [components/PromotionDialog.vue](apps/desktop/src/components/PromotionDialog.vue).

## Deleting / trash

### Delete a pipeline from the sidebar

In the pipeline sidebar ([components/PipelineSidebar.vue](apps/desktop/src/components/PipelineSidebar.vue)) hover a row and use the delete control. A confirmation dialog appears.

### Delete from the action bar

In Done mode the toolbar action bar shows **Delete** when the pipeline is in a terminal state (`completed`, `cancelled`, `abandoned`, `stuck`) or `active`-not-running. Confirmed deletion is required.

### 30-day quarantine

Deleted pipelines are not immediately purged — the backend `pipeline` tool quarantines them for 30 days before final removal, so a recent delete can be recovered.

> ⚠ Currently inoperative: there is no UI surface to browse the trash or restore a quarantined pipeline. Restore must be done via the MCP `pipeline` tool with `sessionId` + `restore=true`. See [assets/usage/tools/pipeline.md](assets/usage/tools/pipeline.md).

## Inspector panels

### Node config dialog (double-click a node)

Opens [components/DesignNodeConfigModal.vue](apps/desktop/src/components/DesignNodeConfigModal.vue). Fields per type:

- **Workflow** — workflow id (with autocomplete from your saved workflows), acpClient (agent), prompt, agentConfig, adopt (role/color/perspective), requiresRole, model, cwd.
- **Prompt** — prompt text, agent, optional prompt template, agentConfig, model, tool mode, session reference, adopt, requiresRole.
- **SequentialThinking** — topic, agent, totalThoughts, analysisType, agentConfig, model, adopt.
- **Tool** — tool name (from the discovered MCP catalog), JSON `toolArgs` editor (validated live), agent (for context), agentConfig.

### Edge config dialog (double-click an edge)

Opens the same modal in edge mode and shows [components/DesignNodeConfigEdges.vue](apps/desktop/src/components/DesignNodeConfigEdges.vue):

- **Path** — read-only `source → target` summary.
- **Edge type** — Success / Always / Conditional (see [edge types](#edge-types)).
- **Conditional fields** (when type is Conditional) — `path` (dotted JSON pointer into the source node's output), `equals` literal, value type selector (string/number/boolean).
- **Mapping table** — zero or more `source path → target key` rows that copy fields from upstream output into downstream inputs.

### Read-only inspector (run mode)

Single-click a node in any mode to select it; in run mode this opens the read-only side inspector ([components/PipelineNodeInspector.vue](apps/desktop/src/components/PipelineNodeInspector.vue)) showing node id, status, attempts, session id, prompt, config, and an output preview.

> ⚠ Currently inoperative: in design mode, single-clicking a node selects it but the only visible affordance is a **Delete Selected** button in the toolbar — the read-only inspector does not open from a single click. Use **double-click** to open the node config dialog instead.

## Keyboard shortcuts

Bindings actually wired in [composables/useComposerCanvas.ts](apps/desktop/src/composables/useComposerCanvas.ts) and [composables/useComposerCanvasHandlers.ts](apps/desktop/src/composables/useComposerCanvasHandlers.ts):

| Keys | Action | Where |
|---|---|---|
| **Enter** or **Space** on a focused node | Select the node | Canvas |
| **Backspace** | Delete selected nodes/edges | Canvas, design mode only |
| **Esc** | Close the pipeline flyout (and restore focus to the icon rail) | When the flyout is open |
| **Tab / Shift+Tab** | Cycle focus within the flyout | Flyout |
| **↑ / ↓** on a pipeline row | Move focus between pipeline rows | Flyout |
| **Home / End** on a pipeline row | Jump to first / last pipeline row | Flyout |

## Troubleshooting

### Pipeline stuck

A node has gone into `stuck` state — usually the agent disconnected or a dependency vanished. Open the runner panel to read the event log for the reason, then either right-click the node → **Restart** (to recreate its session and retry) or **Rewind** (to reset it and downstream nodes for a fresh run).

### Node abandoned

The node gave up after retries. Inspect the failure summary in the runner. If the cause was transient, **Restart** the node. If the upstream input was wrong, edit it (re-enter design mode, fix the config, save) and then **Rewind** from that node.

### Pipeline won't start

Common causes:

- No agent configured — templates require at least one agent name to be present in your config.
- Required template input left empty.
- Pipeline isn't saved yet (Run is not offered for unsaved drafts — save first).
- Pipeline is already running, completed, or cancelled — the Run action is only present for `active`-not-running.

### "Direct inference not configured" in the run history

The pipeline tried to resolve a gateway intent through direct inference but the daemon has no direct-inferencing backend configured. See the gateway intent resolver path in the constitution ("GatewayIntent has exactly two resolver paths") and configure a direct-inferencing model in `maenifold.json`, or use an MCP client that exposes `sampling/createMessage`.

## Glossary

- **DAG** — Directed Acyclic Graph. The shape of a pipeline: nodes connected by edges with a direction, no cycles.
- **Node** — A single unit of work in a pipeline: workflow, prompt, sequential-thinking, or tool.
- **Edge** — A directed connection from one node to another, with a type (success / always / conditional) and optional field mapping.
- **Mapping** — A rule on an edge that copies a field from the upstream node's output into a named input on the downstream node.
- **Fan-out** — A node that has multiple outgoing edges, causing several downstream nodes to be eligible at once.
- **Dispatch** — The scheduler claiming a ready node and starting it. Bounded by `maxConcurrency`.
- **Ready** — A node whose upstream predecessors have completed (per their edge types) and whose conditions are met; eligible to be dispatched.
- **Active** — A node currently being worked on.
- **Abandoned** — A node that gave up after retries.
- **Stuck** — A node that cannot make progress without user intervention (no further attempts will be made automatically).
- **Quarantine** — A 30-day window after deletion during which a pipeline can be restored.
- **Graft** — Add new nodes/edges to a live pipeline.
- **Fork** — Clone a node and its downstream subgraph into a new pipeline session.
- **Rewind** — Reset a node (and everything downstream) so it can run again.
- **Promote** — Save a completed node's output into Maenifold memory as a note.
