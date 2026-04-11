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
  | "stuck";

export type PipelineStatus =
  | "active"
  | "paused"
  | "cancelled"
  | "completed"
  | "stuck";

interface PipelineNodeBase {
  id: string;
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

export interface PipelineDefinition {
  nodes: PipelineNode[];
  edges: PipelineEdge[];
  /**
   * Optional human-readable labels keyed by node id.
   * Populated by template instantiation; used by the flow layer and cards
   * so users see intent-first names instead of raw IDs.
   */
  nodeLabels?: Record<string, string>;
}

interface ViewNodeBase {
  nodeId: string;
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
