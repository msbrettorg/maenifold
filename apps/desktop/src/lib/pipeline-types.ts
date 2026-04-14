// Types for the Pipeline MCP tool

export type PipelineNodeType =
  | "workflow"
  | "prompt"
  | "sequentialThinking"
  | "tool";

export type EdgeType = "success" | "always";

export type NodeStatus =
  | "pending"
  | "ready"
  | "active"
  | "completed"
  | "abandoned"
  | "stuck"
  | "retrying";

export type PipelineStatus =
  | "active"
  | "paused"
  | "cancelled"
  | "completed"
  | "abandoned"
  | "stuck";

interface PipelineNodeBase {
  id: string;
  displayLabel?: string;
}

export interface PipelineNodeAgentConfig {
  model?: string;
  mode?: string;
  thoughtLevel?: string;
  configOptions?: Record<string, unknown>;
}

export interface WorkflowNode extends PipelineNodeBase {
  type?: "workflow";
  workflowId: string;
  acpClient?: string;
  prompt?: string;
  cwd?: string;
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
  timeout?: number;
}

export interface PromptNode extends PipelineNodeBase {
  type: "prompt";
  prompt: string;
  agent: string;
  toolMode?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export interface SequentialThinkingNode extends PipelineNodeBase {
  type: "sequentialThinking";
  topic: string;
  agent: string;
  totalThoughts?: number;
  analysisType?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export interface ToolNode extends PipelineNodeBase {
  type: "tool";
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
}

export type PipelineNode =
  | WorkflowNode
  | PromptNode
  | SequentialThinkingNode
  | ToolNode;
export type {
  PipelineNodeByType,
  PipelineNodeDraftByType,
  PipelineNodeInput,
  PipelineNodeInputByType,
  PromptNodeInput,
  SequentialThinkingNodeInput,
  ToolNodeInput,
  WorkflowNodeInput,
} from "./pipeline-node-factory";
export { createNode, createNodeId } from "./pipeline-node-factory";

export interface PipelineEdge {
  source: string;
  target: string;
  type?: EdgeType;
  mapping?: Record<string, string>;
}

export type PipelineInputs = Record<string, unknown>;

export interface PipelineDefinition {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
  inputs?: PipelineInputs;
  /**
   * Optional template identifier used to re-derive default human-readable
   * labels for pipelines instantiated from a template after backend
   * round-trips and app reloads.
   */
  templateId?: string;
}

interface ViewNodeBase {
  nodeId: string;
  displayLabel?: string;
  sessionId: string;
  acpSessionId?: string;
  thinkingSessionId?: string;
  provenance?: NodeProvenance;
  status: NodeStatus;
  attempts: number;
  output?: string;
}

export interface NodeProvenanceDetail {
  sessionId?: string | null;
  thinkingSessionId?: string | null;
  canReconnect: boolean;
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

export interface WorkflowStepEntry {
  step_name: string;
  status: string;
  output?: string | null;
}

export interface WorkflowViewNode extends ViewNodeBase {
  type?: "workflow";
  workflowId: string;
  acpClient?: string;
  prompt?: string;
  cwd?: string;
  model?: string;
  thoughtLevel?: string;
  agentConfig?: PipelineNodeAgentConfig;
  timeout?: number;
  assembled_prompt?: string | null;
  workflow_steps?: WorkflowStepEntry[] | null;
}

export interface PromptViewNode extends ViewNodeBase {
  type: "prompt";
  prompt: string;
  agent: string;
  toolMode?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  assembled_prompt?: string | null;
}

export interface SequentialThinkingViewNode extends ViewNodeBase {
  type: "sequentialThinking";
  topic: string;
  agent: string;
  totalThoughts?: number;
  analysisType?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
  assembled_prompt?: string | null;
  thinking_trace?: Array<{
    thought_number: number;
    text: string;
    status: string;
  }> | null;
}

export interface ToolViewNode extends ViewNodeBase {
  type: "tool";
  toolName: string;
  toolArgs?: Record<string, unknown>;
  toolProvenance?: ToolProvenance | null;
  agent?: string;
  model?: string;
  thoughtLevel?: string;
  allowedTools?: string[];
  maxTurns?: number;
  cwd?: string;
  timeout?: number;
  agentConfig?: PipelineNodeAgentConfig;
}

export type ViewNode =
  | WorkflowViewNode
  | PromptViewNode
  | SequentialThinkingViewNode
  | ToolViewNode;

export interface PipelineView {
  pipelineId: string;
  status: PipelineStatus;
  templateId?: string;
  inputs?: PipelineInputs;
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

const REASONABLE_CWD_PREFIX = /^(\/|~\/|\.{1,2}(?:\/|$)|[A-Za-z]:[\\/])/;

export function cwdHasReasonablePathPrefix(value: string): boolean {
  return REASONABLE_CWD_PREFIX.test(value);
}

export function normalizeNodeCwd(
  value: unknown,
  fieldName = "cwd",
): string | undefined {
  if (value == null) {
    return undefined;
  }

  if (typeof value !== "string") {
    throw new Error(`${fieldName} must be a string`);
  }

  const trimmedValue = value.trim();
  return trimmedValue.length > 0 ? trimmedValue : undefined;
}

export interface LogEntry {
  nodeId: string;
  from: string;
  to: string;
  timestamp: number;
  preview?: string;
}

export function getNodeType(node: PipelineNode): PipelineNodeType {
  return node.type ?? "workflow";
}

export function isWorkflowNode(node: PipelineNode): node is WorkflowNode {
  return getNodeType(node) === "workflow";
}

export function isPromptNode(node: PipelineNode): node is PromptNode {
  return getNodeType(node) === "prompt";
}

export function isSequentialThinkingNode(
  node: PipelineNode,
): node is SequentialThinkingNode {
  return getNodeType(node) === "sequentialThinking";
}

export function isToolNode(node: PipelineNode): node is ToolNode {
  return getNodeType(node) === "tool";
}

export function getPipelineNodeDisplayLabel(node: PipelineNode): string {
  return node.displayLabel ?? node.id;
}

export function getViewNodeDisplayLabel(node: ViewNode): string {
  return node.displayLabel ?? node.nodeId;
}

export function nodeLabel(node: PipelineNode): string {
  if (isWorkflowNode(node)) {
    return `wf: ${node.workflowId}`;
  }

  if (isPromptNode(node)) {
    const preview =
      node.prompt.length > 40 ? `${node.prompt.slice(0, 40)}...` : node.prompt;
    return `prompt: ${preview}`;
  }

  if (isToolNode(node)) {
    return node.toolName;
  }

  return `think: ${node.topic}`;
}
