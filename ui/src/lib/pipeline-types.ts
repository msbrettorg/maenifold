// Pipeline domain types for the Pipelines tab.
//
// Single source of truth for the shapes the `maenifold.workflow` Pipeline tool
// and the `pipeline://` MCP resources emit. The backend mixes camelCase (most
// fields) with snake_case (the cognitive surfaces: `assembled_prompt`,
// `workflow_steps`, `thinking_trace`, and their nested members). Per the
// constitution we do NOT carry dual-read compat shims: these types declare a
// single camelCase convention, and `stores/pipelines.ts` normalizes the raw
// backend payload into these shapes exactly once at the gateway boundary.

// ---------------------------------------------------------------------------
// Lifecycle / status enums
// ---------------------------------------------------------------------------

// Node runtime status. Mirrors the backend `NodeStatus` enum exactly
// (crates/maenifold-core/src/workflow/pipeline/types.rs — serde lowercase):
// pending, ready, active, completed, skipped, abandoned, stuck, failed,
// retrying. `cancelled` is the backend serde alias for `abandoned`, so we
// accept it as a readable status too (the backend may emit either spelling).
export type NodeStatus =
  | 'pending'
  | 'ready'
  | 'active'
  | 'completed'
  | 'skipped'
  | 'abandoned'
  | 'cancelled'
  | 'stuck'
  | 'failed'
  | 'retrying';

// Pipeline (session) level status as emitted by the `view` payload's top-level
// `status` field. The backend either persists this (running/paused/cancelled/
// abandoned, admin force-complete) or computes it
// (compute_pipeline_status_with_definition → completed/stuck/active) and a fresh
// pipeline starts at pending; `retrying`/`failed` surface from in-flight runs.
// Only statuses the pipeline-level field can actually emit are listed here.
export type PipelineStatus =
  | 'pending'
  | 'active'
  | 'running'
  | 'paused'
  | 'cancelled'
  | 'completed'
  | 'abandoned'
  | 'stuck'
  | 'failed'
  | 'retrying';

export type PipelineNodeType = 'workflow' | 'prompt' | 'sequentialThinking' | 'tool';

// ---------------------------------------------------------------------------
// Edge / definition shapes (wire format sent to the backend)
// ---------------------------------------------------------------------------

export type EdgeType = 'success' | 'always' | 'conditional';
export type EdgeMode = 'context' | 'binding' | 'none';
export type EdgeConditionEquals = string | number | boolean;

// Fan-in join policy accepted on every node definition
// (join_policy_schema() → ["all","any","first"]).
export type JoinPolicy = 'all' | 'any' | 'first';

export interface EdgeBinding {
  from: string;
  to: string;
}

export interface PipelineEdge {
  source: string;
  target: string;
  // REQUIRED: the gateway rejects edges without a `type` discriminator (bare
  // {source,target} fails the schema oneOf even though the runtime would
  // otherwise default to `success`). Callers must always supply it.
  type: EdgeType;
  // Flat fields are the backend wire format for conditional edges.
  path?: string;
  equals?: EdgeConditionEquals;
  mode?: EdgeMode;
  bindings?: EdgeBinding[];
  mapping?: Record<string, string> | null;
}

export interface PipelineNodeAgentConfig {
  model?: string;
  mode?: string;
  thoughtLevel?: string;
  configOptions?: Record<string, unknown>;
}

export interface PipelineNodeAdopt {
  role?: string;
  color?: string;
  perspective?: string;
}

export interface SessionRef {
  agent: string;
  sessionId: string;
}

interface PipelineNodeBase {
  id: string;
  displayLabel?: string;
  // Advanced execution controls accepted on every node-definition variant
  // (crates/maenifold-core/src/mcp/schema/pipeline_node_types.rs: join_policy +
  // retry_* fields are present on all four node structs). Optional so simple
  // definitions stay terse while the UI can construct valid advanced ones.
  joinPolicy?: JoinPolicy;
  retryMaxAttempts?: number;
  retryBackoffMinMs?: number;
  retryBackoffMaxMs?: number;
  retryJitter?: boolean;
}

