# Pipeline UI design: node types + per-node prompts

Status: design only  
Scope: `/Users/brett/src/ma-collective/pipeline-ui/`  
Source architecture note: `memory://build/architecture/pipeline-dataflow-v2-design`

## Goals

The pipeline UI should support three pipeline node types:

1. `workflow`
2. `prompt`
3. `sequentialThinking`

It also needs two distinct prompt layers:

- **Run-level prompt**: ambient context for the whole pipeline run
- **Per-node prompt/topic**: the node's own instruction payload

This document describes the UI changes needed without modifying production code.

---

## Design principles

1. **Keep the graph mental model stable**  
   The canvas, edge model, and inspector layout should still feel like the current product. The change is type-aware editing, not a full redesign.

2. **Use a discriminated node type model everywhere**  
   Composer, node card, inspector, and store should all branch off `node.type` instead of assuming `workflowId` exists.

3. **Make node type obvious at a glance**  
   Users should be able to tell node type from accent color, icon/label, and the secondary metadata shown on the card.

4. **Treat prompts as first-class data**  
   Workflow nodes now have a per-node `prompt`; prompt nodes use `prompt` as their primary instruction; sequential-thinking nodes use `topic` as their instruction payload.

5. **Stay backward compatible in the UI**  
   Older workflow-only definitions should be normalized to `type: 'workflow'` when loaded into the client.

---

## Information architecture changes

## 1) Composer toolbar: add a type-aware node creator

### Current

The toolbar is a single inline form:

- workflow ID input
- ACP client input
- Add Node button

### Proposed

Replace it with a compact **type-aware create form** with a shared top row and type-specific fields below.

### Toolbar layout

**Row 1**
- `Node type` select
  - Workflow
  - Prompt
  - Sequential Thinking
- `Node ID` input (optional; auto-generated if blank)
- `Add Node` button
- `Delete Selected`
- `↔ Layout LR`
- `↕ Layout TB`

**Row 2+ (dynamic by type)**

#### When type = Workflow
- `Workflow ID` input with datalist
- `ACP client` input
- `Node prompt / description` textarea

#### When type = Prompt
- `Agent` input
- `Tool mode` segmented toggle or select
  - Disabled
  - Allowed
- `Prompt template` textarea

#### When type = Sequential Thinking
- `Topic` input
- `Agent` input
- `Total thoughts` numeric input
- `Analysis type` input
- `Topic details / description` textarea

### Important note about sequential-thinking textarea

The Rust schema only exposes a single `topic: string` field, but the UX requirement asks for a textarea for every type.

Design recommendation:

- keep **Topic** as the short single-line label
- keep **Topic details / description** as a multiline draft-only field in the composer
- when the node is created, serialize to `topic` as:

```text
<Topic>

<Topic details>
```

If the details textarea is empty, serialize `topic` as the single-line topic value.

This preserves the backend schema while still giving the composer a textarea for richer node authoring.

### Validation rules

| Type | Required fields | Optional fields |
|---|---|---|
| Workflow | `workflowId` | `id`, `acpClient`, `prompt` |
| Prompt | `agent`, `prompt` | `id`, `toolMode` |
| Sequential Thinking | `topic`, `agent` | `id`, `totalThoughts`, `analysisType` |

### UX details

- The `Add Node` button label should stay generic to avoid shifting labels.
- The toolbar should keep its floating overlay style, but become a **2-row card** instead of a single inline strip.
- Only the fields for the selected type should render.
- Workflow keeps the existing datalist behavior.
- Prompt and Sequential Thinking do not use the workflow datalist.
- If `Node ID` is empty, generate one from type + timestamp, e.g.:
  - `workflow-1712345678901`
  - `prompt-1712345678901`
  - `thinking-1712345678901`

### Why this shape works

- It keeps creation fast.
- It avoids opening a modal for simple graph editing.
- It matches the current composer pattern while making room for type-specific fields.

---

## 2) PipelineNodeCard: visually differentiate node types

### Current

All nodes render the same card shape and only differ by workflow ID, ACP client, and runtime status.

### Proposed visual system

Keep the current card form factor, handles, and status strip, but add **type accent**, **type label**, and **type-specific body content**.

### Type accents

| Type | Accent | Suggested token |
|---|---|---|
| Workflow | Blue | `#2563eb` |
| Prompt | Green | `#16a34a` |
| Sequential Thinking | Purple | `#7c3aed` |

Use the accent color for:

- left status/type bar
- handle color
- small type pill in header
- subtle box shadow tint when selected/active

