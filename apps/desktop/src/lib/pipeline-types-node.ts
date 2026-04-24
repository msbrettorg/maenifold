// Pipeline node type definitions and guards.

export type PipelineNodeType =
  | "workflow"
  | "prompt"
  | "sequentialThinking"
  | "tool";

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

export interface PipelineNodeAdopt {
  role?: string;
  color?: string;
  perspective?: string;
}

export interface SessionRef {
  agent: string;
  sessionId: string;
}

export interface WorkflowNode extends PipelineNodeBase {
  type?: "workflow";
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
}

export interface PromptNode extends PipelineNodeBase {
  type: "prompt";
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
}

export interface SequentialThinkingNode extends PipelineNodeBase {
  type: "sequentialThinking";
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