export interface WorkflowNode extends PipelineNodeBase {
  type?: 'workflow';
  workflowId: string;
  acpClient?: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  prompt?: string;
  cwd?: string;
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
  timeout?: number;
  inputSchema?: Record<string, unknown>;
  outputSchema?: Record<string, unknown>;
}

export interface PromptNode extends PipelineNodeBase {
  type: 'prompt';
  prompt: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  toolMode?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  sessionRef?: SessionRef;
  inputSchema?: Record<string, unknown>;
  outputSchema?: Record<string, unknown>;
}

export interface SequentialThinkingNode extends PipelineNodeBase {
  type: 'sequentialThinking';
  topic: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  totalThoughts?: number;
  analysisType?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  inputSchema?: Record<string, unknown>;
  outputSchema?: Record<string, unknown>;
}

export interface ToolNode extends PipelineNodeBase {
  type: 'tool';
  toolName: string;
  toolArgs?: Record<string, unknown>;
  agent?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  inputSchema?: Record<string, unknown>;
  outputSchema?: Record<string, unknown>;
}

export type PipelineNode = WorkflowNode | PromptNode | SequentialThinkingNode | ToolNode;

export type PipelineInputs = Record<string, unknown>;

export interface PipelineDefinition {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
  inputs?: PipelineInputs;
  inputSchema?: Record<string, unknown>;
  templateId?: string;
}

// ---------------------------------------------------------------------------
// Provenance (camelCase only — store normalizes lineage_* → lineage*)
// ---------------------------------------------------------------------------

export interface NodeAdoption {
  role: string;
  color: string;
  perspective: string;
}

export interface NodeProvenanceDetail {
  sessionId?: string | null;
  thinkingSessionId?: string | null;
  parentWorkflowId?: string | null;
  lineageGateTaskId?: string | null;
  lineagePipelineRunId?: string | null;
  lineageThinkingSessionId?: string | null;
  lineageMemoryNoteId?: string | null;
  adoption?: NodeAdoption;
}

export interface NodeProvenance {
  agent?: string | null;
  inspectable: boolean;
  detail: NodeProvenanceDetail;
}

export interface ToolProvenance {
  toolName: string;
  resolvedArgs?: Record<string, unknown> | null;
  responseSummary?: string | null;
}

// ---------------------------------------------------------------------------
// Cognitive surfaces (camelCase only — store normalizes snake_case once)
// ---------------------------------------------------------------------------

export interface WorkflowStepEntry {
  stepName: string;
  status: string;
  output?: string | null;
}

export interface ThinkingTraceEntry {
  thoughtNumber: number;
  totalThoughts?: number;
  text: string;
  isRevision?: boolean;
  revisesThought?: number | null;
  branchId?: string | null;
  agent?: string | null;
}

// ---------------------------------------------------------------------------
// View nodes (runtime state, returned by `{ sessionId, view: true }`)
// ---------------------------------------------------------------------------

interface ViewNodeBase {
  nodeId: string;
  displayLabel?: string;
  sessionId: string;
  acpSessionId?: string | null;
  thinkingSessionId?: string | null;
  provenance?: NodeProvenance;
  status: NodeStatus;
  attempts: number;
  output?: string | null;
  inputArtifact?: Record<string, unknown> | null;
  outputArtifact?: Record<string, unknown> | null;
  failureReason?: string | null;
  reconnected?: boolean;
  stale?: boolean;
  startedAt?: string | number | null;
  completedAt?: string | number | null;
}

export interface WorkflowViewNode extends ViewNodeBase {
  type?: 'workflow';
  workflowId: string;
  acpClient?: string | null;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  prompt?: string | null;
  cwd?: string | null;
  model?: string | null;
  thoughtLevel?: string | null;
  agentConfig?: PipelineNodeAgentConfig | null;
  timeout?: number | null;
  inputSchema?: Record<string, unknown> | null;
  outputSchema?: Record<string, unknown> | null;
  assembledPrompt?: string | null;
  workflowSteps?: WorkflowStepEntry[] | null;
}