### Header content

**Header row**
- status emoji
- node title (`label || nodeId`)
- type pill on the right:
  - `Workflow`
  - `Prompt`
  - `Thinking`

### Type-specific secondary content

#### Workflow node card
- subtitle: `workflowId`
- metadata row:
  - ACP client badge if present
  - attempts badge if `attempts > 1`
- optional prompt preview row if `prompt` exists
  - clamp to 2 lines
  - lower emphasis than title

#### Prompt node card
- subtitle: `agent`
- prompt preview block:
  - first 2 lines of `prompt`
  - monospaced or slightly inset to signal template text
- metadata row:
  - tool mode badge (`tools off` / `tools on`)
  - attempts badge if present

#### Sequential-thinking node card
- subtitle: topic summary
- metadata row:
  - `agent` badge
  - `thoughts: N` badge if present
  - `analysisType` badge if present
- no long preview block unless topic is short enough; prefer compactness

### Status emoji mapping

Per requirement, keep the prominent runtime emoji behavior.

Primary mapping:
- `pending` → `⏳`
- `active` → `🔄`
- `completed` → `✅`
- `abandoned` → `⚠️`

Recommended to keep existing extended states too:
- `ready` → `🟡`
- `stuck` → `🚫`

### Text-based mockup

```text
┌─────────────────────────────────────┐
│▌ 🔄  spec-node              Prompt  │
│  researcher                           │
│  Summarize {{ nodes.research.output }}│
│  [tools off] [×2]                     │
└─────────────────────────────────────┘
```

```text
┌─────────────────────────────────────┐
│▌ ✅  implement             Workflow │
│  tdd-implement                        │
│  Node prompt preview…                 │
│  [swe]                                │
└─────────────────────────────────────┘
```

```text
┌─────────────────────────────────────┐
│▌ ⏳  architecture          Thinking │
│  Compare orchestration tradeoffs     │
│  [researcher] [thoughts: 6] [arch]   │
└─────────────────────────────────────┘
```

---

## 3) PipelineInspector: type-aware editing and viewing

### Current

Design mode assumes every node is a workflow node.

### Proposed

Keep the inspector split by mode:

- design overview
- design node
- design edge
- server overview
- server node

But make the **design node** and **server node** sections render per node type.

### Design mode inspector

#### Shared section for all node types
- `ID`
- `Type` (read-only pill or disabled select)

#### Workflow design inspector
Show:
- `Workflow ID` input + datalist
- `ACP client` input
- `Per-node prompt` textarea

Behavior:
- textarea edits `node.prompt`
- blank textarea serializes as `undefined`

#### Prompt design inspector
Show:
- `Agent` input
- `Tool mode` toggle
- `Prompt template` textarea
- `Resolved preview` panel

Resolved preview behavior:
- if predecessor outputs are available, replace `{{ nodes.X.output }}` in preview
- unresolved placeholders stay highlighted
- preview is read-only
- if no predecessor outputs are known, show empty state text:
  - `Preview available once upstream outputs exist.`

#### Sequential-thinking design inspector
Show:
- `Topic` textarea or large text input
- `Agent` input
- `Total thoughts` numeric input
- `Analysis type` input

Recommendation:
- in the inspector, use a **single multiline Topic field** instead of re-exposing the toolbar's split `topic + details` draft state
- once a node exists, the serialized `topic` should be edited as one field

### Server mode inspector

The runtime inspector should also become type-aware.

#### Shared runtime fields
- `Node ID`
- `Type`
- `Status`
- `Session`
- `Attempts`
- optional `Output preview` (if backend view includes `output`)

#### Workflow runtime inspector
Show:
- `Workflow ID`
- `ACP client`
- `Per-node prompt`
- `Output preview`

#### Prompt runtime inspector
Show:
- `Agent`
- `Tool mode`
- `Prompt template`
- `Resolved preview` if upstream outputs are available in the loaded view
- `Output preview`

#### Sequential-thinking runtime inspector
Show:
- `Topic`
- `Agent`
- `Total thoughts`
- `Analysis type`
- `Output preview`

### Node actions

Keep current runtime actions:
- Restart
- Rewind
- Complete
- Abandon

No type-specific action differences are needed in v1.

### Text-based mockup

```text
Inspector
────────────────────────
Node
ID
[prompt-1                ]

Type
[ Prompt ]

Agent
[ researcher              ]

Tool mode
(•) Disabled   ( ) Allowed

Prompt template
┌────────────────────────┐
│ Summarize              │
│ {{ nodes.research... }}│
└────────────────────────┘

Resolved preview
┌────────────────────────┐
│ Summarize              │
│ <resolved content...>  │
└────────────────────────┘
```

