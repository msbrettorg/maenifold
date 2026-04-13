# Product Requirements Document — Pipeline UI

## Vision

Pipeline UI is a **cognitive pipeline designer** — a visual tool for composing, executing, and operating thought graphs. It enables a human operator to orchestrate MCP tool calls and ACP agent dispatches into executable directed acyclic graphs where the data flowing along edges represents reasoning: tool outputs, agent analyses, thinking traces, and workflow conclusions.

The system is a **reasoning amplifier**, not a workflow runner. Nodes are reasoning steps. Edges are reasoning dependencies. Fork, join, rewind, and graft are cognitive operations on the thought graph. The UI makes this reasoning visible, inspectable, and operable.

## Execution Model

The pipeline engine has a clean two-model split:

- **MCP Tool Nodes** execute via direct `ToolRegistry` invocation — no agent involved. The pipeline runner is the MCP client. Tool nodes are deterministic, pre-configured tool calls with structured JSON input/output.
- **ACP Agent Nodes** (Prompt, Workflow, SequentialThinking) dispatch work to external agent processes via the ACP protocol. Agents receive assembled prompts containing predecessor context, ambient instructions, and node-specific prompts.

This MCP/ACP split is a first-class architectural concept. Tool nodes and agent nodes have different input mechanisms, different execution paths, and different output characteristics.

## Data Flow

Data flows through the pipeline along edges. The flow mechanism depends on the target node type:

| Flow | Mechanism | Status |
|------|-----------|--------|
| Any to ACP | Predecessor output assembled into downstream prompt | Works |
| Any to MCP Tool | Source output mapped to tool input params via edge mapping | Requires FR-16/FR-17 |

**Edge mappings** are optional, explicit, and user-configured. Keys are target parameter names; values are dot-paths into source output JSON. No auto-mapping, no type coercion, no template languages. Unresolved paths produce null — downstream tools fail with real errors. Errors are information.

## Cognitive Operations

- **Fork** — Explore an alternative reasoning path. Copies a subgraph into a new pipeline session. The fork's root nodes inherit predecessor context from the source pipeline so the alternative path starts with the same reasoning input.
- **Join** — Converge two reasoning paths. Merges two pipeline sessions via bridge edges, preserving all outputs from both paths for downstream synthesis.
- **Rewind** — Re-explore a reasoning step. Resets target and all downstream nodes to Pending. Previous session files are preserved as historical artifacts — non-destructive.
- **Graft** — Extend reasoning after seeing results. Adds new nodes and edges to a stopped pipeline. Grafted nodes connected to completed predecessors become immediately ready.

---

## Functional Requirements

### Design System (T-63)

- **FR-1**: System SHALL provide a design system with CSS custom properties for colors, spacing, typography, and component tokens that all UI components consume
- **FR-2**: System SHALL surface MCP transport errors and tool-level errors to the user with visible, dismissible banners. Errors SHALL be preserved in a persistent error log; errors SHALL NOT auto-dismiss without user action
- **FR-3**: System SHALL render node cards, inspector, and forms correctly in both light and dark modes
- **FR-4**: System SHALL support four pipeline node types: Workflow, Prompt, SequentialThinking, and Tool — each with type-specific fields, colors, icons, and editing surfaces
- **FR-5**: System SHALL expose all pipeline operations through the Pinia store: create, view, run, pause, resume, cancel, delete, graft, fork, join, dispatch, rewind, restart, updateNodeStatus
- **FR-6**: System SHALL provide a unified button system with consistent variants (primary, success, danger, pause, ghost, outline-danger) and states (hover, disabled, focus-visible, busy)
- **FR-7**: System SHALL provide a unified input system with consistent styling, focus rings, and accessibility attributes for all form fields

### Backend Contracts (T-63, amended T-64)