export interface PromptViewNode extends ViewNodeBase {
  type: 'prompt';
  prompt: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  toolMode?: string | null;
  model?: string | null;
  thoughtLevel?: string | null;
  allowedTools?: string[] | null;
  maxTurns?: number | null;
  cwd?: string | null;
  timeout?: number | null;
  agentConfig?: PipelineNodeAgentConfig | null;
  sessionRef?: SessionRef;
  inputSchema?: Record<string, unknown> | null;
  outputSchema?: Record<string, unknown> | null;
  assembledPrompt?: string | null;
}

export interface SequentialThinkingViewNode extends ViewNodeBase {
  type: 'sequentialThinking';
  topic: string;
  agent: string;
  adopt?: PipelineNodeAdopt;
  requiresRole?: string;
  totalThoughts?: number | null;
  analysisType?: string | null;
  model?: string | null;
  thoughtLevel?: string | null;
  allowedTools?: string[] | null;
  maxTurns?: number | null;
  cwd?: string | null;
  timeout?: number | null;
  agentConfig?: PipelineNodeAgentConfig | null;
  inputSchema?: Record<string, unknown> | null;
  outputSchema?: Record<string, unknown> | null;
  assembledPrompt?: string | null;
  thinkingTrace?: ThinkingTraceEntry[] | null;
}

export interface ToolViewNode extends ViewNodeBase {
  type: 'tool';
  toolName: string;
  toolArgs?: Record<string, unknown> | null;
  toolProvenance?: ToolProvenance | null;
  agent?: string | null;
  model?: string | null;
  thoughtLevel?: string | null;
  allowedTools?: string[] | null;
  maxTurns?: number | null;
  cwd?: string | null;
  timeout?: number | null;
  agentConfig?: PipelineNodeAgentConfig | null;
  inputSchema?: Record<string, unknown> | null;
  outputSchema?: Record<string, unknown> | null;
}

export type ViewNode =
  | WorkflowViewNode
  | PromptViewNode
  | SequentialThinkingViewNode
  | ToolViewNode;

export interface PipelineView {
  pipelineId: string;
  name?: string | null;
  description?: string | null;
  status: PipelineStatus;
  // `final` is the backend's terminal-state flag (true once the run can no
  // longer make progress); distinct from `running` (a node is executing now).
  final?: boolean;
  templateId?: string;
  inputs?: PipelineInputs;
  inputSchema?: Record<string, unknown>;
  nodes: ViewNode[];
  edges: PipelineEdge[];
  ready: string[];
  active: string[];
  completed: string[];
  mermaid: string;
  running: boolean;
}

// ---------------------------------------------------------------------------
// List / sessions shapes (`{ list: true }` and `pipeline://sessions`)
// ---------------------------------------------------------------------------

export interface PipelineRunSummary {
  nodeId: string;
  startedAt: number | null;
  completedAt: number | null;
  status: string;
  outcome: string | null;
}

export interface PipelineListItem {
  pipelineId: string;
  name?: string | null;
  status: PipelineStatus;
  nodeCount: number;
  completedCount: number;
  description?: string | null;
  lastRunAt?: string | number | null;
  updatedAt?: string | number | null;
  modifiedAt?: string | number | null;
  runs: PipelineRunSummary[];
}

// Raw envelopes the backend returns (pre-normalization). Internal to the store.
export interface PipelineListResponse {
  pipelines?: PipelineListItem[];
}

export interface PipelineSessionsResource {
  sessions?: PipelineListItem[];
}

// ---------------------------------------------------------------------------
// Type guards
// ---------------------------------------------------------------------------

export function viewNodeType(node: ViewNode): PipelineNodeType {
  return node.type ?? 'workflow';
}

export function viewNodeDisplayLabel(node: ViewNode): string {
  return node.displayLabel ?? node.nodeId;
}

export function isWorkflowViewNode(node: ViewNode): node is WorkflowViewNode {
  return viewNodeType(node) === 'workflow';
}

export function isPromptViewNode(node: ViewNode): node is PromptViewNode {
  return node.type === 'prompt';
}

export function isSequentialThinkingViewNode(
  node: ViewNode
): node is SequentialThinkingViewNode {
  return node.type === 'sequentialThinking';
}

export function isToolViewNode(node: ViewNode): node is ToolViewNode {
  return node.type === 'tool';
}