---

## 4) PipelineRunner: add run-level prompt input

### Current

Runner only shows action buttons and status.

### Proposed

Add a **run-level prompt textarea** above the action buttons.

### Placement

Inside `PipelineRunner.vue`, above `.control-bar`:

- label: `Run-level prompt`
- help text: `Optional ambient context prepended to every node at run time.`
- textarea rows: `3`

### Behavior

- value is bound to store state, not a local-only ref
- `Run` passes this value to `store.runPipeline(runPrompt)`
- `Pause`, `Resume`, `Cancel`, `Delete` do not mutate the prompt
- prompt persists while the current pipeline is loaded
- when `newPipeline()` is called, prompt resets to empty

### Empty / helper text

Show helper copy under the textarea:

```text
Execution order: predecessor context → run-level prompt → node prompt/topic.
```

### Text-based mockup

```text
Run-level prompt
┌──────────────────────────────────────────────┐
│ You are coordinating a design review.       │
│ Prefer concise output and call out risks.   │
└──────────────────────────────────────────────┘
Optional ambient context prepended to every node at run time.

[Create] [Run] [Pause] [Resume] [Cancel] [Delete]
```

---

## 5) TypeScript type definitions (`pipeline-types.ts`)

The UI should move from a single workflow-only `PipelineNode` shape to a discriminated union that matches the Rust design.

### Exact code

```ts
export type PipelineNodeType = 'workflow' | 'prompt' | 'sequentialThinking';

export type ToolMode = 'disabled' | 'allowed';

export type EdgeType = 'success' | 'always';

export type NodeStatus =
  | 'pending'
  | 'ready'
  | 'active'
  | 'completed'
  | 'abandoned'
  | 'stuck';

export type PipelineStatus = 'active' | 'paused' | 'cancelled' | 'completed' | 'stuck';

interface PipelineNodeBase {
  id: string;
  type: PipelineNodeType;
}

export interface WorkflowPipelineNode extends PipelineNodeBase {
  type: 'workflow';
  workflowId: string;
  acpClient?: string;
  prompt?: string;
}

export interface PromptPipelineNode extends PipelineNodeBase {
  type: 'prompt';
  prompt: string;
  agent: string;
  toolMode?: ToolMode;
}

export interface SequentialThinkingPipelineNode extends PipelineNodeBase {
  type: 'sequentialThinking';
  topic: string;
  agent: string;
  totalThoughts?: number;
  analysisType?: string;
}

export type PipelineNode =
  | WorkflowPipelineNode
  | PromptPipelineNode
  | SequentialThinkingPipelineNode;

export interface PipelineEdge {
  source: string;
  target: string;
  type: EdgeType;
}

export interface PipelineDefinition {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
}

interface ViewNodeBase {
  nodeId: string;
  type: PipelineNodeType;
  sessionId: string;
  status: NodeStatus;
  attempts: number;
  output?: string;
}

export interface WorkflowViewNode extends ViewNodeBase {
  type: 'workflow';
  workflowId: string;
  acpClient?: string;
  prompt?: string;
}

export interface PromptViewNode extends ViewNodeBase {
  type: 'prompt';
  prompt: string;
  agent: string;
  toolMode?: ToolMode;
}

export interface SequentialThinkingViewNode extends ViewNodeBase {
  type: 'sequentialThinking';
  topic: string;
  agent: string;
  totalThoughts?: number;
  analysisType?: string;
}

export type ViewNode =
  | WorkflowViewNode
  | PromptViewNode
  | SequentialThinkingViewNode;

export interface PipelineView {
  pipelineId: string;
  status: PipelineStatus;
  nodes: ViewNode[];
  edges: PipelineEdge[];
  ready: string[];
  active: string[];
  completed: string[];
  mermaid: string;
  running: boolean;
}

export interface PipelineListItem {
  pipelineId: string;
  status: PipelineStatus;
  nodeCount: number;
  completedCount: number;
}

export interface LogEntry {
  nodeId: string;
  from: string;
  to: string;
  timestamp: number;
}

export interface WorkflowComposerDraft {
  type: 'workflow';
  nodeId: string;
  workflowId: string;
  acpClient: string;
  prompt: string;
}

export interface PromptComposerDraft {
  type: 'prompt';
  nodeId: string;
  prompt: string;
  agent: string;
  toolMode: ToolMode;
}

export interface SequentialThinkingComposerDraft {
  type: 'sequentialThinking';
  nodeId: string;
  topic: string;
  topicDetails: string;
  agent: string;
  totalThoughts: number | null;
  analysisType: string;
}

export type ComposerNodeDraft =
  | WorkflowComposerDraft
  | PromptComposerDraft
  | SequentialThinkingComposerDraft;

export interface PromptPreviewResult {
  resolved: string;
  unresolvedRefs: string[];
  hasAllInputs: boolean;
}
```