- **FR-8**: Backend SHALL use a consistent `agent` field name for Prompt, Workflow, and SequentialThinking node types and accept legacy `acpClient` for backwards compatibility. Tool nodes are exempt — see FR-13
- **FR-9**: Backend SHALL compute pipeline status correctly for all terminal states including abandoned nodes
- **FR-10**: Backend view response SHALL include all node-type fields (prompt, toolMode, totalThoughts, analysisType, agent) plus cognitive execution data per FR-20, FR-21, FR-22
- **FR-11**: Backend SHALL validate agent existence for Prompt and SequentialThinking nodes at creation time. Tool nodes SHALL NOT be subject to agent validation
- **FR-12**: Backend dispatch response SHALL include `type` discriminant and node configuration fields for all node types

### Tool Node Independence (T-64)

- **FR-13**: Tool node type definition SHALL make `agent` optional with no default. Existing pipelines with `agent` on tool nodes SHALL continue to deserialize. Tool node execution SHALL NOT be gated on ACP client availability
- **FR-14**: Frontend SHALL NOT render an agent field in AddNodeForm or PipelineInspector for tool node type. PipelineNodeCard SHALL NOT display an agent pill for tool nodes

### Edge Data Flow (T-64)

- **FR-15**: `PipelineEdge` SHALL support an optional `mapping` field (`HashMap<String, String>`): keys are target parameter names, values are dot-paths into source node output JSON. Edges without mappings remain valid topology. Field SHALL be skipped in serialization when None
- **FR-16**: Executor SHALL resolve edge mappings at dispatch time for tool nodes: collect incoming edges with mappings where source node is completed, resolve dot-paths against source node `state.output`, merge resolved values into `tool_args` (mapped values override static values). Unresolved paths produce `null` — no retry, no fallback
- **FR-17**: Frontend SHALL provide an EdgeMappingEditor component for configuring field mappings on edges. The editor SHALL show target node input parameters (from tool inputSchema for tool targets; informational for ACP targets). The editor SHALL handle empty states: tools not yet loaded, source node has no output, no mappings configured. Mapping is always optional — edges without mappings are valid
- **FR-18**: Edges with mappings SHALL display a data-flow indicator on the edge path. Edges to tool node targets without mappings SHALL display a warning indicator. Edge hover SHALL show a mapping summary (source field to target field)
- **FR-19**: Server-mode edge inspector SHALL display: source node output (when source is completed), mapping configuration, and resolved target input values

### Cognitive Visibility (T-64)

- **FR-20**: Backend view response SHALL include `assembled_prompt` for completed Prompt, Workflow, and SequentialThinking nodes — reconstructed from predecessor context + ambient prompt + node instruction
- **FR-21**: Backend view response SHALL include `thinking_trace` array for SequentialThinking nodes — each element containing `thought_number`, `text`, and `status`. Read from thinking session data via `session_id`
- **FR-22**: Backend view response SHALL include `workflow_steps` array for Workflow nodes — each element containing `step_name`, `status`, and `output`
- **FR-23**: Frontend SHALL provide a PromptAssemblyPreview component showing predecessor context, ambient prompt, and node instruction as visually separated, collapsible sections. Renders only when `assembled_prompt` data is present in the view response
- **FR-24**: Frontend SHALL provide a ThinkingTraceView component rendering sequential thoughts as numbered cards with per-thought status indicators and revision markers. Renders only when `thinking_trace` data is present in the view response
- **FR-25**: Frontend SHALL provide a WorkflowStepProgress component rendering workflow steps as a vertical timeline with per-step status indicators and step output display. Renders only when `workflow_steps` data is present in the view response

### Cognitive Operations (T-64)

- **FR-26**: Fork SHALL carry predecessor context into the forked pipeline: completed predecessor outputs from the source pipeline's fork-point predecessors SHALL be copied or referenced so forked root nodes can access the same reasoning input as the original path
- **FR-27**: Frontend SHALL validate graph structure before create and graft operations: cycle detection, unreachable node detection, dangling edge check. Validation errors SHALL be displayed as inline graph feedback, not deferred to backend error responses