### Notes on schema alignment

- `PipelineNode` matches the Rust `PipelineNode` + `PipelineNodeKind` split, flattened for TypeScript ergonomics.
- `type: 'sequentialThinking'` matches the Rust serde `rename_all = "camelCase"` representation.
- `output?: string` is included on `ViewNode` to match the architecture note's `NodeState.output` extension and enable prompt preview / output inspection.
- `ComposerNodeDraft` is intentionally UI-only and does **not** need to exist in Rust.

### Backward compatibility strategy in the client

When loading older workflow-only nodes that have no `type`, normalize them at the store boundary to:

```ts
{ type: 'workflow', ...legacyNode }
```

That keeps the rest of the UI purely discriminated-union based.

---

## 6) Pinia store changes (`src/stores/pipeline.ts`)

The store should own both **creation drafts** and **run prompt state** so the UI remains consistent across composer, inspector, and runner.

### New state

```ts
const runPrompt = ref('');

const composerDraft = ref<ComposerNodeDraft>({
  type: 'workflow',
  nodeId: '',
  workflowId: '',
  acpClient: '',
  prompt: '',
});
```

Recommended additional derived state/helpers:

- `nodeOutputsById` — computed map of `nodeId -> output`
- `canAddDraftNode` — computed validation flag
- `normalizedLocalDefinition` — optional internal helper if legacy data is ever loaded

### New actions

#### `setComposerNodeType(type: PipelineNodeType)`
Switches the draft type and resets only the type-specific fields.

#### `resetComposerDraft(type?: PipelineNodeType)`
Resets the draft after add or when the user changes type.

#### `addNodeFromDraft()`
Builds a typed `PipelineNode` from `composerDraft` and appends it to `localDefinition.nodes`.

Rules:
- auto-generate ID if blank
- serialize sequential-thinking `topic + topicDetails` into `topic`
- trim empty optional fields to `undefined`

#### `updateLocalNode(nodeId: string, patch: Partial<PipelineNode>)`
A type-aware node patcher used by the inspector.

Recommendation:
- implement with a `map()` over `localDefinition.nodes`
- if the ID changes, also rewrite connected edges exactly as current logic does

#### `normalizeNode(node: unknown): PipelineNode`
Converts legacy workflow-only nodes into typed nodes.

#### `setRunPrompt(value: string)`
Keeps runner input in store state.

#### `resolvePromptPreview(template: string): PromptPreviewResult`
Replaces `{{ nodes.X.output }}` from current known outputs.

Behavior:
- use `currentView.nodes[].output` when available
- if `output` is missing, collect unresolved refs
- do not throw; return preview metadata for the inspector to render

### Existing actions that should change

#### `newPipeline()`
Also resets:
- `runPrompt`
- `composerDraft`

#### `loadPipeline()`
Normalize incoming node types if needed.

#### `runPipeline(prompt?: string)`
Call with `runPrompt.value` from the runner.

#### `graftNodes(nodes, edges)`
Should accept typed nodes, not workflow-only nodes.

### What does **not** need to move into the store

- currently selected node/edge IDs can remain in parent component state
- mermaid rendering can remain local to `PipelineRunner.vue`

---

## Component-by-component change list

## `src/components/PipelineComposer.vue`

### Changes
- add `node type` select
- add shared `nodeId` draft field
- replace current workflow-only creator inputs with type-aware form
- pass richer node data into Vue Flow node `data`
- update `pipelineNodesToFlow()` to branch by node type
- update `emitDefinitionUpdate()` to emit typed nodes instead of `{ id, workflowId, acpClient }`
- update `syncFromView()` to preserve prompt/prompt-node/thinking-node fields from server view
- keep workflow datalist only for workflow nodes

### Complexity note
This is the highest-impact component because it currently hardcodes `workflowId` assumptions in both design and view sync paths.

---

## `src/components/PipelineNodeCard.vue`

### Changes
- extend `data` typing to accept discriminated node payloads
- compute accent color by `data.type`
- compute subtitle/preview/badges by type
- switch abandoned emoji from `❌` to `⚠️`
- keep ready/stuck support
- optionally add a type pill in header