### MCP Connection Resilience (T-68)

- **FR-40**: MCP client SHALL detect session expiry (HTTP 404 on `Mcp-Session-Id`) and automatically re-establish the session by re-running the `initialize` → `notifications/initialized` handshake, then retrying the failed request. Reconnection SHALL be transparent to callers — pending tool calls wait for reconnection rather than failing immediately
- **FR-41**: MCP client SHALL expose reactive connection state (`connected`, `reconnecting`, `error`) so the UI can display connection status. Pipeline polling SHALL pause during reconnection and resume automatically after session is re-established

### Agent Runtime Surfaces (Founder Directive — April 2026)

- **FR-42**: Runtime agent discovery surfaces SHALL render agent availability, capability metadata, and selection options from the backend-provided `ListAgents` catalog rather than raw persisted config. Persisted config MAY remain the authoring/write model and MAY be surfaced in settings or repair flows, but it SHALL NOT act as an independent runtime discovery read model
- **FR-43**: Runtime execution in `pipeline-ui` SHALL migrate toward backend-managed execution primitives (`AcpExec` and/or Rust ACP driver surfaces) and SHALL NOT continue expanding browser-owned ACP runtime responsibilities. Any retained browser ACP runtime behavior is migration debt, not target-state architecture
- **FR-44**: `PermissionDialog.vue` SHALL clamp to viewport dimensions and SHALL scroll internally when content exceeds available space. The dialog header and action controls SHALL remain visible while long code, raw payload, and location content scrolls within the dialog body

### Kinetic Lumina Design System (T-67)