---

## `src/components/PipelineInspector.vue`

### Changes
- replace workflow-only design node editor with type-aware rendering
- add per-node prompt textarea for workflow nodes
- add prompt template + resolved preview for prompt nodes
- add sequential-thinking fields
- use store helper for prompt preview resolution
- in runtime mode, show type-specific metadata and `output` if available

---

## `src/components/PipelineRunner.vue`

### Changes
- add run-level prompt textarea above controls
- bind textarea to store state
- pass `store.runPrompt` into `store.runPipeline(...)`
- keep control buttons unchanged
- add helper copy explaining prompt assembly order

---

## `src/lib/pipeline-types.ts`

### Changes
- replace workflow-only node types with discriminated unions
- add `ToolMode`
- add `output?: string` to view nodes
- add UI-only composer draft types
- keep edge/log/list types mostly unchanged

---

## `src/stores/pipeline.ts`

### Changes
- add `runPrompt`
- add `composerDraft`
- add type-aware node creation/edit helpers
- add legacy node normalization
- add prompt preview helper based on node outputs
- update any actions that assume every node has `workflowId`

---

## Mockup descriptions

## A. Composer toolbar mockup

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ Type [Prompt ▾]  Node ID [prompt-1          ]  [Add Node] [Delete Selected]│
│ Agent [researcher         ]  Tool mode [Disabled ▾]                        │
│ Prompt template                                                          │
│ ┌───────────────────────────────────────────────────────────────────────┐ │
│ │ Summarize {{ nodes.research.output }} for the implementation agent.  │ │
│ └───────────────────────────────────────────────────────────────────────┘ │
│ [↔ Layout LR] [↕ Layout TB]                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## B. Inspector mockup

```text
┌──────────────────────────────┐
│ Node                         │
│ ID                           │
│ [workflow-1               ]  │
│ Type                         │
│ [ Workflow ]                 │
│ Workflow ID                  │
│ [tdd-implement            ]  │
│ ACP client                   │
│ [swe                      ]  │
│ Per-node prompt              │
│ ┌──────────────────────────┐ │
│ │ Implement the approved   │ │
│ │ design and note risks.   │ │
│ └──────────────────────────┘ │
└──────────────────────────────┘
```

## C. Runner mockup

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Run-level prompt                                                        │
│ ┌──────────────────────────────────────────────────────────────────────┐ │
│ │ You are working on a UI design review. Keep outputs concise.        │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
│ Optional ambient context prepended to every node at run time.           │
│                                                                          │
│ [Create] [Run] [Pause] [Resume] [Cancel] [Delete]                       │
│ Design mode — 3 nodes, 2 edges                                           │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## File change list + estimated line count

| File | Change summary | Estimated delta |
|---|---|---:|
| `src/components/PipelineComposer.vue` | type picker, dynamic toolbar fields, typed node mapping | 140-220 lines |
| `src/components/PipelineNodeCard.vue` | node union rendering, color system, preview/badges | 70-120 lines |
| `src/components/PipelineInspector.vue` | type-aware inspector sections, prompt preview, workflow prompt editing | 180-280 lines |
| `src/components/PipelineRunner.vue` | run-level prompt textarea + binding | 35-70 lines |
| `src/lib/pipeline-types.ts` | discriminated unions + view/output/draft types | 90-140 lines |
| `src/stores/pipeline.ts` | draft state, runPrompt state, normalization, type-aware node actions | 120-200 lines |

### Optional / follow-on

| File | Why it may change | Estimated delta |
|---|---|---:|
| `src/composables/usePipelineMcp.ts` | only if response normalization is needed for legacy nodes or new view payloads | 10-30 lines |

---

## Recommended implementation order

1. update `pipeline-types.ts`
2. update store state/actions
3. update composer creation flow
4. update node card rendering
5. update inspector rendering and preview logic
6. update runner run-level prompt input

This order minimizes type breakage and keeps the UI functional as each layer is upgraded.

---

## Non-goals for this design

- no backend changes are specified here
- no production source files are modified by this task
- no modal-based node editor redesign
- no conditional-edge UI
- no structured output browser beyond plain text `output`

---

## Grounding

This design is grounded in:

- `memory://build/architecture/pipeline-dataflow-v2-design`
- current UI files in `pipeline-ui/src/components`, `pipeline-ui/src/lib`, and `pipeline-ui/src/stores`
- current Rust pipeline types in `crates/maenifold-core/src/workflow/pipeline/types.rs`