- **FR-28**: System SHALL implement the Obsidian Engine design system with a surface hierarchy of five tonal levels (`surface` #131313, `surface_container_low` #1c1b1b, `surface_container` #201f1f, `surface_container_high` #2a2a2a, `surface_container_highest` #353534). Structural boundaries SHALL be defined solely through background color shifts — no 1px solid borders for sectioning (the "No-Line Rule")
- **FR-29**: System SHALL bundle and load three font families as Tauri assets: Space Grotesk (display/headlines), Inter (body/UI), JetBrains Mono (code/IDs). Typography tokens SHALL map `--font-headline`, `--font-sans`, and `--font-mono` to these families
- **FR-30**: System SHALL render as dark-only (no light mode). The light mode `:root` token block SHALL be removed. All components SHALL use dark palette tokens exclusively
- **FR-31**: System SHALL provide a glassmorphism CSS system for expanded/overlay states: `surface_variant` at 60% opacity with 24px `backdrop-filter: blur()`. Ghost borders SHALL use `outline_variant` at 15% opacity for high-density containment
- **FR-32**: Pipeline node cards SHALL use `surface_container_highest` body with no borders, a 4px vertical left "light bar" using the node type accent color, and glassmorphism on expanded state. Node type accent colors: Prompt=`primary` (#adc6ff), SequentialThinking=`secondary` (#d0bcff), Tool=`tertiary` (#4edea3), Workflow=`primary_container` (#4d8eff)
- **FR-33**: All `box-shadow` declarations SHALL be replaced with tonal layering or ambient glows (highly diffused: `0 24px 48px rgba(0,0,0,0.5)`). Active nodes SHALL emit a subtle outer glow using their functional accent color with 20px blur at 10% opacity

### Semantic Zoom (T-67)

- **FR-34**: System SHALL provide three-level semantic zoom: L0 (Graph Overview), L1 (Node Inspection), L2 (Session Dive). Zoom level SHALL be managed by a Pinia store (`useSemanticZoomStore`) with state machine transitions: L0→L1 on node double-click, L1→L2 on "Enter Session" action, reverse via Escape key
- **FR-35**: L0→L1 transition SHALL use a FLIP animation (350ms, `cubic-bezier(0.4, 0, 0.2, 1)`): capture source node `getBoundingClientRect()`, mount overlay via `<Teleport>`, animate from source rect to centered panel position. Non-focused nodes SHALL fade to 40% opacity with 2px CSS blur (300ms, 50ms stagger). Connected edges remain full opacity; others fade to 15%
- **FR-36**: L1 expanded card SHALL display: node header (ID, type pill, status, agent), predecessor context as collapsible sub-blocks, ambient prompt section, node instruction section, output (if completed), thinking trace (if SequentialThinking). "Enter Session →" button SHALL appear only for ACP node types with a `sessionId`
- **FR-37**: L2 session dive SHALL mount ChatView bound to the focused node's `sessionId`. A context breadcrumb SHALL show the node's position in the DAG as navigable pills. Inspector panel SHALL collapse. Minimap optional
- **FR-38**: Active edges SHALL render animated particles: 3px SVG circles traveling along edge `<path>` elements at ~120px/sec using `getPointAtLength()`. Particles use source node accent color at 70% opacity. Single `requestAnimationFrame` loop for all particles
- **FR-39**: Nodes SHALL display status animations: ready nodes pulse (CSS scale oscillation), active nodes emit animated border glow, completion triggers a 600ms radial ripple (accent color at 20% opacity fading to 0%)

---

## Non-Functional Requirements

- **NFR-1**: System SHALL be accessible: visible focus indicators on all interactive elements, aria-labels on icon buttons, aria-live on dynamic regions, keyboard navigation for node selection
- **NFR-2**: System SHALL maintain responsive render performance: no full-canvas re-render on inspector keystrokes, mermaid initialized once
- **NFR-3**: System SHALL be usable at viewport widths down to 900px with responsive layout breakpoints
- **NFR-4**: System SHALL use a consistent 5-step type scale and spacing system derived from design tokens, replacing ad-hoc pixel values
- **NFR-5**: System SHALL NOT emit console.log in production code paths
- **NFR-6**: New frontend components SHOULD target 300 lines of code as a complexity signal. This is guidance — correct implementation takes priority over line count
- **NFR-7**: System SHALL align with maenifold philosophy: no fake AI (no auto-mapping, no type coercion, no retry on mapping failure), no unnecessary abstractions, errors are information (preserved and inspectable, not hidden or auto-dismissed)
- **NFR-8**: Tests SHALL verify behavior, not just compilation. Requirements verified only by type-checking indicate type-level coverage; behavioral requirements additionally require integration or component tests that exercise real user flows per TESTING_PHILOSOPHY.md
- **NFR-9**: FLIP animation transitions SHALL complete within 500ms total (350ms geometry + 150ms content reveal). CSS transitions SHALL use `cubic-bezier(0.4, 0, 0.2, 1)` universally — not `ease` or `linear`
- **NFR-10**: Edge particle animation SHALL maintain 60fps with up to 50 concurrent particles. Implementation SHALL use a single `requestAnimationFrame` loop with imperative SVG updates — not reactive Vue template re-renders

---

## Reference

- Architecture: `memory://build/architecture/pipeline-t64-architectural-intent-analysis`
- Gap analysis: `memory://build/architecture/pipeline-t64-gap-analysis`
- Semantic zoom vision: `memory://build/architecture/pipeline-ui-semantic-zoom-vision-navigating-into-the-graph`
- Kinetic Lumina design system: `memory://pipeline-ui/the-kinetic-lumina-pipeline-ui-design-system-vision`
- T-67 research synthesis: `memory://build/sprints/pipelineui-sprint-t67-research-synthesis`
- Stitch comps: `projects/1113463130459108073` (Pipeline UI — Kinetic Lumina Implementation)
- Philosophy: `maenifold-rs/docs/ma/WHAT_WE_DONT_DO.md`, `TESTING_PHILOSOPHY.md`
